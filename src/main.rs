//! Aero-Sync — Universal ASUS RGB Engine 
//! Professional Distribution Version v1.1.0

use anyhow::{Context, Result};
use ashpd::desktop::screencast::{
    CursorMode, Screencast, SelectSourcesOptions, 
    StartCastOptions, OpenPipeWireRemoteOptions
};
use ashpd::desktop::PersistMode;
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use std::fs;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{Ordering, AtomicBool};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::signal;
use zbus::proxy;

// ─── UNIVERSAL ASUS AURA PROXY ───────────────────────────────────────────────

#[proxy(
    default_service = "xyz.ljones.Asusd",
    interface = "xyz.ljones.Aura"
)]
pub trait AsusAura {
    #[zbus(property)]
    fn brightness(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn set_brightness(&self, value: u32) -> zbus::Result<()>;
    #[zbus(property)]
    fn set_led_mode_data(&self, value: (u32, u32, (u8, u8, u8), (u8, u8, u8), String, String)) -> zbus::Result<()>;
}

// ─── PERCEPTUAL COLOR ENGINE (Oklab) ──────────────────────────────────────────

static SRGB_LUT: OnceLock<[f32; 256]> = OnceLock::new();

fn get_srgb_lut() -> &'static [f32; 256] {
    SRGB_LUT.get_or_init(|| {
        let mut lut = [0.0f32; 256];
        for i in 0..256 {
            let v = i as f32 / 255.0;
            lut[i] = if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
        }
        lut
    })
}

#[derive(Default)]
struct AtomicColor {
    r: std::sync::atomic::AtomicU8,
    g: std::sync::atomic::AtomicU8,
    b: std::sync::atomic::AtomicU8,
}

impl AtomicColor {
    fn store(&self, r: u8, g: u8, b: u8) {
        self.r.store(r, Ordering::Relaxed);
        self.g.store(g, Ordering::Relaxed);
        self.b.store(b, Ordering::Relaxed);
    }
    fn load(&self) -> (u8, u8, u8) {
        (self.r.load(Ordering::Relaxed), self.g.load(Ordering::Relaxed), self.b.load(Ordering::Relaxed))
    }
}

// ─── MAIN ENGINE ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    println!("--------------------------------------------");
    println!("   AERO-SYNC: UNIVERSAL ASUS RGB ENGINE     ");
    println!("   Version 1.1.0 - Sovereign Distribution   ");
    println!("--------------------------------------------");

    // --- STEP 1: HARDWARE DISCOVERY ---
    let conn = zbus::Connection::system().await.context("Failed to connect to system bus")?;
    let aura_paths = vec!["/xyz/ljones/aura/tuf", "/xyz/ljones/aura/rog", "/xyz/ljones/aura/aura"];
    let mut active_aura = None;

    for path in aura_paths {
        if let Ok(proxy) = AsusAuraProxy::builder(&conn).path(path)?.build().await {
            if proxy.brightness().await.is_ok() {
                println!("[INFO] Found ASUS Aura controller at {}", path);
                active_aura = Some(proxy);
                break;
            }
        }
    }

    let aura = active_aura.context("[ERROR] No compatible ASUS controller found")?;
    
    // --- STEP 2: INTEGRITY CHECK ---
    gst::init()?;
    for el in ["pipewiresrc", "videoconvert", "appsink", "vapostproc", "videoscale"] {
        if gst::ElementFactory::find(el).is_none() {
            println!("[WARN] GStreamer element '{}' not found", el);
        }
    }

    // --- STEP 3: WAYLAND HANDSHAKE ---
    let screencast = Screencast::new().await?;
    let session = screencast.create_session(Default::default()).await?;
    
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let token_path = format!("{}/.cache/aero_sync_token", home);
    let _ = fs::create_dir_all(format!("{}/.cache", home));

    let mut select_options = SelectSourcesOptions::default()
        .set_cursor_mode(CursorMode::Hidden)
        .set_persist_mode(PersistMode::ExplicitlyRevoked);
    
    if let Ok(t) = fs::read_to_string(&token_path) { 
        select_options = select_options.set_restore_token(Some(t.as_str()));
    }
    
    println!("[INFO] Initializing Wayland screen-cast session...");
    let _ = screencast.select_sources(&session, select_options).await?;
    let start_response = screencast.start(&session, None, StartCastOptions::default()).await?;
    let start_data = start_response.response()?;
    
    if let Some(token) = start_data.restore_token() {
        let _ = fs::write(&token_path, token);
    }
    
    let stream_node = start_data.streams().first().context("[ERROR] Failed to obtain stream node")?.pipe_wire_node_id();
    let fd = screencast.open_pipe_wire_remote(&session, OpenPipeWireRemoteOptions::default()).await?;

    let is_moving = Arc::new(AtomicBool::new(true));
    let target_color = Arc::new(AtomicColor::default());
    let shutdown = Arc::new(AtomicBool::new(false));
    
    let beat_color = Arc::clone(&target_color);
    let beat_aura = aura.clone();
    let beat_moving = Arc::clone(&is_moving);
    let beat_shutdown = Arc::clone(&shutdown);

    // --- HEARTBEAT SMOOTHING ---
    tokio::spawn(async move {
        let mut cr = 0u32; let mut cg = 0u32; let mut cb = 0u32;
        let mut last_kb_color = (0u8, 0u8, 0u8);
        loop {
            if beat_shutdown.load(Ordering::Relaxed) { break; }
            let tick_start = Instant::now();
            let moving = beat_moving.load(Ordering::Relaxed);
            let (sr, sg, sb) = beat_color.load();
            let (tr, tg, tb) = (sr as u32, sg as u32, sb as u32);
            
            cr = ((cr * 210) + (tr * 46)) >> 8;
            cg = ((cg * 210) + (tg * 46)) >> 8;
            cb = ((cb * 210) + (tb * 46)) >> 8;
            
            let out = (cr as u8, cg as u8, cb as u8);
            if (out.0 as i16 - last_kb_color.0 as i16).abs() > 1 || (out.1 as i16 - last_kb_color.1 as i16).abs() > 1 || (out.2 as i16 - last_kb_color.2 as i16).abs() > 1 {
                let _ = beat_aura.set_led_mode_data((0, 0, out, out, "Med".to_string(), "Right".to_string())).await;
                last_kb_color = out;
            }
            let hz = if moving { 60.0 } else { 5.0 }; 
            tokio::time::sleep(Duration::from_secs_f32(1.0 / hz).saturating_sub(tick_start.elapsed())).await;
        }
    });

    // --- MULTI-TARGET PIPELINE ---
    let nvd_str = format!("pipewiresrc fd={} path={} ! nvvideoconvert ! video/x-raw,width=16,height=16,format=RGB ! appsink name=sink sync=false drop=true max-buffers=1", fd.as_raw_fd(), stream_node);
    let va_str = format!("pipewiresrc fd={} path={} ! vapostproc ! video/x-raw,width=16,height=16 ! videoconvert ! video/x-raw,format=RGB ! appsink name=sink sync=false drop=true max-buffers=1", fd.as_raw_fd(), stream_node);
    let sw_str = format!("pipewiresrc fd={} path={} ! videoconvert ! videoscale ! video/x-raw,width=16,height=16,format=RGB ! appsink name=sink sync=false drop=true max-buffers=1", fd.as_raw_fd(), stream_node);

    let pipeline = if let Ok(p) = gst::parse::launch(&nvd_str) {
        println!("[INFO] Target acceleration: NVIDIA/NVMM"); p
    } else if let Ok(p) = gst::parse::launch(&va_str) {
        println!("[INFO] Target acceleration: VA-API"); p
    } else {
        println!("[INFO] Target acceleration: Software (Fallback)");
        gst::parse::launch(&sw_str).context("[ERROR] Pipeline initialization failed")?
    };

    let bin = pipeline.clone().dynamic_cast::<gst::Bin>().unwrap();
    let sink = bin.by_name("sink").unwrap().dynamic_cast::<gst_app::AppSink>().unwrap();
    let _ = pipeline.set_state(gst::State::Playing);

    println!("[INFO] RGB synchronization active");

    let mut last_tc = (0u8, 0u8, 0u8);
    let mut idle_frames = 0u32;
    let s_aura = aura.clone();

    tokio::select! {
        _ = async {
            loop {
                let frame_time = Duration::from_micros(1_000_000 / 60);
                let pulse_start = Instant::now();
                
                if let Some(sample) = sink.try_pull_sample(gst::ClockTime::from_mseconds(10)) {
                    if let Some(buffer) = sample.buffer() {
                        if let Ok(map) = buffer.map_readable() {
                            let tc = get_perceptual_color(map.as_slice());
                            if (tc.0 as i16 - last_tc.0 as i16).abs() > 2 || (tc.1 as i16 - last_tc.1 as i16).abs() > 2 || (tc.2 as i16 - last_tc.2 as i16).abs() > 2 {
                                idle_frames = 0; last_tc = tc;
                                is_moving.store(true, Ordering::Relaxed);
                                target_color.store(tc.0, tc.1, tc.2);
                            } else {
                                idle_frames += 1;
                                if idle_frames > 60 { is_moving.store(false, Ordering::Relaxed); }
                            }
                        }
                    }
                }
                tokio::time::sleep(frame_time.saturating_sub(pulse_start.elapsed())).await;
            }
        } => {},
        _ = signal::ctrl_c() => {
            println!("\n[INFO] Shutdown signal received: Reseting state");
            shutdown.store(true, Ordering::SeqCst);
            let _ = pipeline.set_state(gst::State::Null);
            let _ = s_aura.set_led_mode_data((0, 0, (255,255,255), (255,255,255), "Med".to_string(), "Right".to_string())).await;
        }
    }

    Ok(())
}

fn get_perceptual_color(raw: &[u8]) -> (u8, u8, u8) {
    let n = (raw.len() / 3) as f32;
    if n == 0.0 { return (0, 0, 0); }

    let lut = get_srgb_lut();
    let mut r_lin = 0.0f32; let mut g_lin = 0.0f32; let mut b_lin = 0.0f32;

    for chunk in raw.chunks_exact(3) {
        r_lin += lut[chunk[0] as usize];
        g_lin += lut[chunk[1] as usize];
        b_lin += lut[chunk[2] as usize];
    }

    let r_avg = r_lin / n; let g_avg = g_lin / n; let b_avg = b_lin / n;

    let l = 0.81893 * r_avg + 0.36186 * g_avg - 0.12885 * b_avg;
    let m = 0.03298 * r_avg + 0.92931 * g_avg + 0.03614 * b_avg;
    let s = 0.04820 * r_avg + 0.26436 * g_avg + 0.63385 * b_avg;

    let l_ = l.max(0.0).cbrt(); let m_ = m.max(0.0).cbrt(); let s_ = s.max(0.0).cbrt();

    let lab_l = 0.21045 * l_ + 0.79361 * m_ - 0.00407 * s_;
    let mut lab_a = 1.97799 * l_ - 2.42859 * m_ + 0.45059 * s_;
    let mut lab_b = 0.02590 * l_ + 0.78277 * m_ - 0.80867 * s_;

    let chroma = (lab_a * lab_a + lab_b * lab_b).sqrt();
    if chroma > 0.0 {
        let scale = (0.2 / (chroma + 0.01)).min(1.8) + 1.1;
        lab_a *= scale; lab_b *= scale;
    }

    if lab_l > 0.7 && chroma < 0.05 {
        if r_avg > g_avg && r_avg > b_avg { lab_a += 0.1; }
        else if b_avg > r_avg && b_avg > g_avg { lab_b -= 0.1; }
    }

    let l_back = lab_l + 0.3963 * lab_a + 0.2158 * lab_b;
    let m_back = lab_l - 0.1055 * lab_a - 0.0638 * lab_b;
    let s_back = lab_l - 0.0894 * lab_a - 1.2914 * lab_b;

    let r_f = (1.2270 * l_back.powi(3) - 0.5577 * m_back.powi(3) + 0.2812 * s_back.powi(3)).clamp(0.0, 1.0);
    let g_f = (-0.0405 * l_back.powi(3) + 1.1122 * m_back.powi(3) - 0.0716 * s_back.powi(3)).clamp(0.0, 1.0);
    let b_f = (-0.0763 * l_back.powi(3) - 0.4214 * m_back.powi(3) + 1.5861 * s_back.powi(3)).clamp(0.0, 1.0);

    let d = |v: f32| if v <= 0.0031308 { v * 12.92 } else { 1.055 * v.powf(1.0/2.4) - 0.055 };
    ((d(r_f) * 255.0) as u8, (d(g_f) * 255.0) as u8, (d(b_f) * 255.0) as u8)
}

//! Aero-Sync — The Sovereign RGB Engine 🏛️🛰️🦾
//! Universal "Plug-and-Play" Architecture for the Arch Linux Community.

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
use std::sync::atomic::{AtomicU32, Ordering, AtomicBool};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::time;
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

struct AtomicColor {
    rgb: AtomicU32,
}

impl AtomicColor {
    fn new() -> Self { Self { rgb: AtomicU32::new(0) } }
    fn store(&self, r: u8, g: u8, b: u8) {
        self.rgb.store((r as u32) << 16 | (g as u32) << 8 | b as u32, Ordering::Relaxed);
    }
    fn load(&self) -> (u8, u8, u8) {
        let v = self.rgb.load(Ordering::Relaxed);
        ((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }
}

// ─── MAIN ENGINE ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════╗");
    println!("║     Aero-Sync v1.1.0 Sovereign 🏛️        ║");
    println!("║     Universal ASUS RGB Master          ║");
    println!("╚══════════════════════════════════════════╝");

    // --- 🔍 STEP 1: HARDWARE DISCOVERY (THE HUNT) ---
    println!("🔍 Probing ASUS Hardware...");
    let conn = zbus::Connection::system().await.context("Cannot connect to System Bus (asusd required)")?;
    
    let aura_paths = vec!["/xyz/ljones/aura/tuf", "/xyz/ljones/aura/rog", "/xyz/ljones/aura/aura"];
    let mut active_aura = None;

    for path in aura_paths {
        if let Ok(proxy) = AsusAuraProxy::builder(&conn).path(path)?.build().await {
            if proxy.brightness().await.is_ok() {
                println!("✅ Found ASUS Controller: {}", path);
                active_aura = Some(proxy);
                break;
            }
        }
    }

    let aura = active_aura.context("❌ NO ASUS KEYBOARD DETECTED. Make sure asusctl is running.")?;
    
    // --- 🩺 STEP 2: INTEGRITY CHECK ---
    gst::init()?;
    let required_elements = vec!["pipewiresrc", "videoconvert", "appsink"];
    for el in required_elements {
        if gst::ElementFactory::find(el).is_none() {
            println!("❌ MISSING DEPENDENCY: {}", el);
            println!("   Please install 'gst-plugin-pipewire' and 'gst-plugins-base'.");
            return Ok(());
        }
    }

    // --- 🛰️ STEP 3: WAYLAND HANDSHAKE ---
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
    
    println!("📡 Handshaking with Wayland Portal...");
    let _ = screencast.select_sources(&session, select_options).await?;
    let start_response = screencast.start(&session, None, StartCastOptions::default()).await?;
    let start_data = start_response.response()?;
    
    if let Some(token) = start_data.restore_token() {
        let _ = fs::write(&token_path, token);
    }
    
    let stream_node = start_data.streams().first().context("No streams")?.pipe_wire_node_id();
    let fd = screencast.open_pipe_wire_remote(&session, OpenPipeWireRemoteOptions::default()).await?;

    let is_moving = Arc::new(AtomicBool::new(true));
    let target_color = Arc::new(AtomicColor::new());
    
    let beat_color = Arc::clone(&target_color);
    let beat_aura = aura.clone();
    let beat_moving = Arc::clone(&is_moving);

    // --- 💓 HEARTBEAT SMOOTHING ---
    tokio::spawn(async move {
        let mut cr = 0u32; let mut cg = 0u32; let mut cb = 0u32;
        let mut last_kb_color = (0u8, 0u8, 0u8);
        loop {
            let tick_start = Instant::now();
            let moving = beat_moving.load(Ordering::Relaxed);
            let (sr, sg, sb) = beat_color.load();
            let (tr, tg, tb) = (sr as u32, sg as u32, sb as u32);
            
            // Exponential Smoothing (Immortal 42.0)
            cr = ((cr * 218) + (tr * 38)) >> 8;
            cg = ((cg * 218) + (tg * 38)) >> 8;
            cb = ((cb * 218) + (tb * 38)) >> 8;
            
            let r_u8 = cr as u8; let g_u8 = cg as u8; let b_u8 = cb as u8;
            if (r_u8 as i16 - last_kb_color.0 as i16).abs() > 1 || (g_u8 as i16 - last_kb_color.1 as i16).abs() > 1 || (b_u8 as i16 - last_kb_color.2 as i16).abs() > 1 {
                let out = (r_u8, g_u8, b_u8);
                let _ = beat_aura.set_led_mode_data((0, 0, out, out, "Med".to_string(), "Right".to_string())).await;
                last_kb_color = out;
            }
            let hz = if moving { 60.0 } else { 5.0 }; 
            tokio::time::sleep(Duration::from_secs_f32(1.0 / hz).saturating_sub(tick_start.elapsed())).await;
        }
    });

    // --- 🏎️ TRIPLE-FALLBACK PIPELINE (Universal) ---
    let nvd_str = format!("pipewiresrc fd={} path={} ! nvvideoconvert ! video/x-raw,width=16,height=16,format=RGB ! appsink name=sink sync=false drop=true max-buffers=1", fd.as_raw_fd(), stream_node);
    let intel_str = format!("pipewiresrc fd={} path={} ! vapostproc ! video/x-raw,width=16,height=16 ! videoconvert ! video/x-raw,format=RGB ! appsink name=sink sync=false drop=true max-buffers=1", fd.as_raw_fd(), stream_node);
    let soft_str = format!("pipewiresrc fd={} path={} ! videoconvert ! video/x-raw,width=16,height=16,format=RGB ! appsink name=sink sync=false drop=true max-buffers=1", fd.as_raw_fd(), stream_node);

    let pipeline = if let Ok(p) = gst::parse::launch(&nvd_str) {
        println!("🔱 Titan GPU Acceleration ENGAGED.");
        p
    } else if let Ok(p) = gst::parse::launch(&intel_str) {
        println!("🕊️ Phoenix VA-API Acceleration ENGAGED.");
        p
    } else {
        println!("🕯️ Software Fallback ACTIVE (Universal Mode).");
        gst::parse::launch(&soft_str).context("Critical GStreamer Failure")?
    };

    let bus = pipeline.bus().unwrap();
    tokio::spawn(async move {
        for msg in bus.iter_timed(gst::ClockTime::NONE) {
            use gst::MessageView;
            match msg.view() {
                MessageView::Error(err) => { eprintln!("❌ GStreamer Error: {}", err.error()); }
                _ => (),
            }
        }
    });

    let bin = pipeline.clone().dynamic_cast::<gst::Bin>().unwrap();
    let sink = bin.by_name("sink").unwrap().dynamic_cast::<gst_app::AppSink>().unwrap();
    let _ = pipeline.set_state(gst::State::Playing);

    println!("🏎️ Zero-Wattage Engine ACTIVE. Syncing...");

    let mut last_tc = (0u8, 0u8, 0u8);
    let mut idle_frames = 0u32;

    loop {
        let frame_time = Duration::from_micros(1_000_000 / 60);
        let pulse_start = Instant::now();
        
        if let Some(sample) = sink.try_pull_sample(gst::ClockTime::from_mseconds(10)) {
            if let Some(buffer) = sample.buffer() {
                if let Ok(map) = buffer.map_readable() {
                    let tc = get_perceptual_color_integer(map.as_slice());
                    if (tc.0 as i16 - last_tc.0 as i16).abs() > 2 || (tc.1 as i16 - last_tc.1 as i16).abs() > 2 || (tc.2 as i16 - last_tc.2 as i16).abs() > 2 {
                        idle_frames = 0;
                        last_tc = tc;
                        is_moving.store(true, Ordering::Relaxed);
                        target_color.store(tc.0, tc.1, tc.2);
                    } else {
                        idle_frames += 1;
                        if idle_frames > 60 { is_moving.store(false, Ordering::Relaxed); }
                    }
                }
            }
        }

        let elapsed = pulse_start.elapsed();
        if let Some(sleep_dur) = frame_time.checked_sub(elapsed) {
            time::sleep(sleep_dur).await;
        }
    }
}

fn get_perceptual_color_integer(raw: &[u8]) -> (u8, u8, u8) {
    let n = (raw.len() / 3) as f32;
    if n == 0.0 { return (0, 0, 0); }

    let lut = get_srgb_lut();
    let mut r_lin_sum = 0.0f32;
    let mut g_lin_sum = 0.0f32;
    let mut b_lin_sum = 0.0f32;

    for chunk in raw.chunks_exact(3) {
        r_lin_sum += lut[chunk[0] as usize];
        g_lin_sum += lut[chunk[1] as usize];
        b_lin_sum += lut[chunk[2] as usize];
    }

    let r_avg = r_lin_sum / n;
    let g_avg = g_lin_sum / n;
    let b_avg = b_lin_sum / n;

    // LMS -> Oklab
    let l = 0.8189330101 * r_avg + 0.3618667424 * g_avg - 0.1288597137 * b_avg;
    let m = 0.0329845436 * r_avg + 0.9293118715 * g_avg + 0.0361456387 * b_avg;
    let s = 0.0482003018 * r_avg + 0.2643662691 * g_avg + 0.6338517070 * b_avg;

    let l_ = l.max(0.0).cbrt();
    let m_ = m.max(0.0).cbrt();
    let s_ = s.max(0.0).cbrt();

    let lab_l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let mut lab_a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let mut lab_b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    // Chroma Boost
    let chroma = (lab_a * lab_a + lab_b * lab_b).sqrt();
    if chroma > 0.0 {
        let scale = (0.2 / (chroma + 0.01)).min(1.8) + 1.1;
        lab_a *= scale;
        lab_b *= scale;
    }

    // White-Wash Protection
    if lab_l > 0.7 && chroma < 0.05 {
        if r_avg > g_avg && r_avg > b_avg { lab_a += 0.1; }
        else if g_avg > r_avg && g_avg > b_avg { lab_a -= 0.05; lab_b += 0.05; }
        else if b_avg > r_avg && b_avg > g_avg { lab_b -= 0.1; }
    }

    // Oklab -> LMS -> Linear RGB
    let l_back = lab_l + 0.39633779 * lab_a + 0.21580376 * lab_b;
    let m_back = lab_l - 0.10556134 * lab_a - 0.06385417 * lab_b;
    let s_back = lab_l - 0.08948418 * lab_a - 1.29148554 * lab_b;

    let l_f = l_back.powi(3);
    let m_f = m_back.powi(3);
    let s_f = s_back.powi(3);

    let mut r_f = 1.22701385 * l_f - 0.55779998 * m_f + 0.28125615 * s_f;
    let mut g_f = -0.04058018 * l_f + 1.11225687 * m_f - 0.07167668 * s_f;
    let mut b_f = -0.07638128 * l_f - 0.42148198 * m_f + 1.58616322 * s_f;

    r_f = r_f.clamp(0.0, 1.0);
    g_f = g_f.clamp(0.0, 1.0);
    b_f = b_f.clamp(0.0, 1.0);

    // De-linearize (sRGB)
    let r_s = if r_f <= 0.0031308 { r_f * 12.92 } else { 1.055 * r_f.powf(1.0/2.4) - 0.055 };
    let g_s = if g_f <= 0.0031308 { g_f * 12.92 } else { 1.055 * g_f.powf(1.0/2.4) - 0.055 };
    let b_s = if b_f <= 0.0031308 { b_f * 12.92 } else { 1.055 * b_f.powf(1.0/2.4) - 0.055 };

    ((r_s * 255.0) as u8, (g_s * 255.0) as u8, (b_s * 255.0) as u8)
}

# Aero-Sync 🏎️🛡️
![Arch Linux](https://img.shields.io/badge/Arch%20Linux-1793D1?style=for-the-badge&logo=arch-linux&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![AUR Package](https://img.shields.io/aur/version/aero-sync-git?style=for-the-badge&color=blue&label=AUR)
![License](https://img.shields.io/github/license/sathwik4444/aerosync?style=for-the-badge&color=green)

**Sovereign RGB Synchronization for ASUS TUF/ROG Laptops.**

Aero-Sync is a high-performance, zero-wattage background engine that synchronizes your ASUS keyboard lighting with your screen colors in real-time. Built in Rust with a GStreamer/VA-API pipeline for ultra-low latency.

## 🏛️ Features
- **Triple-Fallback Pipeline**: Automatic selection of NVIDIA (NVMM), Intel/AMD (VA-API), or Software rendering.
- **Perceptual Color Engine**: Uses Oklab color space for fluid, natural lighting transitions.
- **Sovereign Architecture**: Standalone binary with no external daemon dependencies.
- **Wayland Native**: Designed specifically for modern Wayland sessions (GNOME/KDE).

## 🚀 Installation (Arch Linux)
The easiest way is via the AUR:
```bash
yay -S aero-sync-git
```

## 🛠️ Usage

### Manual Start
To start the synchronization engine manually:
```bash
aero-sync
```

### Background Service (Recommended)
Aero-Sync includes a Systemd user service for seamless background operation.

**To start it now:**
```bash
systemctl --user enable --now aero-sync
```

**To stop it:**
```bash
systemctl --user stop aero-sync
```

## 🛡️ License
Distributed under the MIT License. See `LICENSE` for more information.

## Requirements

Aero-Sync currently relies on the ASUS Linux ecosystem and requires ASUS Aura support available through tools/interfaces such as `asusctl`.

At the moment, the project has mainly been tested on an ASUS TUF F15 (single-zone RGB keyboard) running Arch Linux + Wayland.


## 🤝 Acknowledgments
- Inspired by the ASUS Linux community.
- Built with GStreamer and ASHPD.


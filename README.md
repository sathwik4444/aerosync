# Aero-Sync 🏎️🛡️

![Arch Linux](https://img.shields.io/badge/Arch%20Linux-1793D1?style=for-the-badge\&logo=arch-linux\&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge\&logo=rust\&logoColor=white)
![AUR Package](https://img.shields.io/aur/version/aero-sync-git?style=for-the-badge\&color=blue\&label=AUR)
![License](https://img.shields.io/github/license/sathwik4444/aerosync?style=for-the-badge\&color=green)

> Lightweight hardware-accelerated RGB synchronization for ASUS TUF/ROG laptops on Linux.

Aero-Sync is a Rust-based RGB synchronization engine that synchronizes ASUS keyboard lighting with on-screen colors in real time using a hardware-accelerated Wayland pipeline.

Built with:

* Rust
* GStreamer
* VA-API / NVMM
* Oklab color processing
* ASUS Linux ecosystem (`asusctl/asusd`)

Originally developed and tested on an ASUS TUF F15 running Arch Linux + Wayland.

---

# 🚀 Quick Install (Arch Linux)

Install directly from the AUR:

```bash
yay -S aero-sync-git
```

AUR Package:

* https://aur.archlinux.org/packages/aero-sync-git

---

# ✨ Features

* Hardware-accelerated screen processing

* Automatic backend selection:

  * NVIDIA (NVMM)
  * Intel/AMD (VA-API)
  * Software fallback

* Oklab perceptual color processing

* Smooth real-time RGB synchronization

* Adaptive refresh logic for lower idle overhead

* Wayland-native architecture

* ASUS Aura integration through `asusctl/asusd`

* Lightweight background execution

* Arch Linux AUR package available

---

# ✅ Supported Devices

### Tested

* ASUS TUF F15 FX507VV (single-zone RGB)

### Untested / Experimental

* ASUS ROG series
* 4-zone RGB keyboards
* Per-key RGB keyboards

Feedback and compatibility reports are highly appreciated.

---

# ⚠️ Requirements

Aero-Sync currently relies on the ASUS Linux ecosystem and requires:

* `asusctl`
* `asusd`
* Wayland session
* GStreamer

Compatible or potentially compatible distributions include:

* Arch Linux
* Fedora
* Ubuntu (22.04+)
* Debian-based distributions
* Pop!_OS
* Linux Mint

---

# 🛠️ Manual Build (Non-Arch Distros)

For non-Arch Linux distributions:

## Clone the repository

```bash
git clone https://github.com/sathwik4444/aerosync.git
cd aerosync
```

## Build with Cargo

```bash
cargo build --release
```

## Run manually

```bash
./target/release/aero-sync
```

Before running:

* Ensure `asusctl/asusd` is installed
* Ensure Wayland is being used
* Ensure GStreamer dependencies are available

---

# ⚙️ Usage

## Manual Start

```bash
aero-sync
```

## Background Service (Recommended)

Enable and start:

```bash
systemctl --user enable --now aero-sync
```

Stop:

```bash
systemctl --user stop aero-sync
```

Disable:

```bash
systemctl --user disable aero-sync
```

---

# ⚙️ How It Works

Aero-Sync captures screen colors using Wayland screen capture APIs, processes them using Oklab color conversion, and sends synchronized lighting updates through ASUS Aura interfaces.

The synchronization pipeline dynamically selects the best available acceleration backend to improve responsiveness and reduce CPU overhead.

---

# ⚠️ Known Limitations

* Mainly tested on ASUS TUF F15
* Compatibility with all ASUS RGB layouts is not yet fully verified
* Wayland-focused currently
* Requires ASUS Aura support

---

# 🤝 Contributing

Feedback, testing, bug reports, and pull requests are welcome.

If you test Aero-Sync on other ASUS TUF/ROG devices, feel free to share:

* compatibility results
* performance metrics
* bugs/issues
* RGB behavior differences

GitHub Issues:

* https://github.com/sathwik4444/aerosync/issues

---

# 🔗 Project Links

* GitHub Repository:

  * https://github.com/sathwik4444/aerosync

* AUR Package:

  * https://aur.archlinux.org/packages/aero-sync-git

* ASUS Linux:

  * https://asus-linux.org/

---

# 🛡️ License

Distributed under the MIT License.

---

# 🙏 Acknowledgments

* ASUS Linux community
* GStreamer
* ASHPD
* Rust ecosystem
* Arch Linux community

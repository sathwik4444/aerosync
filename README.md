# 🏛️ Aero-Sync: The Immortal RGB Engine

**Aero-Sync** is a high-performance, standalone RGB synchronization engine for ASUS TUF Gaming laptops running Linux. 

Built with **Rust**, it utilizes the most advanced perceptual color math available today (**Oklab**) combined with **Zero-Wattage** optimizations to provide a visually stunning and energy-efficient experience.

## ✨ Key Features

*   **🧪 Oklab Perceptual Math**: Colors match what your eyes see, not just raw pixel averages.
*   **🏎️ Zero-Wattage LUT**: Uses a pre-computed sRGB Lookup Table to eliminate heavy CPU calculations in the pixel loop.
*   **🧪 DMA-Stealth Architecture**: Zero-copy screen capture via PipeWire and DMA-BUF (Wayland native).
*   **🕊️ Silk-Smooth Smoothing**: High-frequency exponential smoothing for fluid color transitions.
*   **🛡️ White-Wash Protection**: Prevents "graying out" in bright scenes, maintaining vibrant hues.

## 🚀 Installation (Arch Linux)

Requires `asusctl` to be installed and running.

```bash
# Install dependencies
sudo pacman -S gstreamer gst-plugins-base gst-plugins-good gst-plugin-pipewire

# Build the project
cargo build --release

# Run the engine
./target/release/aero-sync
```

## 🛡️ License

Built for the Arch Linux Community by **aero**. Inspired by the vision of Absolute Perfection.

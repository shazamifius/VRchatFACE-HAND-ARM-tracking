# 🎭 VRChat Bridge Hub (V2)
>
> **The Ultimate Face & Hand Tracking Solution**

[![Rust](https://img.shields.io/badge/Built_with-Rust_1.80+-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/UI_Frame-Tauri_v2-blue?style=for-the-badge&logo=tauri)](https://tauri.app/)
[![VRChat](https://img.shields.io/badge/Works_with-VRChat_OSC-MediumPurple?style=for-the-badge)](https://vrchat.com)
[![Status](https://img.shields.io/badge/Status-Stable-success?style=for-the-badge)](#)

**Transform your Phone into a High-End Face Tracker. Zero Install. Instant Connect.**

---

## ✨ Why This Project?

🚀 **Native Performance**
Forget clunky Python scripts. The core engine is rewritten in **Rust**, delivering lightning-fast inference (<10ms latency) with minimal CPU usage.

📱 **Phone Camera Magic**
Don't have a professional webcam? Your iPhone or Android is better.
Simply scan the QR code to stream HD video directly to the engine. No app instllation required.

🔒 **Privacy First**
Everything runs **locally** on your PC. Your video feed never touches a remote server (unless you explicitly enable Cloudflare Tunnel for remote connections).

🎨 **Beautiful "Glass" UI**
A modern, dark-themed interface built with Tauri that gives you full control over your tracking parameters without looking like a developer console.

---

## 🚀 Getting Started in 2 Minutes

### 1. Simple Setup

Double-click `START_PROJECT.bat`.
*The script will automatically check for Rust and Cloudflare, and install them if missing.*

### 2. Launch

Select **Option 1** ("Full Start") from the menu.
*The Hub will launch and generate a local QR code.*

### 3. Connect Phone

Scan the QR code with your phone's camera app.
*Your phone will instantly start streaming video to the Hub.*

### 4. VRChat Ready

Open your VRChat Radial Menu -> **Options** -> **OSC** -> **Enabled**.
*Your avatar will immediately start mimicking your face and hands!*

---

## 🛠️ Advanced Features

### 📡 Cloudflare Tunnel (Remote Mode)

Network firewall blocking local connections? The Hub can automatically create a secure Cloudflare Tunnel.

- Select **"Use Cloudflare"** on the phone interface if local Wi-Fi fails.
- Works even if your PC and Phone are on different networks (e.g., 4G vs Wi-Fi).

### ⚡ Smart Bandwidth

The engine automatically adjusts video quality based on your network speed to prevent lag and flickering.

- **Auto-Throttle**: Drops frames cleanly instead of glitching.
- **Anti-Epilepsy**: Stable video rendering preventing white flashes.

### 🧠 Hybrid AI Engine

- **Face**: 468-point High-Density Mesh (BlazeFace Short Range).
- **Hands**: 21-point Skeleton Tracking per hand.
- **Solver**: Custom Rust solver converting raw points to VRChat Parameters (JawOpen, EyeBlink, etc.).

---

## 🔧 Troubleshooting

| Problem | Solution |
| :--- | :--- |
| **"OSC Not Working"** | Ensure VRChat is running and OSC is enabled in the Radial Menu. Check Port 9000. |
| **"Phone won't connect"** | Make sure Phone and PC are on the same Wi-Fi. If not, click "Use Cloudflare". |
| **"Video is lagging"** | Lower the quality slider on your phone screen. |
| **"Build Failed"** | Run `START_PROJECT.bat` -> Option 6 ("Clean & Rebuild"). |

---

## 🤝 Credits

Created for the VRChat Community.
Powered by **Rust**, **Tauri**, **ONNX Runtime**, and **Google MediaPipe** models.

*Inspired by the work of AlbertaBeef (BlazeApp).*

---

### [📂 View Technical Progress](avancer_du_projet.md)

# VRChat Universal Video Bridge

High-performance, AI-powered video tracking bridge for VRChat. Use your Webcam or Phone for full facial, hand, and arm tracking without expensive hardware.

## Features

- **Zero-Dependency**: Standalone application.
- **Adaptive AI**: Automatically switches between Ultra/Balanced/Eco modes.
- **VRChat Native**: Direct OSC integration.
- **Phone Link**: Scan a QR code to use your phone camera as a wireless tracker (no app install required).
- **Cloud Tunnel**: Automatic worldwide access via Cloudflare (coming soon).

## Installation

1. Download the latest release (`.zip`).
2. Extract the folder.
3. Run `scripts/download_models.bat` to fetch AI models.
4. Run `Start_VRC_Bridge.bat`.

## Setup

### 1. Firewall

If avatars don't move, run `scripts/setup_firewall.bat` as Administrator.

### 2. VRChat config

Ensure OSC is enabled in your VRChat Radial Menu: `Options -> OSC -> Enabled`.

## Building from Source

### Requirements

- CMake 3.20+
- Visual Studio 2022 (C++20)
- vcpkg
- Python 3.8+ (for model download script)

### Build

```powershell
cmake -B build -S . -DCMAKE_TOOLCHAIN_FILE=vcpkg/scripts/buildsystems/vcpkg.cmake
cmake --build build --config Release
```

# VRChat Phone Link - Web Interface

This directory contains the GitHub Pages website for the Phone Link feature.

## Files

- **index.html** - Main interface with camera controls
- **style.css** - Modern dark theme with glassmorphism
- **app.js** - Camera streaming logic and Cloudflare Tunnel integration

## How It Works

1. Desktop app launches and starts Cloudflare Tunnel
2. App generates QR code with URL: `https://shazamifius.github.io/VRchatFACE-HAND-ARM-tracking?tunnel=xxx.trycloudflare.com`
3. User scans QR code with phone
4. Website reads `tunnel` parameter and streams camera frames via POST
5. Frames sent to: `https://xxx.trycloudflare.com/video`
6. Cloudflare tunnels to desktop app on localhost:8080

## Features

- ✅ No app install required (pure web)
- ✅ Automatic front/back camera flip
- ✅ HD/SD quality toggle
- ✅ FPS counter
- ✅ Connection status indicator
- ✅ Automatic retry on errors
- ✅ Screen wake lock (prevent sleep)

## GitHub Pages Setup

1. Go to repository Settings → Pages
2. Source: `main` branch, `/docs` folder
3. Save and wait ~1 minute for deployment
4. URL will be: `https://shazamifius.github.io/VRchatFACE-HAND-ARM-tracking`

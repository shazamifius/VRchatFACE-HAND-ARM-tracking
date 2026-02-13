$ErrorActionPreference = "Stop"

$url = "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe"
$dest = ".\src-tauri\cloudflared.exe"

Write-Host "Downloading Cloudflared from $url..."
try {
    Invoke-WebRequest -Uri $url -OutFile $dest
    Write-Host "Download complete: $dest"
    Write-Host "You can now use Phone Mode."
} catch {
    Write-Error "Failed to download Cloudflared. Please download manually from: $url and place it in src-tauri renamed as cloudflared.exe"
}

Pause

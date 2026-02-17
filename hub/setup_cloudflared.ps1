[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$url = "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe"
# [FIX] Use absolute path relative to this script location
$destDir = Join-Path $PSScriptRoot "src-tauri"
$dest = Join-Path $destDir "cloudflared.exe"

if (-not (Test-Path $destDir)) {
    Write-Host "Creating directory: $destDir"
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
}

if (Test-Path "$dest") {
    Write-Host "Cloudflared already exists at $dest. Skipping download."
}
else {
    Write-Host "Downloading Cloudflared from $url..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing
        if (Test-Path "$dest") {
             Write-Host "Download complete: $dest"
             Write-Host "You can now use Phone Mode."
        } else {
             throw "File not found after download."
        }
    }
    catch {
        Write-Error "Failed to download Cloudflared. Error: $_"
        Write-Error "Please download manually from: $url and place it in $destDir named 'cloudflared.exe'"
        exit 1
    }
}



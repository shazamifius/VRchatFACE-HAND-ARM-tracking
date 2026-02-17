# Fix Network Settings for VRChat Bridge
# This script sets the network to "Private" to allow local device connections
# and ensures Port 9001 is open.

$ErrorActionPreference = "SilentlyContinue"

Write-Host "=== VRChat Bridge Connection Fixer ===" -ForegroundColor Cyan
Write-Host "1. Opening Port 9001 in Firewall..."
New-NetFirewallRule -DisplayName "VRChat Bridge Hub" -Direction Inbound -LocalPort 9001 -Protocol TCP -Action Allow -Profile Any -Force
Write-Host "   -> Done." -ForegroundColor Green

Write-Host "2. Checking Network Profile..."
# Build list of active connections
$adapters = Get-NetConnectionProfile

foreach ($adapter in $adapters) {
    if ($adapter.NetworkCategory -eq "Public") {
        Write-Host "   -> Found Public Network: $($adapter.InterfaceAlias)" -ForegroundColor Yellow
        Write-Host "   -> Switching to PRIVATE (Required for Local Connect)..."
        Set-NetConnectionProfile -InterfaceIndex $adapter.InterfaceIndex -NetworkCategory Private
        Write-Host "   -> Fixed." -ForegroundColor Green
    }
    else {
        Write-Host "   -> Network '$($adapter.InterfaceAlias)' is already Private/Domain. Good." -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "=== SUCCESS ===" -ForegroundColor Cyan
Write-Host "Your PC is now ready to accept connections from your phone."
Write-Host "1. Restart the VRChat Bridge App."
Write-Host "2. Connect your phone using the QR Code."
Write-Host ""
Write-Host "Press Enter to close this window..."
Read-Host

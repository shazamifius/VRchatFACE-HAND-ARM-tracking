# Package Release Script
$ErrorActionPreference = "Stop"

$ProjectRoot = ".."
$BuildDir = "$ProjectRoot/build/Release"
$DistDir = "$ProjectRoot/dist"

# Clean dist
if (Test-Path $DistDir) {
    Remove-Item -Recurse -Force $DistDir
}
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
New-Item -ItemType Directory -Force -Path "$DistDir/models" | Out-Null
New-Item -ItemType Directory -Force -Path "$DistDir/logs" | Out-Null

Write-Host "Copying Executable..."
Copy-Item "$BuildDir/*.exe" $DistDir

Write-Host "Copying DLLs (from vcpkg bin)..."
# Locate vcpkg bin dir - heuristic
$VcpkgBin = "$ProjectRoot/build/vcpkg_installed/x64-windows/bin"
if (Test-Path $VcpkgBin) {
    Copy-Item "$VcpkgBin/*.dll" $DistDir
} else {
    Write-Warning "Could not find vcpkg bin directory at $VcpkgBin"
}

Write-Host "Creating Launcher..."
$LauncherContent = @"
@echo off
echo Starting VRChat Video Bridge...
start "" "VRchatFACE-HAND-ARM-tracking.exe"
"@
Set-Content "$DistDir/Start_VRC_Bridge.bat" $LauncherContent

Write-Host "Done! Distribution ready at $DistDir"

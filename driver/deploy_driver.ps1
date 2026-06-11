# Deploy the freshly built vrcbridge driver DLL and rebuild the app.
#
# SteamVR locks driver_vrcbridge.dll while running, and cargo cannot overwrite
# the app .exe while it is running, so this script stops all of those first,
# installs the new DLL into the registered driver root, then rebuilds the app
# in release. Run it whenever the C++ driver or the Rust app changed.
#
# Usage (from the project root or anywhere):
#   powershell -ExecutionPolicy Bypass -File driver\deploy_driver.ps1

$ErrorActionPreference = "Stop"

$proj = Split-Path -Parent $PSScriptRoot
$dll  = Join-Path $PSScriptRoot "build\driver_vrcbridge.dll"
$destDir = Join-Path $proj "hub\tools\vrcbridge\bin\win64"
$dest = Join-Path $destDir "driver_vrcbridge.dll"

if (-not (Test-Path $dll)) {
    Write-Host "ERROR: $dll not found. Build the driver first (cmake --build driver\build)." -ForegroundColor Red
    exit 1
}

Write-Host "Stopping SteamVR / VRChat / the app so files can be replaced..." -ForegroundColor Cyan
$procs = @(
    "vrserver", "vrmonitor", "vrcompositor", "vrdashboard", "vrwebhelper",
    "vrstartup", "vrserverhelper", "VRChat", "vrchat-bridge-hub"
)
foreach ($p in $procs) {
    Get-Process -Name $p -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 2

Write-Host "Installing driver DLL -> $dest" -ForegroundColor Cyan
if (-not (Test-Path $destDir)) {
    New-Item -ItemType Directory -Force -Path $destDir | Out-Null
}
Copy-Item -Path $dll -Destination $dest -Force
Write-Host ("  driver DLL installed (" + (Get-Item $dest).Length + " bytes)") -ForegroundColor Green

Write-Host "Rebuilding the app (cargo build --release)..." -ForegroundColor Cyan
$manifest = Join-Path $proj "hub\src-tauri\Cargo.toml"
# cargo writes warnings to stderr; under ErrorActionPreference=Stop PowerShell
# would treat those as terminating, so relax it just for the build and judge
# success by the real exit code instead.
$prev = $ErrorActionPreference
$ErrorActionPreference = "Continue"
& cargo build --release --manifest-path $manifest
$code = $LASTEXITCODE
$ErrorActionPreference = $prev
if ($code -ne 0) {
    Write-Host "ERROR: app build failed (exit $code)." -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "Done. Now relaunch SteamVR, then start the app (START_PROJECT.bat)." -ForegroundColor Green
Write-Host "Controls: MOUSE = turn head (laser = gaze reticle), left click = select," -ForegroundColor Green
Write-Host "          WASD/ZQSD = move, Tab = menu, Space = jump, HOLD Left Alt = free cursor." -ForegroundColor Green

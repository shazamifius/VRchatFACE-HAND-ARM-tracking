# Build the diagnostic probe (diag\build\vrcbridge_probe.exe) and copy the
# OpenVR runtime DLL next to it. Run once (or after changing probe.cpp). Needs
# the same MSVC + cmake toolchain as the driver.
$ErrorActionPreference = "Continue"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Diag = Join-Path $ScriptDir "diag"

# Locate vcvars64.bat (VS 2022 / 18 Insiders, Community/Professional/Enterprise).
$vcvars = $null
foreach ($p in @(
    "C:\Program Files\Microsoft Visual Studio\18\Insiders\VC\Auxiliary\Build\vcvars64.bat",
    "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat",
    "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat",
    "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat"
)) { if (Test-Path $p) { $vcvars = $p; break } }
if (-not $vcvars) { Write-Host "vcvars64.bat not found - install Visual Studio C++ tools." -ForegroundColor Red; exit 1 }

$cmd = '"' + $vcvars + '" >nul && cmake -S "' + $Diag + '" -B "' + (Join-Path $Diag "build") + '" -G "NMake Makefiles" -DCMAKE_BUILD_TYPE=Release && cmake --build "' + (Join-Path $Diag "build") + '"'
& cmd /c $cmd 2>&1 | Select-Object -Last 8

$exe = Join-Path $Diag "build\vrcbridge_probe.exe"
if (Test-Path $exe) {
    # Copy the runtime openvr_api.dll from SteamVR next to the probe.
    $steamvrDll = "C:\Program Files (x86)\Steam\steamapps\common\SteamVR\bin\win64\openvr_api.dll"
    if (Test-Path $steamvrDll) { Copy-Item $steamvrDll (Split-Path $exe) -Force }
    Write-Host "Probe built: $exe" -ForegroundColor Green
} else {
    Write-Host "Probe build FAILED." -ForegroundColor Red
}

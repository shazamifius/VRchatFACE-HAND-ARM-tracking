# ============================================================================
#  DIAGNOSE.ps1  -  One-shot diagnostic snapshot of the vrcbridge VR rig.
# ============================================================================
#  Run this any time something is wrong in SteamVR / VRChat. It collects the
#  GROUND TRUTH into a single report so we stop guessing:
#    1. steamvr.vrsettings   - which HMD driver is forced / null enabled?
#    2. vrpathreg show       - which drivers are registered?
#    3. running processes    - SteamVR / VRChat / the app alive?
#    4. vrcbridge_probe      - LIVE device state SteamVR/VRChat see (HMD active?
#                              pose? angular velocity? controller roles? size?)
#    5. driver log           - what the driver received/emitted (%TEMP%)
#    6. vrserver.txt         - SteamVR's own log lines about our driver / errors
#
#  Output -> diag\report.txt   (paste that file back for analysis)
#
#  Usage:
#    powershell -ExecutionPolicy Bypass -File DIAGNOSE.ps1
#    powershell -ExecutionPolicy Bypass -File DIAGNOSE.ps1 -Watch   # live pose
# ============================================================================
param([switch]$Watch)

$ErrorActionPreference = "Continue"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Report = Join-Path $ScriptDir "diag\report.txt"
$Probe  = Join-Path $ScriptDir "diag\build\vrcbridge_probe.exe"

function Find-SteamPath {
    foreach ($key in @("HKCU:\Software\Valve\Steam", "HKLM:\SOFTWARE\WOW6432Node\Valve\Steam")) {
        try {
            $p = (Get-ItemProperty -Path $key -Name SteamPath -ErrorAction Stop).SteamPath
            if ($p -and (Test-Path $p)) { return (Resolve-Path $p).Path }
        } catch {}
    }
    return $null
}
function Find-SteamVR($steam) {
    $cands = @( (Join-Path $steam "steamapps\common\SteamVR") )
    $vdf = Join-Path $steam "steamapps\libraryfolders.vdf"
    if (Test-Path $vdf) {
        foreach ($l in Get-Content $vdf) {
            if ($l -match '"path"\s+"([^"]+)"') { $cands += (Join-Path ($matches[1] -replace '\\\\','\') "steamapps\common\SteamVR") }
        }
    }
    foreach ($c in $cands) { if (Test-Path (Join-Path $c "bin\win64\vrpathreg.exe")) { return (Resolve-Path $c).Path } }
    return $null
}

$Steam = Find-SteamPath
$SteamVR = if ($Steam) { Find-SteamVR $Steam } else { $null }

# Live watch mode: just stream the probe's pose view.
if ($Watch) {
    if (Test-Path $Probe) { & $Probe watch } else { Write-Host "Probe not built. Run BUILD first." -ForegroundColor Red }
    return
}

$lines = New-Object System.Collections.Generic.List[string]
function Add($s) { $lines.Add($s) }

Add("==================== vrcbridge DIAGNOSE ====================")
Add("time      : " + (Get-Date -Format "yyyy-MM-dd HH:mm:ss"))
Add("steam     : $Steam")
Add("steamvr   : $SteamVR")
Add("")

# --- 1. steamvr.vrsettings -------------------------------------------------
Add("---- steamvr.vrsettings (HMD driver selection) ----")
$vs = if ($Steam) { Join-Path $Steam "config\steamvr.vrsettings" } else { $null }
if ($vs -and (Test-Path $vs)) {
    try {
        $j = Get-Content $vs -Raw | ConvertFrom-Json
        Add("  forcedDriver           = " + $j.steamvr.forcedDriver)
        Add("  requireHmd             = " + $j.steamvr.requireHmd)
        Add("  activateMultipleDrivers= " + $j.steamvr.activateMultipleDrivers)
        Add("  driver_null.enable     = " + $j.driver_null.enable)
        $expected = ($j.steamvr.forcedDriver -eq "vrcbridge" -and $j.driver_null.enable -eq $false)
        $warn = $(if (-not $expected) { "   <-- PROBLEM: run hub\setup_vmt.ps1" } else { "" })
        Add("  >> vrcbridge is the active-HMD config: " + $expected + $warn)
    } catch { Add("  (could not parse: $($_.Exception.Message))") }
} else { Add("  (not found)") }
Add("")

# --- 2. registered drivers -------------------------------------------------
Add("---- vrpathreg show (registered drivers) ----")
if ($SteamVR) {
    $vpr = Join-Path $SteamVR "bin\win64\vrpathreg.exe"
    try { (& $vpr show) 2>$null | ForEach-Object { Add("  $_") } } catch { Add("  (vrpathreg failed)") }
} else { Add("  (SteamVR not found)") }
Add("")

# --- 3. processes ----------------------------------------------------------
Add("---- processes ----")
foreach ($p in @("vrserver","vrmonitor","vrcompositor","VRChat","vrchat-bridge-hub")) {
    $running = [bool](Get-Process -Name $p -ErrorAction SilentlyContinue)
    Add("  {0,-22} {1}" -f $p, $(if ($running) { "RUNNING" } else { "stopped" }))
}
Add("")

# --- 4. live probe ---------------------------------------------------------
Add("---- vrcbridge_probe (LIVE OpenVR device state) ----")
if (Test-Path $Probe) {
    try { (& $Probe) 2>&1 | ForEach-Object { Add("  $_") } } catch { Add("  (probe error: $($_.Exception.Message))") }
} else { Add("  (probe not built - run BUILD_DIAG.ps1)") }
Add("")

# --- 5. driver log ---------------------------------------------------------
Add("---- driver log (last 40 lines of %TEMP%\vrcbridge_driver.log) ----")
$dlog = Join-Path $env:TEMP "vrcbridge_driver.log"
if (Test-Path $dlog) { Get-Content $dlog -Tail 40 | ForEach-Object { Add("  $_") } } else { Add("  (no driver log yet - driver not loaded since last build)") }
Add("")

# --- 6. SteamVR vrserver.txt -----------------------------------------------
Add("---- vrserver.txt (lines about vrcbridge / HMD / standby / errors) ----")
$vlog = if ($Steam) { Join-Path $Steam "logs\vrserver.txt" } else { $null }
if ($vlog -and (Test-Path $vlog)) {
    Get-Content $vlog -Tail 400 | Select-String -Pattern "vrcbridge","Active HMD","standby","null","error","failed","Added device" |
        Select-Object -Last 30 | ForEach-Object { Add("  " + $_.Line) }
} else { Add("  (vrserver.txt not found at $vlog)") }
Add("")
Add("==================== end ====================")

# Write + echo
$lines | Set-Content -Path $Report -Encoding utf8
Write-Host ("Report written to: " + $Report) -ForegroundColor Green
Write-Host ""
$lines | ForEach-Object { Write-Host $_ }

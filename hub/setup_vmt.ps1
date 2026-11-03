# ============================================================================
#  setup_vmt.ps1  -  Automatic, self-repairing VirtualMotionTracker installer
# ============================================================================
#  Goal (product vision): zero manual steps. Run this every launch; it makes
#  the SteamVR side "just work" and repairs itself if a SteamVR update wiped
#  the driver registration or reset the settings.
#
#  It is fully IDEMPOTENT - safe to re-run any number of times:
#    1. Locate Steam + SteamVR.
#    2. Download + extract the VMT driver (only if missing).
#    3. Register the VMT driver AND our vrcbridge HMD driver via vrpathreg.
#    4. Patch steamvr.vrsettings so OUR vrcbridge HMD is the active headset
#       (null HMD DISABLED) and VRChat can enter VR mode + accept our trackers.
#
#  Exit code 0 = VMT ready (or best-effort done). Non-zero = SteamVR missing.
# ============================================================================

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ToolsDir  = Join-Path $ScriptDir "tools"
$VmtDir    = Join-Path $ToolsDir "vmt"   # extracted VMT lives here

function Info($m)  { Write-Host "[VMT] $m" -ForegroundColor Cyan }
function Ok($m)    { Write-Host "[VMT] $m" -ForegroundColor Green }
function Warn($m)  { Write-Host "[VMT] $m" -ForegroundColor Yellow }
function Err($m)   { Write-Host "[VMT] $m" -ForegroundColor Red }

# --- 1. Locate Steam + SteamVR ---------------------------------------------
function Find-SteamPath {
    foreach ($key in @("HKCU:\Software\Valve\Steam", "HKLM:\SOFTWARE\WOW6432Node\Valve\Steam", "HKLM:\SOFTWARE\Valve\Steam")) {
        try {
            $p = (Get-ItemProperty -Path $key -Name SteamPath -ErrorAction Stop).SteamPath
            if ($p -and (Test-Path $p)) { return (Resolve-Path $p).Path }
        } catch {}
    }
    return $null
}

$SteamPath = Find-SteamPath
if (-not $SteamPath) {
    Err "Steam not found. Install Steam, then SteamVR, then re-run."
    Start-Process "https://store.steampowered.com/about/"
    exit 2
}
Info "Steam: $SteamPath"

# SteamVR can live in any library folder, not just the main one. Scan libraryfolders.vdf.
function Find-SteamVR($steam) {
    $candidates = @( Join-Path $steam "steamapps\common\SteamVR" )
    $vdf = Join-Path $steam "steamapps\libraryfolders.vdf"
    if (Test-Path $vdf) {
        foreach ($line in Get-Content $vdf) {
            if ($line -match '"path"\s+"([^"]+)"') {
                $lib = $matches[1] -replace '\\\\', '\'
                $candidates += (Join-Path $lib "steamapps\common\SteamVR")
            }
        }
    }
    foreach ($c in $candidates) {
        if (Test-Path (Join-Path $c "bin\win64\vrpathreg.exe")) { return (Resolve-Path $c).Path }
    }
    return $null
}

$SteamVR = Find-SteamVR $SteamPath
if (-not $SteamVR) {
    Warn "SteamVR not installed. Triggering Steam install (app 250820)..."
    Start-Process "steam://install/250820"
    Err "Once SteamVR finishes installing, re-run the launcher (this step self-repairs)."
    exit 3
}
Ok "SteamVR: $SteamVR"
$VrPathReg = Join-Path $SteamVR "bin\win64\vrpathreg.exe"

# --- 2. Download + extract VMT (only if missing) ---------------------------
function Ensure-VmtFiles {
    # Already extracted and valid? (driver manifest present somewhere under VmtDir)
    if (Test-Path $VmtDir) {
        $existing = Get-ChildItem -Path $VmtDir -Recurse -Filter "driver.vrdrivermanifest" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($existing) { return $existing.Directory.FullName }
    }

    New-Item -ItemType Directory -Force -Path $ToolsDir | Out-Null
    Info "Fetching VMT releases from GitHub..."
    $headers = @{ "User-Agent" = "vrchat-bridge-hub-setup" }
    # VMT switched from a portable .zip to a GUI .exe installer at v0.14a. We want
    # the portable build (no installer GUI, no elevation, we control the path), so
    # scan releases newest-first and take the most recent one that ships a .zip.
    # The OSC protocol (/VMT/Room/Unity) is identical across these versions.
    $releases = Invoke-RestMethod -Uri "https://api.github.com/repos/gpsnmeajp/VirtualMotionTracker/releases?per_page=30" -Headers $headers
    $asset = $null
    foreach ($r in $releases) {
        $z = $r.assets | Where-Object { $_.name -match '\.zip$' } | Select-Object -First 1
        if ($z) { $asset = $z; Info "Using portable VMT $($r.tag_name)"; break }
    }
    if (-not $asset) { throw "No portable .zip asset found in any recent VMT release." }

    $zipPath = Join-Path $ToolsDir $asset.name
    Info "Downloading $($asset.name) ..."
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zipPath -Headers $headers

    if (Test-Path $VmtDir) { Remove-Item -Recurse -Force $VmtDir }
    New-Item -ItemType Directory -Force -Path $VmtDir | Out-Null
    Info "Extracting..."
    Expand-Archive -Path $zipPath -DestinationPath $VmtDir -Force
    Remove-Item -Force $zipPath -ErrorAction SilentlyContinue

    $manifest = Get-ChildItem -Path $VmtDir -Recurse -Filter "driver.vrdrivermanifest" -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $manifest) { throw "VMT extracted but driver.vrdrivermanifest not found." }
    return $manifest.Directory.FullName
}

try {
    $DriverDir = Ensure-VmtFiles
    Ok "VMT driver files: $DriverDir"
} catch {
    Err "Failed to obtain VMT: $($_.Exception.Message)"
    exit 4
}

# --- 3. Register the driver with SteamVR (re-register = repair) -------------
# 'adddriver' is idempotent in practice; we removedriver first to heal stale
# paths left behind by a previous VMT location or a SteamVR update.
try {
    & $VrPathReg removedriver "$DriverDir" 2>$null | Out-Null
    & $VrPathReg adddriver "$DriverDir" | Out-Null
    $show = (& $VrPathReg show) 2>$null
    if ($show -match [regex]::Escape($DriverDir)) {
        Ok "VMT driver registered with SteamVR."
    } else {
        Warn "Driver registration could not be verified (continuing)."
    }
} catch {
    Err "vrpathreg failed: $($_.Exception.Message)"
    exit 5
}

# --- 3b. Register OUR vrcbridge HMD driver (self-healing) -------------------
# The virtual HMD (head rotation + controllers + flat view) lives under
# hub/tools/vrcbridge. Re-register every launch so a SteamVR update or a moved
# folder can't silently drop it.
try {
    $VrcBridgeDir = Join-Path $ScriptDir "tools\vrcbridge"
    if (Test-Path (Join-Path $VrcBridgeDir "bin\win64\driver_vrcbridge.dll")) {
        & $VrPathReg removedriver "$VrcBridgeDir" 2>$null | Out-Null
        & $VrPathReg adddriver "$VrcBridgeDir" | Out-Null
        Ok "vrcbridge HMD driver registered with SteamVR."
    } else {
        Warn "vrcbridge driver DLL not found at $VrcBridgeDir (run driver\deploy_driver.ps1)."
    }
} catch {
    Warn "vrcbridge registration failed: $($_.Exception.Message)"
}

# --- 4. Patch steamvr.vrsettings for OUR virtual HMD ------------------------
# Webcam-only, no physical headset: force OUR vrcbridge HMD (head rotation,
# anti-standby, flat full-screen view) as the active headset and DISABLE the
# stock null HMD so they don't conflict. activateMultipleDrivers keeps VMT's
# body trackers alive alongside it. Backed up once.
#
# IMPORTANT: this used to force "null", which silently overrode the vrcbridge
# HMD every launch (head wouldn't turn, stuck "En veille"). Do NOT revert.
$VrSettings = Join-Path $SteamPath "config\steamvr.vrsettings"
try {
    if (-not (Test-Path (Split-Path $VrSettings))) {
        New-Item -ItemType Directory -Force -Path (Split-Path $VrSettings) | Out-Null
    }
    if (Test-Path $VrSettings) {
        $backup = "$VrSettings.bak"
        if (-not (Test-Path $backup)) { Copy-Item $VrSettings $backup -Force }
        $json = Get-Content $VrSettings -Raw | ConvertFrom-Json
    } else {
        $json = [PSCustomObject]@{}
    }

    function Set-Prop($obj, $name, $value) {
        if ($obj.PSObject.Properties.Name -contains $name) { $obj.$name = $value }
        else { $obj | Add-Member -NotePropertyName $name -NotePropertyValue $value -Force }
    }

    # steamvr section
    if (-not ($json.PSObject.Properties.Name -contains "steamvr")) {
        $json | Add-Member -NotePropertyName "steamvr" -NotePropertyValue ([PSCustomObject]@{}) -Force
    }
    Set-Prop $json.steamvr "requireHmd" $false
    Set-Prop $json.steamvr "forcedDriver" "vrcbridge"
    Set-Prop $json.steamvr "activateMultipleDrivers" $true

    # driver_null section (stock static HMD) - DISABLE so it can't take over as
    # the active HMD instead of our vrcbridge driver.
    if (-not ($json.PSObject.Properties.Name -contains "driver_null")) {
        $json | Add-Member -NotePropertyName "driver_null" -NotePropertyValue ([PSCustomObject]@{}) -Force
    }
    Set-Prop $json.driver_null "enable" $false

    $json | ConvertTo-Json -Depth 20 | Set-Content -Path $VrSettings -Encoding utf8
    Ok "steamvr.vrsettings patched: vrcbridge HMD active, null HMD disabled. Backup: steamvr.vrsettings.bak"
    if (Get-Process -Name vrserver, vrmonitor -ErrorAction SilentlyContinue) {
        Warn "SteamVR is running - restart it so the new settings take effect."
    }
} catch {
    Warn "Could not patch steamvr.vrsettings automatically: $($_.Exception.Message)"
    Warn "VMT trackers will still register; you may need to enable null HMD manually."
}

Ok "VMT setup complete. SteamVR will load the VMT driver on next start."
exit 0

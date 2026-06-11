# Head-rotation transport test for the vrcbridge virtual HMD driver.
#
# Sends a slowly panning YAW orientation to the driver's UDP head channel
# (127.0.0.1:39571) so you can SEE the head turn left/right in VRChat. This
# proves the Rust-brain -> driver head-pose path end to end, the same way the
# VMT_TEST circle proved the body path. Stop with Ctrl+C.
#
# Packet = 7 little-endian float32: qx,qy,qz,qw, px,py,pz (xyzw quaternion +
# position in metres, y up).

$ErrorActionPreference = "Stop"
$udp = New-Object System.Net.Sockets.UdpClient
$udp.Connect("127.0.0.1", 39571)

Write-Host "Sending panning yaw to 127.0.0.1:39571 ... (Ctrl+C to stop)"
$t = 0.0
while ($true) {
    # Pan +/- 60 degrees around the up (Y) axis.
    $yaw = [Math]::Sin($t) * (60.0 * [Math]::PI / 180.0)
    $half = $yaw / 2.0
    $qx = 0.0
    $qy = [Math]::Sin($half)
    $qz = 0.0
    $qw = [Math]::Cos($half)

    $bytes = [byte[]]@()
    foreach ($f in @([float]$qx, [float]$qy, [float]$qz, [float]$qw, [float]0.0, [float]1.6, [float]0.0)) {
        $bytes += [System.BitConverter]::GetBytes($f)
    }
    [void]$udp.Send($bytes, $bytes.Length)

    Start-Sleep -Milliseconds 16   # ~60 Hz
    $t += 0.03
}

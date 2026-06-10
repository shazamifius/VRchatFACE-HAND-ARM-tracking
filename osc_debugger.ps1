$socket = New-Object System.Net.Sockets.UdpClient 9005
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "   Simple OSC Debugger (Listen: 9005)    " -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "Press Ctrl+C to stop." -ForegroundColor Gray
Write-Host ""

$remote = $null
try {
    while ($true) {
        if ($socket.Available -gt 0) {
            $bytes = $socket.Receive([ref]$remote)
            # Basic brute-force string extraction to keep it simple without external libraries
            $ascii = [System.Text.Encoding]::ASCII.GetString($bytes)
            
            # Simple regex to extract just the path like /avatar/parameters/HeadPitch
            $regex = "(/avatar/parameters/[a-zA-Z0-9_/]+|/tracking/[a-zA-Z0-9_/]+|/input/[a-zA-Z0-9_]+)"
            $match = [regex]::Match($ascii, $regex)
            
            if ($match.Success) {
                # We won't decode the exact float bytes to keep this script robust and dependency-free, 
                # but we will print the exact parameter that is being blasted over the network.
                $paramName = $match.Value
                $time = Get-Date -Format "HH:mm:ss.fff"
                Write-Host "[$time] RECEIVED -> $paramName"
            }
        }
        else {
            Start-Sleep -Milliseconds 10
        }
    }
}
finally {
    $socket.Close()
}

# Fix VRChat Bridge Firewall Issues
# Run this script as Administrator

Write-Host "Checking Windows Firewall rules..."

$Port = 9001
$RuleName = "VRChat Bridge Hub (Port $Port)"

# Check if rule exists
$Rule = Get-NetFirewallRule -DisplayName $RuleName -ErrorAction SilentlyContinue

if ($Rule) {
    Write-Host "Firewall rule already exists." -ForegroundColor Green
}
else {
    Write-Host "Creating firewall rule for Port $Port..."
    try {
        New-NetFirewallRule -DisplayName $RuleName -Direction Inbound -LocalPort $Port -Protocol TCP -Action Allow
        Write-Host "Success! Firewall rule created." -ForegroundColor Green
    }
    catch {
        Write-Error "Failed to create firewall rule. Please run this script as Administrator."
    }
}

Write-Host "Please restart the VRChat Bridge application and try connecting again."
Write-Host "Press any key to exit..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

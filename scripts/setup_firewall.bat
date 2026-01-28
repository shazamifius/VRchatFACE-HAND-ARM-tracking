@echo off
echo ==================================================
echo   VRChat Bridge Firewall Setup
echo ==================================================
echo.
echo This script sends a request to open UDP ports for OSC.
echo Please allow Administrator privileges if asked.
echo.

net session >nul 2>&1
if %errorLevel% == 0 (
    echo Admin privileges confirmed.
) else (
    echo Requesting Admin privileges...
    powershell -Command "Start-Process '%0' -Verb RunAs"
    exit
)

echo Adding Firewall Rule for VRChat Bridge...
netsh advfirewall firewall add rule name="VRChat Video Bridge OSC" dir=in action=allow protocol=UDP localport=9000
netsh advfirewall firewall add rule name="VRChat Video Bridge OSC" dir=out action=allow protocol=UDP localport=9000

echo.
echo Done! You can now close this window.
pause

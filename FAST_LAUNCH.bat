@echo off
echo Starting VRChat Bridge Hub...

cd /d "%~dp0hub\src-tauri\target\release"
if not exist "vrchat-bridge-hub.exe" (
    echo ERROR: Could not find vrchat-bridge-hub.exe
    echo Please run LAUNCH_HUB.bat once to build it, or check the path.
    pause
    exit /b
)

echo Found executable. Launching...
vrchat-bridge-hub.exe

echo.
echo Application closed.
pause

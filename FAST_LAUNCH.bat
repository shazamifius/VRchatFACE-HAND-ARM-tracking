@echo off
echo Starting VRChat Bridge Hub...

set "TARGET_DIR=%~dp0hub\src-tauri\target\release"

if exist "%TARGET_DIR%\vrchat-bridge-hub.exe" (
    cd /d "%TARGET_DIR%"
) else (
    set "TARGET_DIR=%~dp0hub\src-tauri\target\debug"
    cd /d "%TARGET_DIR%"
)

if not exist "vrchat-bridge-hub.exe" (
    echo ERROR: Could not find vrchat-bridge-hub.exe in Release OR Debug.
    echo Please run LAUNCH_HUB.bat once to build it.
    pause
    exit /b
)

echo Found executable in %TARGET_DIR%. Launching...
vrchat-bridge-hub.exe

echo.
echo Application closed.
pause

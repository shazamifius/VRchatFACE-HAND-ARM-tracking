@echo off
echo [INFO] Building VRChat Bridge Hub...

cd /d "%~dp0hub\src-tauri"

echo [INFO] Running cargo build (Debug)...
cargo build
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Build failed! Check the errors above.
    pause
    exit /b %ERRORLEVEL%
)

echo [INFO] Build successful. Launching...
target\debug\vrchat-bridge-hub.exe

@echo off
echo [INFO] Starting VRChat OSC Bridge System...

:: 1. Start Rust Hub (Bridge & Solver)
echo [START] Launching Rust Hub...
cd /d "%~dp0hub\src-tauri"
start "" "target\debug\vrchat-bridge-hub.exe"

:: 2. Wait for Hub to initialize
echo [WAIT] Waiting 5 seconds for Rust Hub to bind ports...
timeout /t 5 /nobreak >nul

:: 3. Start Python Tracker (Camera & AI)
echo [START] Launching Python Tracker...
cd /d "%~dp0blaze_app_python-main"

:: Use the existing run_tracker.bat which handles venv/deps better
:: "start" with title "Python Tracker" and running the bat file
start "Python Tracker - VRChat Bridge" call run_tracker.bat

echo [INFO] System is running!
echo - Rust Hub: Bridge and Solving
echo - Python: Camera and Tracking
echo.
echo If Python window closes immediately, there is an error.
echo Check the 'Python Tracker' window for details.
pause

@echo off
echo [DEV] Starting Python Tracker (in new window)...
cd ..\blaze_app_python-main
start "Python Tracker" run_tracker.bat
cd ..\hub

echo [DEV] Starting Tauri UI...
echo (Waiting 3s for tracker to init...)
timeout /t 3 /nobreak >nul
call npm run dev
pause

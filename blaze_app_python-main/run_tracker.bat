@echo off
cd /d "%~dp0"
setlocal

echo [1/3] Detecting Valid Python...
set PYTHON_CMD=py
%PYTHON_CMD% --version >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    if exist "%LocalAppData%\Programs\Python\Python313\python.exe" (
        set PYTHON_CMD="%LocalAppData%\Programs\Python\Python313\python.exe"
    ) else (
        set PYTHON_CMD=python
    )
)
echo Using Python: %PYTHON_CMD%

echo [2/3] Restoring Environment...
echo Upgrading Numpy to stable version (2.x)...
%PYTHON_CMD% -m pip install --upgrade numpy opencv-python

echo [3/3] Launching Invisible Tracker...
echo    - MJPEG Stream: http://localhost:8080
echo    - OSC Target:   127.0.0.1:9002
echo.
%PYTHON_CMD% tracker_headless.py

echo.
echo If the window closes immediately, there was an error above.
pause

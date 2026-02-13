@echo off
echo ========================================
echo   VRChat Tracking - DIAGNOSTIC LAUNCHER
echo ========================================

REM Try to find Python automatically (Same logic as Rust App)
set "PY_PATH=%LOCALAPPDATA%\Programs\Python\Python313\python.exe"
if exist "%PY_PATH%" goto :FOUND

set "PY_PATH=%LOCALAPPDATA%\Programs\Python\Python312\python.exe"
if exist "%PY_PATH%" goto :FOUND

set "PY_PATH=%LOCALAPPDATA%\Programs\Python\Python311\python.exe"
if exist "%PY_PATH%" goto :FOUND

REM Fallback to PATH
set "PY_PATH=python"

:FOUND
echo Found Python at: %PY_PATH%
echo Launching diagnostic tool...
echo.

"%PY_PATH%" hub\python\diagnostic.py

if %errorlevel% neq 0 (
    echo.
    echo ERROR: Failed to run diagnostic. 
    echo Please make sure you have installed python dependencies.
    echo Try running: pip install opencv-python numpy python-osc
)

echo.
pause

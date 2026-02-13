@echo off
cd /d "%~dp0"
setlocal

echo [1/5] Detecting Valid Python...

:: 1. Try standard Windows Python Launcher 'py'
set PYTHON_CMD=py
%PYTHON_CMD% --version >nul 2>&1
if %ERRORLEVEL% EQU 0 goto :FoundPython

:: 2. Try explicit paths for standard python installations
if exist "%LocalAppData%\Programs\Python\Python313\python.exe" (
    set PYTHON_CMD="%LocalAppData%\Programs\Python\Python313\python.exe"
    goto :FoundPython
)
:: (Add other versions if needed)

:: 3. Fallback to PATH but warn if it looks like MinGW
set PYTHON_CMD=python
%PYTHON_CMD% --version >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: No python found. Please install Python from python.org
    pause
    exit /b
)

:FoundPython
echo Using Python: %PYTHON_CMD%
%PYTHON_CMD% --version

echo [2/5] Setting up Virtual Environment...
if not exist venv (
    echo Creating venv...
    %PYTHON_CMD% -m venv venv
)

if exist venv\Scripts\activate.bat (
    call venv\Scripts\activate.bat
) else (
    echo ERROR: Failed to create venv. Using global python fallback.
)

echo [3/5] Installing Dependencies...
:: Force use of the python executable we found to call pip
%PYTHON_CMD% -m pip install --upgrade pip
%PYTHON_CMD% -m pip install opencv-python numpy tensorflow-cpu tf2onnx

echo [4/5] Downloading Models...
%PYTHON_CMD% download_models.py

echo [5/5] Syncing & Converting Models for Rust Hub...
%PYTHON_CMD% sync_models.py

echo.
echo ===================================================
echo  READY TO LAUNCH BLAZE APP
echo ===================================================
echo.
echo Launching Demo...
%PYTHON_CMD% blaze_detect_live.py --blaze face,hand --pipeline tfl_face_v0_10_short,tfl_hand_v0_10_lite --fps --debug

pause
endlocal

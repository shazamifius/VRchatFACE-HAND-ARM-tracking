@echo off
cd /d "%~dp0"
setlocal

echo [1/2] Detecting Valid Python...

:: 1. Try standard Windows Python Launcher 'py'
set PYTHON_CMD=py
%PYTHON_CMD% --version >nul 2>&1
if %ERRORLEVEL% EQU 0 goto :FoundPython

:: 2. Try explicit paths for standard python installations
if exist "%LocalAppData%\Programs\Python\Python313\python.exe" (
    set PYTHON_CMD="%LocalAppData%\Programs\Python\Python313\python.exe"
    goto :FoundPython
)
if exist "%LocalAppData%\Programs\Python\Python312\python.exe" (
    set PYTHON_CMD="%LocalAppData%\Programs\Python\Python312\python.exe"
    goto :FoundPython
)

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

echo [2/2] Fixing Dependencies & Forcing Conversion...
echo Installing compatible libraries (Numpy 1.x)...
%PYTHON_CMD% -m pip install "numpy<2.0" --upgrade
%PYTHON_CMD% -m pip install tf2onnx --upgrade

echo Running Conversion...
%PYTHON_CMD% sync_models.py

echo.
echo DONE! Models should now be in hub/src-tauri/models.
pause

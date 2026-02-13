@echo off
cd /d "%~dp0"
setlocal

echo [1/2] Detecting Valid Python...
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
%PYTHON_CMD% --version

echo [2/2] Installing python-osc...
%PYTHON_CMD% -m pip install python-osc

pause

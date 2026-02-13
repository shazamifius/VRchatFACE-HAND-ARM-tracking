@echo off
setlocal
cd /d "%~dp0.."

if not exist "models" mkdir "models"

echo ==================================================
echo   Downloading AI Models...
echo ==================================================
echo.

@echo off
setlocal
cd /d "%~dp0.."

if not exist "models" mkdir "models"

echo ==================================================
echo   Downloading AI Models...
echo ==================================================
echo.

REM Try using Python first as it handles redirects/LFS better
python scripts/download_models.py
if %errorlevel% neq 0 (
    echo [WARNING] Python script failed or python not found.
    echo Please make sure you have Python installed.
    echo.
    echo Falling back to simple curl...
    REM --- Fallback CURL logic (Original) ---
    REM ... (Omitted to force update to Python usage for user)
    pause
    exit /b 1
)

echo.
echo ==================================================
echo   Models Setup Complete!
echo ==================================================
pause

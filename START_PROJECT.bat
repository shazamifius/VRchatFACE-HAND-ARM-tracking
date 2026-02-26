@echo off
setlocal

:: [NEW] Auto-Request Admin Privileges
>nul 2>&1 "%SYSTEMROOT%\system32\cacls.exe" "%SYSTEMROOT%\system32\config\system"
if '%errorlevel%' NEQ '0' (
    echo Requesting Administrator privileges...
    goto UACPrompt
) else ( goto gotAdmin )

:UACPrompt
    echo Set UAC = CreateObject^("Shell.Application"^) > "%temp%\getadmin.vbs"
    echo UAC.ShellExecute "%~s0", "", "", "runas", 1 >> "%temp%\getadmin.vbs"
    "%temp%\getadmin.vbs"
    exit /B

:gotAdmin
    if exist "%temp%\getadmin.vbs" ( del "%temp%\getadmin.vbs" )
    pushd "%CD%"
    CD /D "%~dp0"

:: [NEW] Run Network Fix Automatically
echo [INFO] Enforcing Network Settings (Firewall/Profile)...
powershell -ExecutionPolicy Bypass -File "hub\fix_network.ps1"

:: [NEW] Check Critical Dependencies
echo [INFO] Checking system requirements...

:: Check for Rust
where cargo >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [CRITICAL] RUST is missing!
    echo [INFO] The project requires the Rust compiler to build.
    echo [INFO] I will now open the installer for you.
    echo.
    echo 1. The installer 'rustup-init.exe' will be downloaded.
    echo 2. Please run it and press '1' for default installation.
    echo 3. IMPORTANT: You MUST restart this window after installation!
    echo.
    pause
    start https://win.rustup.rs/x86_64
    exit
)

:: Check for Python
where python >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [WARNING] PYTHON is missing!
    echo [INFO] The tracker engine needs Python to run.
    echo [INFO] Opening Python download page...
    start https://www.python.org/downloads/
    pause
)

:: Check for Cloudflare (Phone Mode)
if not exist "%~dp0hub\src-tauri\cloudflared.exe" (
    echo.
    echo [WARNING] Cloudflared is missing!
    echo [INFO] This is required for Phone Tracking connection.
    echo [INFO] Downloading Cloudflared automatically...
    powershell -ExecutionPolicy Bypass -File "%~dp0hub\setup_cloudflared.ps1"
    
    if not exist "%~dp0hub\src-tauri\cloudflared.exe" (
        echo [ERROR] Failed to download Cloudflared!
        echo Please check your internet connection or download manually.
        pause
    ) else (
        echo [SUCCESS] Cloudflared installed.
    )
)

:MENU
cls
echo ========================================================
echo   VRChat Bridge Hub - Unified Laucher (Admin Mode)
echo ========================================================
echo.
echo  [1] Full Start (Release - Recommended)
echo  [2] Quick Start (Release - Skip Build)
echo  [3] Build Rust Hub Only (Release)
echo  [4] Run Diagnostics
echo  [5] Setup Cloudflare Only
echo  [6] Clean ^& Rebuild
echo  [7] DEBUG START (With Logs ^& Console)
echo  [0] Exit
echo.
set /p choice="Select an option: "

if "%choice%"=="1" goto FULL_START
if "%choice%"=="2" goto QUICK_START
if "%choice%"=="3" goto BUILD_ONLY
if "%choice%"=="4" goto DIAGNOSTIC
if "%choice%"=="5" goto SETUP_CLOUDFLARE
if "%choice%"=="6" goto CLEAN_BUILD
if "%choice%"=="7" goto DEBUG_START
if "%choice%"=="0" exit
echo Invalid choice. Please try again.
pause
goto MENU

:FULL_START
echo.
echo [INFO] Step 1: Building Rust Hub...
call :BUILD_HUB
if %ERRORLEVEL% NEQ 0 goto ERROR_EXIT

echo.
echo [INFO] Step 2: Setting up Cloudflare...
pushd "%~dp0hub"
powershell -ExecutionPolicy Bypass -File "setup_cloudflared.ps1"
popd

echo.
echo [INFO] Step 3: Launching Applications...
goto LAUNCH_APPS

:QUICK_START
echo.
echo [INFO] Launching Applications (Skipping Build)...
goto LAUNCH_APPS

:BUILD_ONLY
echo.
echo [INFO] Building Rust Hub...
call :BUILD_HUB
if %ERRORLEVEL% NEQ 0 goto ERROR_EXIT
echo [INFO] Build successful.
pause
goto MENU

:DIAGNOSTIC
echo.
echo [INFO] Running Diagnostics...

:: 1. Check for Visual C++ Redistributables (Merged from check_deps.bat)
echo [INFO] Checking system dependencies...
set "SYS32=%SystemRoot%\System32"
set "MISSING_DEPS=0"

if not exist "%SYS32%\vcruntime140.dll" (
    echo [ERROR] vcruntime140.dll is MISSING.
    set "MISSING_DEPS=1"
) else (
    echo [OK] vcruntime140.dll found.
)

if not exist "%SYS32%\msvcp140.dll" (
    echo [ERROR] msvcp140.dll is MISSING.
    set "MISSING_DEPS=1"
) else (
    echo [OK] msvcp140.dll found.
)

if "%MISSING_DEPS%"=="1" (
    echo.
    echo [CRITICAL] Missing Visual C++ Redistributable 2019/2022.
    echo Please install VC_Redist.x64.exe: https://aka.ms/vs/17/release/vc_redist.x64.exe
    pause
)

:: 2. Dependencies check
echo [INFO] Dependencies check skipped (Rust native)
goto MENU

:SETUP_CLOUDFLARE
echo.
echo [INFO] Setting up Cloudflare...
pushd "%~dp0hub"
powershell -ExecutionPolicy Bypass -File "setup_cloudflared.ps1"
popd
goto MENU

:CLEAN_BUILD
echo.
echo [INFO] Cleaning and Rebuilding...
cd /d "%~dp0hub\src-tauri"
cargo clean
cd /d "%~dp0"
call :BUILD_HUB
if %ERRORLEVEL% NEQ 0 goto ERROR_EXIT
echo [INFO] Clean build successful.
pause
goto MENU

:DEBUG_START
echo.
echo [INFO] Killing any existing instances...
taskkill /F /IM vrchat-bridge-hub.exe >nul 2>&1

echo [INFO] Building in DEBUG mode (Console enabled)...
cd /d "%~dp0hub\src-tauri"
cargo build
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Build failed!
    pause
    goto MENU
)
cd /d "%~dp0"

echo [INFO] Step 2: Setting up Cloudflare...
pushd "%~dp0hub"
powershell -ExecutionPolicy Bypass -File "setup_cloudflared.ps1"
popd

echo [INFO] Step 3: Launching Debug Hub...
pushd "%~dp0hub\src-tauri"
set "ORT_DYLIB_PATH=%~dp0hub\src-tauri\onnxruntime.dll"
start "VRChat Hub DEBUG" "target\debug\vrchat-bridge-hub.exe"
popd
echo [INFO] Debug Window launched. Check that window for logs!
pause
goto MENU

:: ==========================================
:: Internal Functions
:: ==========================================

:BUILD_HUB
cd /d "%~dp0hub\src-tauri"
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Build failed!
    exit /b %ERRORLEVEL%
)
cd /d "%~dp0"
exit /b 0

:: :SETUP_CLOUDFLARE_INTERNAL Removed (Inlined)

:LAUNCH_APPS
:: 1. Check if executable exists
set "EXE_PATH=%~dp0hub\src-tauri\target\release\vrchat-bridge-hub.exe"
if not exist "%EXE_PATH%" (
    echo [WARNING] Release build not found. Checking Debug build...
    set "EXE_PATH=%~dp0hub\src-tauri\target\debug\vrchat-bridge-hub.exe"
)

if not exist "%EXE_PATH%" (
    echo [ERROR] No executable found! Please Build first.
    pause
    goto MENU
)

:: 2. Start Rust Hub
echo [START] Launching Rust Hub...
pushd "%~dp0hub\src-tauri"
set "ORT_DYLIB_PATH=%~dp0hub\src-tauri\onnxruntime.dll"
start "" "%EXE_PATH%"
popd

:: 3. Wait for Hub
echo [WAIT] Waiting 5 seconds...
timeout /t 5 /nobreak >nul

:: 4. Python Tracker (Disabled - Rust Native)
echo [INFO] Python Tracker disabled (using native Rust engine).

echo.
echo [INFO] System is running!
echo [INFO] Closing launcher...
timeout /t 3 >nul
exit

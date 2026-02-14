@echo off
echo [INFO] Checking for Visual C++ Redistributable dependencies...

set "SYS32=%SystemRoot%\System32"
set "MISSING=0"

if not exist "%SYS32%\vcruntime140.dll" (
    echo [ERROR] vcruntime140.dll is MISSING from System32.
    set "MISSING=1"
) else (
    echo [OK] vcruntime140.dll found.
)

if not exist "%SYS32%\msvcp140.dll" (
    echo [ERROR] msvcp140.dll is MISSING from System32.
    set "MISSING=1"
) else (
    echo [OK] msvcp140.dll found.
)

if not exist "%SYS32%\vcruntime140_1.dll" (
    echo [WARN] vcruntime140_1.dll is NOT found. Some ONNX versions need this.
) else (
    echo [OK] vcruntime140_1.dll found.
)

if "%MISSING%"=="1" (
    echo.
    echo [CRITICAL] You are missing Visual C++ Redistributable 2019/2022.
    echo Please download and install "VC_Redist.x64.exe" from Microsoft.
    echo Link: https://aka.ms/vs/17/release/vc_redist.x64.exe
    pause
    exit /b 1
)

echo [SUCCESS] Dependencies look OK.
exit /b 0

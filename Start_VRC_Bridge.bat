@echo off
setlocal
echo ==================================================
echo   VRChat Universal Video Bridge - Launcher
echo ==================================================

REM 1. Configure Check
if not exist "build\CMakeCache.txt" goto :configure
goto :build

:configure
echo [INFO] Configuring Project (First Run or Clean)...
cmake -B build -S . -DCMAKE_TOOLCHAIN_FILE="vcpkg/scripts/buildsystems/vcpkg.cmake" -G "Visual Studio 17 2022"
if errorlevel 1 goto :error_config
goto :build

:build
echo [INFO] Building Project...
cmake --build build --config Release
if errorlevel 1 goto :error_build
goto :run

:run
echo.
echo [SUCCESS] Starting VRChat Universal Video Bridge...
REM Run directly to keep log window if it crashes
start build\Release\VRChatUniversalVideoBridge.exe
exit /b 0

:error_config
echo.
echo [ERROR] Configuration failed! 
echo Please ensure Visual Studio 2022 is installed.
pause
exit /b 1

:error_build
echo.
echo [ERROR] Compilation failed!
pause
exit /b 1
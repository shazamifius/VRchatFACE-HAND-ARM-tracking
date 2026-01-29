@echo off
setlocal
echo ==================================================
echo   VRChat Universal Video Bridge - Launcher
echo ==================================================

REM 1. Configure Check
echo [INFO] Configuring Project...
cmake -B build -S . -DCMAKE_TOOLCHAIN_FILE="vcpkg/scripts/buildsystems/vcpkg.cmake" -G "Visual Studio 17 2022"
if errorlevel 1 goto :retry_clean
goto :build

:retry_clean
echo.
echo [WARNING] Configuration failed (Possible path mismatch).
echo [INFO] Cleaning CMakeCache.txt and retrying...
if exist "build\CMakeCache.txt" del /f /q "build\CMakeCache.txt"
cmake -B build -S . -DCMAKE_TOOLCHAIN_FILE="vcpkg/scripts/buildsystems/vcpkg.cmake" -G "Visual Studio 17 2022"
if errorlevel 1 goto :error_config
goto :build

:build
REM Always Build (compiles changes)
echo [INFO] Compiling Project...
cmake --build build --config Release
if errorlevel 1 goto :error_build

:run
echo.
echo [SUCCESS] Starting VRChat Universal Video Bridge...
REM Run directly to see logs in this window
"build\Release\VRChatUniversalVideoBridge.exe"
echo [INFO] Application exited.
pause
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
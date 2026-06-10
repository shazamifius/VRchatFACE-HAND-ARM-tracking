@echo off
title Simple OSC Debugger
echo Starting PowerShell OSC Listener...
powershell -ExecutionPolicy Bypass -File "%~dp0osc_debugger.ps1"
pause

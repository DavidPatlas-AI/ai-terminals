@echo off
chcp 65001 >nul
cd /d "%~dp0"

if "%~1"=="" (
    call "%~dp0start.bat"
    exit /b %errorlevel%
)

powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model "%~1"

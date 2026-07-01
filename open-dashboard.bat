@echo off
chcp 65001 >nul
cd /d "%~dp0"
if not exist dashboard.html (
    echo Building dashboard...
    powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0check-all.ps1"
)
start "" "%~dp0dashboard.html"
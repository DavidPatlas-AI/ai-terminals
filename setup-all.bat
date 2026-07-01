@echo off
chcp 65001 >nul
title AI Terminals - Full Setup
cd /d "%~dp0"
cls
echo.
echo   ========================================
echo      AI Terminals - Setup Complete
echo   ========================================
echo.
echo   Step 1/4: Desktop shortcuts (Hub + Dashboard)
call setup-desktop.bat
echo.
echo   Step 2/4: Work folder shortcuts (Codex, Grok...)
call setup-work-shortcuts.bat
echo.
echo   Step 3/4: Model status check + dashboard
call check-all.bat
echo.
echo   Step 4/4: Opening dashboard...
start "" "%~dp0open-dashboard.bat"
echo.
echo   Done! On your desktop:
echo     AI Hub.lnk          - menu
echo     AI Dashboard.lnk    - tokens + folders
echo     AI Work - Codex.lnk - where Codex saves files
echo.
pause
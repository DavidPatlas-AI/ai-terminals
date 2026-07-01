@echo off
chcp 65001 >nul
title AI Terminals - Full Setup
cd /d "%~dp0"
cls
echo.
echo   ========================================
echo      AI Terminals - Setup (in folder)
echo   ========================================
echo.
echo   Everything stays in this folder.
echo   Desktop stays clean.
echo.
if exist "%~dp0AI-Terminals\hub.ps1" call fix-folder-structure.bat
echo   Step 1/3: Clean desktop + shortcuts inside folder
call setup-desktop.bat
echo.
echo   Step 2/3: Model check + dashboard
call check-all.bat
echo.
echo   Step 3/3: Open dashboard
start "" "%~dp0open-dashboard.bat"
echo.
echo   Done!
echo     Folder:  Desktop\מודלים טרמנילים
echo     Entry:   START-HERE.bat  or  shortcuts\
echo     Codex:   shortcuts\AI Work - Codex.lnk
echo.
pause
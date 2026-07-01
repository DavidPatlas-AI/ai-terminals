@echo off
chcp 65001 >nul
title AI Work Folders
cd /d "%~dp0"
echo.
echo   AI Work Folders — open in Explorer
echo.
echo   1  All projects (פרויקטים)
echo   2  Codex outputs (Documents\Codex)
echo   3  Codex latest folder
echo   4  Grok sessions
echo   5  Claude data
echo   6  Cursor chats
echo   7  Create desktop shortcuts (setup)
echo   0  Back
echo.
set /p c="  Choose: "
if "%c%"=="1" start "" "%USERPROFILE%\Desktop\פרויקטים"
if "%c%"=="2" start "" "%USERPROFILE%\Documents\Codex"
if "%c%"=="3" powershell -NoProfile -Command ". '%~dp0_work-folders.ps1'; $p=Get-CodexLatestFolder; if($p){Start-Process explorer.exe $p}else{Write-Host 'No Codex folder found'}"
if "%c%"=="4" start "" "%USERPROFILE%\.grok\sessions"
if "%c%"=="5" start "" "%USERPROFILE%\.claude\projects"
if "%c%"=="6" start "" "%USERPROFILE%\.cursor\projects"
if "%c%"=="7" call "%~dp0setup-work-shortcuts.bat"
if "%c%"=="0" exit /b 0
pause
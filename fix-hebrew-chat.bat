@echo off
chcp 65001 >nul
title Fix Hebrew RTL in Cursor
color 0B
cls
echo.
echo  ========================================================
echo     Hebrew RTL in Cursor / Grok chat
echo  ========================================================
echo.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0check-rtl-status.ps1"
echo.
echo  [1] Install permanent fix (Administrator / UAC Yes)
echo  [2] Temporary fix (paste in DevTools Console)
echo  [3] Restart Cursor (after patch is installed)
echo  [0] Back
echo.
choice /C 1230 /N /M "  Choose: "
if errorlevel 4 exit /b 0
if errorlevel 3 goto RESTART
if errorlevel 2 goto TEMP
if errorlevel 1 goto PERM

:PERM
echo.
start "" "%~dp0fix-hebrew-rtl.vbs"
echo  After DONE: close ALL Cursor windows and reopen.
pause
exit /b 0

:RESTART
call "%~dp0restart-cursor-for-rtl.bat"
exit /b 0

:TEMP
echo.
echo  1. Ctrl+Shift+P -^> Developer: Toggle Developer Tools
echo  2. Console tab -^> Ctrl+V -^> Enter
echo.
powershell -NoProfile -Command "$p='%USERPROFILE%\.cursor\extensions\satan2049.cursor-rtl-1.0.2-universal\resources\rtl.js'; if(Test-Path -LiteralPath $p){Set-Clipboard -Value (Get-Content -LiteralPath $p -Raw -Encoding UTF8)} else {Set-Clipboard -Value (Get-Content -LiteralPath '%USERPROFILE%\.cursor\cursor-rtl-fix.js' -Raw -Encoding UTF8)}"
echo  Code copied to clipboard.
pause
exit /b 0
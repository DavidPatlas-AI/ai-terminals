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
echo  [1] NOW - paste V2 fix in Console (immediate!)
echo  [2] V2 permanent - Streamdown/Glass fix (Admin)
echo  [3] Full RTL install (Admin)
echo  [4] Restart Cursor
echo  [0] Back
echo.
choice /C 12340 /N /M "  Choose: "
if errorlevel 5 exit /b 0
if errorlevel 4 goto RESTART
if errorlevel 3 goto FULL
if errorlevel 2 goto V2
if errorlevel 1 goto TEMP

:V2
echo.
start "" "%~dp0fix-streamdown-rtl.vbs"
echo  After DONE: close ALL Cursor and reopen.
pause
exit /b 0

:FULL
echo.
start "" "%~dp0fix-hebrew-rtl-full.vbs"
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
echo  3. You should see: [RTL V2] streamdown/glass fix active
echo.
powershell -NoProfile -Command "Set-Clipboard -Value (Get-Content -LiteralPath '%USERPROFILE%\.cursor\cursor-rtl-fix.js' -Raw -Encoding UTF8)"
echo  Fix copied to clipboard (agent panel included).
pause
exit /b 0
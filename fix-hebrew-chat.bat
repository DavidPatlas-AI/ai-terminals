@echo off
chcp 65001 >nul
title Fix Hebrew RTL in Cursor
color 0B
cls
echo.
echo  ========================================================
echo     Fix reversed Hebrew in Cursor / Grok chat
echo  ========================================================
echo.
echo  This is a Cursor display bug - not your text.
echo  Permanent fix needs Administrator (once).
echo.
echo  [1] Permanent fix (recommended) - opens UAC, click Yes
echo  [2] Temporary fix (until Cursor closes)
echo.
choice /C 12 /N /M "  Choose [1] permanent  [2] temporary: "
if errorlevel 2 goto TEMP
if errorlevel 1 goto PERM

:PERM
echo.
echo  Running permanent fix...
start "" "%~dp0fix-hebrew-rtl.vbs"
echo.
echo  After you see DONE in the blue window:
echo    - Close ALL Cursor windows
echo    - Reopen Cursor
echo.
pause
exit /b 0

:TEMP
echo.
echo  Temporary fix:
echo    1. In Cursor: Ctrl+Shift+P
echo    2. Type: Developer: Toggle Developer Tools
echo    3. Console tab
echo    4. Ctrl+V then Enter
echo.
powershell -NoProfile -Command "$p='%USERPROFILE%\.cursor\extensions\satan2049.cursor-rtl-1.0.2-universal\resources\rtl.js'; if(Test-Path -LiteralPath $p){Set-Clipboard -Value (Get-Content -LiteralPath $p -Raw -Encoding UTF8); 'OK'} else {Set-Clipboard -Value (Get-Content -LiteralPath '%USERPROFILE%\.cursor\cursor-rtl-fix.js' -Raw -Encoding UTF8); 'fallback'}"
echo.
echo  Code copied to clipboard - paste in Cursor Console.
echo.
pause
exit /b 0
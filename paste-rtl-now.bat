@echo off
chcp 65001 >nul
title Paste RTL fix NOW
cls
echo.
echo  IMMEDIATE Hebrew fix - paste in Cursor DevTools Console
echo.
echo  1. Ctrl+Shift+P
echo  2. Developer: Toggle Developer Tools
echo  3. Console tab
echo  4. Ctrl+V then Enter
echo.
powershell -NoProfile -Command "Set-Clipboard -Value (Get-Content -LiteralPath '%USERPROFILE%\.cursor\cursor-rtl-fix.js' -Raw -Encoding UTF8)"
echo.
echo  RTL V2 fix copied (Streamdown/Glass chat).
pause
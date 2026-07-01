@echo off
chcp 65001 >nul
title Restart Cursor for Hebrew RTL
echo.
echo  RTL patch is installed. Closing Cursor...
echo.
taskkill /IM Cursor.exe /F >nul 2>&1
ping 127.0.0.1 -n 3 >nul
if exist "%LOCALAPPDATA%\Programs\cursor\Cursor.exe" (
    start "" "%LOCALAPPDATA%\Programs\cursor\Cursor.exe"
) else (
    start "" "C:\Program Files\cursor\Cursor.exe"
)
echo  Cursor restarted. Hebrew in chat should display correctly.
echo.
pause
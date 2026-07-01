@echo off
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo.
    echo  Run as Administrator:
    echo  Right-click enable-hebrew-rtl.bat -^> Run as administrator
    echo.
    pause
    exit /b 1
)
powershell -ExecutionPolicy Bypass -File "%~dp0enable-hebrew-rtl.ps1"
pause
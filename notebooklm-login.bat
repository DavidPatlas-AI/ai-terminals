@echo off
chcp 65001 >nul
title NotebookLM Login
cd /d "%~dp0"
set PY=%LOCALAPPDATA%\Microsoft\WindowsApps\PythonSoftwareFoundation.Python.3.12_qbz5n2kfra8p0\python.exe
if not exist "%PY%" (
    echo Python 3.12 not found. Run install-notebooklm.bat first.
    pause
    exit /b 1
)
echo   Google login for NotebookLM (one time or when expired)
echo.
"%PY%" -m notebooklm login
pause
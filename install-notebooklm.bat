@echo off
chcp 65001 >nul
title Install NotebookLM CLI
cd /d "%~dp0"
echo.
echo   ========================================
echo      NotebookLM for Terminal
echo   ========================================
echo.
echo   Step 1: Install Python package...
echo.
set PY=%LOCALAPPDATA%\Microsoft\WindowsApps\PythonSoftwareFoundation.Python.3.12_qbz5n2kfra8p0\python.exe
if not exist "%PY%" (
    echo   Python 3.12 not found. Install from Microsoft Store or python.org
    pause
    exit /b 1
)
"%PY%" -m pip install "notebooklm-py[browser]"
echo.
echo   Step 2: Install browser for login...
"%PY%" -m playwright install chromium
echo.
echo   Step 3: Google login (one time)...
echo   A browser window will open. Sign in with your Google account.
echo.
pause
"%PY%" -m notebooklm login
echo.
echo   Done! Open NotebookLM.bat to chat with your notebook.
echo   Notebook: ca413bde-2973-4125-963e-7e2eaf0ddd95
echo.
pause
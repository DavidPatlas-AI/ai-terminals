@echo off
chcp 65001 >nul
title Install Free AI Tools
cd /d "%~dp0"
echo.
echo   ========================================
echo      התקנת כלים חינמיים לטרמינל
echo   ========================================
echo.
echo   1. Gemini CLI     - already installed
echo   2. Crush          - terminal agent (free keys)
echo   3. OpenCode       - open source agent
echo   4. Ollama         - open browser to download
echo   5. All npm tools  - crush + opencode
echo   0. Back
echo.
set /p c="  Choose: "
if "%c%"=="2" goto crush
if "%c%"=="3" goto opencode
if "%c%"=="4" goto ollama
if "%c%"=="5" goto all
if "%c%"=="1" goto keys
goto end

:keys
start https://aistudio.google.com/apikey
call setup-keys.bat
goto end

:crush
echo Installing Crush...
call npm install -g @charmland/crush
goto end

:opencode
echo Installing OpenCode...
call npm install -g opencode-ai
goto end

:ollama
start https://ollama.com/download
echo After install, run: AI-Ollama.bat
pause
goto end

:all
call npm install -g @charmland/crush opencode-ai
goto end

:end
echo.
echo Next: setup-keys.bat then check-all.bat
pause
@echo off
chcp 65001 >nul
title Try Free AI Now
cd /d "%~dp0"
echo.
echo   Nothing works? Try FREE options:
echo.
echo   FASTEST (2 min):
echo     1. Get Gemini key: aistudio.google.com/apikey
echo     2. Run setup-keys.bat
echo     3. Run AI-Gemini.bat
echo.
echo   ALSO FREE:
echo     Groq key:  console.groq.com/keys  -^> AI-Crush.bat
echo     Ollama:    ollama.com/download    -^> AI-Ollama.bat (no key!)
echo.
set /p go="Open Gemini key page now? (y/n): "
if /i "%go%"=="y" start https://aistudio.google.com/apikey
call setup-keys.bat
call check-all.bat
start "" "%~dp0dashboard.html"
@echo off
chcp 65001 >nul
title Ollama Local AI
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ollama.ps1"
if errorlevel 1 pause
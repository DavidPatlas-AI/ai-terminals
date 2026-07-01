@echo off
chcp 65001 >nul
title Ollama Local AI
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model ollama
if errorlevel 1 pause

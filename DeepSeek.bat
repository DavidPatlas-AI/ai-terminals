@echo off
chcp 65001 >nul
title DeepSeek AI
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-deepseek.ps1"
pause
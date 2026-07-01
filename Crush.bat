@echo off
chcp 65001 >nul
title Crush AI
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model crush
if errorlevel 1 pause

@echo off
chcp 65001 >nul
title Claude AI
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model claude
if errorlevel 1 pause

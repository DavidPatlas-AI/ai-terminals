@echo off
chcp 65001 >nul
title OpenCode AI
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model opencode
if errorlevel 1 pause

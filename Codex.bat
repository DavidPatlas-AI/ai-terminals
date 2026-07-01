@echo off
chcp 65001 >nul
title Codex AI
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-codex.ps1"
if errorlevel 1 pause
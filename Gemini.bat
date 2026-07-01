@echo off
chcp 65001 >nul
title Gemini AI
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-gemini.ps1"
if errorlevel 1 pause
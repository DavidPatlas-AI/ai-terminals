@echo off
chcp 65001 >nul
title Crush AI
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-crush.ps1"
if errorlevel 1 pause
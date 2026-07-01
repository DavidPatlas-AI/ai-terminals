@echo off
chcp 65001 >nul
title Grok AI
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-grok.ps1"
if errorlevel 1 pause
@echo off
chcp 65001 >nul
title DeepSeek API Key Setup
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup-keys.ps1"
pause
@echo off
chcp 65001 >nul
title AI Work Folder Shortcuts
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0setup-work-shortcuts.ps1"
pause
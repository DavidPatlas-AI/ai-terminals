@echo off
chcp 65001 >nul
title NotebookLM
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-notebooklm.ps1"
if errorlevel 1 pause
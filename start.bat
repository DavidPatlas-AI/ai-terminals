@echo off
chcp 65001 >nul
title AI Terminals
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0hub.ps1"
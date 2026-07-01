@echo off
chcp 65001 >nul
title מודלים טרמינלים
cd /d "%~dp0"
powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0hub.ps1"
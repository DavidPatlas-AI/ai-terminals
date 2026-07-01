@echo off
chcp 65001 >nul
title AI Chats - All
cd /d "%~dp0"

where wt >nul 2>&1
if %errorlevel% neq 0 (
    echo Windows Terminal not found. Opening separate PowerShell windows...
    start "Grok" powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model grok
    start "Claude" powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model claude
    start "Gemini" powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model gemini
    start "DeepSeek" powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model deepseek
    start "Codex" powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model codex
    exit /b
)

wt -w 0 new-tab --title "Grok" powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model grok ; new-tab --title "Claude" powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model claude ; new-tab --title "Gemini" powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model gemini ; new-tab --title "DeepSeek" powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model deepseek ; new-tab --title "Codex" powershell -NoExit -NoProfile -ExecutionPolicy Bypass -File "%~dp0launch-ai.ps1" -Model codex

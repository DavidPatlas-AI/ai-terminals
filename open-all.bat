@echo off
chcp 65001 >nul
title AI Chats - All
cd /d "%~dp0"

where wt >nul 2>&1
if %errorlevel% neq 0 (
    echo Windows Terminal not found. Opening Grok only...
    call "%~dp0Grok.bat"
    exit /b
)

wt -w 0 new-tab -p Grok ; new-tab -p Claude ; new-tab -p Gemini ; new-tab -p DeepSeek ; new-tab -p Codex
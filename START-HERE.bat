@echo off
chcp 65001 >nul
title AI Terminals — Start Here
cd /d "%~dp0"

:menu
cls
echo.
echo   ========================================================
echo        AI Terminals — התחל כאן
echo   ========================================================
echo.
echo   הכל בתיקייה הזאת — שולחן עבודה נקי
echo     • מודלים טרמנילים  (כאן)
echo     • shortcuts\        קיצורים בפנים
echo     • AI-Terminals      junction (שם אנגלי)
echo.
echo   --------------------------------------------------------
echo   A  הכל ביחד — סידור מלא (מומלץ!)
echo   1  נקה שולחן עבודה + shortcuts\
echo   2  פתח תפריט מודלים (AI Hub)
echo   3  פתח דשבורד (טוקנים ושיחות)
echo   4  הגדר מפתחות API
echo   5  NotebookLM — התקנה / התחברות
echo   6  בדיקת חיבור מלאה
echo   7  תיקון עברית הפוכה ב-Cursor
echo   W  קיצורי דרך לתיקיות עבודה (Codex, Grok...)
echo   --------------------------------------------------------
echo   G  פרסום ל-GitHub (ציבורי)
echo   S  בדיקת אבטחה לפני פרסום
echo   M  הסבר מלא (MAP.txt)
echo   H  הוראות (הוראות.txt)
echo   0  יציאה
echo.
set /p c="  בחר: "

if /i "%c%"=="A" goto all
if "%c%"=="1" goto desktop
if "%c%"=="2" goto hub
if "%c%"=="3" goto dash
if "%c%"=="4" goto keys
if "%c%"=="5" goto nblm
if "%c%"=="6" goto check
if "%c%"=="7" goto hebrew
if /i "%c%"=="W" goto work
if /i "%c%"=="G" goto publish
if /i "%c%"=="S" goto security
if /i "%c%"=="M" goto map
if /i "%c%"=="H" goto help
if "%c%"=="0" exit /b 0
goto menu

:all
call setup-all.bat
goto menu

:desktop
call setup-desktop.bat
goto menu

:hub
call start.bat
goto menu

:dash
call open-dashboard.bat
goto menu

:keys
call setup-keys.bat
goto menu

:nblm
echo.
echo   install-notebooklm.bat  — התקנה + Google login (פעם אחת)
echo   notebooklm-login.bat    — התחברות מחדש
echo   NotebookLM.bat          — צ'אט עם המחברת
echo.
set /p n="  I=install  L=login  N=chat  Enter=back: "
if /i "%n%"=="I" call install-notebooklm.bat
if /i "%n%"=="L" call notebooklm-login.bat
if /i "%n%"=="N" call NotebookLM.bat
goto menu

:check
call check-all.bat
pause
goto menu

:hebrew
call fix-hebrew-chat.bat
goto menu

:work
call setup-work-shortcuts.bat
goto menu

:security
call security-check.bat
pause
goto menu

:publish
call publish-github.bat
goto menu

:map
notepad "%~dp0MAP.txt"
goto menu

:help
notepad "%~dp0הוראות.txt"
goto menu
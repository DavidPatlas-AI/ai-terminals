@echo off
chcp 65001 >nul
echo.
echo  ══ השלמת פרופיל GitHub — DavidPatlas-AI ══
echo.
echo  1. GitHub Sponsors (הרשמה + Stripe)
echo  2. נעיצת 6 repos בפרופיל
echo  3. Buy Me a Coffee (יצירת חשבון)
echo  4. פרופיל GitHub
echo  5. תיק עבודות חי
echo.
choice /C 12345 /N /M "בחר מספר (1-5) או Esc לביטול: "
if errorlevel 5 start https://storied-alfajores-6f10d2.netlify.app/portfolio.html & goto end
if errorlevel 4 start https://github.com/DavidPatlas-AI & goto end
if errorlevel 3 start https://buymeacoffee.com/signup & goto end
if errorlevel 2 start https://github.com/DavidPatlas-AI?tab=repositories & goto end
if errorlevel 1 start https://github.com/sponsors & goto end
:end
echo.
pause
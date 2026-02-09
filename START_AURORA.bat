@echo off
title AURORA SPOT NEXUS - Launcher
echo ============================================
echo   AURORA SPOT NEXUS - Starting...
echo ============================================
echo.

:: Start Bot in background
echo [1/2] Starting Bot...
start "AURORA-BOT" /D "C:\Users\COOL\Desktop\sbot\6\backend" "C:\Users\COOL\Desktop\sbot\6\backend\target\release\aurora-bot.exe"
timeout /t 3 >nul

:: Start Dashboard
echo [2/2] Starting Dashboard...
start "AURORA-DASHBOARD" /D "C:\Users\COOL\Desktop\sbot\6\dashboard_v2" cmd /c "npm run dev"
timeout /t 5 >nul

echo.
echo ============================================
echo   Bot:       http://127.0.0.1:3002
echo   Dashboard: http://localhost:5173
echo ============================================
echo.
echo Opening dashboard in browser...
start http://localhost:5173

echo.
echo Press any key to STOP both services...
pause >nul

:: Stop both
taskkill /FI "WINDOWTITLE eq AURORA-BOT" /F 2>nul
taskkill /FI "WINDOWTITLE eq AURORA-DASHBOARD" /F 2>nul
echo Stopped.

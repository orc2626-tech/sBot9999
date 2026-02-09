# ============================================
#   AURORA SPOT NEXUS - PowerShell Launcher
# ============================================

Write-Host ""
Write-Host "  AURORA SPOT NEXUS" -ForegroundColor Cyan
Write-Host "  =================" -ForegroundColor Cyan
Write-Host ""

# Start Bot
Write-Host "[1/2] Starting Bot..." -ForegroundColor Yellow
$bot = Start-Process -FilePath "C:\Users\COOL\Desktop\sbot\6\backend\target\release\aurora-bot.exe" `
    -WorkingDirectory "C:\Users\COOL\Desktop\sbot\6\backend" `
    -PassThru -WindowStyle Normal
Write-Host "  Bot PID: $($bot.Id)" -ForegroundColor Green
Start-Sleep -Seconds 3

# Start Dashboard
Write-Host "[2/2] Starting Dashboard..." -ForegroundColor Yellow
$dash = Start-Process -FilePath "cmd.exe" `
    -ArgumentList "/c cd /d C:\Users\COOL\Desktop\sbot\6\dashboard_v2 && npm run dev" `
    -PassThru -WindowStyle Normal
Write-Host "  Dashboard PID: $($dash.Id)" -ForegroundColor Green
Start-Sleep -Seconds 5

# Open browser
Write-Host ""
Write-Host "  Bot API:   http://127.0.0.1:3002" -ForegroundColor Cyan
Write-Host "  Dashboard: http://localhost:5173" -ForegroundColor Cyan
Write-Host ""
Start-Process "http://localhost:5173"

Write-Host "Press ENTER to stop both services..." -ForegroundColor Red
Read-Host

# Cleanup
Stop-Process -Id $bot.Id -Force -ErrorAction SilentlyContinue
Stop-Process -Id $dash.Id -Force -ErrorAction SilentlyContinue
Get-Process -Name "aurora-bot" -ErrorAction SilentlyContinue | Stop-Process -Force
Write-Host "Stopped." -ForegroundColor Green

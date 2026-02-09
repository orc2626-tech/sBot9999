# Free the bot API port (default 3001) so you can start the bot again.
# Usage: .\stop-bot.ps1   or   .\stop-bot.ps1 -Port 3002

param([int]$Port = 3001)

$conn = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
if (-not $conn) {
    Write-Host "Port $Port is not in use. You can start the bot." -ForegroundColor Green
    exit 0
}
foreach ($c in $conn) {
    $pid = $c.OwningProcess
    $proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
    $name = if ($proc) { $proc.ProcessName } else { "PID $pid" }
    Write-Host "Stopping $name (PID $pid) on port $Port..." -ForegroundColor Yellow
    Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue
}
Write-Host "Done. You can start the bot now." -ForegroundColor Green

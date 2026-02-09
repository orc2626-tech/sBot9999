# Build Aurora Bot — stops running bot if needed, then builds release
# Usage: .\build.ps1

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot
$BackendDir = Join-Path $ProjectRoot "backend"
$BotExe = Join-Path $BackendDir "target\release\aurora-bot.exe"

# Read backend port from .env
$apiPort = "3001"
$backendEnv = Join-Path $BackendDir ".env"
if (Test-Path $backendEnv) {
    $envContent = Get-Content $backendEnv -Raw -ErrorAction SilentlyContinue
    if ($envContent -match "AURORA_API_LISTEN=[^:]+:(\d+)") { $apiPort = $matches[1] }
}

# 1) Kill process using API port (often the bot)
$conn = Get-NetTCPConnection -LocalPort $apiPort -State Listen -ErrorAction SilentlyContinue
if ($conn) {
    foreach ($c in $conn) {
        $pid = $c.OwningProcess
        $proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
        $name = if ($proc) { $proc.ProcessName } else { "PID $pid" }
        Write-Host "Stopping process on port $apiPort : $name (PID $pid)"
        Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Seconds 2
}

# 2) Kill any aurora-bot.exe by name (in case it runs on another port)
Get-Process -Name "aurora-bot" -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "Stopping aurora-bot (PID $($_.Id))"
    Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 1

# 3) Build
Set-Location $BackendDir
Write-Host "Building release..."
cargo build --release
$exit = $LASTEXITCODE
Set-Location $ProjectRoot
if ($exit -ne 0) { exit $exit }
Write-Host "Build OK: backend\target\release\aurora-bot.exe"

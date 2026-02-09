# Aurora Bot + Dashboard - Unified Start
# Bot: backend\target\release\aurora-bot.exe
# Dashboard: dashboard_v2 + npm run dev
# Usage: .\start.ps1

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot
$BackendDir = Join-Path $ProjectRoot "backend"
$DashboardDir = Join-Path $ProjectRoot "dashboard_v2"
$BotExe = Join-Path $BackendDir "target\release\aurora-bot.exe"

Write-Host ""
Write-Host "  Aurora Spot Bot - Unified Start" -ForegroundColor Cyan
Write-Host "  ================================" -ForegroundColor Cyan
Write-Host ""

# 1) Check backend .env
$backendEnv = Join-Path $BackendDir ".env"
$backendExample = Join-Path $BackendDir ".env.example"
if (-not (Test-Path $backendEnv)) {
    if (Test-Path $backendExample) {
        Copy-Item $backendExample $backendEnv
        Write-Host "  [OK] Created backend\.env from .env.example - set AURORA_ADMIN_TOKEN." -ForegroundColor Yellow
    } else {
        Write-Host "  [ERROR] No backend\.env or .env.example" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "  [OK] backend\.env exists" -ForegroundColor Green
}

# 2) Check release binary exists
if (-not (Test-Path $BotExe)) {
    Write-Host "  [ERROR] Bot binary not found: backend\target\release\aurora-bot.exe" -ForegroundColor Red
    Write-Host "  Build first from project root:" -ForegroundColor Yellow
    Write-Host "    cd backend; cargo build --release" -ForegroundColor White
    Write-Host ""
    exit 1
}
Write-Host "  [OK] Bot binary: target\release\aurora-bot.exe" -ForegroundColor Green

# 3) Check dashboard .env
$dashboardEnv = Join-Path $DashboardDir ".env"
$dashboardExample = Join-Path $DashboardDir ".env.example"
if (-not (Test-Path $dashboardEnv)) {
    if (Test-Path $dashboardExample) {
        Copy-Item $dashboardExample $dashboardEnv
        Write-Host "  [OK] Created dashboard_v2\.env - set VITE_API_URL and VITE_ADMIN_TOKEN." -ForegroundColor Yellow
    }
}
if (Test-Path $dashboardEnv) {
    Write-Host "  [OK] dashboard_v2\.env exists" -ForegroundColor Green
}

# 4) API port from backend .env (default 3001)
$apiListen = "127.0.0.1:3001"
$envContent = Get-Content $backendEnv -Raw -ErrorAction SilentlyContinue
if ($envContent -match "AURORA_API_LISTEN=(.+)") {
    $apiListen = $matches[1].Trim()
}
$configDir = "."
if ($envContent -match "AURORA_CONFIG_DIR=(.+)") {
    $configDir = $matches[1].Trim()
}
$runtimeConfigPath = if ($configDir -eq "." -or [string]::IsNullOrWhiteSpace($configDir)) {
    Join-Path $BackendDir "runtime_config.json"
} else {
    Join-Path $configDir "runtime_config.json"
}

# 4a) Force Demo + Paused at startup (AccountMode = demo, TradingMode = PAUSED)
if (Test-Path $runtimeConfigPath) {
    try {
        $rc = Get-Content $runtimeConfigPath -Raw | ConvertFrom-Json
        $rc | Add-Member -NotePropertyName account_mode -NotePropertyValue "demo" -Force
        $rc | Add-Member -NotePropertyName trading_mode -NotePropertyValue "PAUSED" -Force
        $rc.account_mode = "demo"
        $rc.trading_mode = "PAUSED"
        ($rc | ConvertTo-Json -Depth 20) | Set-Content -Path $runtimeConfigPath -Encoding UTF8
        Write-Host "  [OK] Startup account_mode=demo and trading_mode=PAUSED forced (runtime_config.json)" -ForegroundColor Yellow
    } catch {
        Write-Host "  [WARN] Could not update runtime_config.json (startup safety). Starting anyway." -ForegroundColor Yellow
    }
}
$healthUrl = "http://" + $apiListen + "/api/v1/health"
$healthUrl = $healthUrl -replace "0\.0\.0\.0", "127.0.0.1"
$apiPort = if ($apiListen -match ":(\d+)$") { $matches[1] } else { "3001" }
$apiHost = $apiListen -replace "0\.0\.0\.0", "127.0.0.1"
$dashboardBaseUrl = "http://" + $apiHost

# Sync dashboard .env VITE_API_URL with backend port (so Arena and API match)
if (Test-Path $dashboardEnv) {
    $dashLines = Get-Content $dashboardEnv -ErrorAction SilentlyContinue
    $found = $false
    $dashLines = $dashLines | ForEach-Object {
        if ($_ -match "^VITE_API_URL=") { $found = $true; "VITE_API_URL=" + $dashboardBaseUrl }
        else { $_ }
    }
    if (-not $found) { $dashLines += "VITE_API_URL=" + $dashboardBaseUrl }
    $dashLines | Set-Content -Path $dashboardEnv -Encoding UTF8
}

# 4b) Free port if already in use (e.g. previous bot instance)
$conn = Get-NetTCPConnection -LocalPort $apiPort -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
if ($conn) {
    $pid = $conn.OwningProcess
    $proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
    $procName = if ($proc) { $proc.ProcessName } else { "PID $pid" }
    Write-Host "  [INFO] Port $apiPort in use by $procName (PID $pid) - stopping it." -ForegroundColor Yellow
    Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
}

# 5) Start bot in new window (from backend dir, run release binary)
Write-Host ""
Write-Host "  Starting bot (new window): backend\target\release\aurora-bot.exe" -ForegroundColor Cyan
$botCmd = "Set-Location '" + $BackendDir + "'; .\target\release\aurora-bot.exe"
Start-Process powershell -ArgumentList "-NoExit", "-Command", $botCmd -WindowStyle Normal
Start-Sleep -Seconds 3

# 6) Wait for API ready (up to 30 seconds)
$maxAttempts = 30
$attempt = 0
$ready = $false
while ($attempt -lt $maxAttempts) {
    try {
        $r = Invoke-WebRequest -Uri $healthUrl -UseBasicParsing -TimeoutSec 2 -ErrorAction SilentlyContinue
        if ($r.StatusCode -eq 200) {
            $ready = $true
            break
        }
    } catch {
        # expected until backend starts
    }
    $attempt++
    Write-Host "  Waiting for API... ($attempt/$maxAttempts)" -ForegroundColor Gray
    Start-Sleep -Seconds 1
}

if (-not $ready) {
    Write-Host ""
    Write-Host "  [WARN] Bot did not respond in 30s. Check bot window, then run dashboard manually." -ForegroundColor Yellow
} else {
    Write-Host "  [OK] Bot is up - $healthUrl" -ForegroundColor Green
}

# 7) Start dashboard in current window
Write-Host ""
Write-Host "  Starting dashboard (this window - Ctrl+C to stop)..." -ForegroundColor Cyan
Write-Host "  Open browser: http://localhost:5173" -ForegroundColor White
Write-Host ""

Set-Location $DashboardDir
if (-not (Test-Path "node_modules")) {
    Write-Host "  Installing dashboard deps (npm install)..." -ForegroundColor Gray
    npm install
}
npm run dev

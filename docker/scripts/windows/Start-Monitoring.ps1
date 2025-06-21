# Windows start script
param (
    [switch]$Detached = $true
)

Write-Host "🚀 Starting monitoring stack..." -ForegroundColor Cyan

# Check if .env exists
if (-not (Test-Path .\.env)) {
    Write-Host "❌ Error: .env not found. Run setup.ps1 first." -ForegroundColor Red
    exit 1
}

# Load environment variables
Get-Content .\.env | ForEach-Object {
    $name, $value = $_.Split('=', 2)
    if ($name -and $value) {
        Set-Item -Path "env:$name" -Value $value
    }
}

# Start the stack
$composeFiles = @("-f", "docker-compose.base.yml", "-f", "docker-compose.windows.yml")
$detachArg = if ($Detached) { "-d" } else { "" }

Write-Host "Starting containers..." -ForegroundColor Cyan
docker-compose @composeFiles up $detachArg

if ($Detached) {
    # Wait for containers to be healthy
    $timeout = 30
    $interval = 2
    $elapsed = 0
    $healthy = $false

    Write-Host "⏳ Waiting for containers to be healthy..." -ForegroundColor Cyan

    while (-not $healthy -and $elapsed -lt $timeout) {
        Start-Sleep -Seconds $interval
        $elapsed += $interval
        
        $unhealthy = docker ps --filter "health=unhealthy" --format '{{.Names}}'
        $starting = docker ps --filter "health=starting" --format '{{.Names}}'
        
        if (-not $unhealthy -and -not $starting) {
            $healthy = $true
        }
    }

    if (-not $healthy) {
        Write-Host "⚠️  Some containers may not be fully healthy. Check with 'docker ps'" -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "📊 Grafana: http://localhost:3000" -ForegroundColor Green
    Write-Host "   - User: $env:GRAFANA_ADMIN_USER"
    if ($env:GRAFANA_ADMIN_PASSWORD) {
        Write-Host "   - Password: $env:GRAFANA_ADMIN_PASSWORD"
    }
    Write-Host ""
    Write-Host "📈 InfluxDB: http://localhost:8086" -ForegroundColor Green
    Write-Host "   - User: $env:INFLUXDB_USER"
    if ($env:INFLUXDB_PASSWORD) {
        Write-Host "   - Password: $env:INFLUXDB_PASSWORD"
    }
    Write-Host ""
    Write-Host "🔍 Prometheus: http://localhost:9090" -ForegroundColor Green
    Write-Host ""
    Write-Host "✅ Stack started successfully!" -ForegroundColor Green
}

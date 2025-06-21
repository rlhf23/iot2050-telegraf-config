# Windows setup script
Write-Host "🚀 Setting up monitoring stack..." -ForegroundColor Cyan

# Create required directories
$directories = @(
    "telegraf",
    "config\grafana\provisioning",
    "config\prometheus"
)

foreach ($dir in $directories) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        Write-Host "✅ Created directory: $dir"
    }
}

# Check if .env exists
if (-not (Test-Path .\.env)) {
    Write-Host "ℹ️ .env file not found. Generating new credentials..." -ForegroundColor Yellow
    
    # Generate random credentials
    $influxdbPass = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 16 | ForEach-Object {[char]$_})
    $grafanaPass = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 16 | ForEach-Object {[char]$_})
    $influxdbToken = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
    $telegrafToken = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | ForEach-Object {[char]$_})
    
    # Create .env file
    @"
# InfluxDB
INFLUXDB_USER=admin
INFLUXDB_PASSWORD=$influxdbPass
INFLUXDB_ORG=iot2050
INFLUXDB_BUCKET=telegraf
INFLUXDB_TOKEN=$influxdbToken

# Grafana
GRAFANA_ADMIN_USER=admin
GRAFANA_ADMIN_PASSWORD=$grafanaPass

# Telegraf
TELEGRAF_TOKEN=$telegrafToken
"@ | Out-File -FilePath .\.env -Encoding utf8
    
    Write-Host "✅ Created .env file with generated credentials" -ForegroundColor Green
} else {
    Write-Host "ℹ️ .env file already exists" -ForegroundColor Green
}

# Copy Telegraf config if example exists
if (Test-Path "config\telegraf\telegraf.conf.example") {
    if (-not (Test-Path "telegraf\telegraf.conf")) {
        Copy-Item "config\telegraf\telegraf.conf.example" "telegraf\telegraf.conf"
        Write-Host "✅ Copied Telegraf config from example" -ForegroundColor Green
    }
} else {
    Write-Host "⚠️  WARNING: config\telegraf\telegraf.conf.example not found!" -ForegroundColor Yellow
    Write-Host "Telegraf will use default configuration"
}

Write-Host "✅ Setup complete!" -ForegroundColor Green
Write-Host "Run 'Start-Monitoring.ps1' to start the stack" -ForegroundColor Cyan

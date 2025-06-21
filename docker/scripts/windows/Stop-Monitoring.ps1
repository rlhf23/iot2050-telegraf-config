# Windows stop script
Write-Host "🛑 Stopping monitoring stack..." -ForegroundColor Cyan

# Load environment variables if .env exists
if (Test-Path .\.env) {
    Get-Content .\.env | Where-Object { $_ -and -not $_.StartsWith('#') } | ForEach-Object {
        $name, $value = $_.Split('=', 2)
        if ($name -and $value) {
            Set-Item -Path "env:$name" -Value $value
        }
    }
}

# Stop the stack
docker-compose -f docker-compose.base.yml -f docker-compose.windows.yml down

Write-Host "✅ Stack stopped successfully!" -ForegroundColor Green

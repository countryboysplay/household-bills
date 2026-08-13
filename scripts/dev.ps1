$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

if (-not (Test-Path "node_modules")) {
    Write-Host "node_modules is missing. Running npm install..." -ForegroundColor Yellow
    npm install
}

npm run dev

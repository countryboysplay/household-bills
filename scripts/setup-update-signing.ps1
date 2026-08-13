param(
    [string]$KeyPath = "$env:USERPROFILE\.tauri\household-bills.key"
)
$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path $PSScriptRoot -Parent
Set-Location $ProjectRoot

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw "npm was not found. Run scripts\setup-dev.ps1 first."
}

# The Tauri CLI is a project-local dev dependency. A freshly extracted source ZIP
# does not include node_modules, so install dependencies before invoking the signer.
$tauriCmd = Join-Path $ProjectRoot "node_modules\.bin\tauri.cmd"
if (-not (Test-Path $tauriCmd)) {
    Write-Host "Installing project dependencies required for the Tauri signer..." -ForegroundColor Cyan
    npm install
    if ($LASTEXITCODE -ne 0) { throw "npm install failed with exit code $LASTEXITCODE." }
}

if (-not (Test-Path $tauriCmd)) {
    throw "The project-local Tauri CLI was not found after npm install."
}

$keyDir = Split-Path $KeyPath -Parent
New-Item -ItemType Directory -Force -Path $keyDir | Out-Null

if (Test-Path $KeyPath) {
    Write-Host "Signing key already exists: $KeyPath" -ForegroundColor Yellow
    Write-Host "Do not replace this key after updater-enabled releases are installed." -ForegroundColor Yellow
} else {
    Write-Host "Generating the permanent Household Bills updater signing key..." -ForegroundColor Cyan
    Write-Host "Choose a password you can store safely. Losing the private key prevents future automatic updates." -ForegroundColor Yellow
    & $tauriCmd signer generate -w $KeyPath
    if ($LASTEXITCODE -ne 0) { throw "Tauri signing-key generation failed." }
}

$publicCandidates = @("$KeyPath.pub", [IO.Path]::ChangeExtension($KeyPath, ".pub")) | Select-Object -Unique
$publicPath = $publicCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $publicPath) {
    Write-Warning "The public-key file was not found automatically. Use the public key printed by the Tauri signer when running configure-updater.ps1."
} else {
    Write-Host ""; Write-Host "Private key: $KeyPath" -ForegroundColor Green
    Write-Host "Public key:  $publicPath" -ForegroundColor Green
    Write-Host ""; Write-Host "PUBLIC KEY (safe to place in the app):" -ForegroundColor Cyan
    Get-Content $publicPath
}
Write-Host ""; Write-Host "NEVER commit the private .key file to GitHub." -ForegroundColor Red

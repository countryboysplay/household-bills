param(
    [Parameter(Mandatory=$true)][string]$GitHubOwner,
    [string]$GitHubRepo = "household-bills",
    [string]$PublicKeyPath = "$env:USERPROFILE\.tauri\household-bills.key.pub",
    [string]$PublicKey
)
$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path $PSScriptRoot -Parent
$configPath = Join-Path $ProjectRoot "src-tauri\tauri.conf.json"
if (-not (Test-Path $configPath)) { throw "tauri.conf.json was not found." }

if (-not $PublicKey) {
    if (-not (Test-Path $PublicKeyPath)) {
        $alt = [IO.Path]::ChangeExtension("$env:USERPROFILE\.tauri\household-bills.key", ".pub")
        if (Test-Path $alt) { $PublicKeyPath = $alt }
        else { throw "Public key was not found. Pass -PublicKey or -PublicKeyPath." }
    }
    $PublicKey = (Get-Content $PublicKeyPath -Raw).Trim()
}
if (-not $PublicKey) { throw "Public key is empty." }

$config = Get-Content $configPath -Raw | ConvertFrom-Json
$config.plugins.updater.pubkey = $PublicKey
$config.plugins.updater.endpoints = @("https://github.com/$GitHubOwner/$GitHubRepo/releases/latest/download/latest.json")
$json = $config | ConvertTo-Json -Depth 30
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($configPath, $json, $utf8NoBom)

Write-Host "Updater configured." -ForegroundColor Green
Write-Host "Repository: https://github.com/$GitHubOwner/$GitHubRepo" -ForegroundColor Cyan
Write-Host "Endpoint:   https://github.com/$GitHubOwner/$GitHubRepo/releases/latest/download/latest.json" -ForegroundColor Cyan
Write-Host "Run .\scripts\test.ps1 before committing." -ForegroundColor Yellow
param([Parameter(Mandatory=$true)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version)
$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent

$packagePath = Join-Path $Root "package.json"
$package = Get-Content $packagePath -Raw | ConvertFrom-Json
$package.version = $Version
$package | ConvertTo-Json -Depth 20 | Set-Content $packagePath -Encoding UTF8

$tauriPath = Join-Path $Root "src-tauri\tauri.conf.json"
$tauri = Get-Content $tauriPath -Raw | ConvertFrom-Json
$tauri.version = $Version
$tauri | ConvertTo-Json -Depth 30 | Set-Content $tauriPath -Encoding UTF8

$cargoPath = Join-Path $Root "src-tauri\Cargo.toml"
$cargo = Get-Content $cargoPath -Raw
$cargo = [regex]::Replace($cargo, '(?m)^(version\s*=\s*")([^"]+)(")', ('${1}' + $Version + '${3}'), 1)
Set-Content $cargoPath $cargo -Encoding UTF8

Write-Host "Household Bills version set to $Version in package.json, tauri.conf.json, and Cargo.toml." -ForegroundColor Green

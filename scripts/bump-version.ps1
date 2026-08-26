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
# Only the [package] version is rewritten, so replace the FIRST match and no more.
# [regex]::Replace has no static overload taking a count -- passing one binds it to
# the RegexOptions parameter (1 = IgnoreCase) and silently rewrites every match.
# The count argument exists only on the instance method.
$cargoVersionPattern = [regex]'(?m)^(version\s*=\s*")([^"]+)(")'
if (-not $cargoVersionPattern.IsMatch($cargo)) { throw "Could not find a package version line in $cargoPath." }
$cargo = $cargoVersionPattern.Replace($cargo, ('${1}' + $Version + '${3}'), 1)
Set-Content $cargoPath $cargo -Encoding UTF8

Write-Host "Household Bills version set to $Version in package.json, tauri.conf.json, and Cargo.toml." -ForegroundColor Green

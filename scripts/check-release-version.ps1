$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root
$current = [version]((Get-Content package.json -Raw | ConvertFrom-Json).version)
$lines = @(git ls-remote --tags origin "refs/tags/v*")
if ($LASTEXITCODE -ne 0) { throw "Could not read release tags from origin." }
$versions = @()
foreach ($line in $lines) {
    if ($line -match 'refs/tags/v(\d+\.\d+\.\d+)$') { $versions += [version]$Matches[1] }
}
if ($versions.Count -eq 0) {
    Write-Host "No previous stable release tag exists. $current is valid for the first release." -ForegroundColor Green
    exit 0
}
$latest = $versions | Sort-Object -Descending | Select-Object -First 1
if ($current -le $latest) {
    throw "Version $current must be greater than latest stable release $latest before merging to main. Run scripts\bump-version.ps1 with a newer version."
}
Write-Host "Release version gate passed: $current > $latest" -ForegroundColor Green

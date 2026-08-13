param(
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path $PSScriptRoot -Parent
Set-Location $ProjectRoot

function Assert-NativeSuccess {
    param([string]$Step)
    if ($LASTEXITCODE -ne 0) {
        throw "$Step failed with exit code $LASTEXITCODE."
    }
}

Write-Host "Household Bills release build" -ForegroundColor Cyan
Write-Host "Project: $ProjectRoot" -ForegroundColor DarkGray

foreach ($tool in @("node", "npm", "cargo", "rustc")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "$tool is not available in PATH. Run scripts\setup-dev.ps1 first."
    }
}

if (-not (Test-Path "node_modules")) {
    Write-Host "Installing frontend dependencies..." -ForegroundColor Cyan
    npm install
    Assert-NativeSuccess "npm install"
}

if (-not $SkipTests) {
    & "$PSScriptRoot\test.ps1"
}

$package = Get-Content "package.json" -Raw | ConvertFrom-Json
$version = $package.version
$releaseDir = Join-Path $ProjectRoot "release"
if (Test-Path $releaseDir) {
    Remove-Item $releaseDir -Recurse -Force
}
New-Item -ItemType Directory -Path $releaseDir | Out-Null

Write-Host "Building optimized NSIS installer for Household Bills $version..." -ForegroundColor Cyan
npm run build
Assert-NativeSuccess "Tauri release build"

$bundleDir = Join-Path $ProjectRoot "src-tauri\target\release\bundle\nsis"
if (-not (Test-Path $bundleDir)) {
    throw "Tauri completed but the NSIS output directory was not found: $bundleDir"
}

$sourceInstaller = Get-ChildItem $bundleDir -Filter "*.exe" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (-not $sourceInstaller) {
    throw "No NSIS setup executable was produced."
}

$finalName = "Household Bills Setup $version.exe"
$finalPath = Join-Path $releaseDir $finalName
Copy-Item $sourceInstaller.FullName $finalPath -Force

$hash = Get-FileHash $finalPath -Algorithm SHA256
$size = (Get-Item $finalPath).Length
$builtAt = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss K")
$manifest = @(
    "Household Bills Release Manifest",
    "Version: $version",
    "Built: $builtAt",
    "Installer: $finalName",
    "SizeBytes: $size",
    "SHA256: $($hash.Hash)",
    "Architecture: $env:PROCESSOR_ARCHITECTURE Windows",
    "InstallerType: NSIS current-user",
    "DataLocation: Windows application-data directory (outside install directory)",
    "Signing: Unsigned unless a Windows code-signing identity was configured externally"
) -join [Environment]::NewLine
Set-Content -Path (Join-Path $releaseDir "RELEASE_MANIFEST.txt") -Value $manifest -Encoding UTF8

Copy-Item "RELEASE_TEST_CHECKLIST.md" (Join-Path $releaseDir "RELEASE_TEST_CHECKLIST.md") -Force
Copy-Item "RELEASE_NOTES_1.0.0.md" (Join-Path $releaseDir "RELEASE_NOTES_1.0.0.md") -Force

Write-Host ""
Write-Host "Release build complete." -ForegroundColor Green
Write-Host "Installer: $finalPath" -ForegroundColor Green
Write-Host "SHA256:   $($hash.Hash)" -ForegroundColor Green
Write-Host ""
Write-Host "Run the installer test in RELEASE_TEST_CHECKLIST.md before treating this build as final." -ForegroundColor Yellow

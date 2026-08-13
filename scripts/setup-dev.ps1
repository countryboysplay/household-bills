$ErrorActionPreference = "Stop"

Write-Host "Household Bills - Windows development setup" -ForegroundColor Cyan
Write-Host "This script prepares a development machine. End users will not need these tools." -ForegroundColor DarkGray

$ProjectRoot = Split-Path -Parent $PSScriptRoot

function Require-Winget {
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        throw "winget was not found. Install App Installer from Microsoft, then rerun this script."
    }
}

function Refresh-ProcessPath {
    $machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $env:Path = "$machinePath;$userPath"
}

function Require-Command {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Help
    )

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name is still unavailable after installation. $Help"
    }
}

Require-Winget

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Node.js LTS..."
    winget install -e --id OpenJS.NodeJS.LTS --accept-package-agreements --accept-source-agreements
    if ($LASTEXITCODE -ne 0) {
        throw "Node.js installation failed with exit code $LASTEXITCODE."
    }
    Refresh-ProcessPath
}

Require-Command -Name "node" -Help "Close PowerShell, open a new PowerShell window, and rerun this script."
Require-Command -Name "npm" -Help "Close PowerShell, open a new PowerShell window, and rerun this script."

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Rustup..."
    winget install -e --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements
    if ($LASTEXITCODE -ne 0) {
        throw "Rustup installation failed with exit code $LASTEXITCODE."
    }
    Refresh-ProcessPath
}

Require-Command -Name "rustup" -Help "Close PowerShell, open a new PowerShell window, and rerun this script."

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vsSetup = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\setup.exe"

function Find-MSVCInstallation {
    if (-not (Test-Path $vswhere)) {
        return $null
    }

    $path = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
    if ($path) {
        return ($path | Select-Object -First 1).Trim()
    }
    return $null
}

$msvcInstall = Find-MSVCInstallation
if (-not $msvcInstall) {
    Write-Host "Microsoft C++ Build Tools workload was not detected." -ForegroundColor Yellow
    Write-Host "Installing Visual Studio 2022 Build Tools with Desktop C++ support..."

    winget install -e --id Microsoft.VisualStudio.2022.BuildTools --accept-package-agreements --accept-source-agreements --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --norestart"

    # winget may report that Build Tools already exists without adding the workload.
    # If that happens, explicitly modify the existing Visual Studio instance.
    if (Test-Path $vswhere) {
        $msvcInstall = Find-MSVCInstallation
        if (-not $msvcInstall -and (Test-Path $vsSetup)) {
            $existingInstall = & $vswhere -latest -products * -property installationPath 2>$null | Select-Object -First 1
            if ($existingInstall) {
                $existingInstall = $existingInstall.Trim()
                Write-Host "Adding the C++ workload to the existing Visual Studio Build Tools installation..." -ForegroundColor Yellow
                $args = @(
                    "modify",
                    "--installPath", "`"$existingInstall`"",
                    "--add", "Microsoft.VisualStudio.Workload.VCTools",
                    "--includeRecommended",
                    "--passive",
                    "--norestart"
                )
                $process = Start-Process -FilePath $vsSetup -ArgumentList $args -Verb RunAs -Wait -PassThru
                if ($process.ExitCode -ne 0) {
                    throw "Visual Studio Installer modify operation failed with exit code $($process.ExitCode)."
                }
            }
        }
    }

    Refresh-ProcessPath
    $msvcInstall = Find-MSVCInstallation
    if (-not $msvcInstall) {
        throw "Microsoft C++ Build Tools are still missing. Open Visual Studio Installer, modify Build Tools, select 'Desktop development with C++', install it, then rerun this script."
    }
}

Write-Host "MSVC Build Tools detected at: $msvcInstall" -ForegroundColor DarkCyan

Write-Host "Updating stable Rust toolchain..."
rustup default stable
rustup update stable
Refresh-ProcessPath

Require-Command -Name "cargo" -Help "Close PowerShell, open a new PowerShell window, and rerun this script."
Require-Command -Name "rustc" -Help "Close PowerShell, open a new PowerShell window, and rerun this script."

Write-Host "Tool versions:" -ForegroundColor DarkCyan
Write-Host "  Node:  $(node --version)"
Write-Host "  npm:   $(npm --version)"
Write-Host "  Rust:  $(rustc --version)"
Write-Host "  Cargo: $(cargo --version)"

Write-Host "Installing JavaScript dependencies..."
Push-Location $ProjectRoot
try {
    npm install
    if ($LASTEXITCODE -ne 0) {
        throw "npm install failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

Write-Host "Development prerequisites are ready." -ForegroundColor Green
Write-Host "Next: run .\scripts\dev.ps1 from the HouseholdBillsApp folder." -ForegroundColor Cyan

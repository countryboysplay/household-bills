param([Parameter(Mandatory=$true)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version)
$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent

# All three manifests are edited as text rather than parsed and re-serialised.
#
# Round-tripping through ConvertFrom-Json/ConvertTo-Json corrupted this project:
# Windows PowerShell 5.1 reads a BOM-less file using the ANSI codepage, so the
# UTF-8 bytes for "(c)" in tauri.conf.json were decoded as two Latin-1 characters
# and then re-encoded as UTF-8 -- mojibake that compounded on every bump. The
# round-trip also reformatted whole files, burying a one-line change in a
# fifty-line diff.
#
# Reading and writing UTF-8 explicitly, and replacing only the version field,
# keeps both the bytes and the formatting intact.

$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Set-VersionField {
    param(
        [Parameter(Mandatory=$true)][string]$Path,
        [Parameter(Mandatory=$true)][regex]$Pattern,
        [Parameter(Mandatory=$true)][string]$Replacement,
        [Parameter(Mandatory=$true)][string]$Label
    )
    if (-not (Test-Path $Path)) { throw "$Label not found at $Path." }
    $text = [System.IO.File]::ReadAllText($Path, $Utf8NoBom)
    if (-not $Pattern.IsMatch($text)) { throw "Could not find a version field in $Path." }
    # Instance .Replace takes a count; the static [regex]::Replace does not --
    # passing one there binds to RegexOptions and rewrites every match.
    $updated = $Pattern.Replace($text, $Replacement, 1)
    [System.IO.File]::WriteAllText($Path, $updated, $Utf8NoBom)
}

$packagePath = Join-Path $Root "package.json"
$tauriPath   = Join-Path $Root "src-tauri\tauri.conf.json"
$cargoPath   = Join-Path $Root "src-tauri\Cargo.toml"

Set-VersionField -Path $packagePath -Label "package.json" `
    -Pattern ([regex]'("version"\s*:\s*")[^"]+(")') -Replacement ('${1}' + $Version + '${2}')

Set-VersionField -Path $tauriPath -Label "tauri.conf.json" `
    -Pattern ([regex]'("version"\s*:\s*")[^"]+(")') -Replacement ('${1}' + $Version + '${2}')

Set-VersionField -Path $cargoPath -Label "Cargo.toml" `
    -Pattern ([regex]'(?m)^(version\s*=\s*")[^"]+(")') -Replacement ('${1}' + $Version + '${2}')

# Read the versions back so a silent mismatch cannot reach a release build.
$packageVersion = (Get-Content $packagePath -Raw | ConvertFrom-Json).version
$tauriVersion   = (Get-Content $tauriPath -Raw | ConvertFrom-Json).version
$cargoVersion   = [regex]::Match((Get-Content $cargoPath -Raw), '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
if ($packageVersion -ne $Version -or $tauriVersion -ne $Version -or $cargoVersion -ne $Version) {
    throw "Version bump did not apply cleanly. package=$packageVersion tauri=$tauriVersion cargo=$cargoVersion expected=$Version"
}

Write-Host "Household Bills version set to $Version in package.json, tauri.conf.json, and Cargo.toml." -ForegroundColor Green

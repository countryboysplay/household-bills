<#
.SYNOPSIS
Fails if the release binary would open a console window.

.DESCRIPTION
Household Bills 1.0.1 shipped without the `windows_subsystem = "windows"`
attribute in main.rs. Rust defaults to the console subsystem on Windows, so the
installed app opened a blank black console beside its window -- and because that
console owns the process, closing it killed the app.

Nothing caught it: `cargo test` only ever builds the debug profile, where the
attribute is deliberately inert. This check reads the Subsystem field out of the
PE header of the actual release binary, so the regression cannot ship again.

A source-level grep for the attribute would not do: it would pass whether or not
the attribute survived compilation, which is precisely the class of assertion
that let the bug through in the first place.
#>
[CmdletBinding()]
param(
    # Skip the build and inspect an existing binary (used after a bundle step).
    [switch]$NoBuild
)

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent
Set-Location $Root

$IMAGE_SUBSYSTEM_WINDOWS_GUI = 2
$IMAGE_SUBSYSTEM_WINDOWS_CUI = 3

function Get-PESubsystem {
    param([Parameter(Mandatory=$true)][string]$Path)

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $reader = New-Object System.IO.BinaryReader($stream)

        # DOS header: e_lfanew at 0x3C points at the PE signature.
        $stream.Seek(0x3C, 'Begin') | Out-Null
        $peOffset = $reader.ReadInt32()

        $stream.Seek($peOffset, 'Begin') | Out-Null
        $signature = $reader.ReadUInt32()
        if ($signature -ne 0x00004550) { throw "$Path is not a PE image (bad signature)." }

        # Subsystem sits 68 bytes into the optional header, which follows the
        # 4-byte signature and the 20-byte COFF header. That offset is the same
        # for PE32 and PE32+, because PE32+ drops BaseOfData and widens ImageBase.
        $stream.Seek($peOffset + 4 + 20 + 68, 'Begin') | Out-Null
        return $reader.ReadUInt16()
    }
    finally {
        $stream.Dispose()
    }
}

$exePath = Join-Path $Root "src-tauri\target\release\household-bills.exe"

if (-not $NoBuild) {
    Write-Host "Building the release binary to inspect..." -ForegroundColor Cyan
    cargo build --release --manifest-path src-tauri/Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed with exit code $LASTEXITCODE." }
}

if (-not (Test-Path $exePath)) { throw "Release binary not found at $exePath." }

$subsystem = Get-PESubsystem -Path $exePath

switch ($subsystem) {
    $IMAGE_SUBSYSTEM_WINDOWS_GUI {
        Write-Host "Windows subsystem check passed: release binary is a GUI app (subsystem 2)." -ForegroundColor Green
        exit 0
    }
    $IMAGE_SUBSYSTEM_WINDOWS_CUI {
        throw @"
Release binary is a CONSOLE subsystem app (subsystem 3).

The installed app will open a blank console window, and closing that console
will kill the app. Add this to the top of src-tauri/src/main.rs:

    #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
"@
    }
    default {
        throw "Release binary has unexpected PE subsystem $subsystem (expected $IMAGE_SUBSYSTEM_WINDOWS_GUI)."
    }
}

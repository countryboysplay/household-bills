param([switch]$SkipTests)
$ErrorActionPreference = "Stop"
& "$PSScriptRoot\build-release.ps1" -SkipTests:$SkipTests

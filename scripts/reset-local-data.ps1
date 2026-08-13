$ErrorActionPreference = "Stop"
Write-Warning "This developer utility deletes the local Household Bills application data for the current user."
$answer = Read-Host "Type RESET to continue"
if ($answer -ne "RESET") { Write-Host "Cancelled."; exit 0 }

$paths = @(
    Join-Path $env:APPDATA "com.householdbills.desktop",
    Join-Path $env:LOCALAPPDATA "com.householdbills.desktop"
)
foreach ($path in $paths) {
    if (Test-Path $path) {
        Remove-Item -Recurse -Force $path
        Write-Host "Removed $path"
    }
}

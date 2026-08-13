$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

function Assert-NativeSuccess {
    param([string]$Step)
    if ($LASTEXITCODE -ne 0) {
        throw "$Step failed with exit code $LASTEXITCODE."
    }
}

Write-Host "Preparing frontend dependencies..." -ForegroundColor Cyan
$needsInstall = -not (Test-Path "node_modules") -or -not (Test-Path "node_modules/@types/node")
if ($needsInstall) {
    npm install
    Assert-NativeSuccess "npm install"
}

Write-Host "Running frontend type check..." -ForegroundColor Cyan
npm run typecheck
Assert-NativeSuccess "Frontend type check"

Write-Host "Checking Tauri command permissions..." -ForegroundColor Cyan
$permissionFile = "src-tauri/permissions/household.toml"
$capabilityFile = "src-tauri/capabilities/default.json"
$permissionText = Get-Content $permissionFile -Raw
$capabilityText = Get-Content $capabilityFile -Raw
$requiredCommands = @(
    "get_app_bootstrap", "complete_onboarding", "get_dashboard_summary", "create_backup",
    "list_bills", "save_bill", "get_bill_detail", "mark_bill_paid", "archive_bill",
    "list_paychecks", "save_paycheck", "delete_paycheck", "list_paycheck_schedules", "save_paycheck_schedule", "run_scheduler", "get_planner",
    "get_dashboard_data", "get_spending_view", "add_transaction", "reconcile_account", "get_calendar_data", "get_history_data",
    "get_payment_guidance", "get_savings_debt_view", "save_savings_goal", "save_debt",
    "record_savings_contribution", "record_debt_payment", "archive_savings_goal", "archive_debt",
    "get_settings_view", "save_settings", "open_app_folder", "list_backups", "request_restore_backup",
    "get_reports_data", "export_report_csv", "check_for_update", "install_update"
)
foreach ($command in $requiredCommands) {
    if ($permissionText -notmatch ('commands\.allow\s*=\s*\[[^\]]*"' + [regex]::Escape($command) + '"')) {
        throw "Tauri ACL is missing permission for command '$command'."
    }
}
if ($capabilityText -notmatch '"household-default"') {
    throw "Tauri main capability does not include the household-default permission set."
}
Write-Host "Tauri command permissions passed." -ForegroundColor Green

Write-Host "Checking Phase 5 payment-guidance schema..." -ForegroundColor Cyan
$migrationText = Get-Content "src-tauri/migrations/005_phase5.sql" -Raw
$phase3Text = Get-Content "src-tauri/src/phase3.rs" -Raw
if ($migrationText -notmatch "recommended_payment_date") {
    throw "Phase 5 migration is missing bill_allocations.recommended_payment_date."
}
if ($phase3Text -notmatch "recommended_payment_date") {
    throw "Scheduler persistence is not saving recommended provider-payment dates."
}
Write-Host "Phase 5 payment-guidance schema passed." -ForegroundColor Green


Write-Host "Checking release version alignment..." -ForegroundColor Cyan
$packageVersion = (Get-Content "package.json" -Raw | ConvertFrom-Json).version
$tauriVersion = (Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json).version
$cargoText = Get-Content "src-tauri/Cargo.toml" -Raw
$cargoMatch = [regex]::Match($cargoText, '(?m)^version\s*=\s*"([^"]+)"')
if (-not $cargoMatch.Success) { throw "Could not read Cargo package version." }
$cargoVersion = $cargoMatch.Groups[1].Value
if ($packageVersion -ne $tauriVersion -or $packageVersion -ne $cargoVersion) {
    throw "Release versions do not match. package=$packageVersion tauri=$tauriVersion cargo=$cargoVersion"
}
Write-Host "Release version alignment passed: $packageVersion" -ForegroundColor Green

Write-Host "Checking GitHub updater configuration..." -ForegroundColor Cyan
$tauriConfig = Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json
if (-not $tauriConfig.bundle.createUpdaterArtifacts) { throw "Tauri updater artifacts are not enabled." }
if ($tauriConfig.plugins.updater.pubkey -match '^__TAURI_' -or [string]::IsNullOrWhiteSpace($tauriConfig.plugins.updater.pubkey)) { throw "Updater public key is not configured. Run scripts\configure-updater.ps1." }
$updateEndpoint = [string]$tauriConfig.plugins.updater.endpoints[0]
if ($updateEndpoint -match '__GITHUB_') { throw "Updater GitHub repository is not configured. Run scripts\configure-updater.ps1." }
$cargoUpdater = Get-Content "src-tauri/Cargo.toml" -Raw
if ($cargoUpdater -notmatch 'tauri-plugin-updater') { throw "Cargo.toml is missing tauri-plugin-updater." }
$libUpdater = Get-Content "src-tauri/src/lib.rs" -Raw
if ($libUpdater -notmatch 'tauri_plugin_updater' -or $libUpdater -notmatch 'updates::check_for_update') { throw "Rust updater integration is incomplete." }
Write-Host "GitHub updater configuration passed." -ForegroundColor Green


Write-Host "Checking 1.0 release migration safety..." -ForegroundColor Cyan
$releaseMigration = Get-Content "src-tauri/migrations/006_release_1_0.sql" -Raw
$dbSource = Get-Content "src-tauri/src/db/mod.rs" -Raw
$libSource = Get-Content "src-tauri/src/lib.rs" -Raw
if ($releaseMigration -notmatch "last_app_version") { throw "Release migration is missing last_app_version tracking." }
if ($dbSource -notmatch "006_release_1_0") { throw "Release migration is not registered." }
if ($libSource -notmatch "pending_migrations" -or $libSource -notmatch "last_app_version") { throw "Release startup is missing upgrade backup/version tracking." }
Write-Host "1.0 release migration safety passed." -ForegroundColor Green

Write-Host "Running Rust tests..." -ForegroundColor Cyan
Push-Location "src-tauri"
try {
    cargo test
    Assert-NativeSuccess "Rust test suite"
} finally {
    Pop-Location
}

Write-Host "All frontend and Rust tests passed." -ForegroundColor Green

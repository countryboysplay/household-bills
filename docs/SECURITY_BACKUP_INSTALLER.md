# Security, Backup, and Installer

> **Phase 5 scope override (current):** AI has been removed from the Household Bills roadmap. Any AI, llama.cpp, model, Ask AI, or AI proposal material later in this historical document is superseded by `AI_SPEC.md` and must not be implemented. The product remains deterministic and local.

## 1. Security posture

This is a local personal app, so security should be strong but simple.

Threat priorities:

- accidental data loss
- accidental duplicate payments
- unintended household edits
- unsafe restore/upgrade
- local AI overreach
- obvious plaintext credential storage

Do not turn the project into an enterprise security platform.

## 2. Profiles

Jonathan and Tiffany have separate profiles.

Recommended login:

- profile selector
- optional PIN/passphrase
- secure local hash using a modern password hashing algorithm such as Argon2id
- session remains active until profile switch/app close or configured timeout

Both users have full access to shared household data.

The value of separate profiles is attribution and avoiding payment confusion, not permission separation.

## 3. Database

Store under the standard Tauri app-data location.

Do not place the live database in the install directory.

Enable SQLite foreign keys.

Use transactions for critical writes.

Use WAL/checkpoint safely.

## 4. Sensitive local data

Do not log:

- PINs
- password hashes
- full AI prompts containing excessive financial data unless needed for debugging
- raw database contents

Logs should be local and rotate.

## 5. Backup folder

Default to a user-visible local location or app-data backup directory with an "Open Backup Folder" control.

Allow user to select a different folder such as:

- another drive
- USB drive
- NAS-mounted Windows path

Do not require cloud backup.

## 6. Automatic backup policy

Recommended default:

- backup once daily when app is first opened, if no successful backup exists for that day
- backup before database migration
- backup before restore
- optional backup on app close if data changed materially

Default retention:

- 14 daily backups
- 8 weekly backups
- 12 monthly backups

For personal simplicity, retention may be implemented as a smaller configurable "keep last N backups" in V1 if needed.

## 7. Backup format

Use a packaged backup file, e.g.:

`HouseholdBills_2026-08-12_112500.hbbackup`

Internally it may be a ZIP containing:

```text
manifest.json
household-bills.sqlite
attachments/        optional
```

Manifest:

```json
{
  "format_version": 1,
  "app_version": "1.0.0",
  "schema_version": 12,
  "created_at": "...",
  "database_sha256": "..."
}
```

## 8. Backup consistency

Use SQLite backup API or another safe consistent snapshot method.

Do not naïvely copy an active database in a way that can omit WAL data.

Verify backup before reporting success.

## 9. Restore flow

1. User selects backup.
2. Validate file format/checksum.
3. Show backup date/app/schema version.
4. Warn that current data will be replaced.
5. Create safety backup of current state.
6. Close DB connections.
7. Restore.
8. Run compatible migrations if needed.
9. Reopen DB.
10. Run integrity check.
11. Recalculate schedule.
12. Confirm success.

On failure, restore the safety backup.

## 10. Installer

Use Tauri's Windows installer support to produce NSIS `.exe`.

Desired artifact:

`HouseholdBillsSetup.exe`

Installer responsibilities:

- install application
- install bundled runtime resources
- create Start Menu shortcut
- optional desktop shortcut
- register uninstall entry
- configure WebView2 handling using supported Tauri mechanism
- preserve app data during upgrade/uninstall unless user explicitly chooses data removal

## 11. Build-time vs runtime dependencies

Development machine may need:

- Rust toolchain
- Node/npm
- Microsoft C++ Build Tools
- WebView2 development/runtime prerequisites
- Tauri CLI

End user must not manually install those development tools.

The application installer should handle runtime prerequisites as supported by Tauri.

## 12. AI runtime packaging

If AI is enabled in the release:

Bundle/provision:

- llama-server executable
- required llama.cpp runtime DLLs
- CUDA-capable build appropriate for Windows if validated
- CPU-capable fallback or clear fallback strategy

Model may be downloaded after installation.

AI runtime must be versioned separately from the database.

## 13. Model download

If model is not bundled:

- show model name
- show approximate download size
- download to temporary `.partial`
- verify checksum
- atomic rename to final path
- resume/retry if practical
- do not leave corrupt model marked installed

## 14. Updates

Version 1 can support manual installer upgrades.

If updater is added:

- use signed update metadata
- always backup before migration
- never update model and app transactionally unless needed
- AI runtime/model updates should not risk financial DB

## 15. Code signing

Personal unsigned builds may trigger Windows SmartScreen.

Structure the release pipeline so Windows code signing can be added without redesigning the installer.

Do not block development on commercial code-signing unless Jonathan chooses to obtain a certificate.

## 16. Crash recovery

Critical writes should be transactional.

On app start:

- open DB
- run integrity/basic health checks
- ensure migrations complete
- detect interrupted restore/update markers
- recover if possible
- surface actionable message if not

## 17. Duplicate-payment protection

Once a bill occurrence is fully paid:

- normal Mark Paid button disappears
- user can view/edit existing payment
- adding another payment requires an explicit advanced action

This is a product safety feature.

## 18. AI security

AI process:

- loopback only
- no shell command execution from model
- no database path exposed as a tool
- no arbitrary file browsing
- structured app-owned tools only
- proposal validation before any mutation

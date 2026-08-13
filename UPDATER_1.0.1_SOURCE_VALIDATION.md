# Updater 1.0.1 Source Validation

Validated in the build environment before packaging:

- package.json, tauri.conf.json, capabilities JSON parse successfully.
- Cargo.toml parses successfully.
- App.tsx, Settings.tsx, backend.ts, and types.ts pass TypeScript syntax transpilation.
- Version alignment set to 1.0.1.
- tauri-plugin-updater dependency is declared.
- updater plugin is initialized in Rust.
- check_for_update and install_update are registered in the invoke handler.
- custom command permissions include check_for_update and install_update.
- updater artifact generation is enabled.
- GitHub updater configuration intentionally contains placeholders until scripts/configure-updater.ps1 is run with the real public GitHub repository and signing public key.
- GitHub CI, testing artifact, release, signing, version-gate, and version-bump workflows/scripts are included.

Windows remains the authoritative Cargo/Tauri compile and updater test environment after repository configuration.

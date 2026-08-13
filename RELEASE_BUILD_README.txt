HOUSEHOLD BILLS 1.0.0 RELEASE CANDIDATE

This folder is ready to build the Windows NSIS installer on the Windows desktop that has already passed the development tests.

BUILD THE INSTALLER

1. Open PowerShell in this HouseholdBillsApp folder.
2. Run:

   Set-ExecutionPolicy -Scope Process Bypass
   .\scripts\build-release.ps1

3. The script will:
   - run the full frontend and Rust test suite
   - verify command permissions and release version alignment
   - build the optimized Tauri application
   - create the NSIS setup executable
   - copy it into the local release folder as:
       Household Bills Setup 1.0.0.exe
   - create a SHA256 release manifest
   - copy the release test checklist into the release folder

4. When the build finishes, open the release folder and follow RELEASE_TEST_CHECKLIST.md.

IMPORTANT

- The installer is intended for current-user installation and should not require administrator rights under normal conditions.
- The installer is not code-signed. Windows SmartScreen may display a warning on first launch/install.
- Household financial data is stored in the Windows application-data directory, outside the installed program folder.
- The application creates a safety backup before schema migrations and before app-version upgrades.
- Normal use does not connect to a bank or cloud service.

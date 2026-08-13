# Windows First Run

## 1. Extract the source

Extract the project to a normal development folder, for example:

```text
C:\HouseholdBills
```

Avoid building directly inside a ZIP file or cloud-synced temporary folder.

## 2. Open PowerShell as your normal Windows user

```powershell
cd C:\HouseholdBills
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\setup-dev.ps1
```

If the script installs Node.js, Rust, or Visual Studio Build Tools, close PowerShell after it finishes and open a new PowerShell window before continuing.

## 3. Run the application in development mode

```powershell
cd C:\HouseholdBills
.\scripts\dev.ps1
```

The native Household Bills window should open. On a fresh local data directory it will show the onboarding screen.

## 4. Run tests

```powershell
.\scripts\test.ps1
```

## 5. Build the installer

```powershell
.\scripts\build-installer.ps1
```

The `.exe` installer should be produced under:

```text
src-tauri\target\release\bundle\nsis\
```

## If the first Tauri build reports a Windows prerequisite error

Tauri development on Windows requires Microsoft C++ Build Tools and Microsoft Edge WebView2. Windows 11 normally already contains WebView2. The setup script installs the Visual Studio Build Tools C++ workload if Visual Studio Build Tools is not detected.

Copy the complete error output back into the development chat before manually changing project code. Build errors are much easier to fix from the exact output.

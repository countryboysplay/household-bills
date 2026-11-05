import { defineConfig } from "@playwright/test";

// End-to-end tests drive the real UI in Chromium against the real Rust commands,
// served by the dev-only browser bridge (src-tauri/examples/browser_bridge.rs).
// The bridge uses its own data folder and starts empty, so real household data is never touched.
export default defineConfig({
  testDir: "e2e",
  timeout: 120_000,
  workers: 1,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: { baseURL: "http://127.0.0.1:1420", headless: true, trace: "retain-on-failure" },
  webServer: [
    {
      command: "cargo run --quiet --example browser_bridge --manifest-path src-tauri/Cargo.toml",
      port: 1430,
      timeout: 15 * 60_000, // first run compiles the backend
      reuseExistingServer: false,
      env: { HB_BRIDGE_IDENTIFIER: "com.householdbills.e2e", HB_BRIDGE_FRESH: "1" },
    },
    {
      command: "npx vite --host 127.0.0.1 --port 1420 --strictPort",
      url: "http://127.0.0.1:1420",
      timeout: 120_000,
      reuseExistingServer: false,
      env: { HB_BRIDGE: "1" },
    },
  ],
});

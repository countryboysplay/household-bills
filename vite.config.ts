import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;
// Dev-only: HB_BRIDGE=1 runs the UI in a normal browser against the real Rust commands
// served by `cargo run --example browser_bridge` (src-tauri/examples/browser_bridge.rs).
const bridge = process.env.HB_BRIDGE === "1";
const bridgeShim = `window.__TAURI_INTERNALS__ = { metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main", windowLabel: "main" } }, transformCallback: () => 0,
  invoke: async (cmd, args) => { const r = await fetch("/ipc/" + cmd, { method: "POST", body: JSON.stringify(args ?? {}) }); const t = await r.text(); const v = t ? JSON.parse(t) : null; if (!r.ok) throw v; return v; } };`;

export default defineConfig({
  plugins: [react(), bridge && { name: "hb-bridge", transformIndexHtml: () => [{ tag: "script", children: bridgeShim, injectTo: "head-prepend" }] }],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
    proxy: bridge ? { "/ipc": "http://127.0.0.1:1430" } : undefined,
  },
});

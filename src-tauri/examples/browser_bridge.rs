//! Dev-only: serves the real Rust commands over HTTP so the frontend can run in a normal browser.
//! `cargo run --example browser_bridge` then `HB_BRIDGE=1 npm run dev:frontend`.
//! Uses the real tauri.conf.json/capabilities on a mock runtime; data goes to that platform's app-data dir.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use tauri::Manager;
use tauri::ipc::{CallbackFn, InvokeBody, InvokeResponseBody};
use tauri::test::{get_ipc_response, MockRuntime};
use tauri::webview::InvokeRequest;

fn main() {
    let mut app = household_bills_lib::builder::<MockRuntime>()
        .build(tauri::generate_context!())
        .expect("build app");
    // One iteration runs the app's setup hook (DB, managed state) and creates the "main" window from tauri.conf.json.
    #[allow(deprecated)]
    app.run_iteration(|_, _| {});
    let webview = app.get_webview_window("main").expect("main window");
    let invoke_key = app.invoke_key().to_string();
    let addr = std::env::var("HB_BRIDGE_ADDR").unwrap_or_else(|_| "127.0.0.1:1430".into());
    let listener = TcpListener::bind(&addr).expect("bind");
    eprintln!("browser bridge on http://{addr}/ipc/<command>");

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() { continue; }
        let path = line.split_whitespace().nth(1).unwrap_or("").to_string();
        let mut len = 0usize;
        loop {
            let mut h = String::new();
            if reader.read_line(&mut h).unwrap_or(0) == 0 || h == "\r\n" { break; }
            if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") { len = v.trim().parse().unwrap_or(0); }
        }
        let mut body = vec![0; len];
        if reader.read_exact(&mut body).is_err() { continue; }
        let Some(cmd) = path.strip_prefix("/ipc/") else {
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
            continue;
        };
        // The updater plugin is not loaded here (it panics on a mock runtime); report it unavailable.
        if cmd.contains("update") {
            let msg = br#""Updates are not available in the browser bridge.""#;
            let _ = stream.write_all(format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n", msg.len()).as_bytes()).and_then(|_| stream.write_all(msg));
            continue;
        }
        let args: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::json!({}));
        let request = InvokeRequest {
            cmd: cmd.to_string(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: invoke_key.clone(),
        };
        let (status, out) = match get_ipc_response(&webview, request) {
            Ok(InvokeResponseBody::Json(s)) => ("200 OK", s.into_bytes()),
            Ok(InvokeResponseBody::Raw(b)) => ("200 OK", b),
            Err(e) => ("400 Bad Request", e.to_string().into_bytes()),
        };
        eprintln!("{cmd} -> {status}");
        let head = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", out.len());
        let _ = stream.write_all(head.as_bytes()).and_then(|_| stream.write_all(&out));
    }
}

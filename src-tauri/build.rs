fn main() {
    // tauri-build embeds the Common Controls v6 manifest into the app binary only.
    // Examples (the dev-only browser bridge) link the same comctl32 v6 imports, so
    // without it Windows refuses to start them (0xC0000139, entry point not found).
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-examples=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg-examples=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    }
    tauri_build::build()
}

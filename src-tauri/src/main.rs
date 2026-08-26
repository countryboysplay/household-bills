// Without this, the release binary links as a console subsystem executable:
// Windows opens a blank console window alongside the app, and closing that
// console kills the process. Debug builds keep the console for `cargo`/`tauri dev`
// output.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    household_bills_lib::run();
}

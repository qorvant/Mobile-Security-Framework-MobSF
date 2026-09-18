#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::analyze_apk,
            commands::list_entries,
            commands::decompile_apk
        ])
        .run(tauri::generate_context!())
        .expect("error while running MobSF-Win");
}

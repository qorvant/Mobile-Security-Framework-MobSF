//! Tauri command handlers. Each handler is a thin bridge between the WebView
//! frontend and the pure-Rust analysis engine. Errors are returned as `String`
//! so they serialize cleanly to the JS side.

use mobsf_engine::{
    discover_tools, list_apk_entries, read_manifest_from_apk, run_apktool, run_jadx,
    DecompileResult, ToolConfig,
};
use std::path::Path;

#[tauri::command]
pub fn analyze_apk(path: String) -> Result<mobsf_engine::ApkManifest, String> {
    read_manifest_from_apk(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_entries(path: String) -> Result<Vec<String>, String> {
    list_apk_entries(&path).map_err(|e| e.to_string())
}

/// Decompile an APK with the chosen external tool. `tool` is `"apktool"` or
/// `"jadx"`; anything else defaults to apktool.
#[tauri::command]
pub fn decompile_apk(path: String, tool: String, out_dir: String) -> Result<DecompileResult, String> {
    let cfg: ToolConfig = discover_tools();
    let apk = Path::new(&path);
    let out = Path::new(&out_dir);
    match tool.as_str() {
        "jadx" => run_jadx(apk, out, &cfg).map_err(|e| e.to_string()),
        _ => run_apktool(apk, out, &cfg).map_err(|e| e.to_string()),
    }
}

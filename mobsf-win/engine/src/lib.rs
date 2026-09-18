//! MobSF-Win analysis engine.
//!
//! This crate is intentionally free of any GUI / Tauri dependency so it can be
//! unit-tested and reused from both the Tauri desktop shell and (in the future)
//! a CLI or headless service.
//!
//! Current capabilities:
//! * Pure-Rust parsing of the binary AndroidManifest.xml (AXML) embedded in APKs.
//! * Reading APK archives (zip) and extracting manifest / entry listings.
//! * Orchestration of external Android reverse-engineering tools (apktool, jadx)
//!   by invoking them as subprocesses (never by reimplementing them).

pub mod aapt;
pub mod apk;
pub mod axml;
pub mod manifest;
pub mod tools;

pub use aapt::parse_aapt_badging;
pub use apk::{list_apk_entries, read_manifest_from_apk, EngineError};
pub use manifest::{ApkManifest, Component, Permission};
pub use tools::{
    discover_tools, run_aapt_badging, run_apktool, run_jadx, DecompileResult, ToolConfig,
};

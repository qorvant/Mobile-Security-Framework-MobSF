//! Orchestration of external Android reverse-engineering tools.
//!
//! MobSF's real value comes from tools like `apktool` and `jadx`. Rewriting
//! those in Rust is out of scope; instead we invoke them as subprocesses.
//! Arguments are always passed as a list (never via shell string formatting) so
//! that file paths with spaces or special characters cannot be abused for
//! command injection — consistent with MobSF's security model.

use crate::EngineError;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Configuration describing where the external tools live on the host.
#[derive(Debug, Clone)]
pub struct ToolConfig {
    pub java: PathBuf,
    pub aapt: Option<PathBuf>,
    pub apktool_jar: Option<PathBuf>,
    pub jadx: Option<PathBuf>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        ToolConfig {
            java: PathBuf::from("java"),
            aapt: None,
            apktool_jar: None,
            jadx: None,
        }
    }
}

/// Result of running an external decompiler.
#[derive(Debug, Clone, Serialize)]
pub struct DecompileResult {
    pub success: bool,
    pub tool: String,
    pub output_dir: String,
    pub stdout: String,
    pub stderr: String,
}

/// Look for the external tools in a few conventional places and on PATH.
pub fn discover_tools() -> ToolConfig {
    let mut cfg = ToolConfig::default();
    for cand in ["aapt", "aapt2", "aapt.exe", "aapt2.exe"] {
        if let Some(p) = which(cand) {
            cfg.aapt = Some(p);
            break;
        }
    }
    for cand in ["./tools/apktool.jar", "./apktool.jar", "apktool.jar"] {
        if Path::new(cand).exists() {
            cfg.apktool_jar = Some(PathBuf::from(cand));
            break;
        }
    }
    for cand in ["jadx", "jadx.bat", "./tools/jadx/bin/jadx.bat", "./tools/jadx/bin/jadx"] {
        if let Some(p) = which(cand) {
            cfg.jadx = Some(p);
            break;
        }
    }
    cfg
}

fn which(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        for candidate in [name, &format!("{}.exe", name)] {
            let p = dir.join(candidate);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

fn ensure_dir(out: &Path) {
    let _ = std::fs::create_dir_all(out);
}

/// Run `aapt dump badging <apk>` and return its stdout. `aapt`/`aapt2` is the
/// canonical, version-robust way to read an APK's manifest, so the desktop app
/// prefers this over the built-in AXML reader.
pub fn run_aapt_badging(apk: &Path, cfg: &ToolConfig) -> Result<String, EngineError> {
    let aapt = cfg
        .aapt
        .clone()
        .ok_or_else(|| {
            EngineError::Tool(
                "aapt/aapt2 not found. Install Android SDK build-tools (provides aapt) or place \
                 aapt.exe on PATH; without it only AAPT1 APKs can be parsed."
                    .into(),
            )
        })?;
    let apk = apk.to_str().ok_or_else(|| EngineError::Tool("invalid APK path".into()))?;
    let status = Command::new(&aapt)
        .args(["dump", "badging", apk])
        .output()
        .map_err(|e| EngineError::Tool(format!("failed to launch aapt: {e}")))?;
    Ok(String::from_utf8_lossy(&status.stdout).into_owned())
}

/// Decompile an APK with `apktool` (resources + smali).
pub fn run_apktool(apk: &Path, out: &Path, cfg: &ToolConfig) -> Result<DecompileResult, EngineError> {
    let jar = cfg
        .apktool_jar
        .clone()
        .ok_or_else(|| {
            EngineError::Tool(
                "apktool.jar not found. Drop apktool.jar into ./tools/ or set ToolConfig.apktool_jar."
                    .into(),
            )
        })?;
    ensure_dir(out);
    let java = cfg.java.to_str().unwrap_or("java");
    let jar = jar.to_str().ok_or_else(|| EngineError::Tool("invalid apktool.jar path".into()))?;
    let apk = apk.to_str().ok_or_else(|| EngineError::Tool("invalid APK path".into()))?;
    let out = out.to_str().ok_or_else(|| EngineError::Tool("invalid output path".into()))?;
    let status = Command::new(java)
        .args(["-jar", jar, "d", "-f", "-o", out, apk])
        .output()
        .map_err(|e| EngineError::Tool(format!("failed to launch apktool: {e}")))?;
    Ok(DecompileResult {
        success: status.status.success(),
        tool: "apktool".into(),
        output_dir: out.into(),
        stdout: String::from_utf8_lossy(&status.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&status.stderr).into_owned(),
    })
}

/// Decompile an APK with `jadx` (Java/Kotlin source).
pub fn run_jadx(apk: &Path, out: &Path, cfg: &ToolConfig) -> Result<DecompileResult, EngineError> {
    let jadx = cfg
        .jadx
        .clone()
        .ok_or_else(|| EngineError::Tool("jadx not found on PATH. Install jadx (https://github.com/skylot/jadx).".into()))?;
    ensure_dir(out);
    let apk = apk.to_str().ok_or_else(|| EngineError::Tool("invalid APK path".into()))?;
    let out = out.to_str().ok_or_else(|| EngineError::Tool("invalid output path".into()))?;
    let status = Command::new(&jadx)
        .args(["-d", out, apk])
        .output()
        .map_err(|e| EngineError::Tool(format!("failed to launch jadx: {e}")))?;
    Ok(DecompileResult {
        success: status.status.success(),
        tool: "jadx".into(),
        output_dir: out.into(),
        stdout: String::from_utf8_lossy(&status.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&status.stderr).into_owned(),
    })
}

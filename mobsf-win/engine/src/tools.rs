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
///
/// Each candidate is tried in order:
///   1. as a path relative to the current directory (or absolute),
///   2. resolved via `PATH`,
///   3. as a path relative to the *executable's own directory*, so a portable
///      single-exe deployment can ship `tools/` next to the `.exe`.
///
/// The previous implementation only called [`which`] for every candidate, which
/// meant relative paths such as `./tools/jadx/bin/jadx.bat` were silently never
/// checked — so a tool dropped into the documented `tools/` directory was
/// reported as "not found on PATH".
pub fn discover_tools() -> ToolConfig {
    let mut cfg = ToolConfig::default();
    cfg.aapt = find_tool(&["aapt", "aapt2", "aapt.exe", "aapt2.exe"]);
    cfg.apktool_jar = find_file(&["./tools/apktool.jar", "./apktool.jar", "apktool.jar"]);
    cfg.jadx = find_tool(&[
        "jadx",
        "jadx.exe",
        "jadx.bat",
        "./tools/jadx/bin/jadx",
        "./tools/jadx/bin/jadx.exe",
        "./tools/jadx/bin/jadx.bat",
    ]);
    cfg
}

/// Resolve a tool executable: filesystem path first, then `PATH`, then the
/// directory of the current executable.
fn find_tool(cands: &[&str]) -> Option<PathBuf> {
    for c in cands {
        if Path::new(c).exists() {
            return Some(PathBuf::from(c));
        }
    }
    for c in cands {
        if let Some(p) = which(c) {
            return Some(p);
        }
    }
    find_next_to_exe(cands)
}

/// Resolve a data file (e.g. a `.jar`): filesystem path, then the directory of
/// the current executable. Jars are shipped with the app, not installed on PATH.
fn find_file(cands: &[&str]) -> Option<PathBuf> {
    for c in cands {
        if Path::new(c).exists() {
            return Some(PathBuf::from(c));
        }
    }
    find_next_to_exe(cands)
}

/// Try each candidate relative to the executable's own directory. Leading
/// `./`, `/` or `\` are stripped so the layout is the same whether the app is
/// run from its own folder or installed elsewhere.
fn find_next_to_exe(cands: &[&str]) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    for c in cands {
        let rel = c.trim_start_matches(['.', '/', '\\']);
        let p = dir.join(rel);
        if p.exists() {
            return Some(p);
        }
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_resolves_executable_on_path() {
        // Drop a dummy `jadx.exe` into a temp dir, prepend it to PATH, and make
        // sure `which` finds the bare name through it. This guards the helper
        // used by `find_tool` (and therefore `discover_tools`).
        let tmp = std::env::temp_dir().join(format!("mobsf-win-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let dummy = tmp.join("jadx.exe");
        std::fs::write(&dummy, b"").unwrap();

        let original = std::env::var_os("PATH").unwrap_or_default();
        let new_path = std::env::join_paths(
            std::iter::once(tmp.clone()).chain(std::env::split_paths(&original)),
        )
        .unwrap();
        std::env::set_var("PATH", &new_path);
        let found = which("jadx");
        std::env::set_var("PATH", &original);

        let _ = std::fs::remove_file(&dummy);
        let _ = std::fs::remove_dir(&tmp);

        assert_eq!(found, Some(dummy));
    }
}

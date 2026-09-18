//! APK (zip) access: extract the manifest and list entries.

use crate::axml::AxmlError;
use crate::manifest::ApkManifest;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("axml parse error: {0}")]
    Axml(#[from] AxmlError),
    #[error("required entry not found in APK: {0}")]
    MissingEntry(String),
    #[error("external tool error: {0}")]
    Tool(String),
}

const MANIFEST_NAME: &str = "AndroidManifest.xml";

/// Open an APK and extract its manifest facts.
///
/// Strategy: if `aapt`/`aapt2` is available it is used (`dump badging`), which
/// reliably understands both AAPT1 and AAPT2 binary XML. Otherwise we fall back
/// to the built-in pure-Rust AXML reader (best-effort, AAPT1).
pub fn read_manifest_from_apk<P: AsRef<Path>>(apk: P) -> Result<ApkManifest, EngineError> {
    let apk = apk.as_ref();
    let cfg = crate::tools::discover_tools();
    if cfg.aapt.is_some() {
        let out = crate::tools::run_aapt_badging(apk, &cfg)?;
        let manifest = crate::aapt::parse_aapt_badging(&out);
        if !manifest.package.is_empty() {
            return Ok(manifest);
        }
    }
    // Fallback: pure-Rust AXML reader (AAPT1 manifests).
    let file = File::open(apk)?;
    let mut zip = ZipArchive::new(file)?;
    let mut entry = zip
        .by_name(MANIFEST_NAME)
        .map_err(|_| EngineError::MissingEntry(MANIFEST_NAME.to_string()))?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)?;
    crate::axml::parse_axml(&buf).map_err(Into::into)
}

/// Return the list of every entry name inside the APK (useful for the file
/// tree view and for choosing what to decompile).
pub fn list_apk_entries<P: AsRef<Path>>(apk: P) -> Result<Vec<String>, EngineError> {
    let file = File::open(apk)?;
    let mut zip = ZipArchive::new(file)?;
    let mut names = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let e = zip.by_index(i)?;
        names.push(e.name().to_string());
    }
    Ok(names)
}

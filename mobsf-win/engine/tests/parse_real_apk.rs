//! Smoke test: ensure `read_manifest_from_apk` does not panic on real APK
//! fixtures shipped with upstream MobSF, and prints whatever it extracted.
//!
//! With `aapt`/`aapt2` installed on the host this exercises the full (reliable)
//! path and yields complete results. Without it, the built-in pure-Rust AXML
//! reader (AAPT1-oriented) is used as a best-effort fallback.
//!
//! Paths are resolved from `CARGO_MANIFEST_DIR` (the engine crate dir) so the
//! test does not depend on the process working directory.

use mobsf_engine::{discover_tools, read_manifest_from_apk};
use std::path::PathBuf;

fn fixture(name: &str) -> Option<PathBuf> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../mobsf/DynamicAnalyzer/tools/onDevice/xposed/modules");
    let p = base.join(name);
    if p.exists() {
        Some(p)
    } else {
        eprintln!("skip (missing): {}", p.display());
        None
    }
}

#[test]
fn read_real_apks_does_not_panic() {
    let names = [
        "JustTrustMe.apk",
        "com.devadvance.rootcloak2_v18_c43b61.apk",
        "Droidmon.apk",
        "AndroidBluePill.apk",
        "mobi.acpm.proxyon_v1_419b04.apk",
        "mobi.acpm.sslunpinning_v2_37f44f.apk",
    ];
    let has_aapt = discover_tools().aapt.is_some();
    eprintln!("aapt available: {has_aapt}");

    let mut tested = 0;
    for name in names {
        let Some(path) = fixture(name) else {
            continue;
        };
        tested += 1;
        let m = read_manifest_from_apk(&path)
            .unwrap_or_else(|e| panic!("read failed for {}: {e}", path.display()));
        println!(
            "=== {name} === package={:?} perms={} act={} svc={} rcv={} prv={}",
            m.package,
            m.permissions.len(),
            m.activities.len(),
            m.services.len(),
            m.receivers.len(),
            m.providers.len()
        );
    }
    assert!(tested > 0, "no fixture APKs were found");
}

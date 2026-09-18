//! Parser for the textual output of `aapt dump badging <apk>`.
//!
//! This is the primary, version-robust way to read an APK manifest: it
//! delegates decoding to Google's own `aapt`/`aapt2` tool, which understands
//! both AAPT1 and AAPT2 binary XML. The format is line oriented; each record is
//! `record-type: key='value' key='value' ...`, with a few records
//! (`sdkVersion`, `targetSdkVersion`, …) carrying a single bare quoted value.

use crate::manifest::{ApkManifest, Component, Permission};
use std::collections::HashMap;

/// Extract all `key='value'` pairs (and bare `'value'`) from a badging record
/// body. Values may contain any character except a single quote.
fn parse_kv(body: &str) -> HashMap<String, String> {
    let b = body.as_bytes();
    let mut map = HashMap::new();
    let mut i = 0;
    while i < b.len() {
        // Skip to the start of a key (letters/digits/underscore, not '=' or ').
        if b[i] == b'\'' || b[i] == b' ' || b[i] == b'=' {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && b[i] != b'=' {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        let key = body[start..i].trim().to_string();
        i += 1; // consume '='
        if i < b.len() && b[i] == b'\'' {
            i += 1;
            let vstart = i;
            while i < b.len() && b[i] != b'\'' {
                i += 1;
            }
            let val = body[vstart..i.min(b.len())].to_string();
            i += 1; // consume closing quote
            map.insert(key, val);
        } else {
            let vstart = i;
            while i < b.len() && !b[i].is_ascii_whitespace() {
                i += 1;
            }
            let val = body[vstart..i].to_string();
            map.insert(key, val);
        }
    }
    map
}

/// Return the first bare `'quoted'` value in a body (used by sdkVersion etc.).
fn first_quoted(body: &str) -> Option<String> {
    let b = body.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\'' {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && b[j] != b'\'' {
                j += 1;
            }
            return Some(body[start..j].to_string());
        }
        i += 1;
    }
    None
}

fn component_from_attrs(attrs: &HashMap<String, String>) -> Component {
    Component {
        name: attrs.get("name").cloned().unwrap_or_default(),
        exported: attrs.get("exported").and_then(|s| s.parse::<bool>().ok()),
        enabled: None,
        permission: attrs.get("permission").cloned(),
    }
}

/// Parse `aapt dump badging` output into an [`ApkManifest`].
pub fn parse_aapt_badging(output: &str) -> ApkManifest {
    let mut m = ApkManifest::default();
    for raw in output.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        // `aapt` is inconsistent: `package: name='x'` has a space after the
        // colon while `sdkVersion:'16'` does not — so split on the bare colon
        // and trim both sides.
        let (record, body) = match line.split_once(':') {
            Some((r, v)) => (r.trim(), v.trim()),
            None => (line, ""),
        };
        match record {
            "package" => {
                let kv = parse_kv(body);
                m.package = kv.get("name").cloned().unwrap_or_default();
                m.version_code = kv.get("versionCode").cloned();
                m.version_name = kv.get("versionName").cloned();
            }
            "uses-permission" | "uses-permission-sdk-23" => {
                let kv = parse_kv(body);
                if let Some(name) = kv.get("name") {
                    let max = kv.get("maxSdkVersion").and_then(|s| s.parse().ok());
                    m.permissions.push(Permission {
                        name: name.clone(),
                        max_sdk_version: max,
                    });
                }
            }
            "sdkVersion" => m.min_sdk_version = first_quoted(body).and_then(|s| s.parse().ok()),
            "targetSdkVersion" => {
                m.target_sdk_version = first_quoted(body).and_then(|s| s.parse().ok())
            }
            "maxSdkVersion" => m.max_sdk_version = first_quoted(body).and_then(|s| s.parse().ok()),
            "compileSdkVersion" => {
                m.compile_sdk_version = first_quoted(body).and_then(|s| s.parse().ok())
            }
            "application" => m.application = Some(component_from_attrs(&parse_kv(body))),
            "launchable-activity" | "activity" | "activity-alias" => {
                m.activities.push(component_from_attrs(&parse_kv(body)))
            }
            "service" => m.services.push(component_from_attrs(&parse_kv(body))),
            "receiver" => m.receivers.push(component_from_attrs(&parse_kv(body))),
            "provider" => m.providers.push(component_from_attrs(&parse_kv(body))),
            _ => {}
        }
    }
    m
}

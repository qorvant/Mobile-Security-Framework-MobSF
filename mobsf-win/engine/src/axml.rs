//! Minimal parser for the Android Binary XML (AXML) format used for
//! `AndroidManifest.xml` inside APK files.
//!
//! This is *not* a full AXML implementation — it only extracts what MobSF needs
//! for the first pass of static analysis: the manifest attributes, permissions
//! and component declarations. It is pure Rust and has no external dependencies
//! so it can be unit-tested against real APKs without any tooling installed.

use crate::manifest::{ApkManifest, Component, Permission};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum AxmlError {
    #[error("invalid AXML: {0}")]
    Invalid(&'static str),
    #[error("read past end of buffer at offset {0}")]
    OutOfBounds(usize),
}

fn rd_u8(b: &[u8], o: usize) -> Result<u8, AxmlError> {
    b.get(o).copied().ok_or(AxmlError::OutOfBounds(o))
}
fn rd_u16(b: &[u8], o: usize) -> Result<u16, AxmlError> {
    if o + 2 > b.len() {
        return Err(AxmlError::OutOfBounds(o));
    }
    Ok(u16::from_le_bytes([b[o], b[o + 1]]))
}
fn rd_u32(b: &[u8], o: usize) -> Result<u32, AxmlError> {
    if o + 4 > b.len() {
        return Err(AxmlError::OutOfBounds(o));
    }
    Ok(u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]))
}

// ---- Chunk types ---------------------------------------------------------
const RES_XML_TYPE: u16 = 0x0003;
const RES_STRING_POOL_TYPE: u16 = 0x0001;
const RES_XML_RESOURCE_MAP_TYPE: u16 = 0x0180;
const RES_XML_START_NAMESPACE_TYPE: u16 = 0x0100;
const RES_XML_END_NAMESPACE_TYPE: u16 = 0x0101;
const RES_XML_START_ELEMENT_TYPE: u16 = 0x0102;
const RES_XML_END_ELEMENT_TYPE: u16 = 0x0103;
const RES_XML_CDATA_TYPE: u16 = 0x0104;

// ---- Res_value data types ------------------------------------------------
const TYPE_STRING: u8 = 0x03;
const TYPE_INT_DEC: u8 = 0x10;
const TYPE_INT_HEX: u8 = 0x11;
const TYPE_INT_BOOLEAN: u8 = 0x12;

/// Map an Android framework attribute resource ID (`android.R.attr.<name>`,
/// encoding `0x0101XXXX`) to its attribute name. Only the subset used by
/// manifests is included; unknown IDs surface as `attr_0x...` so they are easy
/// to spot and extend.
fn attr_name_from_id(id: u32) -> Option<&'static str> {
    Some(match id {
        0x01010000 => "package",
        0x01010001 => "label",
        0x01010002 => "icon",
        0x01010003 => "name",
        0x01010006 => "permission",
        0x0101000e => "enabled",
        0x0101000f => "debuggable",
        0x01010010 => "exported",
        0x01010011 => "process",
        0x01010012 => "taskAffinity",
        0x0101021b => "versionCode",
        0x0101021c => "versionName",
        0x0101020c => "minSdkVersion",
        0x01010270 => "targetSdkVersion",
        0x01010271 => "maxSdkVersion",
        0x01010572 => "compileSdkVersion",
        0x01010280 => "allowBackup",
        0x0101027b => "supportsRtl",
        0x010103a2 => "usesCleartextTraffic",
        0x010104f6 => "networkSecurityConfig",
        _ => return None,
    })
}

fn parse_string_pool(b: &[u8], start: usize, _size: usize) -> Result<Vec<String>, AxmlError> {
    let string_count = rd_u32(b, start + 8)? as usize;
    let flags = rd_u32(b, start + 16)?;
    let strings_start = rd_u32(b, start + 20)? as usize;
    // ResStringPool_header: type(2) headerSize(2) size(4) + 5 * u32 = 28 bytes.
    let offsets_base = start + 28;
    let strings_base = start + strings_start;
    let is_utf8 = (flags & 0x100) != 0;
    let mut out = Vec::with_capacity(string_count);
    for i in 0..string_count {
        let rel = rd_u32(b, offsets_base + i * 4)? as usize;
        let s = if is_utf8 {
            decode_utf8(b, strings_base + rel)?
        } else {
            decode_utf16(b, strings_base + rel)?
        };
        out.push(s);
    }
    Ok(out)
}

fn decode_utf16(b: &[u8], start: usize) -> Result<String, AxmlError> {
    let len0 = rd_u16(b, start)?;
    let (len, adv) = if len0 & 0x8000 != 0 {
        let hi = (len0 & 0x7fff) as u32;
        let lo = rd_u16(b, start + 2)? as u32;
        ((hi << 16) | lo, 4)
    } else {
        (len0 as u32, 2)
    };
    let mut s = String::new();
    let mut p = start + adv;
    for _ in 0..len {
        let cu = rd_u16(b, p)?;
        p += 2;
        if (0xD800..=0xDBFF).contains(&cu) {
            let lo = rd_u16(b, p)?;
            p += 2;
            let c = 0x10000 + (((cu - 0xD800) as u32) << 10) + ((lo - 0xDC00) as u32);
            s.push(char::from_u32(c).unwrap_or('\u{fffd}'));
        } else {
            s.push(char::from_u32(cu as u32).unwrap_or('\u{fffd}'));
        }
    }
    Ok(s)
}

fn read_utf8_len(b: &[u8], start: usize) -> Result<(u32, usize), AxmlError> {
    let b0 = rd_u8(b, start)?;
    if b0 & 0x80 == 0 {
        Ok((b0 as u32, 1))
    } else {
        let b1 = rd_u8(b, start + 1)?;
        Ok((((b0 & 0x7f) as u32) << 8 | (b1 as u32), 2))
    }
}

fn decode_utf8(b: &[u8], start: usize) -> Result<String, AxmlError> {
    let (_u16len, a1) = read_utf8_len(b, start)?;
    let (u8len, a2) = read_utf8_len(b, start + a1)?;
    let bytes_start = start + a1 + a2;
    let end = bytes_start + u8len as usize;
    if end > b.len() {
        return Err(AxmlError::OutOfBounds(end));
    }
    Ok(String::from_utf8_lossy(&b[bytes_start..end]).into_owned())
}

fn attr_value_to_string(pool: &[String], val_type: u8, data: u32, raw: u32) -> String {
    match val_type {
        TYPE_STRING => pool.get(data as usize).cloned().unwrap_or_default(),
        TYPE_INT_BOOLEAN => (data != 0).to_string(),
        TYPE_INT_DEC => data.to_string(),
        TYPE_INT_HEX => format!("0x{:x}", data),
        _ => {
            if raw != 0xFFFF_FFFF {
                pool.get(raw as usize).cloned().unwrap_or_default()
            } else {
                format!("(type 0x{:02x} data {})", val_type, data)
            }
        }
    }
}

/// Parse a `<...>` start element chunk and return its tag name plus a map of
/// attribute name -> value string.
fn parse_start_element(
    b: &[u8],
    off: usize,
    pool: &[String],
) -> Result<(String, HashMap<String, String>), AxmlError> {
    // ResXMLTree_node layout (offsets from the chunk start):
    //   type(2) headerSize(2) size(4) lineNumber(4) comment(4) ns(4) name(4)
    //   attrCount(2) attrStart(2) attrSize(2) idIndex(2) classIndex(2) styleIndex(2)
    // So: ns @+16, name @+20, attrCount @+24, attrStart @+26, attrSize @+28.
    let name_idx = rd_u32(b, off + 20)?;
    let attr_count = rd_u16(b, off + 24)? as usize;
    let attr_start = rd_u16(b, off + 26)? as usize;
    let attr_size = rd_u16(b, off + 28)? as usize;

    let elem_name = if name_idx == 0xFFFF_FFFF {
        "?".to_string()
    } else {
        pool.get(name_idx as usize).cloned().unwrap_or_default()
    };

    let mut attrs = HashMap::new();
    if attr_size == 0 {
        return Ok((elem_name, attrs));
    }
    for i in 0..attr_count {
        let ab = off + attr_start + i * attr_size;
        let mut p = ab;
        // AAPT1 emits a per-attribute ResChunk_header (8 bytes); AAPT2 does not.
        // The reliable signal is the attribute stride declared in the node
        // header: 28 bytes => header present, 20 bytes => none.
        if attr_size > 20 {
            p += 8;
        }
        let _ns = rd_u32(b, p)?;
        p += 4;
        let name_i = rd_u32(b, p)?;
        p += 4;
        let raw = rd_u32(b, p)?;
        p += 4;
        let _val_size = rd_u16(b, p)?;
        p += 2;
        let _res0 = rd_u8(b, p)?;
        p += 1;
        let val_type = rd_u8(b, p)?;
        p += 1;
        let val_data = rd_u32(b, p)?;

        let aname = if name_i == 0xFFFF_FFFF {
            attr_name_from_id(raw)
                .map(str::to_string)
                .unwrap_or_else(|| format!("attr_0x{:08x}", raw))
        } else {
            pool.get(name_i as usize).cloned().unwrap_or_default()
        };
        let value = attr_value_to_string(pool, val_type, val_data, raw);
        attrs.insert(aname, value);
    }
    Ok((elem_name, attrs))
}

fn component_from_attrs(attrs: &HashMap<String, String>) -> Component {
    Component {
        name: attrs.get("name").cloned().unwrap_or_default(),
        exported: attrs.get("exported").and_then(|s| s.parse::<bool>().ok()),
        enabled: attrs.get("enabled").and_then(|s| s.parse::<bool>().ok()),
        permission: attrs.get("permission").cloned(),
    }
}

fn apply_element(m: &mut ApkManifest, elem: &str, attrs: &HashMap<String, String>) {
    match elem {
        "manifest" => {
            if let Some(p) = attrs.get("package") {
                m.package = p.clone();
            }
            m.version_code = attrs.get("versionCode").cloned();
            m.version_name = attrs.get("versionName").cloned();
        }
        "uses-sdk" => {
            m.min_sdk_version = attrs.get("minSdkVersion").and_then(|s| s.parse().ok());
            m.target_sdk_version = attrs.get("targetSdkVersion").and_then(|s| s.parse().ok());
            m.max_sdk_version = attrs.get("maxSdkVersion").and_then(|s| s.parse().ok());
            m.compile_sdk_version = attrs.get("compileSdkVersion").and_then(|s| s.parse().ok());
        }
        "uses-permission" | "uses-permission-sdk-23" => {
            if let Some(n) = attrs.get("name") {
                let max = attrs.get("maxSdkVersion").and_then(|s| s.parse().ok());
                m.permissions.push(Permission {
                    name: n.clone(),
                    max_sdk_version: max,
                });
            }
        }
        "application" => {
            m.application = Some(component_from_attrs(attrs));
        }
        "activity" | "activity-alias" => {
            m.activities.push(component_from_attrs(attrs));
        }
        "service" => m.services.push(component_from_attrs(attrs)),
        "receiver" => m.receivers.push(component_from_attrs(attrs)),
        "provider" => m.providers.push(component_from_attrs(attrs)),
        _ => {}
    }
}

/// Parse the binary `AndroidManifest.xml` payload and return the extracted
/// manifest facts.
pub fn parse_axml(bytes: &[u8]) -> Result<ApkManifest, AxmlError> {
    if bytes.len() < 8 {
        return Err(AxmlError::Invalid("file too small to be AXML"));
    }
    let mut manifest = ApkManifest::default();
    let mut pool: Vec<String> = Vec::new();
    let end = bytes.len();
    let mut off = 0usize;
    while off + 8 <= end {
        let ctype = rd_u16(bytes, off)?;
        let csize = rd_u32(bytes, off + 4)? as usize;
        if csize == 0 {
            break;
        }
        match ctype {
            RES_STRING_POOL_TYPE => {
                pool = parse_string_pool(bytes, off, csize)?;
            }
            RES_XML_START_ELEMENT_TYPE => {
                let (elem, attrs) = parse_start_element(bytes, off, &pool)?;
                apply_element(&mut manifest, &elem, &attrs);
            }
            RES_XML_TYPE => {
                // Top-level container; skip its header, children follow.
                off += 8;
                continue;
            }
            // Resource map / namespaces / end-element / cdata: skip by size.
            RES_XML_RESOURCE_MAP_TYPE
            | RES_XML_START_NAMESPACE_TYPE
            | RES_XML_END_NAMESPACE_TYPE
            | RES_XML_END_ELEMENT_TYPE
            | RES_XML_CDATA_TYPE => {}
            _ => {}
        }
        off += csize;
    }
    Ok(manifest)
}

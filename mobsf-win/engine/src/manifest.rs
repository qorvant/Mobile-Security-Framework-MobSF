//! Data types describing the interesting parts of an Android manifest.

use serde::Serialize;

/// A component declared in the manifest (activity / service / receiver / provider).
#[derive(Debug, Clone, Serialize, Default)]
pub struct Component {
    pub name: String,
    pub exported: Option<bool>,
    pub enabled: Option<bool>,
    pub permission: Option<String>,
}

/// A `<uses-permission>` declaration.
#[derive(Debug, Clone, Serialize, Default)]
pub struct Permission {
    pub name: String,
    pub max_sdk_version: Option<u32>,
}

/// The subset of `AndroidManifest.xml` that MobSF's static analyzer surfaces
/// first. Everything here is extractable without invoking any external tool.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ApkManifest {
    pub package: String,
    pub version_code: Option<String>,
    pub version_name: Option<String>,
    pub min_sdk_version: Option<u32>,
    pub target_sdk_version: Option<u32>,
    pub max_sdk_version: Option<u32>,
    pub compile_sdk_version: Option<u32>,
    pub permissions: Vec<Permission>,
    pub activities: Vec<Component>,
    pub services: Vec<Component>,
    pub receivers: Vec<Component>,
    pub providers: Vec<Component>,
    pub application: Option<Component>,
}

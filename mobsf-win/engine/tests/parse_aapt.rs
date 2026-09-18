//! Validate the `aapt dump badging` parser against a representative sample.
//! Because `aapt` itself is not installed in CI here, we test the deterministic
//! textual parser directly; the full pipeline (running aapt) is covered by the
//! integration with real APKs once aapt is available on the host.

use mobsf_engine::parse_aapt_badging;

const SAMPLE: &str = r#"
package: name='com.example.justtrustme' versionCode='12' versionName='1.2.3' platformBuildVersionName=''
install-location:'auto'
sdkVersion:'16'
targetSdkVersion:'22'
maxSdkVersion:'28'
uses-permission: name='android.permission.INTERNET'
uses-permission: name='android.permission.READ_CONTACTS' maxSdkVersion='28'
uses-permission-sdk-23: name='android.permission.ACCESS_FINE_LOCATION'
application: label='JustTrustMe' icon='res/drawable/ic_launcher.png'
launchable-activity: name='com.example.justtrustme.MainActivity'  label='' icon='' exported='true'
activity: name='com.example.justtrustme.Settings' exported='false'
service: name='com.example.justtrustme.MyService'
receiver: name='com.example.justtrustme.MyReceiver' exported='true'
provider: name='com.example.justtrustme.MyProvider' exported='false' permission='com.example.PERM'
"#;

#[test]
fn parse_aapt_badging_sample() {
    let m = parse_aapt_badging(SAMPLE);

    assert_eq!(m.package, "com.example.justtrustme");
    assert_eq!(m.version_code.as_deref(), Some("12"));
    assert_eq!(m.version_name.as_deref(), Some("1.2.3"));
    assert_eq!(m.min_sdk_version, Some(16));
    assert_eq!(m.target_sdk_version, Some(22));
    assert_eq!(m.max_sdk_version, Some(28));

    assert_eq!(m.permissions.len(), 3);
    assert_eq!(m.permissions[0].name, "android.permission.INTERNET");
    assert_eq!(m.permissions[1].name, "android.permission.READ_CONTACTS");
    assert_eq!(m.permissions[1].max_sdk_version, Some(28));
    assert_eq!(
        m.permissions[2].name,
        "android.permission.ACCESS_FINE_LOCATION"
    );

    assert_eq!(m.activities.len(), 2);
    assert_eq!(
        m.activities[0].name,
        "com.example.justtrustme.MainActivity"
    );
    assert_eq!(m.activities[0].exported, Some(true));
    assert_eq!(m.activities[1].exported, Some(false));

    assert_eq!(m.services.len(), 1);
    assert_eq!(m.services[0].name, "com.example.justtrustme.MyService");

    assert_eq!(m.receivers.len(), 1);
    assert_eq!(m.receivers[0].exported, Some(true));

    assert_eq!(m.providers.len(), 1);
    assert_eq!(m.providers[0].exported, Some(false));
    assert_eq!(
        m.providers[0].permission,
        Some("com.example.PERM".to_string())
    );
}

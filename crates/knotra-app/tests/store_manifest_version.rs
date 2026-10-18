//! RFC-064: the Microsoft Store manifest's `Identity` `Version` must track
//! the crate's own version, plus a trailing `.0` (Partner Center reserves
//! the fourth part, always 0 — RFC-061 R3). Release prep bumps
//! `Cargo.toml` and `packaging/windows/AppxManifest.xml` together; this is
//! what catches it when only one gets bumped.

use std::fs;
use std::path::Path;

const REPO_RELATIVE_PATH: &str = "packaging/windows/AppxManifest.xml";

/// Finds `Version="..."` within `identity_text` (the `<Identity ...>`
/// element's own text, not the whole document), preceded by whitespace —
/// never a bare substring match. `MinVersion="..."` contains the literal
/// substring `Version="..."` too, with `n` rather than whitespace
/// immediately before it; `MinVersion`/`MaxVersionTested` sit on a
/// different element in this manifest (`Dependencies > TargetDeviceFamily`),
/// and must not be able to satisfy this even if they moved into `Identity`.
fn find_version_attribute(identity_text: &str) -> Option<String> {
    let bytes = identity_text.as_bytes();
    let needle = "Version=\"";
    let mut search_from = 0;
    while let Some(rel) = identity_text[search_from..].find(needle) {
        let at = search_from + rel;
        if at > 0 && bytes[at - 1].is_ascii_whitespace() {
            let value_start = at + needle.len();
            let end = identity_text[value_start..].find('"')?;
            return Some(identity_text[value_start..value_start + end].to_string());
        }
        search_from = at + needle.len();
    }
    None
}

#[test]
fn store_manifest_version_matches_the_crate() {
    let crate_version = env!("CARGO_PKG_VERSION");
    let core = crate_version
        .split(['-', '+'])
        .next()
        .expect("CARGO_PKG_VERSION is never empty");
    let expected = format!("{core}.0");

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(REPO_RELATIVE_PATH);
    let xml = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));

    let identity_start = xml
        .find("<Identity")
        .unwrap_or_else(|| panic!("{REPO_RELATIVE_PATH} has no <Identity ...> element"));
    let identity_rest = &xml[identity_start..];
    let close = identity_rest.find("/>").unwrap_or_else(|| {
        panic!("{REPO_RELATIVE_PATH}'s <Identity ...> element has no closing `/>`")
    });
    let identity_text = &identity_rest[..close];

    let actual = find_version_attribute(identity_text).unwrap_or_else(|| {
        panic!("{REPO_RELATIVE_PATH}'s <Identity ...> element has no Version attribute")
    });

    assert_eq!(
        actual, expected,
        "{REPO_RELATIVE_PATH} Identity Version is {actual} but the crate is {crate_version} \
         (expected {expected}) - bump {REPO_RELATIVE_PATH} with Cargo.toml"
    );
}

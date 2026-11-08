//! RFC-068 D6: the AppStream metainfo's newest `<release version="...">`
//! must track the crate's own version. Release prep bumps `Cargo.toml` and
//! adds a new `<release>` entry together; this is what catches it when
//! only one gets done.

use std::fs;
use std::path::Path;

const REPO_RELATIVE_PATH: &str = "packaging/linux/io.github.nabbisen.knotra.metainfo.xml";

/// Every `version="..."` attribute on a `<release ...>` element, in
/// document order — plain substring scanning, matching
/// `store_manifest_version.rs`'s own approach rather than pulling in an XML
/// parser for one attribute.
fn release_versions(xml: &str) -> Vec<String> {
    let mut versions = Vec::new();
    let mut rest = xml;
    while let Some(rel) = rest.find("<release ") {
        let tag_start = rel + "<release ".len();
        let Some(tag_end) = rest[tag_start..].find('>') else {
            break;
        };
        let attrs = &rest[tag_start..tag_start + tag_end];
        if let Some(v_rel) = attrs.find("version=\"") {
            let value_start = v_rel + "version=\"".len();
            if let Some(end) = attrs[value_start..].find('"') {
                versions.push(attrs[value_start..value_start + end].to_string());
            }
        }
        rest = &rest[tag_start + tag_end..];
    }
    versions
}

#[test]
fn metainfo_newest_release_matches_the_crate() {
    let crate_version = env!("CARGO_PKG_VERSION");

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(REPO_RELATIVE_PATH);
    let xml = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", path.display()));

    let versions = release_versions(&xml);
    let newest = versions
        .first()
        .unwrap_or_else(|| panic!("{REPO_RELATIVE_PATH} has no <release version=\"...\"> entry"));

    assert_eq!(
        newest, crate_version,
        "{REPO_RELATIVE_PATH}'s newest <release> is {newest} but the crate is \
         {crate_version} - add a new <release> entry with release prep"
    );
}

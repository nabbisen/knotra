//! RFC-043 D4/R5: keeps `#[allow(dead_code)]` at its justified floor.
//!
//! RFC-043 eliminated 39 suppressions hiding 176 findings — most of them
//! blanket, on whole enums, exempting every variant forever. What survives
//! this crate's deletion pass (Handoff 053) is a small, explicit, per-item
//! set, each with a comment at its own site naming why. This guard fails if
//! any `#[allow(dead_code)]` appears anywhere in `crates/knotra-app/src/`
//! other than at one of those named locations — so a new suppression has to
//! pass through this file, not get added silently the way the original 39
//! did.
//!
//! Text-scanning, not a real parser, matching this project's established
//! guard style (RFC-042, `knotra-ui/src/i18n.rs`) and its acknowledged
//! tradeoff: simple enough to read at a glance, and R3 requires proving it
//! actually fails before trusting it — see the Handoff 053 review request
//! for the planted-violation failure message this guard produced.
//!
//! `#[cfg(test)]`-only: this module (and its `walkdir`-free scan) never
//! compiles into the shipped binary.

#[cfg(test)]
mod tests {
    /// Recursively collect every `.rs` file under `dir`. No
    /// filesystem-walking dependency exists in this crate, and `src/` is a
    /// few dozen files — small enough that a hand-rolled walk is simpler
    /// than adding one for this test, matching `i18n.rs`'s own guards.
    fn rust_files_under(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return files;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(rust_files_under(&path));
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
        files
    }

    /// `crates/knotra-app/src`, resolved from this crate's own manifest
    /// directory — R1's scope is `crates/knotra-app` specifically, not the
    /// workspace.
    fn knotra_app_src_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    /// This file's own doc comments and string literals describe and
    /// contain the literal text `#[allow(dead_code)]` — the pattern this
    /// guard scans for — so scanning `dead_code_guard.rs` itself finds
    /// those, not real attributes. Confirmed by observing the exact false
    /// positive this produced before the exclusion was added (5 matches,
    /// none of them real): see the Handoff 053 review request. Same
    /// reasoning `i18n.rs`'s own guards use to exclude themselves.
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("dead_code_guard.rs")
    }

    /// The exact, justified set of `#[allow(dead_code)]` occurrences this
    /// crate carries, as (file path relative to `src/`, count in that
    /// file). Both are held back for the owner (`053` §3, `055` §5) —
    /// `LaunchMessage::OpenInMergeTool` (a live control with no producer)
    /// and `TopologyPhase::Ready`'s payload (a live scan whose result is
    /// discarded) — feature and compatibility decisions, not triage
    /// questions. Every item RFC-043's triage found was either deleted or
    /// is one of these two; adding a new suppression means adding it here
    /// too, with the same per-item justification this file's own doc
    /// comment describes — not widening one of these counts to cover
    /// something unrelated. This list's right end state is empty.
    const EXPECTED: &[(&str, usize)] = &[("message.rs", 1), ("state/topology.rs", 1)];

    fn count_occurrences(source: &str, pattern: &str) -> usize {
        let mut count = 0;
        let mut rest = source;
        while let Some(pos) = rest.find(pattern) {
            count += 1;
            rest = &rest[pos + pattern.len()..];
        }
        count
    }

    #[test]
    fn allow_dead_code_stays_at_its_justified_locations() {
        let src = knotra_app_src_dir();
        let files = rust_files_under(&src);
        assert!(
            files.len() > 20,
            "found only {} .rs files under {} -- path resolution is \
             broken (expected 30+), not that nothing needed checking",
            files.len(),
            src.display()
        );

        let mut actual: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for file in files.iter().filter(|f| is_scan_target(f)) {
            let Ok(source) = std::fs::read_to_string(file) else {
                continue;
            };
            let n = count_occurrences(&source, "#[allow(dead_code)]");
            if n > 0 {
                let rel = file
                    .strip_prefix(&src)
                    .unwrap_or(file)
                    .to_string_lossy()
                    .replace('\\', "/");
                actual.insert(rel, n);
            }
        }

        let expected: std::collections::BTreeMap<String, usize> = EXPECTED
            .iter()
            .map(|(path, n)| ((*path).to_owned(), *n))
            .collect();

        assert_eq!(
            actual, expected,
            "#[allow(dead_code)] locations under crates/knotra-app/src/ \
             changed. If you added a new suppression, name it in \
             dead_code_guard.rs's EXPECTED list with the same per-item \
             justification its neighbours carry — a blanket container-level \
             allow or an unexplained new entry is exactly what RFC-043 \
             removed 39 of. If you deleted or resolved one of the existing \
             ones, remove it from EXPECTED here too.\n\
             actual:   {actual:?}\n\
             expected: {expected:?}"
        );
    }
}

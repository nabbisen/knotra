//! RFC-052 D2/D4: keeps every lint suppression, in every spelling, at a
//! justified and named floor — across all three crates.
//!
//! This replaces `dead_code_guard.rs` (RFC-043 D4/R5), which matched only
//! the literal string `#[allow(dead_code)]` in `crates/knotra-app/src/`.
//! Three files carried `#![allow(unused_imports, unused_variables,
//! dead_code)]` on line 1 — a strictly broader form, invisible to that
//! matcher — and five more suppressions across all three crates (two
//! `#[allow(clippy::large_enum_variant)]`, one `#[allow(unreachable_patterns)]`,
//! one `#[allow(clippy::too_many_arguments)]`, one more `#[allow(dead_code)]`
//! outside `knotra-app`) were never tracked by anything at all.
//!
//! This guard matches **any** `#[allow(...)]` or `#![allow(...)]` —
//! inner or outer, single- or multi-lint, any lint name, and any attribute
//! that merely *contains* `allow(` somewhere in its body (so
//! `#[cfg_attr(test, allow(dead_code))]`, the shape RFC-052 A1 narrowed
//! three of these to, is caught exactly as a bare `#[allow(dead_code)]`
//! would be — a guard that could not see its own handoff's output would
//! repeat the defect this RFC exists to fix). It scans `crates/`, covering
//! `knotra-app`, `knotra-ui` and `knotra-vcs`, and asserts an exact
//! expected map: the same shape `dead_code_guard.rs` used, widened in
//! scope rather than changed in kind.
//!
//! Text-scanning, not a real parser — same tradeoff `dead_code_guard.rs`
//! and `i18n.rs`'s guards accept. Bracket depth is tracked to find each
//! attribute's closing `]`, which is enough for every attribute shape this
//! guard has ever needed to match; none of them nest a `[`/`]` pair inside
//! their arguments.
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped
//! binary.

#[cfg(test)]
mod tests {
    /// Recursively collect every `.rs` file under `dir`. Copied rather than
    /// shared with `i18n.rs`'s or `text_outside_catalog_guard.rs`'s own
    /// walkers, matching this project's established precedent of a small
    /// hand-rolled walk per guard over a shared dependency.
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

    /// `crates/`, resolved from this crate's own manifest directory —
    /// `knotra-app` lives at `crates/knotra-app`, so one `..` reaches it.
    /// Same resolution `i18n.rs`'s `crates_dir()` uses to scan across all
    /// three crates from `knotra-ui`.
    fn crates_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    /// This file's own doc comment and prose describe and contain the
    /// literal patterns this guard scans for (`#[allow(`, `#![allow(`,
    /// `allow(`) — scanning `suppressions_guard.rs` itself would find
    /// those, not real attributes. Confirmed by reading this file's own
    /// module doc comment, which names several attribute forms directly.
    /// Same reasoning `dead_code_guard.rs`, `text_outside_catalog_guard.rs`
    /// and `i18n.rs`'s own guards use to exclude themselves — named here so
    /// this is not rediscovered a fourth time (`067`, `068` twice already
    /// have).
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("suppressions_guard.rs")
    }

    /// Every `[...]` body immediately following each occurrence of
    /// `prefix` (`"#!["` for an inner attribute, `"#["` for an outer one)
    /// in `source`, found by counting `[`/`]` depth from the opening
    /// bracket — not a parser, but sufficient for every attribute this
    /// guard has ever needed to match. `"#["` cannot spuriously match
    /// inside `"#!["` (the second byte differs, `!` vs `[`), so the two
    /// prefixes never double-count the same real occurrence.
    fn attribute_bodies_after<'a>(source: &'a str, prefix: &str) -> Vec<&'a str> {
        let mut bodies = Vec::new();
        let mut rest = source;
        while let Some(pos) = rest.find(prefix) {
            // `prefix` already includes the opening `[`, so depth starts at 1.
            let after_open = &rest[pos + prefix.len()..];
            let mut depth = 1i32;
            let mut end = None;
            for (i, c) in after_open.char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(end) = end else { break };
            bodies.push(&after_open[..end]);
            rest = &after_open[end..];
        }
        bodies
    }

    /// The count of `#[...]`/`#![...]` attributes in `source` whose body
    /// contains `allow(` — a bare `#[allow(dead_code)]`, a multi-lint
    /// `#[allow(a, b)]`, an inner `#![allow(x)]`, and a wrapped
    /// `#[cfg_attr(test, allow(dead_code))]` all count once each (R2).
    fn count_allow_attributes(source: &str) -> usize {
        let mut count = 0;
        for prefix in ["#![", "#["] {
            for body in attribute_bodies_after(source, prefix) {
                if body.contains("allow(") {
                    count += 1;
                }
            }
        }
        count
    }

    /// The exact, justified set of suppressions this project carries,
    /// across all three crates, verified at Handoff 072's baseline
    /// (`a09105a`). Each entry has a comment at its own site naming why —
    /// see that site, not this list, for the justification itself.
    ///
    /// RFC-053 removed three of the eight RFC-052 tracked: `message.rs`'s
    /// `large_enum_variant` and `app/freezer.rs`'s `unreachable_patterns`
    /// (both confirmed inert — tested by removing each and re-running the
    /// gate, which stayed clean either way — and this RFC was the
    /// authority Handoff 071 lacked to delete them), and
    /// `knotra-ui/src/widget/overlay.rs`'s `too_many_arguments` (the
    /// `ResolvedWidth` newtype gave `surface()` back its seventh parameter
    /// without losing the enforcement the eighth parameter existed for).
    /// `knotra-vcs/src/vcs/git.rs`'s `tag_exists` entry is deliberately
    /// kept, not up for removal (RFC-052/RFC-053 non-goals, both).
    const EXPECTED: &[(&str, usize)] = &[
        ("knotra-app/src/state/palette.rs", 1),
        ("knotra-app/src/view/command_palette.rs", 1),
        ("knotra-app/src/view/detail_panel.rs", 1),
        ("knotra-app/src/view/shortcuts_overlay.rs", 1),
        ("knotra-vcs/src/vcs/git.rs", 1),
    ];

    #[test]
    fn every_suppression_is_tracked_and_justified() {
        let dir = crates_dir();
        let files = rust_files_under(&dir);
        assert!(
            files.len() > 90,
            "found only {} .rs files under {} -- path resolution is \
             broken (expected 90+ across all three crates), not that \
             nothing needed checking",
            files.len(),
            dir.display()
        );

        let mut actual: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for file in files.iter().filter(|f| is_scan_target(f)) {
            let Ok(source) = std::fs::read_to_string(file) else {
                continue;
            };
            let n = count_allow_attributes(&source);
            if n > 0 {
                let rel = file
                    .strip_prefix(&dir)
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
            "lint suppressions under crates/ changed. If you added a new \
             one, name it in suppressions_guard.rs's EXPECTED list with a \
             justification comment at its own site — a suppression with no \
             comment, in any form, is exactly what this guard exists to \
             catch. If you removed or resolved one, remove its entry from \
             EXPECTED here too.\n\
             actual:   {actual:?}\n\
             expected: {expected:?}"
        );
    }
}

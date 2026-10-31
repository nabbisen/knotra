//! RFC-071 D2: catches a text `Weight` set anywhere outside the one place
//! permitted — `strong()` in `knotra-ui/src/widget/layout.rs`. The shape of
//! `text_size_guard.rs`: text-scanning, skips comment lines (the same
//! reasoning — a doc comment describing this guard's own pattern is prose,
//! not a call site), and asserts an exact expected map rather than "zero
//! everywhere", so the one legitimate site is pinned by name and count, not
//! merely allowed to slip past a looser "nothing outside view/" rule.
//!
//! **Scans the whole `crates/` tree**, not only `view/`/`widget/` the way
//! `text_size_guard.rs` does — a weight is not inherently a view-layer
//! concern the way a text size is, so RFC-071 D2's "No view names a weight
//! itself" is read at its widest: nowhere in any of the three crates names
//! one, except the single helper this guard pins.
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped binary.

#[cfg(test)]
mod tests {
    /// Recursively collect every `.rs` file under `dir` — copied from
    /// `suppressions_guard.rs`/`text_size_guard.rs` rather than shared,
    /// matching this project's established precedent of a small hand-rolled
    /// walk per guard.
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

    /// `crates/`, resolved from this crate's own manifest directory — the
    /// same reach `suppressions_guard.rs`'s own `crates_dir()` uses to cover
    /// all three sibling crates, not only this one.
    fn crates_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    /// This file's own doc comment and prose describe the pattern it scans
    /// for (`Weight::`) inside backtick-quoted text — scanning
    /// `text_weight_guard.rs` itself would find those, not a real call site.
    /// Same self-exclusion every guard in this project uses.
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("text_weight_guard.rs")
    }

    /// True if `trimmed` (a line with leading whitespace already stripped)
    /// is a comment line — `//`, `///`, or `//!`. Rust syntax never places a
    /// real `Weight::` reference at the start of such a line, so this
    /// cannot mask a live site; it exists only to skip prose like this
    /// file's own, and `layout.rs`'s doc comment on `strong()`, which names
    /// the pattern without being one.
    fn is_comment_line(trimmed: &str) -> bool {
        trimmed.starts_with("//")
    }

    /// The count of `Weight::` occurrences in `source`, outside comment
    /// lines.
    fn count_weight_uses(source: &str) -> usize {
        let mut count = 0;
        for line in source.lines() {
            let trimmed = line.trim_start();
            if is_comment_line(trimmed) {
                continue;
            }
            let mut rest = line;
            while let Some(pos) = rest.find("Weight::") {
                count += 1;
                rest = &rest[pos + "Weight::".len()..];
            }
        }
        count
    }

    /// The one site RFC-071 D2 permits: `strong()`'s own
    /// `iced::font::Weight::Semibold`, once.
    const EXPECTED: &[(&str, usize)] = &[("knotra-ui/src/widget/layout.rs", 1)];

    #[test]
    fn a_text_weight_is_set_only_by_strong() {
        let dir = crates_dir();
        let files = rust_files_under(&dir);
        assert!(
            files.len() > 50,
            "found only {} .rs files under {} -- path resolution is \
             broken (expected 50+ across all three crates), not that \
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
            let n = count_weight_uses(&source);
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
            "`Weight::` found outside `strong()` (RFC-071 D2: weight is set \
             in exactly one place). Route a new need for emphasis through \
             `knotra_ui::widget::strong` instead of naming a `Weight` \
             directly.\n\
             actual:   {actual:?}\n\
             expected: {expected:?}"
        );
    }
}

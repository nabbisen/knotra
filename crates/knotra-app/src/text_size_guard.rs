//! RFC-056 Stage 2 D4/R8: catches a raw `.size(<numeric literal>)` re-entering
//! the view tree.
//!
//! `crates/knotra-app/src/view/` and `crates/knotra-ui/src/widget/` had 249
//! such call sites before this stage — 159 via the retired `FONT_BODY`/
//! `FONT_SMALL` constants (and their `+ N.0` arithmetic), 90 as bare
//! literals, 38 of those below snora's 12px legibility floor. All 249 now
//! read a size from a `snora::design::style::text::*_size(tokens)` role
//! helper. This is the guard that keeps a literal from quietly coming back —
//! the same shape `suppressions_guard.rs` and `text_outside_catalog_guard.rs`
//! use for their own regressions, widened to a fourth source-scanning guard.
//!
//! **Text-scanning, not a real parser** — same tradeoff every guard in this
//! project accepts. Two things this scan deliberately does *not* count:
//!
//! - **Comment lines** (`//`, `///`, `//!`). A raw `.size(11)` is common
//!   prose when a doc comment explains what a role replaced — two sites in
//!   `detail_panel.rs` do exactly this, describing `IDENTITY_LABEL_WIDTH`/
//!   `STATUS_LABEL_WIDTH`'s original fit. Counting those would make the
//!   guard fail on its own migration notes, not on live code. A line is
//!   skipped whenever its trimmed text starts with `//` — this cannot be
//!   fooled by a real call site, since Rust syntax never places `.size(`
//!   at the start of a comment-opening line.
//! - **`.size(<identifier>)` calls that are not a bare number** —
//!   `checkbox.rs`'s `.size(BOX_SIZE)` sizes an icon glyph, not text, and a
//!   named constant is not the literal this guard exists to catch. The scan
//!   only flags a `.size(` immediately followed (after optional whitespace)
//!   by an ASCII digit.
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped binary.

#[cfg(test)]
mod tests {
    /// Recursively collect every `.rs` file under `dir` — copied from
    /// `suppressions_guard.rs`/`text_outside_catalog_guard.rs` rather than
    /// shared, matching this project's established precedent of a small
    /// hand-rolled walk per guard.
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

    /// The two directories R8 scopes this guard to, resolved from this
    /// crate's own manifest directory the same way `suppressions_guard.rs`'s
    /// `crates_dir()` reaches a sibling crate.
    fn scan_dirs() -> Vec<std::path::PathBuf> {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        vec![
            manifest.join("src/view"),
            manifest.join("../knotra-ui/src/widget"),
        ]
    }

    /// This file's own doc comment names the exact pattern it scans for
    /// (`.size(11)`, `.size(BOX_SIZE)`) inside backtick-quoted prose —
    /// scanning `text_size_guard.rs` itself would find those, not real call
    /// sites. Same self-exclusion reasoning every guard in this project
    /// uses (`suppressions_guard.rs`, `text_outside_catalog_guard.rs`).
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("text_size_guard.rs")
    }

    /// True if `trimmed` (a line with leading whitespace already stripped)
    /// is a comment line — `//`, `///`, or `//!`. Rust syntax never places
    /// a real `.size(` call at the start of a line beginning with `//`, so
    /// this cannot mask a live call site; it exists only to skip the prose
    /// this guard's own doc comment above explains.
    fn is_comment_line(trimmed: &str) -> bool {
        trimmed.starts_with("//")
    }

    /// The count of `.size(` occurrences in `source`, outside comment
    /// lines, whose next non-whitespace character is an ASCII digit — a
    /// raw numeric literal, not a role helper call (`.size(snora::design::
    /// style::text::body_size(tokens))`, which is never immediately
    /// followed by a digit) or a named constant (`.size(BOX_SIZE)`).
    fn count_raw_size_literals(source: &str) -> usize {
        let mut count = 0;
        for line in source.lines() {
            let trimmed = line.trim_start();
            if is_comment_line(trimmed) {
                continue;
            }
            let mut rest = line;
            while let Some(pos) = rest.find(".size(") {
                let after = &rest[pos + ".size(".len()..];
                if after.trim_start().starts_with(|c: char| c.is_ascii_digit()) {
                    count += 1;
                }
                rest = after;
            }
        }
        count
    }

    /// R8: zero raw `.size(<numeric literal>)` sites remain under either
    /// scanned directory after RFC-056 Stage 2's migration. Unlike
    /// `suppressions_guard.rs`'s non-empty `EXPECTED` map (five tracked,
    /// justified suppressions), this guard's own map is empty by design —
    /// every one of the 249 pre-migration sites has a role, and R3 requires
    /// every future site to get one too, not to earn a place on an
    /// exceptions list. A new entry appearing here means a literal came
    /// back, not that one was blessed.
    #[test]
    fn no_raw_text_size_literal_remains_in_the_view_tree() {
        let mut actual: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        let mut total_files = 0;

        for dir in scan_dirs() {
            let files = rust_files_under(&dir);
            total_files += files.len();
            for file in files.iter().filter(|f| is_scan_target(f)) {
                let Ok(source) = std::fs::read_to_string(file) else {
                    continue;
                };
                let n = count_raw_size_literals(&source);
                if n > 0 {
                    actual.insert(file.display().to_string(), n);
                }
            }
        }

        assert!(
            total_files > 20,
            "found only {total_files} .rs files under the two scanned \
             directories -- path resolution is broken (expected 20+ across \
             view/ and widget/), not that nothing needed checking"
        );

        assert!(
            actual.is_empty(),
            "raw `.size(<numeric literal>)` call sites found outside \
             comments -- RFC-056 R8 requires every text size to come from a \
             `snora::design::style::text::*_size(tokens)` role helper, not \
             a literal:\n{actual:#?}"
        );
    }
}

//! RFC-056 Stage 2 D4/R8: catches a raw `.size(<numeric literal>)` re-entering
//! the view tree. RFC-056 Stage 3 D5/R4 adds a second guard beside it: every
//! `body_size(`/`body_small_size(` call must carry a matching
//! `.line_height(...)` — the pairing Stage 3 exists to establish.
//!
//! `crates/knotra-app/src/view/` and `crates/knotra-ui/src/widget/` had 249
//! such call sites before Stage 2 — 159 via the retired `FONT_BODY`/
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
//! **Why the pairing check is assertable at all (Stage 3 §3/§4)**: Stage 3
//! applies `body`/`body_small` line-height *uniformly*, not per site judged
//! for whether it might wrap. That is what makes "every `body_size(`/
//! `body_small_size(` call has a paired `.line_height(...)`" a rule with no
//! exceptions to track — unlike `suppressions_guard.rs`'s non-empty
//! `EXPECTED` map, this guard's own map stays empty by design. `label_size(`/
//! `title_size(`/`heading_size(`/`display_size(` are deliberately **not**
//! checked for a pairing — `title` is byte-for-byte iced's own default
//! (1.3), and `label`/`heading` are tighter than default and single-line by
//! construction (RFC-056 Stage 3 §2); pairing them would be a defect this
//! guard would then have to un-invent. Do not widen this check to those four
//! roles without a decision to do so — silently "simplifying" the rule to
//! "every role gets a line-height" is exactly the drift this comment exists
//! to head off.
//!
//! The pairing scan walks the whole (comment-blanked) source as one string,
//! not line-by-line — `cargo fmt` wraps a long `.size(...).line_height(...)`
//! chain across several lines, so a per-line check would miss real pairs.
//! It does not compare the `.size(...)`/`.line_height(...)` call's argument
//! token for equality (e.g. catching a `tokens` paired with a stray
//! `other_tokens`) — every real call site in this codebase passes the same
//! binding to both, and a parser precise enough to compare expressions is
//! more machinery than this guard's stated tradeoff accepts.
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

    /// `source` with every comment line (`//`, `///`, `//!`) replaced by
    /// spaces of the same length — preserves every other byte's offset and
    /// every line's length, so the pairing scan below can walk the result as
    /// one string without a comment's prose being mistaken for a call site,
    /// the same reasoning `is_comment_line` applies per-line above.
    fn blank_comment_lines(source: &str) -> String {
        source
            .lines()
            .map(|line| {
                if is_comment_line(line.trim_start()) {
                    " ".repeat(line.len())
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The byte offset of the `)` matching the `(` at `open_paren`
    /// (`source.as_bytes()[open_paren]` must be `b'('`), by depth counting.
    /// Sufficient for this guard's call shapes — none of the arguments it
    /// scans (`tokens`, `&tokens`, `&state.theme.tokens`) contain a `(`.
    fn matching_close_paren(source: &str, open_paren: usize) -> Option<usize> {
        let bytes = source.as_bytes();
        let mut depth = 1i32;
        let mut i = open_paren + 1;
        while i < bytes.len() {
            match bytes[i] {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
            i += 1;
        }
        None
    }

    /// `(size role fn, its required line_height role fn)` — the only two
    /// roles Stage 3 applies leading to (D5/§2).
    const PAIRED_ROLES: &[(&str, &str)] = &[
        ("body_size", "body_line_height"),
        ("body_small_size", "body_small_line_height"),
    ];

    /// True if `after` (already whitespace-trimmed from the left) opens with
    /// `.line_height(` — allowing further whitespace, then a matching
    /// `snora::design::style::text::{lh_fn}(` — before the inner call.
    /// `cargo fmt` wraps `.line_height(` onto its own line when the whole
    /// chain is long, putting a newline *inside* what would otherwise be one
    /// contiguous literal (`".line_height(snora::design::..."`); checking
    /// `.line_height(` and the role call as two separately-trimmed pieces,
    /// rather than one fixed string, is what makes that wrap not a false
    /// mismatch.
    fn starts_with_line_height_call(after: &str, lh_fn: &str) -> bool {
        let Some(rest) = after.strip_prefix(".line_height(") else {
            return false;
        };
        rest.trim_start()
            .starts_with(&format!("snora::design::style::text::{lh_fn}("))
    }

    /// The count of `.size(snora::design::style::text::<role>_size(...))`
    /// call sites in `source` (comment lines already blanked) whose very
    /// next construct, after any whitespace, is **not** the matching
    /// `.line_height(snora::design::style::text::<role>_line_height(...))`.
    fn count_unpaired_body_roles(clean_source: &str) -> usize {
        let mut count = 0;
        for (size_fn, lh_fn) in PAIRED_ROLES {
            let size_prefix = format!(".size(snora::design::style::text::{size_fn}(");
            let mut search_from = 0;
            while let Some(rel) = clean_source[search_from..].find(&size_prefix) {
                let call_start = search_from + rel;
                // Position of `.size`'s own `(` — depth-counting from here
                // balances the inner `<role>_size(...)` call and returns the
                // outer `.size(...)` expression's own closing `)`.
                let outer_open = call_start + ".size".len();
                let Some(outer_close) = matching_close_paren(clean_source, outer_open) else {
                    search_from = call_start + size_prefix.len();
                    continue;
                };
                let after = clean_source[outer_close + 1..].trim_start();
                if !starts_with_line_height_call(after, lh_fn) {
                    count += 1;
                }
                search_from = outer_close + 1;
            }
        }
        count
    }

    /// R4: every `body_size(`/`body_small_size(` call in either scanned
    /// directory carries a matching `.line_height(...)` call — assertable
    /// only because Stage 3 applies it uniformly rather than per site (see
    /// this file's module doc comment). Like `no_raw_text_size_literal_
    /// remains_in_the_view_tree` above, the expected map is empty by
    /// design: a new entry means a `body`/`body_small` size gained no
    /// leading, not that one earned a place on an exceptions list.
    #[test]
    fn every_body_role_size_call_carries_a_matching_line_height() {
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
                let clean = blank_comment_lines(&source);
                let n = count_unpaired_body_roles(&clean);
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
            "`body_size(`/`body_small_size(` call sites found with no \
             matching `.line_height(...)` -- RFC-056 Stage 3 R4 requires \
             every body/body_small text size to carry its role's \
             line-height (label/title/heading/display are correctly \
             unpaired -- see this file's module doc comment):\n{actual:#?}"
        );
    }
}

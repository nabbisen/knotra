//! RFC-048 D4/R5: catches user-facing text that never reaches the catalog.
//!
//! Every existing guard (`dead_code_guard.rs`, `i18n.rs`'s guards) checks text
//! that already made it to `state.t()` or the catalog maps. None of them can
//! see a file that makes zero `t()` calls and hardcodes its strings directly
//! — `detail_panel.rs` did exactly that, for 21 strings, and every guard in
//! the project passed. This is the guard that closes that gap: it scans
//! `crates/knotra-app/src/view/` for `text("<letter>…")` and
//! `text(format!("<letter>…"))` — a literal string argument to `text(...)`
//! that starts with a letter — and asserts an exact expected map, the same
//! shape `dead_code_guard.rs` uses for its own dead-code suppressions.
//!
//! **What this guard cannot see** (say so, rather than let a reader assume
//! more coverage than exists):
//!
//! - **Bare values assigned to a variable and interpolated later** — the
//!   `"Yes"` / `"Unknown"` / `"No"` shape RFC-048 §1 found in
//!   `detail_panel.rs`, which the RFC's own initial survey (grepping for
//!   `text("…")` only) also missed for this exact reason. Those are not
//!   `text(...)` call arguments at their definition site.
//! - **Literals that do not start with a letter.** A string beginning with a
//!   digit or punctuation (not a glyph — glyphs are excluded on purpose, R8)
//!   would slip past the leading-letter check the same way a glyph does.
//! - **Struct fields populated from a `const` table** — a field access is
//!   not a literal, so no scan of `text(...)` call arguments will ever see
//!   it regardless of pattern. `shortcuts_overlay.rs`'s `BINDINGS: &[Binding]`
//!   was exactly this (its `context`/`desc` fields held real English text,
//!   rendered via `text(b.desc)`) until RFC-049 moved those fields to
//!   catalog keys resolved at render time — fixed now, but the blind spot
//!   itself is permanent: the next `const` table of `&'static str` UI text
//!   would be just as invisible here.
//! - **A literal that starts with a glyph but also carries real text** — the
//!   leading-letter check treats it the same as a pure-glyph string.
//!   `shortcuts_overlay.rs`'s `text("✕  Close")` was exactly this (`✕`
//!   leads, `Close` inside it was untranslated English) until RFC-049
//!   resolved the word through the catalog and kept only the glyph literal
//!   — fixed now, same permanent blind spot: a future glyph+text literal
//!   would slip past this check exactly the same way.
//!
//! Text-scanning, not a real parser — same tradeoff `dead_code_guard.rs` and
//! `i18n.rs`'s guards accept, and the same reason R3-style planted-violation
//! proof matters more than it would for a real parser.
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped binary.

#[cfg(test)]
mod tests {
    /// Recursively collect every `.rs` file under `dir` — copied from
    /// `dead_code_guard.rs` rather than shared, matching that file's own
    /// precedent of a small hand-rolled walk over adding a dependency for a
    /// test-only scan of a few dozen files.
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

    /// `crates/knotra-app/src/view`, resolved from this crate's own manifest
    /// directory — RFC-048's scope is `view/` specifically (`main.rs`,
    /// `state.rs`, `app/`, etc. are not UI text).
    fn view_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/view")
    }

    /// This file's own doc comment contains the literal patterns it scans
    /// for (`text("` and `text(format!("`), inside backtick-quoted prose
    /// describing them — scanning `text_outside_catalog_guard.rs` itself
    /// would find those, not real call sites. Same reasoning
    /// `dead_code_guard.rs` and `i18n.rs`'s guards use to exclude
    /// themselves; this file lives outside `view/` so it is never actually
    /// walked by `view_dir()`, but the exclusion is kept for the same
    /// defensive reason those files keep theirs even where it is currently
    /// redundant.
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("text_outside_catalog_guard.rs")
    }

    /// Counts occurrences of `pattern` (`text("` or `text(format!("`) whose
    /// next character is an alphabetic letter — the leading-letter test R8
    /// and this guard's own doc comment describe. A glyph (`✕`, `✓`, `✗`,
    /// `—`), a digit, or punctuation right after the opening quote is not
    /// counted, on purpose (see the module doc comment's "what this cannot
    /// see" list).
    fn count_letter_led_text_calls(source: &str, pattern: &str) -> usize {
        let mut count = 0;
        let mut rest = source;
        while let Some(pos) = rest.find(pattern) {
            let after = &rest[pos + pattern.len()..];
            if after.chars().next().is_some_and(char::is_alphabetic) {
                count += 1;
            }
            rest = after;
        }
        count
    }

    fn count_in_file(source: &str) -> usize {
        count_letter_led_text_calls(source, "text(\"")
            + count_letter_led_text_calls(source, "text(format!(\"")
    }

    /// The exact, justified set of letter-led `text("…")` /
    /// `text(format!("…"))` literals under `view/` that do not route
    /// through the catalog. Both entries predate RFC-048 and are the RFC's
    /// own named exceptions:
    ///
    /// - `overlays/conflict.rs`, 4 — `command:`/`stdout:`/`stderr:`/`error:`
    ///   prefixes on raw tool output, RFC-038 A1's "export text is English
    ///   by design" category, not first-level UI wording.
    /// - `settings.rs`, **2** — the language switcher, both buttons:
    ///   `text("English")` and `text("日本語")`. Each names a language in
    ///   that language, never translatable by definition. RFC-048's own
    ///   survey counted only the first — `char::is_alphabetic` is
    ///   Unicode-aware and treats `日` as a letter exactly as it treats
    ///   `E`, so a scan that is honest about what "letter-led" means finds
    ///   both. Confirmed by reading `settings.rs:111-118`: the two buttons
    ///   are symmetric, same shape, same justification.
    ///
    /// Every other file must be zero. Adding a new letter-led literal here
    /// means adding it to this list with the same per-item justification
    /// its neighbours carry — never silently, and never a blanket
    /// exception for a whole file.
    const EXPECTED: &[(&str, usize)] = &[("overlays/conflict.rs", 4), ("settings.rs", 2)];

    #[test]
    fn user_facing_text_under_view_routes_through_the_catalog() {
        let dir = view_dir();
        let files = rust_files_under(&dir);
        assert!(
            files.len() > 15,
            "found only {} .rs files under {} -- path resolution is \
             broken (expected 20+), not that nothing needed checking",
            files.len(),
            dir.display()
        );

        let mut actual: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for file in files.iter().filter(|f| is_scan_target(f)) {
            let Ok(source) = std::fs::read_to_string(file) else {
                continue;
            };
            let n = count_in_file(&source);
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
            "letter-led text(\"...\") / text(format!(\"...\")) literals under \
             crates/knotra-app/src/view/ changed. If you added a new one, \
             route it through state.t() and a catalog key instead (RFC-048) \
             -- or, if it is genuinely not translatable (a raw-output prefix, \
             a language naming itself), add it to this file's EXPECTED list \
             with the same per-item justification its neighbours carry. If \
             you removed one, remove its entry from EXPECTED too.\n\
             actual:   {actual:?}\n\
             expected: {expected:?}"
        );
    }
}

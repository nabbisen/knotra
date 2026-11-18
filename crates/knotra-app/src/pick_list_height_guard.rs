//! RFC-073 handoff Amendment A5 §3: every `pick_list(...)` built anywhere
//! in `crates/knotra-app/src/view/` or `crates/knotra-ui/src/widget/` must
//! reach `BUTTON_HEIGHT` — a select is a target like any other, the same
//! rule A3 §2's `text_input_height_guard.rs` already holds fields to, aimed
//! at selects instead.
//!
//! **Unlike `text_input_height_guard`, this guard excludes no qualified
//! path.** `text_input_height_guard` excludes `FocusTarget::text_input(` —
//! a genuinely different function on a different type that merely shares
//! the trailing name. There is no such unrelated same-named function here:
//! every one of the nine real call sites in this codebase, before this
//! amendment, read `select::pick_list(...)` — a *qualified* call, and the
//! one this guard exists to catch. Excluding every qualified path (the
//! text_input guard's own rule) would have excluded all nine and made this
//! guard vacuous; only `styled_pick_list(` (the helper itself, and the one
//! legitimate call shape once this amendment lands) is excluded, by the
//! identifier-boundary check alone.
//!
//! **Seen to fail first, against the real pre-amendment source** (`git
//! show` of this project's own history, not a fabricated fixture): one of
//! the nine call sites (`settings.rs`'s locale select), which named no
//! height anywhere in its builder chain before `styled_pick_list` existed.
//!
//! **Text-scanning, not a real parser**, the same tradeoff every guard in
//! this project accepts.
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped
//! binary.

#[cfg(test)]
mod tests {
    /// Recursively collect every `.rs` file under `dir` — copied from the
    /// other guards rather than shared, matching this project's
    /// established precedent of a small hand-rolled walk per guard.
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

    /// The same two directories `text_input_height_guard.rs` scans.
    fn scan_dirs() -> Vec<std::path::PathBuf> {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        vec![
            manifest.join("src/view"),
            manifest.join("../knotra-ui/src/widget"),
        ]
    }

    /// This file's own doc comment and module-doc prose name the exact
    /// patterns it scans for — scanning `pick_list_height_guard.rs` itself
    /// would find those, not real call sites.
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("pick_list_height_guard.rs")
    }

    fn is_comment_line(trimmed: &str) -> bool {
        trimmed.starts_with("//")
    }

    /// `source` with every comment line replaced by spaces of the same
    /// length — preserves byte offsets. Copied from the other guards.
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

    /// True if `callee` (the trimmed source immediately before an opening
    /// paren) is a call to `pick_list` — bare or qualified
    /// (`select::pick_list(`) — never a longer identifier merely ending in
    /// it: `styled_pick_list(`, the helper whose own definition is scanned
    /// instead. This file's own module doc explains why, unlike
    /// `text_input_height_guard`, no qualified path is excluded beyond
    /// that one identifier-boundary check.
    fn is_pick_list_callee(callee: &str) -> bool {
        let Some(rest) = callee.strip_suffix("pick_list") else {
            return false;
        };
        rest.is_empty() || !rest.ends_with(|c: char| c.is_alphanumeric() || c == '_')
    }

    /// The byte offset of the `)` matching the `(` at `open_paren`, by
    /// forward depth counting. Copied from `text_input_height_guard.rs`.
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

    /// One violation this guard found: `file:line`.
    #[derive(Debug)]
    struct Violation {
        location: String,
    }

    /// Scans `source` for every `pick_list(...)` call this guard covers,
    /// and for each one, walks forward from its own closing paren through
    /// the rest of its builder chain — tracking bracket depth, stopping at
    /// the first `;`/`,` at depth 0 or the first point depth would go
    /// negative — pushing a [`Violation`] into `out` unless `BUTTON_HEIGHT`
    /// appears literally somewhere in that span.
    fn check_source(path: &std::path::Path, clean_source: &str, out: &mut Vec<Violation>) {
        let mut search_from = 0;
        while let Some(rel) = clean_source[search_from..].find("pick_list(") {
            let word_start = search_from + rel;
            let paren_idx = word_start + "pick_list".len();
            search_from = paren_idx + 1;

            let callee = clean_source[..paren_idx].trim_end();
            if !is_pick_list_callee(callee) {
                continue;
            }

            let Some(close_paren) = matching_close_paren(clean_source, paren_idx) else {
                continue;
            };

            let line = clean_source[..paren_idx].matches('\n').count() + 1;
            let location = format!("{}:{line}", path.display());

            let bytes = clean_source.as_bytes();
            let mut depth: i32 = 0;
            let mut i = close_paren + 1;
            let mut has_height = false;
            while i < bytes.len() {
                let c = bytes[i];
                match c {
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    }
                    b';' | b',' if depth == 0 => break,
                    b'B' if clean_source[i..].starts_with("BUTTON_HEIGHT") => {
                        has_height = true;
                        break;
                    }
                    _ => {}
                }
                i += 1;
            }

            if !has_height {
                out.push(Violation { location });
            }
        }
    }

    /// Seen to fail first, against the real pre-amendment source (`git
    /// show HEAD` at the point this guard was written, before
    /// `styled_pick_list` existed) — not a fabricated fixture: one of the
    /// nine real call sites (`settings.rs`'s locale select), which named
    /// no height anywhere in its own chain. The *function definition*
    /// itself (`pub fn pick_list<'a, T, Message>(...)`) is never a match
    /// for this scanner, by construction — `pick_list<` (its own generic
    /// parameters) never spells the literal substring `pick_list(` the
    /// scanner looks for, the same way `check_source` only ever walks
    /// forward from a real call's own closing paren, never a
    /// declaration's.
    #[test]
    fn the_scanner_flags_the_real_pre_amendment_call_site() {
        let old_call_site = r#"
        select::pick_list(
            tokens,
            locale_options,
            Some(state.config.locale),
            false,
            |value| Message::Settings(SettingsMessage::LocaleChanged(value)),
        ),
"#;

        let mut violations = Vec::new();
        check_source(
            std::path::Path::new("fixture.rs"),
            &blank_comment_lines(old_call_site),
            &mut violations,
        );
        assert!(
            !violations.is_empty(),
            "the old call site named no BUTTON_HEIGHT and must have been flagged"
        );
    }

    /// `styled_pick_list(` itself — the fixed shape — must not be flagged.
    #[test]
    fn the_scanner_does_not_flag_styled_pick_list() {
        let fixed = r#"
        select::styled_pick_list(
            tokens,
            locale_options,
            Some(state.config.locale),
            false,
            |value| Message::Settings(SettingsMessage::LocaleChanged(value)),
        ),
"#;
        let mut violations = Vec::new();
        check_source(
            std::path::Path::new("fixture.rs"),
            &blank_comment_lines(fixed),
            &mut violations,
        );
        assert!(violations.is_empty(), "{violations:?}");
    }

    /// Every `pick_list(...)` built in either scanned directory reaches
    /// `BUTTON_HEIGHT` — the expected set is empty by design, the same
    /// shape every other guard in this project uses for a regression that
    /// should never reappear.
    #[test]
    fn every_pick_list_reaches_button_height() {
        let mut actual: Vec<String> = Vec::new();
        let mut total_files = 0;

        for dir in scan_dirs() {
            let files = rust_files_under(&dir);
            total_files += files.len();
            for file in files.iter().filter(|f| is_scan_target(f)) {
                let Ok(source) = std::fs::read_to_string(file) else {
                    continue;
                };
                let clean = blank_comment_lines(&source);
                let mut violations = Vec::new();
                check_source(file, &clean, &mut violations);
                for v in violations {
                    actual.push(v.location);
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
            "pick_list(...) site(s) found with no BUTTON_HEIGHT anywhere in \
             their own builder chain -- RFC-073 handoff Amendment A5 §3 \
             requires every pick_list(...) in view/ or widget/ to reach \
             BUTTON_HEIGHT through styled_pick_list: {actual:?}"
        );
    }
}

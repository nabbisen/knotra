//! RFC-073 handoff Amendment A3 §2: every `text_input(...)` built anywhere
//! in `crates/knotra-app/src/view/` or `crates/knotra-ui/src/widget/` must
//! reach `BUTTON_HEIGHT` — a field you click into is a target like any
//! other (the stage 2 capture's own search field, drawn at iced's unset
//! default, is what prompted this). `text_input` has no `.height(...)`
//! builder, so the rule this guard checks is "`BUTTON_HEIGHT` is named
//! somewhere in the call's own builder chain", by padding — the shape
//! `button_height_guard.rs` already checks for buttons, aimed at fields
//! instead.
//!
//! A call to `styled_text_input(...)` (`knotra-ui/src/widget/field.rs`) is
//! not itself a bare `text_input(` call — `is_text_input_callee` only
//! matches the literal identifier `text_input`, never a longer name ending
//! in it — so every such call site is silently out of this guard's scope:
//! `styled_text_input`'s own definition is what gets scanned, and its own
//! bare `text_input(...)` names `BUTTON_HEIGHT` in its own chain.
//! `FocusTarget::text_input(...)` (a different function on a different
//! type, nothing to do with `iced::widget::text_input`) is excluded the
//! same way a qualified path is always excluded: it ends in `::text_input`.
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

    /// The same two directories `button_style_guard.rs` scans.
    fn scan_dirs() -> Vec<std::path::PathBuf> {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        vec![
            manifest.join("src/view"),
            manifest.join("../knotra-ui/src/widget"),
        ]
    }

    /// This file's own doc comment names the exact patterns it scans for
    /// inside backtick-quoted prose — scanning `text_input_height_guard.rs`
    /// itself would find those, not real call sites. Same self-exclusion
    /// every guard in this project uses.
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("text_input_height_guard.rs")
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
    /// paren) is a call to bare `text_input` — `text_input(` or
    /// `iced::widget::text_input(` — never a longer identifier merely
    /// ending in it (`styled_text_input(`, which must stay out of this
    /// guard's scope: it is the helper whose own definition is scanned
    /// instead) and never a qualified path ending in `::text_input`
    /// (`FocusTarget::text_input(`, an unrelated function on an unrelated
    /// type that happens to share this name).
    fn is_text_input_callee(callee: &str) -> bool {
        let Some(rest) = callee.strip_suffix("text_input") else {
            return false;
        };
        if rest.ends_with("::") {
            return false;
        }
        rest.is_empty() || !rest.ends_with(|c: char| c.is_alphanumeric() || c == '_')
    }

    /// The byte offset of the `)` matching the `(` at `open_paren`, by
    /// forward depth counting. Copied from `button_style_guard.rs`.
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

    /// Scans `source` for every bare `text_input(...)` call, and for each
    /// one, walks forward from its own closing paren through the rest of
    /// its builder chain — tracking bracket depth, stopping at the first
    /// `;`/`,` at depth 0 or the first point depth would go negative —
    /// pushing a [`Violation`] into `out` unless `BUTTON_HEIGHT` appears
    /// literally somewhere in that span.
    fn check_source(path: &std::path::Path, clean_source: &str, out: &mut Vec<Violation>) {
        let mut search_from = 0;
        while let Some(rel) = clean_source[search_from..].find("text_input(") {
            let word_start = search_from + rel;
            let paren_idx = word_start + "text_input".len();
            search_from = paren_idx + 1;

            let callee = clean_source[..paren_idx].trim_end();
            if !is_text_input_callee(callee) {
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

    /// Every `text_input(...)` built in either scanned directory reaches
    /// `BUTTON_HEIGHT` — the expected set is empty by design, the same
    /// shape every other guard in this project uses for a regression that
    /// should never reappear. Seen to fail first: before
    /// `styled_text_input` existed, `dashboard/toolbar.rs`'s two search
    /// fields, `command_palette.rs`'s and `history.rs`'s own `text_input(`
    /// sites named no height at all.
    #[test]
    fn every_text_input_reaches_button_height() {
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
            "text_input(...) site(s) found with no BUTTON_HEIGHT anywhere in \
             their own builder chain -- RFC-073 handoff Amendment A3 §2 \
             requires every bare text_input(...) in view/ or widget/ to reach \
             BUTTON_HEIGHT by padding, directly or through styled_text_input: \
             {actual:?}"
        );
    }
}

//! RFC-073 Amendment A2: catches a `button(...)` built in
//! `crates/knotra-app/src/view/`/`crates/knotra-ui/src/widget/` whose own
//! builder chain never calls `.style(...)` — the gap A1 found: an
//! unstyled button falls back to iced's default, which this app's theme
//! renders filled, indistinguishable from `style::primary` by eye, with
//! nothing naming that choice. `button_label_guard.rs`'s discipline and
//! directory roots, applied to a different question.
//!
//! A call to a knotra helper that builds its own `button(...)` internally
//! (`select_mode_button(...)`, `icon_button_maybe(...)`,
//! `centered_button_label(...)`, …) is not itself a bare `button(` call —
//! `is_button_callee` only matches the literal identifier `button`, never
//! a longer name ending in it — so every such call site is silently out
//! of this guard's scope, exactly as intended: the helper's own
//! definition is what gets scanned and must supply the `.style(...)`.
//!
//! **Text-scanning, not a real parser**, the same tradeoff every guard in
//! this project accepts. The one shape assumption: `.style(...)` must
//! appear **somewhere** in the chain directly following `button(...)`'s
//! own closing paren, at the same bracket depth — inside a `move |...| { … }`
//! closure body passed to a *later* `.style(...)` call is still "this
//! chain", since the closure's own braces nest one level deeper and the
//! scan does not stop there; a `.style(...)` belonging to a *different*,
//! unrelated button (reached only after this chain's own statement ends)
//! is outside it, because the scan stops at the first `;` or
//! depth-negative bracket at the chain's own depth.
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

    /// The same two directories `button_label_guard.rs` scans.
    fn scan_dirs() -> Vec<std::path::PathBuf> {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        vec![
            manifest.join("src/view"),
            manifest.join("../knotra-ui/src/widget"),
        ]
    }

    /// This file's own doc comment names the exact patterns it scans for
    /// inside backtick-quoted prose — scanning `button_style_guard.rs`
    /// itself would find those, not real call sites. Same self-exclusion
    /// every guard in this project uses.
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("button_style_guard.rs")
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
    /// paren) is a call to bare `button` — `button(` or
    /// `iced::widget::button(`, the only two spellings this codebase
    /// uses — not a longer identifier merely ending in "button"
    /// (`select_mode_button(`, `icon_button_maybe(`, …), which must stay
    /// out of this guard's scope: those are the helpers whose own
    /// definitions are scanned instead. Copied from
    /// `button_label_guard.rs::is_button_callee`.
    fn is_button_callee(callee: &str) -> bool {
        let Some(rest) = callee.strip_suffix("button") else {
            return false;
        };
        rest.is_empty() || !rest.ends_with(|c: char| c.is_alphanumeric() || c == '_')
    }

    /// The byte offset, within `source`, of the `)` matching the `(` at
    /// `open_paren` (`source.as_bytes()[open_paren]` must be `b'('`), by
    /// forward depth counting — the mirror of
    /// `button_label_guard.rs::matching_open_paren`, needed here because
    /// this guard walks forward from `button(` to find where its own
    /// argument list ends, rather than backward from a later `.height(`.
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

    /// Scans `source` (one file's content; comment lines already blanked)
    /// for every bare `button(...)` call, and for each one, walks forward
    /// from its own closing paren through the rest of its builder chain —
    /// tracking bracket depth, stopping at the first `;`/`,` at depth 0
    /// (the chain's own statement end) or the first point depth would go
    /// negative (the chain's enclosing scope closes) — pushing a
    /// [`Violation`] into `out` unless `.style(` appears somewhere in that
    /// span at depth 0.
    fn check_source(path: &std::path::Path, clean_source: &str, out: &mut Vec<Violation>) {
        let mut search_from = 0;
        while let Some(rel) = clean_source[search_from..].find("button(") {
            let word_start = search_from + rel;
            let paren_idx = word_start + "button".len();
            // Advance past this occurrence's own `(` before the next
            // search, regardless of outcome below, so a `button(` nested
            // inside another chain's arguments is still found on its own
            // pass.
            search_from = paren_idx + 1;

            let callee = clean_source[..paren_idx].trim_end();
            if !is_button_callee(callee) {
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
            let mut styled = false;
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
                    b'.' if depth == 0 && clean_source[i..].starts_with(".style(") => {
                        styled = true;
                        break;
                    }
                    _ => {}
                }
                i += 1;
            }

            if !styled {
                out.push(Violation { location });
            }
        }
    }

    /// Every `button(...)` built in either scanned directory chooses a
    /// style — the expected set is empty by design, the same shape every
    /// other guard in this project uses for a regression that should
    /// never reappear.
    #[test]
    fn every_button_chooses_a_style() {
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
            "button(...) site(s) found with no `.style(...)` anywhere in their own \
             builder chain -- RFC-073 Amendment A2 requires every bare `button(...)` \
             in view/ or widget/ to choose a style explicitly: {actual:?}"
        );
    }
}

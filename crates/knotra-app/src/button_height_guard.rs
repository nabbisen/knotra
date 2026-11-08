//! RFC-073 D8: every pressable widget built by a `crates/knotra-ui/src/widget/`
//! helper must be at least `SMALL_BUTTON_HEIGHT` tall — a hit area an older
//! user can reliably land on, the whole point of this stage. This guard
//! checks the one thing a visual capture cannot prove by omission: that
//! every bare `button(...)` built **inside knotra-ui's own helper
//! functions** names an explicit height (`BUTTON_HEIGHT`/
//! `SMALL_BUTTON_HEIGHT`, bare or `Length::Fixed(...)`-wrapped — both
//! spellings `button_label_guard.rs` already recognises) somewhere in its
//! own builder chain, rather than falling back to iced's own
//! content-sized default.
//!
//! Scans only `crates/knotra-ui/src/widget/` — the helpers themselves, not
//! every ad-hoc `button(` call site in `crates/knotra-app/src/view/`
//! (Amendment A2's `button_style_guard.rs` already enumerates those by
//! hand per D8's own "bring each to the rule or report why not"; this
//! guard is the narrower "per helper" guarantee D8 separately asks for).
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

    /// Only `crates/knotra-ui/src/widget/` — the helpers, not every
    /// ad-hoc call site in `crates/knotra-app/src/view/`.
    fn scan_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../knotra-ui/src/widget")
    }

    /// This file's own doc comment names the exact patterns it scans for
    /// inside backtick-quoted prose — scanning `button_height_guard.rs`
    /// itself would find those, not real call sites. Same self-exclusion
    /// every guard in this project uses.
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("button_height_guard.rs")
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

    /// True if `callee` is a call to bare `button` — `button(` or
    /// `iced::widget::button(`, never a longer identifier merely ending in
    /// "button". Copied from `button_label_guard.rs`/`button_style_guard.rs`.
    fn is_button_callee(callee: &str) -> bool {
        let Some(rest) = callee.strip_suffix("button") else {
            return false;
        };
        rest.is_empty() || !rest.ends_with(|c: char| c.is_alphanumeric() || c == '_')
    }

    /// The byte offset of the `)` matching the `(` at `open_paren`, by
    /// forward depth counting. Copied from `button_style_guard.rs::matching_close_paren`.
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

    /// Scans `source` for every bare `button(...)` call, and for each one,
    /// walks forward from its own closing paren through the rest of its
    /// builder chain — tracking bracket depth, stopping at the first
    /// `;`/`,` at depth 0 or the first point depth would go negative —
    /// pushing a [`Violation`] into `out` unless `BUTTON_HEIGHT` or
    /// `SMALL_BUTTON_HEIGHT` appears literally somewhere in that span
    /// (either name, bare or `Length::Fixed(...)`-wrapped — this guard
    /// only needs to know *a* height constant was named, not which one or
    /// in what shape; `button_label_guard.rs` separately proves a fixed
    /// height's label is centred).
    fn check_source(path: &std::path::Path, clean_source: &str, out: &mut Vec<Violation>) {
        let mut search_from = 0;
        while let Some(rel) = clean_source[search_from..].find("button(") {
            let word_start = search_from + rel;
            let paren_idx = word_start + "button".len();
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
                    b'S' if clean_source[i..].starts_with("SMALL_BUTTON_HEIGHT") => {
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

    /// Every `button(...)` built in `crates/knotra-ui/src/widget/` names
    /// an explicit minimum height — the expected set is empty by design,
    /// the same shape every other guard in this project uses for a
    /// regression that should never reappear.
    #[test]
    fn every_widget_helper_button_names_a_minimum_height() {
        let dir = scan_dir();
        let files = rust_files_under(&dir);
        assert!(
            files.len() > 5,
            "found only {} .rs files under crates/knotra-ui/src/widget/ -- \
             path resolution is broken (expected 5+), not that nothing \
             needed checking",
            files.len()
        );

        let mut actual: Vec<String> = Vec::new();
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

        assert!(
            actual.is_empty(),
            "button(...) site(s) found in a knotra-ui widget helper with no \
             BUTTON_HEIGHT/SMALL_BUTTON_HEIGHT anywhere in their own builder \
             chain -- RFC-073 D8 requires every widget a knotra-ui helper \
             builds to be at least SMALL_BUTTON_HEIGHT tall: {actual:?}"
        );
    }
}

//! Task 120 §1: no view calls `strftime` on a `jiff::Timestamp`/`Zoned`
//! itself — every on-screen time goes through `view/time.rs::format`, the
//! one place that knows whether to show it in the reader's own zone or
//! fall back to labelled UTC. Scoped to `crates/knotra-app/src/view/`
//! only (not `knotra-ui/src/widget/`, which draws no time of its own) —
//! `view/time.rs` itself is the one file allowed to call `.strftime(`
//! directly, since it is the helper being scanned for.
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

    fn scan_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/view")
    }

    /// This file's own doc comment and module-doc prose name the exact
    /// pattern it scans for — scanning `time_format_guard.rs` itself would
    /// find those, not real call sites. `view/time.rs` is the one file
    /// genuinely allowed to call `.strftime(` directly — it is the helper
    /// every other call site must go through instead.
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("time_format_guard.rs") && !path.ends_with("view/time.rs")
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

    /// Every `.strftime(` call outside `view/time.rs`, as `file:line`.
    fn find_violations(path: &std::path::Path, clean_source: &str, out: &mut Vec<String>) {
        let mut search_from = 0;
        while let Some(rel) = clean_source[search_from..].find(".strftime(") {
            let idx = search_from + rel;
            search_from = idx + 1;
            let line = clean_source[..idx].matches('\n').count() + 1;
            out.push(format!("{}:{line}", path.display()));
        }
    }

    /// No view in `crates/knotra-app/src/view/` calls `.strftime(` on a
    /// timestamp directly — every on-screen time goes through
    /// `view/time.rs::format` instead (Task 120 §1). **Seen failing
    /// first**, against `e5c1ce2` (the baseline this task started from,
    /// `git show e5c1ce2:crates/knotra-app/src/view/<file>` into a scratch
    /// copy, reverted after): six hits, not three — `history.rs:228`
    /// (`view_log_entry`'s own on-screen timestamp) and `:650`
    /// (`export_text`'s copied-log one, also since routed through the
    /// helper with a `None` zone, so this scan needs no exception for it);
    /// `detail_panel.rs:226`/`:319` (the two production sites) *and*
    /// `:432`/`:439` (that file's own pre-existing tests, which called
    /// `.strftime` directly too — rewritten to call `view::time::format`
    /// instead, same as the production sites).
    #[test]
    fn no_view_formats_a_time_itself() {
        let dir = scan_dir();
        let files = rust_files_under(&dir);
        assert!(
            files.len() > 5,
            "found only {} .rs files under {} -- path resolution is broken, \
             not that nothing needed checking",
            files.len(),
            dir.display()
        );

        let mut actual: Vec<String> = Vec::new();
        for file in files.iter().filter(|f| is_scan_target(f)) {
            let Ok(source) = std::fs::read_to_string(file) else {
                continue;
            };
            let clean = blank_comment_lines(&source);
            find_violations(file, &clean, &mut actual);
        }

        assert!(
            actual.is_empty(),
            ".strftime(...) call(s) found outside view/time.rs -- Task 120 §1 requires \
             every on-screen time to go through view::time::format instead: {actual:?}"
        );
    }
}

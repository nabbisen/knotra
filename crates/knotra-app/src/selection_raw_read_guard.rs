//! RFC-074 Amendment A1 (D6): no action may read the raw selection
//! (`state.selection.selected_ids`) directly — only ticks that are also
//! shown (`AppState::selection_summary().selected_ids`) may drive what
//! Check for updates, Get latest safely, Save release point, Change work
//! area, Generate notes or the command palette act on. A raw read bypasses
//! that filter silently, exactly the shape Review 316 §2 found four times
//! over (`app/sync.rs` twice, `app/freezer.rs` once, plus the trimming this
//! amendment also removes).
//!
//! **Two files are allowed to contain the literal `selection.selected_ids`:**
//!
//! - `state.rs` — the selection model itself: `SelectionState`'s own
//!   methods, and `AppState::selection_summary()`'s own computation of
//!   "ticked and shown," both have to read the raw field to define it.
//! - `view/selection_bar.rs` — the one legitimate exception. The bar's
//!   "Selected but not shown: n" line (RFC-074 Amendment A1 §3) exists
//!   *only* to report the raw-vs-shown gap, so it alone needs both counts
//!   to subtract.
//!
//! Scans `app/`, `view/`, `state.rs` and `state/` — where a message handler
//! or a view function could read the selection to decide what to act on or
//! display. `tests.rs` and `keyboard_acceptance.rs` (which construct and
//! assert on the raw selection directly, by design, to set up and verify
//! scenarios) are test-only files outside that scope and are not scanned.
//!
//! Text-scanning, not a real parser — same tradeoff every other guard in
//! this project accepts.
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped binary.

#[cfg(test)]
mod tests {
    /// Recursively collect every `.rs` file under `dir` — copied, not
    /// shared, matching this project's established per-guard precedent.
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

    /// This crate's own `src/`, resolved from `main.rs`'s own manifest
    /// directory.
    fn src_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
    }

    /// Only the directories/files where a message handler or a view
    /// function could read the selection: `app/`, `view/`, `state.rs` and
    /// `state/`. Top-level test-only files (`tests.rs`,
    /// `keyboard_acceptance.rs`) construct and assert on the raw selection
    /// by design and are out of this guard's scope entirely.
    fn scan_targets(src: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut files = rust_files_under(&src.join("app"));
        files.extend(rust_files_under(&src.join("view")));
        files.push(src.join("state.rs"));
        files.extend(rust_files_under(&src.join("state")));
        files
    }

    const ALLOWED: &[&str] = &["state.rs", "view/selection_bar.rs"];

    #[test]
    fn no_action_reads_the_raw_selection_directly() {
        let src = src_dir();
        let targets = scan_targets(&src);
        assert!(
            targets.len() > 15,
            "found only {} files to scan under {} -- path resolution is \
             broken (expected 15+ across app/, view/ and state/), not that \
             nothing needed checking",
            targets.len(),
            src.display()
        );

        let mut offenders = Vec::new();
        for file in &targets {
            let Ok(source) = std::fs::read_to_string(file) else {
                continue;
            };
            if !source.contains("selection.selected_ids") {
                continue;
            }
            let rel = file
                .strip_prefix(&src)
                .unwrap_or(file)
                .to_string_lossy()
                .replace('\\', "/");
            if !ALLOWED.contains(&rel.as_str()) {
                offenders.push(rel);
            }
        }

        assert!(
            offenders.is_empty(),
            "these files read the raw selection (`selection.selected_ids`) \
             directly, instead of going through \
             `AppState::selection_summary().selected_ids`: {offenders:?}. \
             An action must act only on ticks that are also shown (RFC-074 \
             Amendment A1, D6). If this is deliberate and new, name the file \
             in this guard's own ALLOWED list with a reason here."
        );
    }
}

//! Task 121 §3: every bold run in `docs/src/guide/*.md` and `README.md` that
//! names a UI control must spell it exactly as `knotra-ui`'s own English
//! catalog does — the instrument the task itself asks for, so a renamed
//! button or window cannot silently drift from what the guide says again.
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped binary.

#[cfg(test)]
mod tests {
    use knotra_ui::i18n::{Catalog, Locale};

    /// A handful of bold runs in scope that are not UI names at all — a
    /// section heading this page coined itself (`Workflow`, `VCS note`), a
    /// platform label or an *operating system's own* control the docs
    /// mention in passing (`Windows:`, `System Settings → Privacy &
    /// Security`, `Open Anyway` — macOS's own dialog, not knotra's), or a
    /// design-philosophy / feature-bullet name the README coined rather
    /// than quoted from the app (`Accessible by Default`, `Transparent`,
    /// `Local-first`). Keep this short — a real rename should fail here,
    /// not get quietly added to the list.
    const ALLOWED: &[&str] = &[
        "Workflow",
        "VCS note",
        "Windows:",
        "Linux:",
        "macOS:",
        "System Settings → Privacy & Security",
        "Open Anyway",
        "Any system with Rust:",
        "Accessible by Default",
        "Transparent",
        "Local-first",
    ];

    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scan_files() -> Vec<std::path::PathBuf> {
        let root = repo_root();
        let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(root.join("docs/src/guide"))
            .expect("docs/src/guide must exist")
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
            .collect();
        files.push(root.join("README.md"));
        files
    }

    /// Every `**bold**` run on its own line (this project's docs never wrap
    /// one across a line break) — copied from this project's other
    /// text-scanning guards' own precedent of a small hand-rolled scan
    /// rather than a `regex` dependency for a few dozen files.
    fn extract_bold_names(source: &str) -> Vec<(usize, String)> {
        let mut out = Vec::new();
        for (zero_based_line, line) in source.lines().enumerate() {
            let mut rest = line;
            while let Some(start) = rest.find("**") {
                let after_start = &rest[start + 2..];
                let Some(end) = after_start.find("**") else {
                    break;
                };
                let name = &after_start[..end];
                if !name.is_empty() {
                    out.push((zero_based_line + 1, name.to_string()));
                }
                rest = &after_start[end + 2..];
            }
        }
        out
    }

    /// Every bold UI name `docs/src/guide/*.md` and `README.md` quote must
    /// be found, word for word, in the current English catalog — Task 121
    /// §2/§3. **Seen failing first**, in a worktree at `a1f7301` (this
    /// task's own baseline, before any doc correction): 16 hits — `README.md`'s
    /// own "Sync & Pull", "Context Switch", "Switch…", "Freezer";
    /// `sync_center.md`'s "Fetch"/"Pull"; `context_ops.md`'s "Switch…";
    /// `freezer.md`'s "Tag…"; `settings.md`'s "Refresh & Performance" /
    /// "External Tools" / "Logs" / "Save Settings"; `dashboard.md`'s
    /// "Unshared changes" / "Needs your choice"; `history.md`'s "▶ Details" /
    /// "Copy" — all renamed or removed since `docs/src/guide/*.md` was last
    /// touched (2026-10-18).
    #[test]
    fn every_bold_ui_name_matches_the_current_catalog() {
        let en = Catalog::for_locale(Locale::En);

        let mut missing = Vec::new();
        for file in scan_files() {
            let Ok(source) = std::fs::read_to_string(&file) else {
                continue;
            };
            for (line, name) in extract_bold_names(&source) {
                if ALLOWED.contains(&name.as_str()) {
                    continue;
                }
                if !en.contains_value(&name) {
                    missing.push(format!("{}:{line}\t{name}", file.display()));
                }
            }
        }

        assert!(
            missing.is_empty(),
            "these bold names in docs/src/guide/*.md or README.md do not match the current \
             English catalog (crates/knotra-ui/src/i18n.rs) — either the UI changed and the \
             docs did not, or the name belongs in this guard's own ALLOWED list: {missing:#?}"
        );
    }
}

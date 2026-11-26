//! Task 123 §2: no call to `<R as Headless>::new(...)` or `Headless::new(...)`
//! anywhere in `crates/` passes `None` as its own `backend` argument —
//! `None` lets `iced_wgpu`'s own `Headless` impl try first, starting a real
//! Vulkan instance and touching the host's display even in a test, which
//! many test threads doing that at once segfaults (Review 310 F2). Every
//! real call site must pass `Some("tiny-skia")` instead, same as
//! `knotra-ui/src/test_support.rs::Harness::new` and
//! `knotra-app/src/keyboard_acceptance.rs::Rig::new`.
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped binary.

#[cfg(test)]
mod tests {
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

    /// `crates/`, resolved from this crate's own manifest directory — every
    /// crate in the workspace, not just this one, since the defect this
    /// guards against (Review 310 F2) was found in both `knotra-ui` and
    /// `knotra-app`.
    fn crates_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates")
    }

    /// This file's own doc comment and module-doc prose name the exact
    /// patterns it scans for — scanning `headless_backend_guard.rs` itself
    /// would find those, not real call sites.
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("headless_backend_guard.rs")
    }

    fn is_comment_line(trimmed: &str) -> bool {
        trimmed.starts_with("//")
    }

    /// `source` with every comment line replaced by spaces of the same
    /// length — preserves byte offsets, so a found violation's own line
    /// number still points at the real call. Needed here for a reason
    /// `is_scan_target` alone does not cover: a doc comment in some *other*
    /// file can still quote this exact call shape (`pick_list_height_guard.rs`'s
    /// own doc comment once did, found by this guard itself before being
    /// reworded) — copied from `time_format_guard.rs`'s own precedent.
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

    /// Splits `args` on commas at paren/bracket/brace depth 0 — good enough
    /// for the plain, unnested argument lists every real call site here
    /// uses (`Font::with_name("...")`, `Pixels(16.0)`, `None`/`Some("...")`),
    /// the same "text-scanning, not a real parser" tradeoff this project's
    /// other guards already accept.
    fn split_top_level(args: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut depth = 0i32;
        let mut start = 0usize;
        for (i, c) in args.char_indices() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(&args[start..i]);
                    start = i + 1;
                }
                _ => {}
            }
        }
        parts.push(&args[start..]);
        parts
    }

    /// Every `Headless::new(`/`Headless>::new(` call whose own last
    /// (`backend`) argument is a bare `None`, as `file:line`.
    fn find_violations(path: &std::path::Path, source: &str, out: &mut Vec<String>) {
        for pattern in ["Headless::new(", "Headless>::new("] {
            let mut search_from = 0;
            while let Some(rel) = source[search_from..].find(pattern) {
                let open_paren = search_from + rel + pattern.len() - 1;
                search_from = open_paren + 1;

                let bytes = source.as_bytes();
                let mut depth = 1i32;
                let mut i = open_paren + 1;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'(' => depth += 1,
                        b')' => depth -= 1,
                        _ => {}
                    }
                    i += 1;
                }
                if depth != 0 {
                    continue; // unbalanced — not a real call this scan can parse
                }
                let args = &source[open_paren + 1..i - 1];
                // A trailing comma after the last real argument (every real
                // call site here has one) leaves an empty, whitespace-only
                // segment after `split_top_level`'s own last comma — trim
                // and drop empty segments before taking the last one, or
                // that trailing segment, not `None`, is what gets compared.
                if split_top_level(args)
                    .into_iter()
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .next_back()
                    .is_some_and(|last| last == "None")
                {
                    let line = source[..open_paren].matches('\n').count() + 1;
                    out.push(format!("{}:{line}", path.display()));
                }
            }
        }
    }

    /// No `Headless::new`/`Headless>::new` call in `crates/` passes `None`
    /// for its own `backend` argument — Task 123 §2. **Seen failing
    /// first**: run in a worktree at `d74933e` (this task's own baseline,
    /// before either real call site was fixed) — failed with exactly
    /// `knotra-ui/src/test_support.rs:83` and
    /// `knotra-app/src/keyboard_acceptance.rs:63`, the same two sites
    /// Review 310 F2 named.
    #[test]
    fn no_headless_renderer_defaults_to_wgpu() {
        let dir = crates_dir();
        let files = rust_files_under(&dir);
        assert!(
            files.len() > 20,
            "found only {} .rs files under {} -- path resolution is broken, not that nothing \
             needed checking",
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
            "Headless::new(...)/Headless>::new(...) call(s) passing None for backend -- Task \
             123 §2 requires Some(\"tiny-skia\") instead, so headless tests never start wgpu: \
             {actual:?}"
        );
    }
}

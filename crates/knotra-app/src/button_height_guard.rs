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
//! **RFC-075 handoff stage 1 §2: extended to `control(...)`, the same
//! check for `snora::design::control`-based widgets (`buttons.rs`'s
//! `_control` family, `chip::filter_control`) — `Control`'s own builder is
//! `button`'s (`Control::height` takes the same `impl Into<Length>`), so
//! the identical scan, parametrised over which callee name it looks for,
//! covers both without a second copy of the walking logic.** Seen failing
//! first: a scratch call `control(tokens, text("x")).on_press(Msg::X)`
//! (no height) inserted into `buttons.rs`, reported by name and line, then
//! removed.
//!
//! **Amendment A1 §4 (Review 299 F4): also scans `crates/knotra-app/src/
//! view/`, for `control(` only** — the ad-hoc `control(...)` call sites
//! RFC-075 stage 2 put directly in view code (`shell.rs`'s `nav_control`,
//! `toolbar.rs`'s `arrange_button`, `section.rs`'s tier toggle, and
//! others: the primitive this stage's own page actually builds on). The
//! dev team found this gap themselves — by removing `nav_control`'s own
//! height and watching every existing gate stay green — and reported it
//! rather than silently leaving it; this closes it with the same
//! `check_source` this file already has.
//!
//! **Not `button(` in `view/`**: every pre-existing `button(...)` found
//! there belongs to a window dialog still on knotra's own pre-RFC-075
//! focus model (`overlays/`, `command_palette.rs`, `add_project_modal.rs`,
//! and the rest — stage 5's own territory, out of scope here), built by a
//! different, already-reviewed design `button_style_guard.rs` already
//! covers for choosing *a* style; D8's own height floor was never asked
//! of them, and this amendment does not retroactively ask it now. The
//! test's own doc comment says more.
//!
//! Previously: only `crates/knotra-ui/src/widget/` — the helpers
//! themselves, not every ad-hoc call site in `crates/knotra-app/src/view/`
//! (Amendment A2's `button_style_guard.rs` already enumerates those by
//! hand per D8's own "bring each to the rule or report why not"); this
//! guard was the narrower "per helper" guarantee D8 separately asks for.
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

    /// True if `callee` is a call to `name` itself — bare or path-qualified
    /// (`button(`/`iced::widget::button(`, `control(`/`snora::design::
    /// control(`) — never a longer identifier merely ending in `name`
    /// (`my_control` must not match `control`). Generalised from
    /// `button_label_guard.rs`'s/`button_style_guard.rs`'s own
    /// `is_button_callee` to cover `control(...)` too (RFC-075 handoff
    /// stage 1 §2), rather than a second, near-identical function.
    fn is_callee(callee: &str, name: &str) -> bool {
        // `FocusTarget::control(...)` (`view/focus.rs` — knotra's own,
        // pre-RFC-075 model the windows still use, stage 5's own
        // territory) is a path-qualified call ending in "control" too,
        // the same shape `snora::design::control(...)` has — found only
        // once `view/` itself was scanned (Amendment A1 §4), since
        // `FocusTarget` has no reason to appear under `knotra-ui/src/
        // widget/`. Excluded by name: it builds an enum variant, never a
        // pressable widget, so a missing height here would mean nothing.
        if callee.ends_with("FocusTarget::control") {
            return false;
        }
        let Some(rest) = callee.strip_suffix(name) else {
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

    /// Scans `source` for every bare `{name}(...)` call, and for each one,
    /// walks forward from its own closing paren through the rest of its
    /// builder chain — tracking bracket depth, stopping at the first
    /// `;`/`,` at depth 0 or the first point depth would go negative —
    /// pushing a [`Violation`] into `out` unless `BUTTON_HEIGHT` or
    /// `SMALL_BUTTON_HEIGHT` appears literally somewhere in that span
    /// (either name, bare or `Length::Fixed(...)`-wrapped — this guard
    /// only needs to know *a* height constant was named, not which one or
    /// in what shape; `button_label_guard.rs` separately proves a fixed
    /// height's label is centred). `name` is `"button"` or `"control"`
    /// (RFC-075 handoff stage 1 §2).
    fn check_source(
        path: &std::path::Path,
        clean_source: &str,
        name: &str,
        out: &mut Vec<Violation>,
    ) {
        let needle = format!("{name}(");
        let mut search_from = 0;
        while let Some(rel) = clean_source[search_from..].find(needle.as_str()) {
            let word_start = search_from + rel;
            let paren_idx = word_start + name.len();
            search_from = paren_idx + 1;

            let callee = clean_source[..paren_idx].trim_end();
            if !is_callee(callee, name) {
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

    /// Every `button(...)`/`control(...)` built in
    /// `crates/knotra-ui/src/widget/`, and every `control(...)` built
    /// directly in `crates/knotra-app/src/view/`, names an explicit
    /// minimum height — the expected set is empty by design, the same
    /// shape every other guard in this project uses for a regression that
    /// should never reappear.
    ///
    /// **`view/` is scanned for `control(` only, not `button(`**: a
    /// window dialog still on knotra's own pre-RFC-075 focus model
    /// (`overlays/`, `command_palette.rs`, `add_project_modal.rs`, and
    /// the rest — stage 5's own territory) builds plain `iced::widget::
    /// button(...)` throughout, with no height named, by a *different,
    /// already-reviewed* design this amendment does not revisit; a real
    /// `rg -F 'button(' crates/knotra-app/src/view/ -l` run shows
    /// `view/dashboard/row.rs`'s own test-only frozen fixture is the one
    /// `button(` this stage's *own* files use, and it already carries
    /// `BUTTON_HEIGHT`. RFC-075 stage 2's own real gap — `shell.rs`'s
    /// `nav_control`, confirmed missing one, found by temporarily
    /// removing it and watching every then-existing gate stay green — was
    /// a `control(...)` call, the primitive this stage's own page
    /// actually builds on; checking for that callee alone is what A1 §4
    /// itself asks for.
    #[test]
    fn every_widget_helper_button_or_control_names_a_minimum_height() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let widget_dir = manifest.join("../knotra-ui/src/widget");
        let view_dir = manifest.join("src/view");

        let mut actual: Vec<String> = Vec::new();

        let widget_files = rust_files_under(&widget_dir);
        assert!(
            widget_files.len() > 5,
            "found only {} .rs files under {} -- path resolution is broken \
             (expected 5+), not that nothing needed checking",
            widget_files.len(),
            widget_dir.display()
        );
        for file in widget_files.iter().filter(|f| is_scan_target(f)) {
            let Ok(source) = std::fs::read_to_string(file) else {
                continue;
            };
            let clean = blank_comment_lines(&source);
            let mut violations = Vec::new();
            check_source(file, &clean, "button", &mut violations);
            check_source(file, &clean, "control", &mut violations);
            actual.extend(violations.into_iter().map(|v| v.location));
        }

        let view_files = rust_files_under(&view_dir);
        assert!(
            view_files.len() > 5,
            "found only {} .rs files under {} -- path resolution is broken \
             (expected 5+), not that nothing needed checking",
            view_files.len(),
            view_dir.display()
        );
        for file in view_files.iter().filter(|f| is_scan_target(f)) {
            let Ok(source) = std::fs::read_to_string(file) else {
                continue;
            };
            let clean = blank_comment_lines(&source);
            let mut violations = Vec::new();
            check_source(file, &clean, "control", &mut violations);
            actual.extend(violations.into_iter().map(|v| v.location));
        }

        assert!(
            actual.is_empty(),
            "button(...)/control(...) site(s) found with no BUTTON_HEIGHT/SMALL_BUTTON_HEIGHT \
             anywhere in their own builder chain -- RFC-073 D8 requires every widget built in \
             crates/knotra-ui/src/widget/ (button or control) or crates/knotra-app/src/view/ \
             (control only) to be at least SMALL_BUTTON_HEIGHT tall: {actual:?}"
        );
    }
}

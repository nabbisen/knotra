//! RFC-062 D3: catches a fixed-height button whose label was not built
//! through [`knotra_ui::widget::centered_button_label`] (D1) — the shape
//! that put every one of the 21 real sites' labels flush against the
//! button's top edge in the first place. iced 0.14's `button` layout
//! (`layout::padded`) places its content at the padding offset, top-left,
//! and never centres it; that only shows once a button's fixed height
//! (`BUTTON_HEIGHT`/`SMALL_BUTTON_HEIGHT`) exceeds its label's own line
//! height.
//!
//! Scans the same two roots `text_size_guard.rs` does —
//! `crates/knotra-app/src/view/` and `crates/knotra-ui/src/widget/` —
//! since both directories build buttons.
//!
//! **Text-scanning, not a real parser** — same tradeoff every guard in this
//! project accepts. One shape assumption worth stating plainly: a fixed
//! `.height(BUTTON_HEIGHT)`/`.height(SMALL_BUTTON_HEIGHT)` call (including
//! `layout::`-qualified spellings) must be the call **directly** chained
//! onto `button(...)`'s own closing paren — `button(content)
//! .height(BUTTON_HEIGHT)`, whitespace and newlines aside — not onto some
//! other call later in the same chain. Every real site in this codebase is
//! written that way; RFC-062's own migration confirmed it site by site
//! rather than assuming it. A `.height(...)` call found on a fixed height
//! that is **not** chained directly off `button(...)` fails loudly, naming
//! the file and line, rather than being silently skipped — that shape
//! would mean either a non-button use of these constants appeared (outside
//! what they exist for) or this guard's own adjacency assumption needs
//! revisiting, and either is worth a human looking rather than a quiet
//! pass.
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped
//! binary.

#[cfg(test)]
mod tests {
    /// Recursively collect every `.rs` file under `dir` — copied from
    /// `text_size_guard.rs`/`suppressions_guard.rs` rather than shared,
    /// matching this project's established precedent of a small
    /// hand-rolled walk per guard.
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

    /// The two directories this guard scans, resolved from this crate's
    /// own manifest directory the same way `text_size_guard.rs`'s
    /// `scan_dirs()` reaches the sibling crate.
    fn scan_dirs() -> Vec<std::path::PathBuf> {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        vec![
            manifest.join("src/view"),
            manifest.join("../knotra-ui/src/widget"),
        ]
    }

    /// This file's own doc comment names the exact pattern it scans for
    /// (`.height(BUTTON_HEIGHT)`, `button(content).height(...)`) inside
    /// backtick-quoted prose — scanning `button_label_guard.rs` itself
    /// would find those, not real call sites. Same self-exclusion every
    /// guard in this project uses.
    fn is_scan_target(path: &std::path::Path) -> bool {
        !path.ends_with("button_label_guard.rs")
    }

    /// True if `trimmed` (a line with leading whitespace already stripped)
    /// is a comment line — `//`, `///`, or `//!`. Mirrors
    /// `text_size_guard.rs::is_comment_line` exactly; a real `.height(` or
    /// `button(` call never opens a line starting with `//`.
    fn is_comment_line(trimmed: &str) -> bool {
        trimmed.starts_with("//")
    }

    /// `source` with every comment line replaced by spaces of the same
    /// length — preserves every other byte's offset so the scan below can
    /// walk the result as one string without a comment's prose (this
    /// file's own doc comment on every *other* file, or a sibling comment
    /// describing the pattern) being mistaken for a call site. Copied from
    /// `text_size_guard.rs::blank_comment_lines`.
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

    /// The byte offset of the `(` matching the `)` at `close_paren`
    /// (`source.as_bytes()[close_paren]` must be `b')'`), by backward
    /// depth counting. A `.height(...)` call is found first and its
    /// owning `button(...)` lies *before* it in the source, which is why
    /// this guard needs the backward form rather than
    /// `text_size_guard.rs`'s forward `matching_close_paren`.
    fn matching_open_paren(source: &str, close_paren: usize) -> Option<usize> {
        let bytes = source.as_bytes();
        let mut depth = 1i32;
        let mut i = close_paren;
        while i > 0 {
            i -= 1;
            match bytes[i] {
                b')' => depth += 1,
                b'(' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// The byte offset, within `source`, of the start of each
    /// `.height(BUTTON_HEIGHT)` / `.height(SMALL_BUTTON_HEIGHT)` call,
    /// including `layout::`-qualified spellings — the position of the
    /// leading `.`.
    fn find_fixed_height_calls(source: &str) -> Vec<usize> {
        let mut starts = Vec::new();
        let mut search_from = 0;
        while let Some(rel) = source[search_from..].find(".height(") {
            let dot = search_from + rel;
            let after_paren = dot + ".height(".len();
            let rest = source[after_paren..].trim_start();
            let rest = rest.strip_prefix("layout::").unwrap_or(rest);
            if rest.starts_with("BUTTON_HEIGHT") || rest.starts_with("SMALL_BUTTON_HEIGHT") {
                let const_name = if rest.starts_with("SMALL_BUTTON_HEIGHT") {
                    "SMALL_BUTTON_HEIGHT"
                } else {
                    "BUTTON_HEIGHT"
                };
                let after_const = rest[const_name.len()..].trim_start();
                if after_const.starts_with(')') {
                    starts.push(dot);
                }
            }
            search_from = after_paren;
        }
        starts
    }

    /// True if `s` is a bare numeric literal — digits, at most one `.`, an
    /// optional trailing `f32`/`f64` suffix (Rust accepts both on a float
    /// literal; neither appears in this codebase's own button heights, but
    /// excluding them costs nothing). Empty is not a literal.
    fn is_numeric_literal(s: &str) -> bool {
        let s = s
            .strip_suffix("f32")
            .or_else(|| s.strip_suffix("f64"))
            .unwrap_or(s);
        !s.is_empty()
            && s.chars().filter(|c| *c == '.').count() <= 1
            && s.chars().all(|c| c.is_ascii_digit() || c == '.')
    }

    /// Task 098 Amendment A1: the byte offset, within `source`, of the
    /// start of each `.height(<numeric literal>)` call — a fixed height
    /// that is **not** one of the two named constants
    /// [`find_fixed_height_calls`] already looks for. Mirrors that
    /// function's own scan shape; the two stay separate rather than one
    /// combined pass, since a literal height is wrong regardless of
    /// whether its label is centred (the violation this function reports
    /// is "not one of the two constants," independent of
    /// [`check_source`]'s own "not centred" check below).
    fn find_literal_height_calls(source: &str) -> Vec<usize> {
        let mut starts = Vec::new();
        let mut search_from = 0;
        while let Some(rel) = source[search_from..].find(".height(") {
            let dot = search_from + rel;
            let after_paren = dot + ".height(".len();
            let rest = &source[after_paren..];
            if let Some(close_rel) = rest.find(')') {
                let arg = rest[..close_rel].trim();
                if is_numeric_literal(arg) {
                    starts.push(dot);
                }
            }
            search_from = after_paren;
        }
        starts
    }

    /// One violation this guard found: `file:line`, plus why.
    #[derive(Debug)]
    struct Violation {
        location: String,
        reason: &'static str,
    }

    /// True if `callee` (the trimmed source immediately before an opening
    /// paren) is a call to `button` — bare (`button(`) or path-qualified
    /// (`iced::widget::button(`), the only two spellings this codebase
    /// uses. Checking the *suffix* rather than the whole string: `callee`
    /// is everything in the file up to that point, so an equality check
    /// against the literal `"button"` would never match anything.
    /// Guards against a longer identifier that merely ends in "button"
    /// (none exist here, but the check costs nothing) by requiring the
    /// character immediately before the suffix, if any, not be an
    /// identifier character.
    fn is_button_callee(callee: &str) -> bool {
        let Some(rest) = callee.strip_suffix("button") else {
            return false;
        };
        rest.is_empty() || !rest.ends_with(|c: char| c.is_alphanumeric() || c == '_')
    }

    /// Scans `source` (one file's content; comment lines already blanked)
    /// for every fixed-height `.height(...)` call, and for each one,
    /// pushes a [`Violation`] into `out` unless it is chained directly off
    /// a `button(...)`/`...::button(...)` call whose own argument span
    /// contains `centered_button_label(`.
    fn check_source(path: &std::path::Path, clean_source: &str, out: &mut Vec<Violation>) {
        for dot in find_fixed_height_calls(clean_source) {
            let line = clean_source[..dot].matches('\n').count() + 1;
            let location = format!("{}:{line}", path.display());

            // The byte immediately before `.height(` (skipping whitespace)
            // must be the `)` this call chains onto.
            let before = clean_source[..dot].trim_end();
            let Some(close_paren) = before.strip_suffix(')').map(|_| before.len() - 1) else {
                out.push(Violation {
                    location,
                    reason: "`.height(...)` on a fixed height is not chained directly \
                             off a preceding call's closing paren",
                });
                continue;
            };

            let Some(open_paren) = matching_open_paren(clean_source, close_paren) else {
                out.push(Violation {
                    location,
                    reason: "could not find the matching opening paren for the call \
                             `.height(...)` is chained onto",
                });
                continue;
            };

            let callee = clean_source[..open_paren].trim_end();
            if !is_button_callee(callee) {
                out.push(Violation {
                    location,
                    reason: "`.height(...)` on a fixed height is chained onto a call \
                             that is not `button(...)` — this guard only knows how to \
                             check button labels",
                });
                continue;
            }

            let args = &clean_source[open_paren + 1..close_paren];
            if !args.contains("centered_button_label(") {
                out.push(Violation {
                    location,
                    reason: "a fixed-height `button(...)`'s content does not go through \
                             `centered_button_label` — its label will sit at the top \
                             of the button, not centred",
                });
            }
        }
    }

    /// Task 098 Amendment A1: scans `source` for every
    /// `.height(<numeric literal>)` call chained directly off a
    /// `button(...)`/`...::button(...)` call, and pushes a [`Violation`]
    /// for each — a fixed height must be `BUTTON_HEIGHT` or
    /// `SMALL_BUTTON_HEIGHT`, never a literal, so this one fires
    /// regardless of whether the label happens to be centred. Shares
    /// [`matching_open_paren`]/[`is_button_callee`] with [`check_source`]
    /// above, but is its own pass over [`find_literal_height_calls`]
    /// rather than folded into that function's loop: the two input sets
    /// are disjoint by construction (a `.height(...)` argument can match
    /// at most one of "is `BUTTON_HEIGHT`/`SMALL_BUTTON_HEIGHT`" or "is a
    /// bare numeric literal"), so there is no double-reporting to
    /// reconcile between them.
    fn check_literal_heights(path: &std::path::Path, clean_source: &str, out: &mut Vec<Violation>) {
        for dot in find_literal_height_calls(clean_source) {
            let line = clean_source[..dot].matches('\n').count() + 1;
            let location = format!("{}:{line}", path.display());

            let before = clean_source[..dot].trim_end();
            let Some(close_paren) = before.strip_suffix(')').map(|_| before.len() - 1) else {
                continue;
            };
            let Some(open_paren) = matching_open_paren(clean_source, close_paren) else {
                continue;
            };
            let callee = clean_source[..open_paren].trim_end();
            if !is_button_callee(callee) {
                continue;
            }

            out.push(Violation {
                location,
                reason: "a fixed-height `button(...)` uses a numeric literal instead of \
                         `BUTTON_HEIGHT`/`SMALL_BUTTON_HEIGHT` — a literal escapes this \
                         guard's other check too, since nothing requires it to match \
                         either constant",
            });
        }
    }

    /// R3: every fixed-height button label in either scanned directory is
    /// built through `centered_button_label`. The expected set is empty by
    /// design, the same shape `text_size_guard.rs` and
    /// `suppressions_guard.rs` use for a regression that should never
    /// reappear: a new entry means a fixed-height button stopped using the
    /// helper (or a non-button use of the height constants appeared), not
    /// that one earned a place on an exceptions list.
    #[test]
    fn every_fixed_height_button_label_is_centered() {
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
                    actual.push(format!("{}: {}", v.location, v.reason));
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
            "fixed-height button(s) found whose label is not centred via \
             `centered_button_label` -- RFC-062 D1/R3 requires every \
             `button(...).height(BUTTON_HEIGHT)` / `.height(SMALL_BUTTON_HEIGHT)` \
             site to build its label through that helper:\n{actual:#?}"
        );
    }

    /// Task 098 Amendment A1: every fixed-height `button(...)` uses
    /// `BUTTON_HEIGHT` or `SMALL_BUTTON_HEIGHT`, never a numeric literal —
    /// a literal both escapes `every_fixed_height_button_label_is_centered`
    /// above (it never matches either constant name, so that scan never
    /// sees it) and is free to drift from both canonical sizes. Found
    /// `conflict.rs`'s "Mark done" this way (Review 236 §3, Task 097's
    /// evidence); the same scan over both directories found three more
    /// pre-RFC-062 sites with the identical shape, migrated in the same
    /// commit as this guard.
    #[test]
    fn every_fixed_height_button_uses_one_of_the_two_height_constants() {
        let mut actual: Vec<String> = Vec::new();

        for dir in scan_dirs() {
            for file in rust_files_under(&dir)
                .into_iter()
                .filter(|f| is_scan_target(f))
            {
                let Ok(source) = std::fs::read_to_string(&file) else {
                    continue;
                };
                let clean = blank_comment_lines(&source);
                let mut violations = Vec::new();
                check_literal_heights(&file, &clean, &mut violations);
                for v in violations {
                    actual.push(format!("{}: {}", v.location, v.reason));
                }
            }
        }

        assert!(
            actual.is_empty(),
            "fixed-height button(s) found using a numeric literal instead of \
             `BUTTON_HEIGHT`/`SMALL_BUTTON_HEIGHT`:\n{actual:#?}"
        );
    }
}

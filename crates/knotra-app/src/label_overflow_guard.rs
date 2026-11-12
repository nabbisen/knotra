//! Task 108 Amendment A1 §2, made strict by Task 110 §2 — every visible
//! button/chip label and status count fits inside its own control, with no
//! clamp, across language, zoom, window width and selection mode.
//!
//! **Enumeration, and why this checks a fixed list of catalog keys.**
//! `iced_test`'s public `Simulator` API only exposes `Selector::find()`
//! (stop at the first match) through `Simulator::find`/`click` — never
//! `Selector::find_all()` (`iced_selector`'s own `Finder<All<S>>`), which
//! is a `widget::Operation`, not a `Selector`, and so cannot be passed to
//! `Simulator::find`'s `S: Selector` bound. There is no other public
//! method on `Simulator` that runs an arbitrary `Operation`. A real
//! enumerate-everything probe was written and run to confirm this
//! directly rather than inferred from the type signatures alone; it is
//! not kept here. So this guard checks a fixed list of catalog keys the
//! view actually calls `state.t()` with for a button, chip or count
//! label, per the task's own fallback.
//!
//! **"Fits inside" means not clamped at all — exactly, not approximately.**
//! Task 108 A1's first version of this guard allowed a label to render as
//! little as 90% of its natural (unclamped) width, reasoning that the
//! remaining 10% was ordinary sub-pixel text-shaping variance. Task 110
//! §2 found that reasoning wrong: the Simulator lays out the same text in
//! the same font at a logical size on every call, so nothing about
//! shaping varies between two renders of the same label — any narrowing
//! at all is the clamp itself, just a smaller one. The 90% tolerance
//! passed a real, user-visible defect (Review 280 §2.1): at English,
//! zoom 150%, an 800px window (logical ~533, Compact), "Updates
//! available" rendered at 128.3px against its natural 133.3px (96.2%) —
//! comfortably inside the old 90% margin — because the chip had already
//! lost its own count digit to the clamp, and the "⋯" control for the
//! other two chips was pushed off the visible window entirely. Fixed in
//! the view (`toolbar.rs`'s chip row now wraps, matching the standard
//! composition's own five-chip row); this guard now requires a label's
//! rendered width to be within 0.5px of its natural width and its
//! height to equal its natural height exactly — a margin for float
//! rounding alone, not for anything text shaping could produce.
//!
//! **Beyond the label itself:** each visible chip's own status count
//! (its own `Candidate::Text`, not part of the label string), the
//! Compact composition's "⋯" overflow trigger, and the five
//! selection-mode action-bar buttons (`selection_bar.rs`, shown only
//! while `state.selection_mode`) are all checked the same way, and all
//! must lie inside the viewport — `x + width <= logical width` — since a
//! control can be unclamped and still pushed off-screen entirely (the
//! "⋯" control's own failure mode above), which the clamp check alone
//! does not catch.
//!
//! **Scope.** The dashboard's toolbar chips (`StatusFilter`'s five
//! labels, each with its own status count), the Compact "⋯" trigger,
//! "Arrange" (standard composition only — compact's own trigger is the
//! bare glyph "▾", not a translated label), "Select"
//! (`plain.selection.enter`), one project row's own action button, and
//! the five selection-mode action-bar buttons. A `NeedsHelp` row's
//! action is either "Resolve" (a merge conflict) or "Show details"
//! (every other `NeedsHelp` cause) — both built by the same
//! `row_action_button` (`row.rs`), with only the catalog key and style
//! function differing, so checking the fixture's conflict case exercises
//! the same construction code the other cause would. "Clear filters" and
//! the per-project name/selection-checkbox buttons (whose own label is
//! the project's name, not a catalog string) are out of this guard's
//! scope.

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;
    use crate::state::AppState;
    use crate::view::dashboard::WidthMode;
    use iced_test::core::Size;
    use knotra_vcs::{
        ConflictStatus, Project, ProjectStatus, RemoteStatus, RepositoryIdentity, VcsKind,
        WorkingTreeStatus, Workspace, WorkspaceStatus,
    };

    /// One project per `StatusFilter` category, each category given a
    /// **different count** (1 through 5) — Task 110 §2 also checks each
    /// chip's own status-count digit, and a fixture that gave every
    /// category the same count (as Task 108 A1's own fixture did, one
    /// project each) would make `find("1")` ambiguous: all five chips
    /// would show "1" at once, and `find` returns only the first match in
    /// traversal order, not necessarily the chip under test. Distinct
    /// counts make `find("1")`/`find("2")`/…/`find("5")` each name
    /// exactly one chip's own count unambiguously.
    fn install_one_project_per_status(state: &mut AppState) {
        state.config.dashboard_all_set_collapsed = false;

        let base_status = |project: &Project| ProjectStatus {
            project_id: project.id.clone(),
            identity: RepositoryIdentity {
                path: project.path.clone(),
                vcs_kind: VcsKind::Git,
            },
            context: None,
            remote: RemoteStatus::default(),
            working_tree: WorkingTreeStatus::default(),
            conflict: ConflictStatus::default(),
            refreshed_at: chrono::Utc::now(),
            read_error: None,
        };

        let mut all_projects = Vec::new();
        let mut all_statuses = Vec::new();

        // Needs help: 1 project, conflicted.
        for n in 1..=1 {
            let project = Project::new(format!("Needs Help Co {n}"), format!("/tmp/needs-{n}"));
            let mut status = base_status(&project);
            status.conflict = ConflictStatus {
                has_conflict: true,
                conflict_count: Some(2),
                detection_unavailable: false,
            };
            all_projects.push(project);
            all_statuses.push(status);
        }
        // Unsaved work: 2 projects, dirty.
        for n in 1..=2 {
            let project = Project::new(format!("Unsaved Work Co {n}"), format!("/tmp/dirty-{n}"));
            let mut status = base_status(&project);
            status.working_tree = WorkingTreeStatus {
                uncommitted_count: 3,
                untracked_count: 0,
            };
            all_projects.push(project);
            all_statuses.push(status);
        }
        // Updates available (behind): 3 projects.
        for n in 1..=3 {
            let project = Project::new(format!("Behind Co {n}"), format!("/tmp/behind-{n}"));
            let mut status = base_status(&project);
            status.remote = RemoteStatus {
                ahead: 0,
                behind: 2,
                upstream: Some("origin/main".to_owned()),
            };
            all_projects.push(project);
            all_statuses.push(status);
        }
        // Local commits (ahead): 4 projects.
        for n in 1..=4 {
            let project = Project::new(format!("Ahead Co {n}"), format!("/tmp/ahead-{n}"));
            let mut status = base_status(&project);
            status.remote = RemoteStatus {
                ahead: 2,
                behind: 0,
                upstream: Some("origin/main".to_owned()),
            };
            all_projects.push(project);
            all_statuses.push(status);
        }
        // All set: 5 projects.
        for n in 1..=5 {
            let project = Project::new(format!("All Set Co {n}"), format!("/tmp/allset-{n}"));
            let status = base_status(&project);
            all_projects.push(project);
            all_statuses.push(status);
        }

        let mut workspace = Workspace::new("Demo");
        workspace.projects = all_projects;
        state.all_workspaces = vec![workspace];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();
        state.workspace_status = Some(WorkspaceStatus {
            projects: all_statuses,
            last_refresh: None,
        });
    }

    /// A generously wide logical size: nothing in the dashboard's own
    /// widths (the search field's 220px, the longest Japanese label) asks
    /// for anything near this, so a label rendered here is at its
    /// natural, unclamped size.
    const REFERENCE_SIZE: Size = Size {
        width: 4000.0,
        height: 2000.0,
    };

    /// Builds a reference [`Simulator`] from the same state (so the same
    /// locale, selection mode and `width_mode`-driven composition apply)
    /// at [`REFERENCE_SIZE`], to read a label's natural size from.
    fn reference_simulator(state: &AppState) -> iced_test::Simulator<'_, crate::message::Message> {
        crate::headless::simulator(crate::view::app_view(state), REFERENCE_SIZE)
    }

    /// Checks that `label` renders at exactly its own natural size (width
    /// within 0.5px, height exactly equal) in `ui`, and that it lies
    /// entirely inside `viewport_width` — a control can be unclamped and
    /// still pushed off-screen, which the size check alone does not catch
    /// (Review 280 §2.1's own "⋯" finding). `reference` is a
    /// [`Simulator`] built from the same state at [`REFERENCE_SIZE`].
    /// Returns the failure message instead of panicking, so the caller
    /// can run the full matrix and report every cell that fails.
    fn check_label(
        ui: &mut iced_test::Simulator<'_, crate::message::Message>,
        reference: &mut iced_test::Simulator<'_, crate::message::Message>,
        label: &str,
        viewport_width: f32,
        context: &str,
    ) -> Result<(), String> {
        let actual = ui
            .find(label)
            .map_err(|e| format!("{context}: label {label:?} not found: {e}"))?
            .bounds();
        let natural = reference
            .find(label)
            .map_err(|e| {
                format!("{context}: label {label:?} not found at the reference size: {e}")
            })?
            .bounds();

        let mut problems = Vec::new();
        if (actual.width - natural.width).abs() > 0.5 {
            problems.push(format!(
                "width {:.2} != natural {:.2} (clamped)",
                actual.width, natural.width
            ));
        }
        if actual.height != natural.height {
            problems.push(format!(
                "height {:.2} != natural {:.2}",
                actual.height, natural.height
            ));
        }
        if actual.x + actual.width > viewport_width + 0.5 {
            problems.push(format!(
                "right edge {:.2} exceeds the viewport's {:.2}",
                actual.x + actual.width,
                viewport_width
            ));
        }

        if problems.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{context}: label {label:?} {actual:?} (natural {natural:?}): {}",
                problems.join("; ")
            ))
        }
    }

    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "the pinned faces (Noto Sans, Noto Sans CJK JP) are installed only on the \
                  Linux CI runner (Task 108 A1 §4) — the widths this guard measures would be \
                  some other font's on macOS/Windows"
    )]
    fn every_label_fits_its_control_across_the_matrix() {
        let mut checked = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for &locale in &[knotra_ui::i18n::Locale::En, knotra_ui::i18n::Locale::Ja] {
            for &zoom_percent in &[100.0_f32, 150.0_f32] {
                for &width in &[800.0_f32, 1280.0_f32] {
                    for selection_mode in [false, true] {
                        let mut state = AppState::new(AppConfig::default());
                        state.catalog = knotra_ui::i18n::Catalog::for_locale(locale);
                        install_one_project_per_status(&mut state);
                        state.selection_mode = selection_mode;

                        // RFC-071 D11/`main.rs`'s own `.scale_factor` hook:
                        // a real window at `zoom_percent` offers the view
                        // tree a *logical* area shrunk from its physical
                        // size, and `width_mode` is derived from that same
                        // logical width — not the physical one — so a
                        // physically-1280px window at 150% zoom still gets
                        // the Compact composition, exactly as it would in
                        // a real window.
                        let logical = crate::headless::logical_size_at_zoom(
                            Size::new(width, 720.0),
                            zoom_percent,
                        );
                        state.width_mode = WidthMode::from_width(logical.width);

                        let context = format!(
                            "locale={locale:?} zoom={zoom_percent}% width={width} \
                             selection_mode={selection_mode} width_mode={:?}",
                            state.width_mode
                        );

                        let mut ui =
                            crate::headless::simulator(crate::view::app_view(&state), logical);
                        let mut reference = reference_simulator(&state);

                        let mut check = |label: &str, failures: &mut Vec<String>| {
                            if let Err(e) =
                                check_label(&mut ui, &mut reference, label, logical.width, &context)
                            {
                                failures.push(e);
                            }
                        };

                        // Every status filter chip that is directly
                        // visible at this width_mode, with its own count
                        // (see `install_one_project_per_status`'s own doc
                        // comment for why each count is distinct: 1
                        // through 5, in filter order). Compact shows only
                        // the first three directly; the other two sit
                        // behind the "⋯" disclosure, out of this guard's
                        // scope (opening it changes nothing about *these*
                        // three chips' own sizing).
                        let chip_keys: &[(&str, &str)] = match state.width_mode {
                            WidthMode::Compact => &[
                                ("filter.needs_help", "1"),
                                ("filter.dirty", "2"),
                                ("filter.behind", "3"),
                            ],
                            WidthMode::Standard | WidthMode::Wide => &[
                                ("filter.needs_help", "1"),
                                ("filter.dirty", "2"),
                                ("filter.behind", "3"),
                                ("filter.ahead", "4"),
                                ("filter.all_set", "5"),
                            ],
                        };
                        for (key, count) in chip_keys {
                            let label = state.t(key).to_owned();
                            check(&label, &mut failures);
                            check(count, &mut failures);
                            checked += 2;
                        }

                        if state.width_mode == WidthMode::Standard
                            || state.width_mode == WidthMode::Wide
                        {
                            let arrange = state.t("dashboard.arrange").to_owned();
                            check(&arrange, &mut failures);
                            checked += 1;
                        } else {
                            // Compact's own "⋯" overflow trigger — the
                            // control Review 280 §2.1 found pushed off
                            // the visible window entirely. Not a catalog
                            // label (a bare glyph), but checked the same
                            // way: unclamped and inside the viewport.
                            check("⋯", &mut failures);
                            checked += 1;
                        }

                        let select = state.t("plain.selection.enter").to_owned();
                        check(&select, &mut failures);
                        checked += 1;

                        let resolve = state.t("dashboard.resolve").to_owned();
                        check(&resolve, &mut failures);
                        checked += 1;

                        // The selection-mode action bar (`selection_bar.rs`):
                        // five buttons, shown only while `selection_mode`.
                        if selection_mode {
                            for key in [
                                "plain.check_for_updates",
                                "plain.get_latest",
                                "plain.save_release_point",
                                "plain.change_work_area",
                                "plain.exit_selection",
                            ] {
                                let label = state.t(key).to_owned();
                                check(&label, &mut failures);
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }

        assert!(checked > 0, "the matrix must check at least one label");
        eprintln!("label_overflow_guard: checked {checked} label/control pairs");
        assert!(
            failures.is_empty(),
            "{} of {checked} label/control pairs were clamped or off-screen:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

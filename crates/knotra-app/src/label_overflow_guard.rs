//! Task 108 Amendment A1 §2 — every visible button/chip label fits inside
//! its own control, across language, zoom and window width.
//!
//! **Enumeration, and why this checks a fixed list of catalog keys.**
//! `iced_test`'s public `Simulator` API only exposes `Selector::find()`
//! (stop at the first match) through `Simulator::find`/`click` — never
//! `Selector::find_all()` (`iced_selector`'s own `Finder<All<S>>`), which
//! is a `widget::Operation`, not a `Selector`, and so cannot be passed to
//! `Simulator::find`'s `S: Selector` bound. There is no other public
//! method on `Simulator` that runs an arbitrary `Operation`. A real
//! enumerate-everything probe was written and run to confirm this
//! directly rather than inferred from the type signatures alone (see the
//! report); it is not kept here. So this guard checks a fixed list of
//! catalog keys the dashboard view actually calls `state.t()` with for a
//! button or chip label, per the task's own fallback.
//!
//! **"Fits inside" means not silently clamped.** A first version of this
//! guard compared each label's own `Candidate::Text` bounds against its
//! nearest enclosing fixed-height `Container`'s bounds, expecting
//! containment to fail once a control was shrunk. It was proven wrong
//! before being kept: a scratch edit (`.width(Length::Fixed(50.0))` on
//! `arrange_button`, and separately on `row_action_button`) showed that
//! iced's own layout **shrinks the label's reported bounds to whatever
//! the control was given** — the "Arrange" label's own width went from
//! its natural ~61 to 8, not past its container's edge — so the label
//! never geometrically exceeds its control; it is clamped instead, always
//! making "contained" trivially true. The real, visible defect a narrow
//! control produces is exactly that clamp: a label rendered **narrower
//! than it would be with room to render fully**, not one that overflows.
//! This guard checks for that instead: each label's width in the matrix
//! cell is compared against the same label's width rendered at a
//! generously wide reference size (same state, same locale, same
//! `width_mode`/composition, logical width 4000) — "fits" means the
//! rendered width is at least 90% of that reference. The threshold is not
//! 100%: a tight-but-adequate control can still shift a label's width by
//! a point or two through ordinary sub-pixel text shaping, confirmed on a
//! real matrix cell ("Updates available" at 96.2% of its reference,
//! visually intact — a scratch PNG, not kept) — and a second, more
//! marginal case at 90.2% (`COMPACT_VISIBLE_CHIP_COUNT` scratch-raised to
//! 5) that is this threshold's own lower edge. A genuinely too-small
//! control clamps a label far harder: forcing real crowding (the chip
//! row's own `.spacing()` scratched to 400) read every crowded label at
//! 0% of its reference, nowhere near 90%.
//!
//! **Scope.** The dashboard's toolbar chips (`StatusFilter`'s five
//! labels), "Arrange" (standard composition only — compact's own trigger
//! is the bare glyph `▾`, not a translated label, so it is not a
//! label-overflow risk) and "Select" (`plain.selection.enter`), plus one
//! project row's own action button. A `NeedsHelp` row's action is either
//! "Resolve" (a merge conflict) or "Show details" (every other
//! `NeedsHelp` cause) — both built by the same `row_action_button`
//! (`row.rs`), with only the catalog key and style function differing, so
//! checking the fixture's "Resolve" exercises the same construction code
//! "Show details" would. "Clear filters" and the per-project name/
//! selection-checkbox buttons (whose own label is the project's name, not
//! a catalog string) are out of this guard's scope.

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

    /// One project per `StatusFilter` category, all in one workspace — the
    /// task's own "fixtures-like state (one project per status)". Named in
    /// English for a stable identity across the language matrix; only the
    /// catalog-driven labels (chip/button text), never a project's own
    /// name, are what this guard checks.
    fn install_one_project_per_status(state: &mut AppState) {
        state.config.dashboard_all_set_collapsed = false;

        let make =
            |name: &str, status_of: fn(&Project) -> ProjectStatus| -> (Project, ProjectStatus) {
                let project = Project::new(name, format!("/tmp/{name}"));
                let status = status_of(&project);
                (project, status)
            };

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

        let (needs_help, mut needs_help_status) = make("Needs Help Co", base_status);
        needs_help_status.conflict = ConflictStatus {
            has_conflict: true,
            conflict_count: Some(2),
            detection_unavailable: false,
        };

        let (dirty, mut dirty_status) = make("Unsaved Work Co", base_status);
        dirty_status.working_tree = WorkingTreeStatus {
            uncommitted_count: 3,
            untracked_count: 0,
        };

        let (behind, mut behind_status) = make("Behind Co", base_status);
        behind_status.remote = RemoteStatus {
            ahead: 0,
            behind: 2,
            upstream: Some("origin/main".to_owned()),
        };

        let (ahead, mut ahead_status) = make("Ahead Co", base_status);
        ahead_status.remote = RemoteStatus {
            ahead: 2,
            behind: 0,
            upstream: Some("origin/main".to_owned()),
        };

        let (all_set, all_set_status) = make("All Set Co", base_status);

        let mut workspace = Workspace::new("Demo");
        workspace.projects = vec![
            needs_help.clone(),
            dirty.clone(),
            behind.clone(),
            ahead.clone(),
            all_set.clone(),
        ];
        state.all_workspaces = vec![workspace];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![
                needs_help_status,
                dirty_status,
                behind_status,
                ahead_status,
                all_set_status,
            ],
            last_refresh: None,
        });
    }

    /// A generously wide logical size: nothing in the dashboard's own
    /// widths (the search field's 220px, the longest Japanese label) asks
    /// for anything near this, so a label rendered here is at its natural,
    /// unclamped width.
    const REFERENCE_SIZE: Size = Size {
        width: 4000.0,
        height: 2000.0,
    };

    /// Builds a reference [`Simulator`] from the same state (so the same
    /// locale, selection mode and `width_mode`-driven composition apply)
    /// at [`REFERENCE_SIZE`], to read a label's natural width from.
    fn reference_simulator(state: &AppState) -> iced_test::Simulator<'_, crate::message::Message> {
        crate::headless::simulator(
            crate::view::dashboard::view(state, state.width_mode),
            REFERENCE_SIZE,
        )
    }

    /// Checks that `label` renders at (within a point of) its own natural
    /// width in `ui` — not clamped narrower by a control too small to
    /// hold it. `reference` is a [`Simulator`] built from the same state
    /// at [`REFERENCE_SIZE`] (see this module's own doc comment for why
    /// comparing widths, not containment, is the real check). Returns the
    /// failure message instead of panicking, so the caller can run the
    /// full matrix and report every cell that fails, not only the first.
    fn check_label_not_truncated(
        ui: &mut iced_test::Simulator<'_, crate::message::Message>,
        reference: &mut iced_test::Simulator<'_, crate::message::Message>,
        label: &str,
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

        // A tight but not actually too-small control can shift a label's
        // rendered width by a few percent through ordinary sub-pixel text
        // shaping/kerning — confirmed visually on this exact matrix cell,
        // captured as a scratch PNG, not kept: "Updates available" at
        // locale=En, zoom=150%, width=800 (logical ~533, Compact) renders
        // 128.3px against a 133.3px reference (96.2%), fully legible, no
        // dropped characters. A real clamp from a too-small control is far
        // larger — the scratch-shrink verification this guard's own doc
        // comment describes dropped a label from ~61px to 8px (13%) — so
        // 90% of the reference width is a tolerance with real margin on
        // both sides, not a loosened threshold that would also pass that
        // shrink.
        let threshold = natural.width * 0.9;
        if actual.width >= threshold {
            Ok(())
        } else {
            Err(format!(
                "{context}: label {label:?} rendered width {:.1} ({actual:?}) but its natural \
                 width at {REFERENCE_SIZE:?} is {:.1} ({natural:?}, threshold {:.1}) — its \
                 control clamped it narrower, the visible defect a too-small control produces",
                actual.width, natural.width, threshold
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

                        let mut ui = crate::headless::simulator(
                            crate::view::dashboard::view(&state, state.width_mode),
                            logical,
                        );
                        let mut reference = reference_simulator(&state);

                        // Every status filter chip that is directly
                        // visible at this width_mode. Compact shows only
                        // the first three directly; the other two sit
                        // behind the "⋯" disclosure, out of this guard's
                        // scope (opening it changes nothing about *these*
                        // three chips' own sizing).
                        let chip_keys: &[&str] = match state.width_mode {
                            WidthMode::Compact => {
                                &["filter.needs_help", "filter.dirty", "filter.behind"]
                            }
                            WidthMode::Standard | WidthMode::Wide => &[
                                "filter.needs_help",
                                "filter.dirty",
                                "filter.behind",
                                "filter.ahead",
                                "filter.all_set",
                            ],
                        };
                        let mut check = |label: &str, failures: &mut Vec<String>| {
                            if let Err(e) =
                                check_label_not_truncated(&mut ui, &mut reference, label, &context)
                            {
                                failures.push(e);
                            }
                        };

                        for key in chip_keys {
                            let label = state.t(key).to_owned();
                            check(&label, &mut failures);
                            checked += 1;
                        }

                        if state.width_mode == WidthMode::Standard
                            || state.width_mode == WidthMode::Wide
                        {
                            let arrange = state.t("dashboard.arrange").to_owned();
                            check(&arrange, &mut failures);
                            checked += 1;
                        }

                        let select = state.t("plain.selection.enter").to_owned();
                        check(&select, &mut failures);
                        checked += 1;

                        let resolve = state.t("dashboard.resolve").to_owned();
                        check(&resolve, &mut failures);
                        checked += 1;
                    }
                }
            }
        }

        assert!(checked > 0, "the matrix must check at least one label");
        eprintln!("label_overflow_guard: checked {checked} label/control pairs");
        assert!(
            failures.is_empty(),
            "{} of {checked} label/control pairs were clamped:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

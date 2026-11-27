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
            refreshed_at: jiff::Timestamp::now(),
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

                        // RFC-073 Amendment A6 §1: Compact shows every
                        // status filter directly now, in the same wrapping
                        // row Standard uses — the three-chip split and the
                        // "⋯" disclosure that used to hide the rest are
                        // gone, so there is no longer a Compact-only
                        // narrower list here.
                        let chip_keys: &[(&str, &str)] = &[
                            ("filter.needs_help", "1"),
                            ("filter.dirty", "2"),
                            ("filter.behind", "3"),
                            ("filter.ahead", "4"),
                            ("filter.all_set", "5"),
                        ];
                        for (key, count) in chip_keys {
                            let label = state.t(key).to_owned();
                            check(&label, &mut failures);
                            check(count, &mut failures);
                            checked += 2;
                        }

                        // RFC-073 Amendment A6 §1: Compact's "Arrange" is
                        // Standard's own control now — the same label, the
                        // same catalog key, no more bare "▾" glyph to check
                        // instead.
                        let arrange = state.t("dashboard.arrange").to_owned();
                        check(&arrange, &mut failures);
                        checked += 1;

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

    /// Task 118: Get latest safely's plan review used to lay a dirty
    /// project's note in a third, narrow column (`3 : 2 : 3` FillPortion),
    /// squeezed further when the middle column held the two disposition
    /// buttons. Japanese has no spaces to break at, so the note wrapped
    /// mid-word — 0.34.0's committed Store image shows 「み」 alone on its
    /// own line. The fix gives the note its own full-width line beneath the
    /// row, aligned with the project name.
    ///
    /// **What this checks, without reaching into `OverlayWidth`'s private
    /// resolved-pixel math (RFC-053 D2/D3 deliberately hides it even from
    /// this crate):** the note's own left edge must sit at the same `x` as
    /// the project name's — true only when the note is the sole content of
    /// its own line, not a sibling column starting partway across the row.
    /// A column position is a strictly stronger signal than a width
    /// threshold would be: no dirty-code accident makes it line up by
    /// coincidence, where a width-based margin might, and unlike a width
    /// check it needs no "natural" reference at all — comparing a
    /// `Length::Fill` text's rendered width against its own unconstrained
    /// render is not meaningful in the first place, since a `Fill` text's
    /// bounds reflect the box it was given, not its glyph content, at any
    /// size. The right edge is still checked against the viewport, the same
    /// overflow check [`check_label`] makes for a button.
    ///
    /// **Seen failing first** against the pre-fix view (the x-alignment
    /// assertion failed: the note's `x` sat well to the right of the
    /// project name's, at the old middle column's own width plus spacing).
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "the pinned faces (Noto Sans, Noto Sans CJK JP) are installed only on the \
                  Linux CI runner (Task 108 A1 §4) — the widths this guard measures would be \
                  some other font's on macOS/Windows"
    )]
    fn the_plan_reviews_note_gets_its_own_full_width_line() {
        use crate::state::sync::SyncPhase;
        use knotra_vcs::{
            OperationId, ProjectId, SmartPullDisposition, SmartPullPlan, SmartPullPlanEntry,
        };

        let mut checked = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for &locale in &[knotra_ui::i18n::Locale::En, knotra_ui::i18n::Locale::Ja] {
            for &zoom_percent in &[100.0_f32, 150.0_f32] {
                for &width in &[800.0_f32, 1280.0_f32] {
                    let mut state = AppState::new(AppConfig::default());
                    state.catalog = knotra_ui::i18n::Catalog::for_locale(locale);

                    // `pull_modal`'s `AwaitingConfirm` entries carry their
                    // own `project_name` directly; no workspace lookup
                    // needed (unlike `sync.retry_exclusions`, untouched
                    // here).
                    state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
                        id: OperationId::new(),
                        entries: vec![SmartPullPlanEntry {
                            project_id: ProjectId::new(),
                            project_name: "billing-service".to_owned(),
                            is_dirty: true,
                            has_conflict: false,
                            skip_reason: None,
                            disposition: SmartPullDisposition::FetchOnly,
                        }],
                    });

                    let logical = crate::headless::logical_size_at_zoom(
                        Size::new(width, 720.0),
                        zoom_percent,
                    );
                    state.window_width = logical.width;

                    let context = format!("locale={locale:?} zoom={zoom_percent}% width={width}");

                    let note = state.t("plain.get_latest.note_unsaved").to_owned();
                    let project_name = "billing-service";

                    let mut ui = crate::headless::simulator(
                        crate::view::overlays::pull_modal(&state),
                        logical,
                    );

                    let project_bounds = ui
                        .find(project_name)
                        .unwrap_or_else(|e| {
                            panic!("{context}: project name {project_name:?} not found: {e}")
                        })
                        .bounds();
                    let note_bounds = ui
                        .find(note.as_str())
                        .unwrap_or_else(|e| panic!("{context}: note {note:?} not found: {e}"))
                        .bounds();

                    let mut problems = Vec::new();
                    if (note_bounds.x - project_bounds.x).abs() > 1.0 {
                        problems.push(format!(
                            "note.x {:.2} != project name's x {:.2} (not on its own full-width \
                             line)",
                            note_bounds.x, project_bounds.x
                        ));
                    }
                    if note_bounds.x + note_bounds.width > logical.width + 0.5 {
                        problems.push(format!(
                            "right edge {:.2} exceeds the viewport's {:.2}",
                            note_bounds.x + note_bounds.width,
                            logical.width
                        ));
                    }
                    if !problems.is_empty() {
                        failures.push(format!("{context}: {}", problems.join("; ")));
                    }
                    checked += 1;
                }
            }
        }

        assert!(checked > 0, "the matrix must check at least one case");
        eprintln!("label_overflow_guard: checked {checked} plan-review note cases");
        assert!(
            failures.is_empty(),
            "{} of {checked} cases squeezed the note:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    /// RFC-073 Amendment A7 §1: the detail panel's own three Actions
    /// buttons — Refresh, Check for updates, Remove from workspace.
    /// Review 288 §2 found "Check for updates" (Stage 4's renaming of
    /// "Fetch") wrapped onto a second line *inside* its own button and was
    /// cut off, sharing one row with Refresh at 1280×900, English, zoom
    /// 100%.
    ///
    /// **Its own test, not folded into the matrix above.** The detail
    /// panel renders as a fixed-300px side panel *beside* the dashboard
    /// (`view.rs`), so opening it inside the existing matrix's loop would
    /// have shrunk the dashboard's own available width by 300px on every
    /// iteration — a confounding variable the dashboard's own checks were
    /// never written to tolerate, confirmed directly: doing exactly that
    /// first produced 24 failures, nearly all of them pre-existing
    /// dashboard controls (`Arrange`, `Select`, `Choose resolution`, the
    /// whole selection-mode action bar) newly clamped or pushed off-screen
    /// by the narrower width, not the one real defect this amendment
    /// fixes. This test instead renders the full app view with the panel
    /// open and nothing else competing for its own fixed width, which
    /// does not vary with the outer window at all — `width` is still
    /// swept here only because the handoff asks for "each window width at
    /// which the panel is drawn", not because the panel's own content is
    /// expected to differ by it.
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "the pinned faces (Noto Sans, Noto Sans CJK JP) are installed only on the \
                  Linux CI runner (Task 108 A1 §4) — the widths this guard measures would be \
                  some other font's on macOS/Windows"
    )]
    fn every_detail_panel_action_label_fits_its_button() {
        use crate::state::detail_panel::DetailPanelState;

        // The panel's own ancestor is a fixed `Length::Fixed(300.0)`
        // container (`detail_panel.rs`), unaffected by the outer window
        // size — so `reference_simulator`'s own trick (render the same
        // view at a 4000×2000 window to let a normally-squeezed label
        // reach its natural width) does not work here: at *any* outer
        // size, the panel's own content area is the same 268px, so a
        // clamped/wrapped label would render identically "clamped" in
        // both the real check and its own reference, comparing a bug
        // against itself and finding no difference. Confirmed directly:
        // written first against `reference_simulator`, this test passed
        // even against the pre-fix code — a false negative, not a real
        // pass. The reference here instead builds the same button in
        // total isolation (no panel, no row, no window-size constraint at
        // all) so its label is genuinely free to reach its own natural
        // width.
        fn reference_bounds(label: &str) -> iced_test::core::Rectangle {
            let button: iced::widget::Button<'static, (), iced::Theme, iced::Renderer> =
                iced::widget::button(knotra_ui::widget::centered_button_label(
                    iced::widget::text(label.to_owned()),
                ))
                .height(iced::Length::Fixed(knotra_ui::widget::BUTTON_HEIGHT))
                .padding([0.0, 18.0]);
            let mut sim = crate::headless::simulator(button, REFERENCE_SIZE);
            sim.find(label)
                .unwrap_or_else(|e| panic!("reference label {label:?} not found: {e}"))
                .bounds()
        }

        let mut checked = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for &locale in &[knotra_ui::i18n::Locale::En, knotra_ui::i18n::Locale::Ja] {
            for &zoom_percent in &[100.0_f32, 150.0_f32] {
                for &width in &[800.0_f32, 1280.0_f32] {
                    let mut state = AppState::new(AppConfig::default());
                    state.catalog = knotra_ui::i18n::Catalog::for_locale(locale);
                    install_one_project_per_status(&mut state);
                    let project_id = state
                        .workspace
                        .as_ref()
                        .and_then(|ws| ws.projects.first())
                        .map(|p| p.id.clone())
                        .expect("install_one_project_per_status adds at least one project");
                    state.detail_panel = DetailPanelState {
                        open_project_id: Some(project_id),
                        ..Default::default()
                    };

                    let logical = crate::headless::logical_size_at_zoom(
                        Size::new(width, 720.0),
                        zoom_percent,
                    );
                    state.width_mode = WidthMode::from_width(logical.width);

                    let context = format!(
                        "locale={locale:?} zoom={zoom_percent}% width={width} (detail panel)"
                    );

                    // The panel alone, not the full `app_view` — the
                    // header shows its own "Refresh"/「再読み込み」 control
                    // since Stage 4 (`plain.check_now`, a *different*
                    // catalog key with the same literal text as this
                    // panel's own `detail.refresh`), and `Simulator::find`
                    // has no way to pick which of two same-text matches is
                    // the one under test — confirmed directly: through
                    // `app_view`, "Refresh" intermittently matched the
                    // header's own control instead (only at width=1280,
                    // the Standard/Wide band where the header shows a text
                    // label, not just an icon), giving a false failure
                    // this isolated render does not have.
                    let panel = crate::view::detail_panel::view(&state)
                        .expect("detail_panel.open_project_id is set above");
                    let mut ui = crate::headless::simulator(panel, logical);

                    for key in [
                        "detail.refresh",
                        "detail.fetch",
                        "detail.remove_from_workspace",
                    ] {
                        let label = state.t(key).to_owned();
                        let natural = reference_bounds(&label);
                        let actual = ui
                            .find(label.as_str())
                            .unwrap_or_else(|e| panic!("{context}: label {label:?} not found: {e}"))
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
                        if actual.x + actual.width > logical.width + 0.5 {
                            problems.push(format!(
                                "right edge {:.2} exceeds the viewport's {:.2}",
                                actual.x + actual.width,
                                logical.width
                            ));
                        }
                        if !problems.is_empty() {
                            failures.push(format!(
                                "{context}: label {label:?} {actual:?} (natural {natural:?}): {}",
                                problems.join("; ")
                            ));
                        }
                        checked += 1;
                    }
                }
            }
        }

        assert!(checked > 0, "the matrix must check at least one label");
        eprintln!("label_overflow_guard: checked {checked} detail-panel label/control pairs");
        assert!(
            failures.is_empty(),
            "{} of {checked} detail-panel label/control pairs were clamped or off-screen:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    /// RFC-074 Stage 1 handoff §1/§3 row 2: "the line never wraps inside a
    /// name". Each segment of the plan's own summary line is its own
    /// `text()` with `Wrapping::None` (`plan_summary_line`'s own module
    /// doc explains why — a plain joined string with non-breaking spaces,
    /// tried first, did not hold for Japanese, whose names have no spaces
    /// at all for cosmic text's CJK line-breaking to respect), so this is
    /// checked exactly the way `every_label_fits_its_control_across_the_
    /// matrix` above already checks the toolbar's own filter chips, built
    /// the same way for the same reason: a segment's own *rendered* width
    /// must equal its natural, unclamped width — any narrowing at all
    /// would mean it wrapped or was clamped internally — and must lie
    /// inside the viewport.
    ///
    /// **Seen failing first**, confirmed directly: against a version of
    /// `plan_summary_line` built as one flowing `text()` with `.replace('
    /// ', "\u{a0}")` instead of per-segment `Wrapping::None` chips (the
    /// first design tried, kept just long enough to run this check
    /// against it), every one of this test's 24 cases failed — each
    /// segment's own text no longer exists as its own widget to find at
    /// all, since the whole line is one joined string now. A narrower,
    /// cosmic-text-level version of this same check (looking inside that
    /// one string's own wrapped `layout_runs()` instead) had already shown
    /// the more specific defect non-breaking spaces alone cannot fix:
    /// Japanese's own names have no spaces for cosmic text's CJK
    /// line-breaking to respect, so a forced-narrow render split 「更新の
    /// 確認のみ」 after its 5th character. Reverted before this test was
    /// committed; the shipped function uses the chip-row design.
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "the pinned faces (Noto Sans, Noto Sans CJK JP) are installed only on the \
                  Linux CI runner (Task 108 A1 §4) — the widths this guard measures would be \
                  some other font's on macOS/Windows"
    )]
    fn the_plan_summary_lines_segments_never_wrap_or_clamp() {
        use crate::state::sync::SyncPhase;
        use knotra_vcs::model::operation::SmartPullSkipReason;
        use knotra_vcs::{
            OperationId, ProjectId, SmartPullDisposition, SmartPullPlan, SmartPullPlanEntry,
        };

        let mut checked = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for &locale in &[knotra_ui::i18n::Locale::En, knotra_ui::i18n::Locale::Ja] {
            let mut state = AppState::new(AppConfig::default());
            state.catalog = knotra_ui::i18n::Catalog::for_locale(locale);
            let plan = SmartPullPlan {
                id: OperationId::new(),
                entries: vec![
                    SmartPullPlanEntry {
                        project_id: ProjectId::new(),
                        project_name: "a".to_owned(),
                        is_dirty: false,
                        has_conflict: false,
                        skip_reason: None,
                        disposition: SmartPullDisposition::Pull,
                    },
                    SmartPullPlanEntry {
                        project_id: ProjectId::new(),
                        project_name: "b".to_owned(),
                        is_dirty: true,
                        has_conflict: false,
                        skip_reason: None,
                        disposition: SmartPullDisposition::FetchOnly,
                    },
                    SmartPullPlanEntry {
                        project_id: ProjectId::new(),
                        project_name: "c".to_owned(),
                        is_dirty: false,
                        has_conflict: false,
                        skip_reason: Some(SmartPullSkipReason::Deselected),
                        disposition: SmartPullDisposition::Excluded,
                    },
                ],
            };
            let segments = crate::view::overlays::plan_summary_segments(&state, &plan);
            assert_eq!(
                segments.len(),
                3,
                "sanity: all three dispositions must appear"
            );
            state.sync.phase = SyncPhase::AwaitingConfirm(plan);

            for &zoom_percent in &[100.0_f32, 150.0_f32] {
                for &width in &[800.0_f32, 1280.0_f32] {
                    let logical = crate::headless::logical_size_at_zoom(
                        Size::new(width, 900.0),
                        zoom_percent,
                    );
                    state.window_width = logical.width;

                    let context = format!("locale={locale:?} zoom={zoom_percent}% width={width}");

                    let mut ui = crate::headless::simulator(
                        crate::view::overlays::pull_modal(&state),
                        logical,
                    );
                    let mut reference = crate::headless::simulator(
                        crate::view::overlays::pull_modal(&state),
                        REFERENCE_SIZE,
                    );

                    for (i, segment) in segments.iter().enumerate() {
                        // The first segment renders verbatim; every later
                        // one carries the `· ` separator `plan_summary_
                        // line` folds into its own chip, so a wrap never
                        // isolates a lone separator.
                        let label = if i == 0 {
                            segment.clone()
                        } else {
                            format!("· {segment}")
                        };
                        if let Err(e) =
                            check_label(&mut ui, &mut reference, &label, logical.width, &context)
                        {
                            failures.push(e);
                        }
                        checked += 1;
                    }
                }
            }
        }

        assert!(checked > 0, "the matrix must check at least one segment");
        eprintln!("label_overflow_guard: checked {checked} plan-summary segments");
        assert!(
            failures.is_empty(),
            "{} of {checked} plan-summary segments were clamped, wrapped or off-screen:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    /// Task 122 §2: extends this guard from button/chip labels to every
    /// description/note paragraph — any UI text long enough to wrap, where
    /// Japanese (no spaces to break at) can strand one short word alone on
    /// the wrapped text's own last line (Review 305 F3: Settings'
    /// file-change description, 「す。」 alone on its own second line).
    ///
    /// **The instrument.** `Simulator::find` gives a paragraph's own
    /// *rendered* width — its real column, after the real layout — but not
    /// its wrapped line boundaries (no public API exposes those from a
    /// live widget tree). So each case renders the real screen first to
    /// read that one real number, `bounds.width`, then builds a second,
    /// independent `Paragraph` (the same `iced::Renderer`-backed, tiny-skia
    /// type every headless test already uses, Task 123) at exactly that
    /// width with the same font/size/line-height/wrapping the real
    /// `text(...)` call uses, and reads its wrapped lines back from
    /// `cosmic_text::Buffer::layout_runs()` (re-exported at
    /// `iced::advanced::graphics::text::cosmic_text`) — each run's own
    /// `text`/glyph `start`/`end` gives the exact substring that landed on
    /// that line, not a pixel-band guess. Confirmed against the known
    /// defect directly before writing this: at Ja/150%/800px, Settings'
    /// `fs_watch_hint` measures a 479.5px column and wraps onto two lines
    /// ending "…プロジェクト" / "…再読み込みします。" (13 chars) — not
    /// stranded at today's measurement, a width this exact for a reason
    /// this guard does not need to know (RFC-065/RFC-073 layout changes
    /// since Review 305 F3's own screenshot).
    ///
    /// **Scope**, surveyed directly across every file under `view/`,
    /// `view/overlays/`, `view/dashboard/` for a `text(state.t(...))` call
    /// styled `body`/`body_small` that is explanatory prose, not a
    /// title/button/chip/badge/field-label/placeholder: Settings (zoom
    /// help, the file-watch hint), Add Project (both step instructions),
    /// Remove project, Workspace delete (both of its own two sentences),
    /// the empty-workspace welcome text, Get latest safely (the preparing
    /// hint, and all five bounded skip-reason notes), Change work area
    /// (the loading hint, "already current", and all five confirm-step
    /// cautions), Save release point (the saving hint, both impact-panel
    /// notes, the share offer), and Resolve conflict (the instruction, no
    /// unresolved files, the working hint, the stop-confirmation body).
    ///
    /// **Left out, and why:** `history.rs`'s recovery hints and
    /// `changelog.rs`'s per-project result lines splice in raw,
    /// un-translated VCS/error text the catalog never touches (confirmed
    /// directly in `knotra-vcs`) or an open-ended project-name/error tail —
    /// neither is a fixed string this instrument can enumerate, the same
    /// reason `detail_panel.rs`'s own `commits_error` is out of scope.
    /// `plain.what_will_happen` (Get latest safely) is a column header, not
    /// a description — confirmed by reading its own row (beside
    /// `plain.project`, the other column's header). `activity_strip.rs`
    /// and `command_palette.rs` hold only single-line status/snackbar
    /// text, never a wrapping paragraph.
    ///
    /// **Task 124**: four cases (`settings.fs_watch_hint`,
    /// `workspace.delete.body_suffix`, `plain.empty.welcome_body`,
    /// `plain.switch.dirty_hint`) strand a word with today's Japanese text
    /// and no layout fix available (each already renders at its own full
    /// available width — confirmed by reading the surrounding view code).
    /// Task 122 pinned these in a named, self-checking
    /// `KNOWN_PENDING_REWORDING` allowlist while awaiting the owner's own
    /// wording (Review 303: a list of accepted defects is exactly what a
    /// suite must not keep, so Review 312 §2 asked for the list itself to
    /// go once the words landed). The owner's wording (Review 312 §3)
    /// fixes all four; the allowlist and its own forcing-function code are
    /// gone — this assertion has no exception.
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "renders and re-lays-out text — the pinned faces (Noto Sans, Noto Sans CJK \
                  JP) are installed only on the Linux CI runner (Task 108 A1 §4)"
    )]
    fn no_japanese_description_strands_a_word_on_its_last_line() {
        use crate::state::conflict_ops::ConflictPhase;
        use crate::state::context::ContextPhase;
        use crate::state::freezer::FreezerPhase;
        use crate::state::sync::SyncPhase;
        use crate::view;
        use iced::Element;
        use iced::advanced::text::{Paragraph as _, Text};
        use knotra_vcs::model::operation::SmartPullSkipReason;
        use knotra_vcs::{
            ConflictMarker, ConflictedFile, ContextCandidate, ContextList, ContextTarget,
            FreezeResult, FreezeValidation, OperationId, ProjectConflictDetail, ProjectId,
            SmartPullDisposition, SmartPullPlan, SmartPullPlanEntry, Workspace,
        };

        type Msg = crate::message::Message;

        /// A case's own `setup` always starts a project with this id, so
        /// `render` (which cannot capture a value `setup` computed — both
        /// are plain `fn`, not closures, the same reason `headless.rs`'s
        /// own helpers stay free functions) can re-derive the same id
        /// deterministically rather than needing it passed in.
        fn fixed_project_id() -> ProjectId {
            ProjectId(uuid::Uuid::nil())
        }

        /// A `'static` handle to the same id, for `resolve_panel`'s own
        /// second argument, whose lifetime ties to the `Element` this
        /// `fn`-typed `render` must return — a fresh local `ProjectId`
        /// does not live long enough to borrow from; a `'static` one does.
        fn fixed_project_id_static() -> &'static ProjectId {
            static ID: std::sync::OnceLock<ProjectId> = std::sync::OnceLock::new();
            ID.get_or_init(fixed_project_id)
        }

        struct ParagraphCase {
            name: &'static str,
            setup: fn(&mut AppState),
            render: fn(&AppState) -> Element<'_, Msg>,
            content: fn(&AppState) -> String,
        }

        let cases: &[ParagraphCase] = &[
            ParagraphCase {
                name: "Settings: zoom help",
                setup: |_state| {},
                render: |state| view::settings::view(state),
                content: |state| state.t("settings.zoom_help").to_owned(),
            },
            ParagraphCase {
                name: "Settings: file-watch hint",
                setup: |state| state.settings_advanced_open = true,
                render: |state| view::settings::view(state),
                content: |state| state.t("settings.fs_watch_hint").to_owned(),
            },
            ParagraphCase {
                name: "Add Project: step 1 instruction",
                setup: |state| {
                    state.add_project_dialog = Some(crate::state::AddProjectDialog::default());
                },
                render: |state| view::add_project_modal::view(state).expect("dialog must be open"),
                content: |state| state.t("plain.add_project.step1_instruction").to_owned(),
            },
            ParagraphCase {
                name: "Add Project: step 2 instruction",
                setup: |state| {
                    state.add_project_dialog = Some(crate::state::AddProjectDialog {
                        step: crate::state::AddProjectStep::NameProject,
                        ..Default::default()
                    });
                },
                render: |state| view::add_project_modal::view(state).expect("dialog must be open"),
                content: |state| state.t("plain.add_project.step2_instruction").to_owned(),
            },
            ParagraphCase {
                name: "Remove project: body",
                setup: |state| {
                    state.confirm_remove_dialog = Some(crate::state::ConfirmRemoveDialog {
                        project_id: fixed_project_id(),
                        project_name: "billing-service".to_owned(),
                    });
                },
                render: |state| {
                    view::remove_project_dialog::view(state).expect("dialog must be open")
                },
                content: |state| state.t("plain.remove.body").to_owned(),
            },
            ParagraphCase {
                name: "Workspace delete: body sentence",
                setup: |state| {
                    state.all_workspaces =
                        vec![Workspace::new("billing-service"), Workspace::new("other")];
                    state.workspace_mgr.confirm_delete =
                        Some(crate::state::workspace_mgr::DeleteWorkspaceDialog {
                            workspace_id: state.all_workspaces[0].id,
                            workspace_name: "billing-service".to_owned(),
                            project_count: 3,
                            error: None,
                        });
                },
                render: |state| view::workspace_manager::view(state).expect("dialog must be open"),
                content: |state| {
                    format!(
                        "{} \"{}\". {}",
                        state.t("workspace.delete.body_prefix"),
                        "billing-service",
                        state.t("workspace.delete.body_suffix"),
                    )
                },
            },
            ParagraphCase {
                name: "Workspace delete: project count sentence",
                setup: |state| {
                    state.all_workspaces =
                        vec![Workspace::new("billing-service"), Workspace::new("other")];
                    state.workspace_mgr.confirm_delete =
                        Some(crate::state::workspace_mgr::DeleteWorkspaceDialog {
                            workspace_id: state.all_workspaces[0].id,
                            workspace_name: "billing-service".to_owned(),
                            project_count: 3,
                            error: None,
                        });
                },
                render: |state| view::workspace_manager::view(state).expect("dialog must be open"),
                content: |state| {
                    format!("{} {}", 3, state.t("workspace.delete.project_count_suffix"))
                },
            },
            ParagraphCase {
                name: "Dashboard: empty-workspace welcome",
                setup: |state| state.workspace = Some(Workspace::new("Empty")),
                render: |state| view::dashboard::view(state, WidthMode::Standard),
                content: |state| state.t("plain.empty.welcome_body").to_owned(),
            },
            ParagraphCase {
                name: "Get latest safely: preparing hint",
                setup: |_state| {},
                render: |state| view::overlays::pull_modal(state),
                content: |state| state.t("plain.get_latest.preparing_hint").to_owned(),
            },
            ParagraphCase {
                name: "Get latest safely: skip reason (deselected)",
                setup: |state| {
                    state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
                        id: OperationId::new(),
                        entries: vec![SmartPullPlanEntry {
                            project_id: fixed_project_id(),
                            project_name: "billing-service".to_owned(),
                            is_dirty: false,
                            has_conflict: false,
                            skip_reason: Some(SmartPullSkipReason::Deselected),
                            disposition: SmartPullDisposition::Excluded,
                        }],
                    });
                },
                render: |state| view::overlays::pull_modal(state),
                content: |state| {
                    state
                        .t(SmartPullSkipReason::Deselected.i18n_key())
                        .to_owned()
                },
            },
            ParagraphCase {
                name: "Get latest safely: skip reason (no upstream)",
                setup: |state| {
                    state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
                        id: OperationId::new(),
                        entries: vec![SmartPullPlanEntry {
                            project_id: fixed_project_id(),
                            project_name: "billing-service".to_owned(),
                            is_dirty: false,
                            has_conflict: false,
                            skip_reason: Some(SmartPullSkipReason::NoUpstream),
                            disposition: SmartPullDisposition::Excluded,
                        }],
                    });
                },
                render: |state| view::overlays::pull_modal(state),
                content: |state| {
                    state
                        .t(SmartPullSkipReason::NoUpstream.i18n_key())
                        .to_owned()
                },
            },
            ParagraphCase {
                name: "Get latest safely: skip reason (conflict)",
                setup: |state| {
                    state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
                        id: OperationId::new(),
                        entries: vec![SmartPullPlanEntry {
                            project_id: fixed_project_id(),
                            project_name: "billing-service".to_owned(),
                            is_dirty: false,
                            has_conflict: true,
                            skip_reason: Some(SmartPullSkipReason::Conflict),
                            disposition: SmartPullDisposition::Excluded,
                        }],
                    });
                },
                render: |state| view::overlays::pull_modal(state),
                content: |state| state.t(SmartPullSkipReason::Conflict.i18n_key()).to_owned(),
            },
            ParagraphCase {
                name: "Get latest safely: skip reason (status missing)",
                setup: |state| {
                    state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
                        id: OperationId::new(),
                        entries: vec![SmartPullPlanEntry {
                            project_id: fixed_project_id(),
                            project_name: "billing-service".to_owned(),
                            is_dirty: false,
                            has_conflict: false,
                            skip_reason: Some(SmartPullSkipReason::MissingStatus),
                            disposition: SmartPullDisposition::Excluded,
                        }],
                    });
                },
                render: |state| view::overlays::pull_modal(state),
                content: |state| {
                    state
                        .t(SmartPullSkipReason::MissingStatus.i18n_key())
                        .to_owned()
                },
            },
            ParagraphCase {
                name: "Get latest safely: skip reason (project not found)",
                setup: |state| {
                    state.sync.phase = SyncPhase::AwaitingConfirm(SmartPullPlan {
                        id: OperationId::new(),
                        entries: vec![SmartPullPlanEntry {
                            project_id: fixed_project_id(),
                            project_name: "billing-service".to_owned(),
                            is_dirty: false,
                            has_conflict: false,
                            skip_reason: Some(SmartPullSkipReason::ProjectNotFound),
                            disposition: SmartPullDisposition::Excluded,
                        }],
                    });
                },
                render: |state| view::overlays::pull_modal(state),
                content: |state| {
                    state
                        .t(SmartPullSkipReason::ProjectNotFound.i18n_key())
                        .to_owned()
                },
            },
            ParagraphCase {
                name: "Change work area: loading hint",
                setup: |state| {
                    state.context_ops.phase = ContextPhase::LoadingList(fixed_project_id())
                },
                render: |state| view::overlays::switch_modal(state),
                content: |state| state.t("plain.switch.loading_hint").to_owned(),
            },
            ParagraphCase {
                name: "Change work area: already current",
                setup: |state| {
                    state.context_ops.phase = ContextPhase::BrowsingList {
                        project_id: fixed_project_id(),
                        list: ContextList {
                            project_id: fixed_project_id(),
                            vcs_kind: knotra_vcs::VcsKind::Git,
                            candidates: vec![ContextCandidate {
                                label: "main".to_owned(),
                                target: ContextTarget::GitLocalBranch {
                                    name: "main".to_owned(),
                                },
                                is_current: true,
                            }],
                            warning: None,
                        },
                        search: String::new(),
                    };
                },
                render: |state| view::overlays::switch_modal(state),
                content: |state| state.t("plain.switch.reason_current").to_owned(),
            },
            ParagraphCase {
                name: "Change work area: confirm caution (unavailable)",
                setup: |state| {
                    state.context_ops.phase = ContextPhase::ConfirmSwitch {
                        project_id: fixed_project_id(),
                        project_name: "billing-service".to_owned(),
                        target: ContextTarget::GitLocalBranch {
                            name: "feature/x".to_owned(),
                        },
                        target_label: "feature/x".to_owned(),
                        is_dirty: false,
                        disabled_reason_key: Some("plain.switch.reason_unavailable"),
                    };
                },
                render: |state| view::overlays::switch_modal(state),
                content: |state| state.t("plain.switch.reason_unavailable").to_owned(),
            },
            ParagraphCase {
                name: "Change work area: confirm caution (conflict)",
                setup: |state| {
                    state.context_ops.phase = ContextPhase::ConfirmSwitch {
                        project_id: fixed_project_id(),
                        project_name: "billing-service".to_owned(),
                        target: ContextTarget::GitLocalBranch {
                            name: "feature/x".to_owned(),
                        },
                        target_label: "feature/x".to_owned(),
                        is_dirty: false,
                        disabled_reason_key: Some("plain.switch.reason_conflict"),
                    };
                },
                render: |state| view::overlays::switch_modal(state),
                content: |state| state.t("plain.switch.reason_conflict").to_owned(),
            },
            ParagraphCase {
                name: "Change work area: confirm caution (dirty, disabled)",
                setup: |state| {
                    state.context_ops.phase = ContextPhase::ConfirmSwitch {
                        project_id: fixed_project_id(),
                        project_name: "billing-service".to_owned(),
                        target: ContextTarget::GitLocalBranch {
                            name: "feature/x".to_owned(),
                        },
                        target_label: "feature/x".to_owned(),
                        is_dirty: true,
                        disabled_reason_key: Some("plain.switch.reason_dirty"),
                    };
                },
                render: |state| view::overlays::switch_modal(state),
                content: |state| state.t("plain.switch.reason_dirty").to_owned(),
            },
            ParagraphCase {
                name: "Change work area: confirm caution (dirty hint)",
                setup: |state| {
                    state.context_ops.phase = ContextPhase::ConfirmSwitch {
                        project_id: fixed_project_id(),
                        project_name: "billing-service".to_owned(),
                        target: ContextTarget::GitLocalBranch {
                            name: "feature/x".to_owned(),
                        },
                        target_label: "feature/x".to_owned(),
                        is_dirty: true,
                        disabled_reason_key: None,
                    };
                },
                render: |state| view::overlays::switch_modal(state),
                content: |state| state.t("plain.switch.dirty_hint").to_owned(),
            },
            ParagraphCase {
                name: "Change work area: confirm caution (no next step)",
                setup: |state| {
                    state.context_ops.phase = ContextPhase::ConfirmSwitch {
                        project_id: fixed_project_id(),
                        project_name: "billing-service".to_owned(),
                        target: ContextTarget::GitLocalBranch {
                            name: "feature/x".to_owned(),
                        },
                        target_label: "feature/x".to_owned(),
                        is_dirty: false,
                        disabled_reason_key: None,
                    };
                },
                render: |state| view::overlays::switch_modal(state),
                content: |state| state.t("plain.no_next_step").to_owned(),
            },
            ParagraphCase {
                name: "Save release point: saving hint",
                setup: |state| state.freezer.phase = FreezerPhase::Executing,
                render: |state| view::overlays::tag_modal(state),
                content: |state| state.t("plain.release.saving_hint").to_owned(),
            },
            ParagraphCase {
                name: "Save release point: impact unchecked",
                setup: |state| {
                    state.freezer.phase = FreezerPhase::ValidationReady(FreezeValidation {
                        freeze_name: "v1.0.0".to_owned(),
                        entries: vec![],
                    });
                    state.freezer.topology_checked = false;
                },
                render: |state| view::overlays::tag_modal(state),
                content: |state| state.t("plain.release.impact_unchecked").to_owned(),
            },
            ParagraphCase {
                name: "Save release point: impact clear",
                setup: |state| {
                    state.freezer.phase = FreezerPhase::ValidationReady(FreezeValidation {
                        freeze_name: "v1.0.0".to_owned(),
                        entries: vec![],
                    });
                    state.freezer.topology_checked = true;
                    state.freezer.impact_warnings = vec![];
                },
                render: |state| view::overlays::tag_modal(state),
                content: |state| state.t("plain.release.impact_clear").to_owned(),
            },
            ParagraphCase {
                name: "Save release point: share offer",
                setup: |state| {
                    state.freezer.phase = FreezerPhase::Done(FreezeResult {
                        freeze_name: "v1.0.0".to_owned(),
                        project_results: vec![],
                        outcome: knotra_vcs::FreezeOutcome::Success,
                    });
                    state.pending_tag_push = Some(crate::state::PendingTagPush {
                        freeze_name: "v1.0.0".to_owned(),
                        project_ids: vec![fixed_project_id()],
                        is_pushing: false,
                    });
                },
                render: |state| view::overlays::tag_modal(state),
                content: |state| state.t("plain.release.share_offer").to_owned(),
            },
            ParagraphCase {
                name: "Resolve conflict: instruction",
                setup: |state| {
                    state.conflict_ops.phase = ConflictPhase::Browsing {
                        project_id: fixed_project_id(),
                        detail: ProjectConflictDetail {
                            project_id: fixed_project_id(),
                            project_name: "billing-service".to_owned(),
                            conflicted_files: vec![ConflictedFile {
                                path: "src/lib.rs".to_owned(),
                                marker: ConflictMarker::BothModified,
                            }],
                            read_error: None,
                        },
                    };
                },
                render: |state| view::overlays::resolve_panel(state, fixed_project_id_static()),
                content: |state| state.t("plain.resolve.instruction").to_owned(),
            },
            ParagraphCase {
                name: "Resolve conflict: no unresolved files",
                setup: |state| {
                    state.conflict_ops.phase = ConflictPhase::Browsing {
                        project_id: fixed_project_id(),
                        detail: ProjectConflictDetail {
                            project_id: fixed_project_id(),
                            project_name: "billing-service".to_owned(),
                            conflicted_files: vec![],
                            read_error: None,
                        },
                    };
                },
                render: |state| view::overlays::resolve_panel(state, fixed_project_id_static()),
                content: |state| state.t("plain.resolve.no_files").to_owned(),
            },
            ParagraphCase {
                name: "Resolve conflict: working hint",
                setup: |state| {
                    state.conflict_ops.phase = ConflictPhase::Operating {
                        project_id: fixed_project_id(),
                        action: "placeholder".to_owned(),
                    };
                },
                render: |state| view::overlays::resolve_panel(state, fixed_project_id_static()),
                content: |state| state.t("plain.resolve.working_hint").to_owned(),
            },
            ParagraphCase {
                name: "Resolve conflict: stop-confirmation body",
                setup: |state| {
                    state.conflict_ops.phase = ConflictPhase::ConfirmStop {
                        project_id: fixed_project_id(),
                        detail: ProjectConflictDetail {
                            project_id: fixed_project_id(),
                            project_name: "billing-service".to_owned(),
                            conflicted_files: vec![ConflictedFile {
                                path: "src/lib.rs".to_owned(),
                                marker: ConflictMarker::BothModified,
                            }],
                            read_error: None,
                        },
                    };
                },
                render: |state| view::overlays::resolve_panel(state, fixed_project_id_static()),
                content: |state| state.t("plain.resolve.stop_confirm_body").to_owned(),
            },
            // RFC-074 Stage 1's own plan summary line is deliberately
            // left out of this guard (unlike the rows above): it is not
            // a flowing paragraph that could strand a word — each of its
            // segments is its own non-wrapping chip (`plan_summary_line`'s
            // own module doc explains why), so there is no "last line" for
            // this guard's own stranding check to apply to. It has its own
            // guard instead: `the_plan_summary_lines_segments_never_wrap_
            // or_clamp`, above.
        ];

        let mut checked = 0usize;
        let mut stranded: Vec<String> = Vec::new();

        for case in cases {
            for &zoom_percent in &[100.0_f32, 150.0_f32] {
                for &width in &[800.0_f32, 1280.0_f32] {
                    let mut state = AppState::new(AppConfig::default());
                    state.catalog =
                        knotra_ui::i18n::Catalog::for_locale(knotra_ui::i18n::Locale::Ja);
                    (case.setup)(&mut state);

                    let logical = crate::headless::logical_size_at_zoom(
                        Size::new(width, 900.0),
                        zoom_percent,
                    );
                    state.window_width = logical.width;

                    let content = (case.content)(&state);
                    let context = format!("{}: zoom={zoom_percent}% width={width}", case.name);

                    let mut ui = crate::headless::simulator((case.render)(&state), logical);
                    let bounds = ui
                        .find(content.as_str())
                        .unwrap_or_else(|e| {
                            panic!("{context}: paragraph {content:?} not found: {e}")
                        })
                        .bounds();

                    let tokens = &state.theme.tokens;
                    let text = Text {
                        content: content.as_str(),
                        bounds: Size::new(bounds.width, f32::INFINITY),
                        size: snora::design::style::text::body_size(tokens),
                        line_height: snora::design::style::text::body_line_height(tokens),
                        font: iced::Font::with_name("Noto Sans"),
                        align_x: iced::advanced::text::Alignment::Default,
                        align_y: iced::alignment::Vertical::Top,
                        shaping: iced::advanced::text::Shaping::Advanced,
                        wrapping: iced::advanced::text::Wrapping::Word,
                    };
                    type P = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph;
                    let paragraph = P::with_text(text);
                    let runs: Vec<_> = paragraph.buffer().layout_runs().collect();
                    let Some(last) = runs.last() else {
                        continue;
                    };
                    let start = last.glyphs.first().map(|g| g.start).unwrap_or(0);
                    let end = last.glyphs.last().map(|g| g.end).unwrap_or(0);
                    let last_line = &last.text[start..end];
                    let char_count = last_line.chars().count();

                    if runs.len() > 1 && char_count <= 3 {
                        stranded.push(format!(
                            "{context}: last line {last_line:?} ({char_count} chars) of {} total \
                             (bounds.width={:.2})",
                            runs.len(),
                            bounds.width
                        ));
                    }
                    checked += 1;
                }
            }
        }

        assert!(checked > 0, "the matrix must check at least one case");
        eprintln!("label_overflow_guard: checked {checked} description/note cases");
        assert!(
            stranded.is_empty(),
            "{} of {checked} cases strand a word alone on the wrapped text's last line:\n{}",
            stranded.len(),
            stranded.join("\n")
        );
    }
}

//! Task 108 §3 — probes whether `iced_test` 0.14's `Simulator` gives knotra
//! a level-2 (headless UI) harness, per the owner's updated
//! `.git-exclude/rules/project-instructions-gui-common.md`.
//!
//! Five questions, each its own test (or an `#[ignore]`d one with the
//! reason it cannot run everywhere): does it render knotra's view at all;
//! does Escape in a dialog behave the way a real window does today (it
//! does not yet — see §2's own comment); can it give a widget's bounds;
//! what can be varied (size/theme/language/zoom); and does a snapshot
//! stay stable.
//!
//! `#[cfg(test)]`-only: this module never compiles into the shipped
//! binary. Unlike the guards elsewhere in this file's siblings, these are
//! not text-scanning — they drive a real (headless) `iced` widget tree.

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;
    use crate::message::Message;
    use crate::state::{AppState, ConfirmRemoveDialog};
    use iced_test::Simulator;
    use iced_test::core::{Size, keyboard};

    fn make_state() -> AppState {
        AppState::new(AppConfig::default())
    }

    fn install_one_project(state: &mut AppState) {
        use knotra_vcs::{Project, Workspace, WorkspaceStatus};

        // `AppConfig::default()`'s own `dashboard_all_set_collapsed: true`
        // would hide a clean project's row entirely (a collapsed section
        // contributes no row targets — `dashboard/mod.rs`'s own test for
        // this exact rule) — open here so a fixture project with no
        // issues is still selectable, the same way a real workspace's
        // "All set" section would be after a user opens it once.
        state.config.dashboard_all_set_collapsed = false;

        let project = Project::new("Alpha", "/tmp/alpha");
        let id = project.id.clone();
        let mut workspace = Workspace::new("Demo");
        workspace.projects.push(project.clone());
        state.all_workspaces = vec![workspace];
        state.active_workspace_idx = 0;
        state.workspace = state.all_workspaces.first().cloned();
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![knotra_vcs::ProjectStatus {
                project_id: id.clone(),
                identity: knotra_vcs::model::status::RepositoryIdentity {
                    path: "/tmp/alpha".into(),
                    vcs_kind: knotra_vcs::model::status::VcsKind::Git,
                },
                context: None,
                remote: knotra_vcs::model::status::RemoteStatus::default(),
                working_tree: knotra_vcs::model::status::WorkingTreeStatus::default(),
                conflict: knotra_vcs::model::status::ConflictStatus::default(),
                refreshed_at: chrono::Utc::now(),
                read_error: None,
            }],
            last_refresh: None,
        });
    }

    // -----------------------------------------------------------------------
    // 1. Does it render knotra's view at all?
    // -----------------------------------------------------------------------

    /// Builds the dashboard from an `AppState` with one project and no
    /// disk/git behind it (the same "fixtures, no side effects" shape
    /// Task 108 §1's own fixtures follow), finds the "Select" button by its
    /// exact label, clicks it, and asserts the one message it produces.
    #[test]
    fn simulator_renders_the_dashboard_and_clicks_select() {
        let mut state = make_state();
        install_one_project(&mut state);

        let mut ui = Simulator::new(crate::view::dashboard::view(&state, state.width_mode));

        ui.click("Select")
            .expect("the Select button must be found and clickable");

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(
            matches!(
                messages[0],
                Message::Selection(crate::message::SelectionMessage::ModeEntered)
            ),
            "{messages:?}"
        );
    }

    // -----------------------------------------------------------------------
    // 2. Escape in a dialog.
    // -----------------------------------------------------------------------

    /// Opens "Remove project" (`Layer::RemoveProject`, driven by
    /// `confirm_remove_dialog`) over the full `app_view` — the dialog lives
    /// in `AppLayout`'s dialog slot, not inside `dashboard::view` alone —
    /// and taps Escape.
    ///
    /// **On snora 0.53 (today): expect zero messages.** knotra's Escape
    /// handling is wired through `app.rs::subscription`'s
    /// `keyboard::listen()`, not through a widget in the view tree — the
    /// `Simulator` only ever drives the view tree's own `update`, and runs
    /// no subscription at all, so the key tap that a real window's
    /// subscription would turn into `Message::Shortcut(ShortcutMessage::
    /// Close)` produces nothing here. This is the honest, current gap
    /// Review 273 §4/Task 107 names.
    ///
    /// **Task 107 (snora 0.57) flips this.** From snora 0.55, `AppLayout`'s
    /// `on_close_modals` captures Escape itself and publishes the close
    /// message *as a widget operation inside the view tree* — something
    /// the `Simulator` *does* run. Once knotra is on 0.55+, this same test
    /// should assert `messages == [Message::Shortcut(ShortcutMessage::
    /// Close)]`, exactly one. Update this test alongside that upgrade,
    /// rather than leaving the zero-message assertion to go stale.
    #[test]
    fn escape_in_a_dialog_produces_no_message_on_snora_0_53() {
        let mut state = make_state();
        state.confirm_remove_dialog = Some(ConfirmRemoveDialog {
            project_id: knotra_vcs::ProjectId::new(),
            project_name: "Alpha".to_owned(),
        });

        let mut ui = Simulator::new(crate::view::app_view(&state));
        let _ = ui.tap_key(keyboard::Key::Named(keyboard::key::Named::Escape));

        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            messages.is_empty(),
            "expected no message on snora 0.53 (Escape is subscription-driven, \
             which the simulator does not run); got {messages:?} — if this \
             starts failing, Task 107's snora 0.57 upgrade has probably \
             landed: update this test to expect exactly one \
             Message::Shortcut(ShortcutMessage::Close) instead, per this \
             test's own doc comment"
        );
    }

    // -----------------------------------------------------------------------
    // 3. Bounds.
    // -----------------------------------------------------------------------

    /// **Yes** — `Simulator::find` returns `iced_selector::target::Text`
    /// for a `&str` selector, and `Text::bounds()`/`visible_bounds()` give
    /// a real `Rectangle`. Demonstrated directly: the "Select" button's
    /// label sits inside its own control's bounds (the thing a
    /// label-fits-inside-its-control guard would need), proven here by
    /// finding both the label's `Text` bounds and the button's own visible
    /// bounds via the same selector's `click()` return value, and
    /// asserting one contains the other on every axis.
    #[test]
    fn find_gives_widget_bounds() {
        let mut state = make_state();
        install_one_project(&mut state);
        let mut ui = Simulator::new(crate::view::dashboard::view(&state, state.width_mode));

        let label = ui.find("Select").expect("label must be found");
        let label_bounds = label.bounds();

        let clicked = ui.click("Select").expect("button must be clickable");
        let button_bounds = clicked
            .visible_bounds()
            .expect("a clicked target must have visible bounds");

        assert!(
            label_bounds.x >= button_bounds.x
                && label_bounds.y >= button_bounds.y
                && label_bounds.x + label_bounds.width <= button_bounds.x + button_bounds.width
                && label_bounds.y + label_bounds.height <= button_bounds.y + button_bounds.height,
            "label {label_bounds:?} must fit inside its button {button_bounds:?}"
        );
    }

    // -----------------------------------------------------------------------
    // 4. What it can vary.
    // -----------------------------------------------------------------------

    /// Window size: **yes**, `Simulator::with_size`.
    #[test]
    fn varies_window_size() {
        let mut state = make_state();
        install_one_project(&mut state);

        for size in [Size::new(800.0, 600.0), Size::new(1280.0, 720.0)] {
            let mut ui = Simulator::with_size(
                iced_test::core::Settings::default(),
                size,
                crate::view::dashboard::view(&state, state.width_mode),
            );
            ui.find("Select")
                .unwrap_or_else(|e| panic!("Select must render at {size:?}: {e}"));
        }
    }

    /// Theme: **yes** — a theme is passed to `Simulator::snapshot`
    /// directly (colour only; it does not change layout, so it is not
    /// part of the bounds guard, only the visual one). Demonstrated by
    /// snapshotting the same view under light and dark without either
    /// panicking.
    #[test]
    fn varies_theme() {
        let mut state = make_state();
        install_one_project(&mut state);

        for theme in [
            knotra_ui::KnotraTheme::light(),
            knotra_ui::KnotraTheme::dark(),
        ] {
            let mut ui = Simulator::new(crate::view::dashboard::view(&state, state.width_mode));
            ui.snapshot(&theme.base)
                .unwrap_or_else(|e| panic!("snapshot must succeed for {theme:?}: {e}"));
        }
    }

    /// Language: **yes** — through `state.catalog`, the same thing
    /// `state.t()` reads; the dashboard's own English/Japanese labels
    /// differ in the rendered tree.
    #[test]
    fn varies_language() {
        let mut state = make_state();
        install_one_project(&mut state);
        assert!(
            Simulator::new(crate::view::dashboard::view(&state, state.width_mode))
                .find("Select")
                .is_ok()
        );

        state.catalog = knotra_ui::i18n::Catalog::for_locale(knotra_ui::i18n::Locale::Ja);
        assert!(
            Simulator::new(crate::view::dashboard::view(&state, state.width_mode))
                .find("選択")
                .is_ok(),
            "the Japanese label must render once the catalog is switched"
        );
    }

    /// Zoom: **indirectly** — `Simulator` has no `scale_factor` knob of
    /// its own (it builds a view from an already-resolved logical `Size`,
    /// the same shape `with_size` takes). knotra's own zoom works by
    /// iced's `Application::scale_factor` hook shrinking the *logical*
    /// area a fixed *physical* window offers (`main.rs`'s
    /// `.scale_factor(|state| zoom_percent / 100)`), so the same layout
    /// effect is reproducible here by passing a smaller logical `Size` to
    /// `with_size` directly — 1280×720 physical at 150% zoom is the same
    /// ~853×480 logical area a real window would give the view tree.
    /// There is no single line that "does" zoom the way `with_size` does
    /// window size; it is this arithmetic, applied by the caller.
    #[test]
    fn zoom_is_simulated_by_a_smaller_logical_size() {
        let mut state = make_state();
        install_one_project(&mut state);

        let physical = Size::new(1280.0, 720.0);
        let zoom_percent = 150.0_f32;
        let logical = Size::new(
            physical.width / (zoom_percent / 100.0),
            physical.height / (zoom_percent / 100.0),
        );

        let mut ui = Simulator::with_size(
            iced_test::core::Settings::default(),
            logical,
            crate::view::dashboard::view(&state, state.width_mode),
        );
        ui.find("Select")
            .expect("the dashboard must still render at the zoomed logical size");
    }

    // -----------------------------------------------------------------------
    // 5. Snapshots.
    // -----------------------------------------------------------------------

    /// Two runs of the same view, same theme, give the same hash — proven
    /// directly rather than assumed. Uses the default font
    /// (`Font::with_name("Fira Sans")`, `simulator.rs`'s own fallback when
    /// no font is set) — English text only, so this does not exercise
    /// Japanese glyph shaping at all; see the next, `#[ignore]`d test for
    /// that question.
    #[test]
    fn snapshot_hash_is_stable_across_two_runs() {
        let mut state = make_state();
        install_one_project(&mut state);
        let theme = knotra_ui::KnotraTheme::light();

        // `Snapshot` only exposes a path-based `matches_hash`/`matches_image`
        // (it writes/reads a file, appending "-{renderer}" to the file stem
        // before the extension — both private details this test does not
        // need to know); the SHA-256 it computes internally is likewise
        // private. A fresh, single-file tempdir per call, globbed for
        // whatever `.sha256` file appears in it, reads the same hash back
        // without this test guessing that transform. Neither directory is
        // committed evidence; both are removed after reading.
        let hash_of = || -> String {
            let dir = tempfile::tempdir().expect("tempdir");
            let mut ui = Simulator::new(crate::view::dashboard::view(&state, state.width_mode));
            let snapshot = ui.snapshot(&theme.base).expect("snapshot must succeed");
            snapshot
                .matches_hash(dir.path().join("snapshot"))
                .expect("write the hash file");
            let entry = std::fs::read_dir(dir.path())
                .expect("read the tempdir")
                .find_map(Result::ok)
                .expect("exactly one .sha256 file must have been written");
            std::fs::read_to_string(entry.path()).expect("read it back")
        };

        assert_eq!(
            hash_of(),
            hash_of(),
            "the same view must hash the same way twice"
        );
    }

    /// Whether Japanese renders with the system's CJK font, and whether a
    /// snapshot of it is CI-stable, is an environment question this test
    /// cannot answer portably: CI's image has no CJK font installed (per
    /// this task's own text), so a hash taken here would differ there for
    /// a reason that has nothing to do with knotra's own code. Marked
    /// `#[ignore]` for that reason, runnable locally with
    /// `cargo test -- --ignored snapshot_of_japanese_text`.
    #[test]
    #[ignore = "CJK glyph shaping is environment-dependent (no CJK font on \
                the CI image); this is a local-only check, not a CI gate"]
    fn snapshot_of_japanese_text_is_only_a_local_check() {
        let mut state = make_state();
        install_one_project(&mut state);
        state.catalog = knotra_ui::i18n::Catalog::for_locale(knotra_ui::i18n::Locale::Ja);
        let theme = knotra_ui::KnotraTheme::light();

        let mut ui = Simulator::new(crate::view::dashboard::view(&state, state.width_mode));
        let path = std::env::temp_dir().join("knotra-headless-probe-ja-snapshot");
        let snapshot = ui.snapshot(&theme.base).expect("snapshot must succeed");
        assert!(
            snapshot
                .matches_hash(&path)
                .expect("the hash file must be readable or writable"),
            "the Japanese dashboard snapshot changed — only meaningful on a \
             machine with the same CJK font each time"
        );
    }
}

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

        let mut ui = crate::headless::simulator(
            crate::view::dashboard::view(&state, state.width_mode),
            Size::new(1024.0, 768.0),
        );

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
    /// **Task 107 (snora 0.57):** `AppLayout`'s `on_close_modals` captures
    /// Escape itself and publishes the close message *as a widget
    /// operation inside the view tree* — something the `Simulator` runs,
    /// unlike `app.rs::subscription`'s own `keyboard::listen()`, which the
    /// `Simulator` never drives at all. Before this upgrade (snora 0.53),
    /// this same tap produced zero messages here, for exactly that reason
    /// — the honest, current gap Review 273 §4/Task 107 named. Now it
    /// produces exactly one: `Message::Shortcut(ShortcutMessage::Close)`,
    /// the same message knotra's own subscription-driven Escape sends, so
    /// a real window's behaviour is unchanged.
    #[test]
    fn escape_in_a_dialog_produces_exactly_one_close_message() {
        let mut state = make_state();
        state.confirm_remove_dialog = Some(ConfirmRemoveDialog {
            project_id: knotra_vcs::ProjectId::new(),
            project_name: "Alpha".to_owned(),
        });

        let mut ui =
            crate::headless::simulator(crate::view::app_view(&state), Size::new(1024.0, 768.0));
        let _ = ui.tap_key(keyboard::Key::Named(keyboard::key::Named::Escape));

        let messages: Vec<Message> = ui.into_messages().collect();
        assert_eq!(messages.len(), 1, "{messages:?}");
        assert!(
            matches!(
                messages[0],
                Message::Shortcut(crate::message::ShortcutMessage::Close)
            ),
            "expected a Close message from snora's own on_close_modals; got {messages:?}"
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
        let mut ui = crate::headless::simulator(
            crate::view::dashboard::view(&state, state.width_mode),
            Size::new(1024.0, 768.0),
        );

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
            let mut ui = crate::headless::simulator(
                crate::view::dashboard::view(&state, state.width_mode),
                size,
            );
            ui.find("Select")
                .unwrap_or_else(|e| panic!("Select must render at {size:?}: {e}"));
        }
    }

    /// Theme: **yes** — a theme is passed to `Simulator::snapshot`
    /// directly (colour only; it does not change layout, so it is not
    /// part of the bounds guard, only the visual one). Demonstrated by
    /// snapshotting the same view under all four themes and asserting
    /// their hashes differ pairwise.
    ///
    /// `state.theme` is set alongside the `base` passed to `snapshot`:
    /// knotra's own styles (`widget/buttons.rs`, `chip.rs`, etc.) read
    /// their colours from `state.theme.tokens`, not from the `base`
    /// `iced::Theme` alone — a view built from a `state` whose `theme`
    /// stayed `Light` would snapshot the same tokens every time no matter
    /// which `base` was passed to `snapshot`, the bug Review 275 §2.3
    /// found in this test's own first version.
    ///
    /// **Task 110 §3: rewritten — the hash-equality version above could
    /// not fail on the bug it was named for.** `base` is itself built
    /// from each choice's own tokens (`snora::design::theme(&tokens)`),
    /// so the four snapshot hashes already differed even with the bug in
    /// place (`state.theme` frozen on Light): reproduced directly,
    /// temporarily not setting `state.theme` below, restored after — the
    /// four hashes still came out pairwise distinct. A test that passes
    /// whether or not the thing it names is true is not a regression
    /// guard. This version instead decodes the PNG `Snapshot::
    /// matches_image` writes (via the `png` crate, a dev-dependency added
    /// for exactly this — `Snapshot` exposes no in-memory pixel accessor
    /// of its own) and reads the actual colour of one token-styled
    /// control's background: the "Needs help" chip's own `surface` fill
    /// (`chip.rs`'s `style()`, unselected/`Active` — `to_iced(tokens.
    /// palette.surface)`), sampled a couple of pixels past the label's
    /// own right edge — inside the deliberate gap `chip::filter`'s own
    /// `Space::new().width(tokens.spacing.xs)` leaves before the count
    /// digit, never inside a glyph's own ink. Asserts it equals that
    /// choice's own `tokens.palette.surface`, for all four — which the
    /// four `surface` values (confirmed distinct: light
    /// `#f4f6f8`-ish, dark near-black-blue, HC light pure white, HC dark
    /// pure black) make a genuine discriminator.
    #[test]
    fn varies_theme() {
        fn chip_background_pixel(state: &mut AppState, base: &iced::Theme) -> [u8; 4] {
            let mut ui = crate::headless::simulator(
                crate::view::dashboard::view(state, state.width_mode),
                Size::new(1024.0, 768.0),
            );
            let label_bounds = ui
                .find("Needs help")
                .expect("the chip's own label must be found")
                .bounds();
            let sample_x_logical = label_bounds.x + label_bounds.width + 2.0;
            let sample_y_logical = label_bounds.y + label_bounds.height / 2.0;

            let dir = tempfile::tempdir().expect("tempdir");
            let base_path = dir.path().join("snapshot");
            let snapshot = ui.snapshot(base).expect("snapshot must succeed");
            snapshot.matches_image(&base_path).expect("write the png");
            let png_path = std::fs::read_dir(dir.path())
                .expect("read the tempdir")
                .find_map(Result::ok)
                .expect("exactly one .png file must have been written")
                .path();

            let file = std::fs::File::open(&png_path).expect("open the png");
            let decoder = png::Decoder::new(std::io::BufReader::new(file));
            let mut reader = decoder.read_info().expect("read the png header");
            let mut bytes = vec![0; reader.output_buffer_size().expect("buffer size")];
            let info = reader.next_frame(&mut bytes).expect("decode the png");
            let width = info.width;
            // The simulator renders at the renderer's own backing-store
            // scale, not necessarily 1 physical pixel per logical point
            // (confirmed: a 1024x768 logical window produced a 2048x1536
            // PNG here) — scale the logical sample point by the PNG's own
            // width against the logical window width actually requested,
            // rather than assuming any fixed factor.
            let scale = width as f32 / 1024.0;
            let sample_x = (sample_x_logical * scale).round() as u32;
            let sample_y = (sample_y_logical * scale).round() as u32;
            let idx = (sample_y * width + sample_x) as usize * 4;
            [bytes[idx], bytes[idx + 1], bytes[idx + 2], bytes[idx + 3]]
        }

        fn to_rgba_bytes(color: snora::design::Color) -> [u8; 4] {
            [
                (color.r * 255.0).round() as u8,
                (color.g * 255.0).round() as u8,
                (color.b * 255.0).round() as u8,
                (color.a * 255.0).round() as u8,
            ]
        }

        for choice in [
            knotra_ui::ThemeChoice::Light,
            knotra_ui::ThemeChoice::Dark,
            knotra_ui::ThemeChoice::HighContrastLight,
            knotra_ui::ThemeChoice::HighContrastDark,
        ] {
            let mut state = make_state();
            install_one_project(&mut state);
            state.theme = knotra_ui::KnotraTheme::from_choice(choice);
            let base = state.theme.base.clone();

            let pixel = chip_background_pixel(&mut state, &base);
            let expected = to_rgba_bytes(state.theme.tokens.palette.surface);
            assert_eq!(
                pixel, expected,
                "{choice:?}: the chip's own background pixel {pixel:?} does not match its \
                 theme's tokens.palette.surface {expected:?} — state.theme is not reaching \
                 this control"
            );
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
            crate::headless::simulator(
                crate::view::dashboard::view(&state, state.width_mode),
                Size::new(1024.0, 768.0),
            )
            .find("Select")
            .is_ok()
        );

        state.catalog = knotra_ui::i18n::Catalog::for_locale(knotra_ui::i18n::Locale::Ja);
        assert!(
            crate::headless::simulator(
                crate::view::dashboard::view(&state, state.width_mode),
                Size::new(1024.0, 768.0),
            )
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
        let logical = crate::headless::logical_size_at_zoom(physical, zoom_percent);

        let mut ui = crate::headless::simulator(
            crate::view::dashboard::view(&state, state.width_mode),
            logical,
        );
        ui.find("Select")
            .expect("the dashboard must still render at the zoomed logical size");
    }

    // -----------------------------------------------------------------------
    // 5. Snapshots.
    // -----------------------------------------------------------------------

    /// Two runs of the same view, same theme, give the same hash — proven
    /// directly rather than assumed. Built through `crate::headless::
    /// simulator`, so this uses "Noto Sans" and the icon font, same as the
    /// real application — English text only, so this does not exercise
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
            let mut ui = crate::headless::simulator(
                crate::view::dashboard::view(&state, state.width_mode),
                Size::new(1024.0, 768.0),
            );
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

        let mut ui = crate::headless::simulator(
            crate::view::dashboard::view(&state, state.width_mode),
            Size::new(1024.0, 768.0),
        );
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

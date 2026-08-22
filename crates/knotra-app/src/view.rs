//! View functions for each screen.
//!
//! Each sub-module exports a single `view(state) -> Element<Message>` function.
//! Views are pure — they only read `AppState` and emit `Message`s.
pub mod activity_strip;
pub mod add_project_modal;
pub mod command_palette;
pub mod detail_panel;
pub mod overlays;
pub mod selection_bar;
pub mod shell;
pub mod shortcuts_overlay;
pub mod workspace_manager;

// RFC-037 Stage 1: `bulk_modals` was renamed to `overlays` (the RFC's own
// name for the split-up module). `tests.rs` still calls two `pub(crate)`
// changelog helpers (`changelog_result_counts`, `changelog_markdown_preview`)
// by their old `crate::view::bulk_modals::...` path, and R8 forbids editing
// `tests.rs` in this stage — so this alias keeps that path resolving without
// touching it. `#[cfg(test)]`-gated the same way `app.rs`'s
// `resolve_project_file_path` re-export is, since nothing outside `tests.rs`
// uses this path — not part of this RFC's scope table; flagged in the
// Stage 1 review request, not silently added.
#[cfg(test)]
pub(crate) use overlays as bulk_modals;

pub mod dashboard;
pub mod history;
pub mod settings;

use crate::{message::Message, state::AppState};
use iced::Element;

/// Render the full application layout.
///
/// Layer structure (bottom to top):
///
/// ```text
/// snora::render(AppLayout)          ← skeleton + modal overlays + snora close sinks
///   ├─ header: shell (RFC-034 R12) — workspace switcher, Dashboard/History
///   │          nav, right cluster (status, refresh, palette, settings)
///   ├─ body: (screen + sel_bar + activity_strip + detail_panel)
///   ├─ header_menu: shell::switcher_menu, when open — dismissed by
///   │               on_close_menus, independent of the dialog/sheet group
///   ├─ dialog: workspace-manager (RFC-034 R9) *or* ActiveModal (Pull / Tag /
///   │          Switch / Changelog) — `AppLayout::dialog` is a single slot
///   ├─ sheet: ActiveModal::Resolve
///   └─ on_close_modals: Message::Shortcut(ShortcutMessage::Close)
/// add_project_layer                 ← knotra-specific, own state channel
/// palette_layer                     ← knotra-specific, own state channel
/// shortcuts_layer                   ← knotra-specific, own state channel
/// ```
///
/// Command palette, shortcuts overlay, and add-project modal are pushed as
/// stack layers above `render(layout)` because they have their own state
/// channels and are not standard modal-close-sink overlays (RFC-036 migrates
/// them). Workspace-manager dialogs and the workspace-tab strip moved off
/// this ad hoc stack in RFC-034 — the former renders as an opaque surface
/// through `AppLayout::dialog`, the latter through the shell `header` /
/// `header_menu` slots, so both get the engine's scrim/backdrop and input
/// blocking, which the ad hoc stack never provided.
pub fn app_view(state: &AppState) -> Element<'_, Message> {
    use crate::message::{ShortcutMessage, WorkspaceMessage};
    use crate::state::ActiveModal;
    use iced::Length;
    use iced::widget::{column, container, row, stack};
    use snora::{AppLayout, Dialog, LayoutDirection, Sheet, SheetEdge, SheetSize, render};

    // -----------------------------------------------------------------------
    // Body: screen content + selection bar + activity strip. The persistent
    // workspace-tab strip moved into the shell header (RFC-034 R12/R13).
    //
    // `mode` is `state.width_mode` — read once here and passed to both
    // `dashboard::view` and `selection_bar::view`, so both siblings agree
    // (RFC-035 R8, Handoff 027 Ruling 6.2). **Reversed from a `responsive`
    // measurement to a state field fed by `Message::WindowResized`**
    // (Handoff 029): `focus_order` runs inside `update()`, where no
    // `responsive` closure's `Size` is reachable, so a widget-local
    // measurement could never be seen by keyboard handling. See
    // `width_mode.rs`'s module doc for the full history.
    // -----------------------------------------------------------------------
    let mode = state.width_mode;

    let screen_content: Element<'_, Message> = match state.screen {
        crate::state::Screen::Dashboard => dashboard::view(state, mode),
        crate::state::Screen::History => history::view(state),
        crate::state::Screen::Settings => settings::view(state),
    };

    let sel_bar = selection_bar::view(state, mode);
    let activity = activity_strip::view(state);

    let mut main_col = column![screen_content].height(Length::Fill);
    if let Some(bar) = sel_bar {
        main_col = main_col.push(bar);
    }
    if let Some(strip) = activity {
        main_col = main_col.push(strip);
    }

    // Detail panel (RFC-0014) — horizontally adjacent to main column.
    let body: Element<'_, Message> = if state.detail_panel.open_project_id.is_some() {
        if let Some(panel) = detail_panel::view(state) {
            row![main_col, panel].into()
        } else {
            main_col.into()
        }
    } else {
        main_col.into()
    };

    // -----------------------------------------------------------------------
    // snora AppLayout: skeleton + standard modal overlays (RFC-0013).
    //
    // `AppLayout::dialog` is a single slot (last write wins), so at most one
    // dialog builder below may run. Workspace-manager dialogs (RFC-034 R9,
    // the overlay-host validating migration) take priority over `ActiveModal`
    // dialogs, matching `close_topmost_layer`'s existing branch order in
    // `app.rs` (workspace_mgr dialogs are checked, and so close, before the
    // active_modal cases) — rendering priority and close priority agree.
    //
    // ActiveModal variants map to snora overlay slots:
    //   Centred dialogs  → AppLayout::dialog(Dialog::new(el))
    //   Right-docked panel → AppLayout::sheet(Sheet::new(el).at(SheetEdge::End))
    //
    // on_close_modals dispatches ShortcutMessage::Close. The update layer
    // closes exactly one topmost visible layer per close action.
    // -----------------------------------------------------------------------
    let mut layout = AppLayout::new(body)
        .header(shell::view(state))
        .direction(LayoutDirection::Ltr)
        .on_close_modals(Message::Shortcut(ShortcutMessage::Close));

    // Switcher dropdown: a `header_menu` layer, distinct from the
    // dialog/sheet modal group above. Dismissed by `on_close_menus`
    // (click-outside) or by choosing a menu item — never by
    // `close_topmost_layer`'s Escape-driven stack (RFC-034 R11 leaves that
    // function's branch ordering untouched; this menu is not part of it).
    if let Some(menu) = shell::switcher_menu(state) {
        layout = layout
            .header_menu(menu)
            .on_close_menus(Message::Workspace(WorkspaceMessage::SwitcherToggled));
    }

    if let Some(el) = workspace_manager::view(state) {
        layout = layout.dialog(Dialog::new(el));
    } else {
        match &state.active_modal {
            ActiveModal::None => {}

            ActiveModal::Pull => {
                let el: Element<'_, Message> = overlays::pull_modal(state);
                layout = layout.dialog(Dialog::new(el));
            }
            ActiveModal::Tag => {
                let el: Element<'_, Message> = overlays::tag_modal(state);
                layout = layout.dialog(Dialog::new(el));
            }
            ActiveModal::Switch => {
                let el: Element<'_, Message> = overlays::switch_modal(state);
                layout = layout.dialog(Dialog::new(el));
            }
            ActiveModal::Changelog => {
                let el: Element<'_, Message> = overlays::changelog_modal(state);
                layout = layout.dialog(Dialog::new(el));
            }
            ActiveModal::Resolve(pid) => {
                // Right-docked resolve panel → snora Sheet anchored to the End edge.
                //
                // RFC-058 D3: `Sheet` exposes only `new`/`at`/`with_size`
                // (`snora-core-0.38.0/src/overlay.rs:206-227`) — no style
                // hook. Its own drawn edge (fill `background.base`, 1px
                // border `background.weak`) measures 1.29–1.35:1, well
                // under the 3:1 SC 1.4.11 floor, and knotra cannot change
                // it. The content passed here — `resolve_panel`, whose
                // outermost element is `overlay::surface` — is what
                // supplies the panel's only perceivable, compliant edge.
                // See `overlay.rs::surface`'s own comment for why that
                // border is load-bearing, not decorative.
                let el: Element<'_, Message> = overlays::resolve_panel(state, pid);
                layout = layout.sheet(Sheet::new(el).at(SheetEdge::End).with_size(SheetSize::Half));
            }
        }
    }

    let snora_layer: Element<'_, Message> = render(layout);

    // -----------------------------------------------------------------------
    // Knotra-specific overlays (own state channels; not wired to
    // on_close_modals — they are pushed as iced stack layers above snora's
    // layer composition). RFC-034 non-goal: migrating these is RFC-036.
    // -----------------------------------------------------------------------
    let palette_layer: Option<Element<'_, Message>> = if state.palette.open {
        Some(
            container(command_palette::view(state))
                .center(Length::Fill)
                .into(),
        )
    } else {
        None
    };

    let add_project_layer: Option<Element<'_, Message>> =
        add_project_modal::view(state).map(|m| container(m).center(Length::Fill).into());

    let shortcuts_layer: Option<Element<'_, Message>> = if state.keyboard.cheat_sheet_open {
        Some(
            container(shortcuts_overlay::view(state))
                .center(Length::Fill)
                .into(),
        )
    } else {
        None
    };

    if palette_layer.is_some() || add_project_layer.is_some() || shortcuts_layer.is_some() {
        let mut layers: Vec<Element<'_, Message>> = vec![snora_layer];
        if let Some(a) = add_project_layer {
            layers.push(a);
        }
        if let Some(p) = palette_layer {
            layers.push(p);
        }
        if let Some(s) = shortcuts_layer {
            layers.push(s);
        }
        stack(layers).into()
    } else {
        snora_layer
    }
}

/// The plain-language label for an operation kind — moved here from
/// `activity_strip.rs` (RFC-038 Stage 4 §1a) so `history.rs` can reuse the
/// same mapping instead of writing a second `match` over the same six
/// variants, which is exactly the parallel-systems shape RFC-034 R7 exists
/// to prevent. `view.rs` is the shared ancestor of both submodules, so
/// `super::operation_kind_label` resolves from either without a new module.
pub(crate) fn operation_kind_label<'a>(
    state: &'a AppState,
    kind: &knotra_vcs::model::operation::OperationKind,
) -> &'a str {
    use knotra_vcs::model::operation::OperationKind;
    state.t(match kind {
        OperationKind::StatusRefresh => "plain.activity.kind_refresh",
        OperationKind::Fetch => "plain.activity.kind_fetch",
        OperationKind::SmartPull => "plain.activity.kind_smart_pull",
        OperationKind::ContextSwitch => "plain.activity.kind_context_switch",
        OperationKind::Freeze => "plain.activity.kind_freeze",
        OperationKind::FreezeRollback => "plain.activity.kind_freeze_rollback",
    })
}

/// Maps a persisted `skip_reason` (RFC-046 D1: a stable `RetryExclusionReason`
/// code) through the catalog for on-screen display, falling back to the
/// stored value verbatim when it is not a recognised code — a value written
/// before that contract was enforced, RFC-046 D4's deliberate forward/backward
/// compatibility, not an error case. Moved here (from `history.rs`, its only
/// caller before RFC-046 A1/D6/R10) so `overlays/smart_pull.rs` can share this
/// one mapping too, rather than writing a second copy that must stay
/// identical to the first — the same parallel-systems shape
/// `operation_kind_label` above exists to prevent (RFC-034 R7), now also the
/// case RFC-038's `label_en` pairing needed a guard for (RFC-046 A1 R11).
pub(crate) fn skip_reason_display<'a>(state: &'a AppState, reason: &'a str) -> &'a str {
    knotra_vcs::model::operation::RetryExclusionReason::from_code(reason)
        .map(|reason| state.t(reason.i18n_key()))
        .unwrap_or(reason)
}

//! View functions for each screen.
//!
//! Each sub-module exports a single `view(state) -> Element<Message>` function.
//! Views are pure — they only read `AppState` and emit `Message`s.
pub mod activity_strip;
pub mod add_project_modal;
pub mod command_palette;
pub mod detail_panel;
pub mod overlays;
pub mod remove_project_dialog;
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
///   ├─ dialog: one of the twelve non-sheet `Layer`s — `AppLayout::dialog`
///   │          is a single slot
///   ├─ sheet: `Layer::Resolve`
///   └─ on_close_modals: Message::Shortcut(ShortcutMessage::Close)
/// ```
///
/// Task 098 Amendment A2: every window — including the command palette,
/// Add project and the shortcuts sheet, previously pushed as their own
/// centred `iced::widget::stack` layers above `render(layout)` — now goes
/// through `AppLayout::dialog`/`sheet`, so every one of them gets the
/// engine's own opaque surface, full-window dim, and scrim click-to-close.
/// `dialog`'s content is chosen from `layer::top_layer(state)` through one
/// exhaustive `match` with no `_` arm (below), rather than the old
/// `if`/`else` chain plus three independent `Option` flags — the window
/// drawn and the window `top_layer` (and so the keyboard) considers open
/// can no longer disagree, and a thirteenth `Layer` variant fails this
/// file to compile until it is given a row here. `WorkspaceMenu` stays in
/// `header_menu`, unchanged — it was never part of the dialog/sheet group.
pub fn app_view(state: &AppState) -> Element<'_, Message> {
    use crate::message::{ShortcutMessage, WorkspaceMessage};
    use crate::state::ActiveModal;
    use crate::state::layer::{self, Layer};
    use iced::Length;
    use iced::widget::{column, row};
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
    // `AppLayout::dialog` is a single slot (last write wins). Task 098
    // Amendment A2: its content is chosen from `layer::top_layer(state)`
    // through one exhaustive `match`, which already encodes the full
    // priority order every window's close site and focus order already
    // agree on (`state/layer.rs`'s own doc comment) — so this match needs
    // no separate priority logic of its own, unlike the `if`/`else` chain
    // it replaces. `Resolve` is the one variant that maps to `sheet`
    // (right-docked) instead of `dialog` (centred); every other variant
    // maps to `dialog` or to nothing (`None`, `WorkspaceMenu` — the
    // latter stays in `header_menu`, set separately below).
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

    let top_layer = layer::top_layer(state);
    match top_layer {
        None | Some(Layer::WorkspaceMenu) => {}
        Some(Layer::ShortcutsSheet) => {
            let el: Element<'_, Message> = shortcuts_overlay::view(state);
            layout = layout.dialog(Dialog::new(el));
        }
        Some(Layer::Palette) => {
            let el: Element<'_, Message> = command_palette::view(state);
            layout = layout.dialog(Dialog::new(el));
        }
        Some(Layer::AddProject) => {
            if let Some(el) = add_project_modal::view(state) {
                layout = layout.dialog(Dialog::new(el));
            }
        }
        Some(Layer::WorkspaceCreate)
        | Some(Layer::WorkspaceRename)
        | Some(Layer::WorkspaceDelete) => {
            if let Some(el) = workspace_manager::view(state) {
                layout = layout.dialog(Dialog::new(el));
            }
        }
        Some(Layer::GetLatest) => {
            let el: Element<'_, Message> = overlays::pull_modal(state);
            layout = layout.dialog(Dialog::new(el));
        }
        Some(Layer::SaveReleasePoint) => {
            let el: Element<'_, Message> = overlays::tag_modal(state);
            layout = layout.dialog(Dialog::new(el));
        }
        Some(Layer::ChangeWorkArea) => {
            let el: Element<'_, Message> = overlays::switch_modal(state);
            layout = layout.dialog(Dialog::new(el));
        }
        Some(Layer::GenerateNotes) => {
            let el: Element<'_, Message> = overlays::changelog_modal(state);
            layout = layout.dialog(Dialog::new(el));
        }
        Some(Layer::Resolve(_)) => {
            // Right-docked resolve panel → snora Sheet anchored to the End edge.
            //
            // RFC-058 D3: `Sheet` exposes only `new`/`at`/`with_size`
            // (`snora-core-0.52.0::overlay::Sheet`) — no style
            // hook. Its own drawn edge (fill `background.base`, 1px
            // border `background.weak`) measures 1.29–1.35:1, well
            // under the 3:1 SC 1.4.11 floor, and knotra cannot change
            // it. The content passed here — `resolve_panel`, whose
            // outermost element is `overlay::surface` — is what
            // supplies the panel's only perceivable, compliant edge.
            // See `overlay.rs::surface`'s own comment for why that
            // border is load-bearing, not decorative.
            //
            // `pid` is read back from `state.active_modal`, not from the
            // `top_layer` match above, so it borrows from `state` (the
            // lifetime `resolve_panel` and this function's return type
            // both need) rather than from a local owned by this function.
            if let ActiveModal::Resolve(ref pid) = state.active_modal {
                let el: Element<'_, Message> = overlays::resolve_panel(state, pid);
                layout = layout.sheet(Sheet::new(el).at(SheetEdge::End).with_size(SheetSize::Half));
            }
        }
        Some(Layer::RemoveProject) => {
            if let Some(el) = remove_project_dialog::view(state) {
                layout = layout.dialog(Dialog::new(el));
            }
        }
    }

    render(layout)
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

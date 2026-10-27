//! RFC-066 D1: the single list of windows knotra can show, and which one is
//! drawn on top right now.

use super::{ActiveModal, AppState};

/// Every window (dialog, sheet, menu) knotra can show, in one list. Keyboard
/// routing, the focus outline and Escape all read [`top_layer`] through
/// **exhaustive** matches, so a new window cannot be added without a focus
/// order and a close path (RFC-066 D1, R1).
///
/// `Resolve` carries the project id (the sheet is keyed to one project);
/// nothing else carries data — `top_layer` reads whichever single field
/// identifies each other window directly from [`AppState`] instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layer {
    ShortcutsSheet,
    Palette,
    AddProject,
    WorkspaceCreate,
    WorkspaceRename,
    WorkspaceDelete,
    RemoveProject,
    WorkspaceMenu,
    GetLatest,
    SaveReleasePoint,
    GenerateNotes,
    ChangeWorkArea,
    Resolve(knotra_vcs::ProjectId),
}

/// The window drawn on top right now, or `None` if none is open — derived
/// from `view.rs`'s own composition order, outermost first:
///
/// 1. `view.rs::app_view`'s outer `stack`, built last-pushed-is-topmost:
///    the shortcuts-sheet layer (`view.rs` ~206-214), then the palette
///    layer (~193-201), then the add-project layer (~203-204) — the add-
///    project layer is pushed first of the three, so it sits *under* the
///    other two in this stack, matching the push order in `app_view`.
/// 2. Inside `snora::render`'s own composition
///    (`snora/src/render.rs::render_with_style`, "layers 4-6" vs
///    "layers 1-3"): the `dialog`/`sheet` slot draws **above**
///    `header_menu`. `view.rs:145-184` fills `dialog`/`sheet` with
///    *either* a workspace-manager dialog (checked first, an `if`) *or*
///    one of `ActiveModal`'s overlays (the `else` branch) — mutually
///    exclusive by construction, so at most one of
///    `WorkspaceCreate`/`WorkspaceRename`/`WorkspaceDelete`/`GetLatest`/
///    `SaveReleasePoint`/`ChangeWorkArea`/`GenerateNotes`/`Resolve` is ever
///    the layer. A workspace-manager dialog open also means the `else`
///    branch — the only place that sets `sheet` for `Resolve` — never
///    runs, so a workspace dialog also suppresses an open `Resolve` sheet
///    entirely, not only the `dialog` slot.
/// 3. `header_menu` (`view.rs:139-143`, the workspace switcher). Below the
///    modals above, not above them — unlike the pre-RFC-066 Escape
///    handling's comment, which called the switcher "the topmost visible
///    layer when open". Never actually exercised either way: every site
///    that opens a workspace-manager dialog clears `switcher_open` first
///    (`app/workspace.rs:241,300,369`), so a workspace dialog and the
///    switcher are never both open to begin with.
/// 4. `confirm_remove_dialog` — drawn inline in the dashboard, not yet a
///    real overlay layer (RFC-066 Stage 3 moves it). Kept last here,
///    matching its pre-RFC-066 close priority (the old `close_topmost_layer`
///    only checked it once `active_modal` was `None`).
pub fn top_layer(state: &AppState) -> Option<Layer> {
    if state.keyboard.cheat_sheet_open {
        return Some(Layer::ShortcutsSheet);
    }
    if state.palette.open {
        return Some(Layer::Palette);
    }
    if state.add_project_dialog.is_some() {
        return Some(Layer::AddProject);
    }
    if state.workspace_mgr.create_dialog.is_some() {
        return Some(Layer::WorkspaceCreate);
    }
    if state.workspace_mgr.rename_dialog.is_some() {
        return Some(Layer::WorkspaceRename);
    }
    if state.workspace_mgr.confirm_delete.is_some() {
        return Some(Layer::WorkspaceDelete);
    }
    match &state.active_modal {
        ActiveModal::Pull => return Some(Layer::GetLatest),
        ActiveModal::Tag => return Some(Layer::SaveReleasePoint),
        ActiveModal::Switch => return Some(Layer::ChangeWorkArea),
        ActiveModal::Changelog => return Some(Layer::GenerateNotes),
        ActiveModal::Resolve(id) => return Some(Layer::Resolve(id.clone())),
        ActiveModal::None => {}
    }
    if state.workspace_mgr.switcher_open {
        return Some(Layer::WorkspaceMenu);
    }
    if state.confirm_remove_dialog.is_some() {
        return Some(Layer::RemoveProject);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    fn state() -> AppState {
        AppState::new(AppConfig::default())
    }

    #[test]
    fn nothing_open_has_no_top_layer() {
        assert_eq!(top_layer(&state()), None);
    }

    #[test]
    fn cheat_sheet_outranks_everything_below_it() {
        let mut s = state();
        s.keyboard.cheat_sheet_open = true;
        s.palette.open = true;
        s.active_modal = ActiveModal::Pull;
        assert_eq!(top_layer(&s), Some(Layer::ShortcutsSheet));
    }

    #[test]
    fn palette_outranks_add_project_and_active_modal() {
        let mut s = state();
        s.palette.open = true;
        s.add_project_dialog = Some(crate::state::AddProjectDialog::default());
        s.active_modal = ActiveModal::Pull;
        assert_eq!(top_layer(&s), Some(Layer::Palette));
    }

    #[test]
    fn add_project_outranks_workspace_dialogs_and_active_modal() {
        let mut s = state();
        s.add_project_dialog = Some(crate::state::AddProjectDialog::default());
        s.workspace_mgr.create_dialog = Some(Default::default());
        s.active_modal = ActiveModal::Pull;
        assert_eq!(top_layer(&s), Some(Layer::AddProject));
    }

    #[test]
    fn workspace_create_outranks_active_modal() {
        let mut s = state();
        s.workspace_mgr.create_dialog = Some(Default::default());
        s.active_modal = ActiveModal::Tag;
        assert_eq!(top_layer(&s), Some(Layer::WorkspaceCreate));
    }

    #[test]
    fn workspace_dialog_suppresses_an_open_resolve_sheet() {
        // The `else` branch that sets `layout.sheet` for `Resolve` never
        // runs while `workspace_manager::view` returns `Some` — matching
        // that, not just picking the `dialog` slot over the `sheet` slot.
        let mut s = state();
        s.workspace_mgr.rename_dialog = Some(crate::state::workspace_mgr::RenameWorkspaceDialog {
            new_name: "x".to_owned(),
            error: None,
        });
        s.active_modal = ActiveModal::Resolve(knotra_vcs::ProjectId::new());
        assert_eq!(top_layer(&s), Some(Layer::WorkspaceRename));
    }

    #[test]
    fn each_active_modal_variant_maps_to_its_own_layer() {
        let mut s = state();
        s.active_modal = ActiveModal::Pull;
        assert_eq!(top_layer(&s), Some(Layer::GetLatest));
        s.active_modal = ActiveModal::Tag;
        assert_eq!(top_layer(&s), Some(Layer::SaveReleasePoint));
        s.active_modal = ActiveModal::Switch;
        assert_eq!(top_layer(&s), Some(Layer::ChangeWorkArea));
        s.active_modal = ActiveModal::Changelog;
        assert_eq!(top_layer(&s), Some(Layer::GenerateNotes));
        let id = knotra_vcs::ProjectId::new();
        s.active_modal = ActiveModal::Resolve(id.clone());
        assert_eq!(top_layer(&s), Some(Layer::Resolve(id)));
    }

    #[test]
    fn active_modal_outranks_the_switcher_and_remove_project() {
        let mut s = state();
        s.active_modal = ActiveModal::Pull;
        s.workspace_mgr.switcher_open = true;
        s.confirm_remove_dialog = Some(crate::state::ConfirmRemoveDialog {
            project_id: knotra_vcs::ProjectId::new(),
            project_name: "svc".to_owned(),
        });
        assert_eq!(top_layer(&s), Some(Layer::GetLatest));
    }

    #[test]
    fn switcher_outranks_remove_project() {
        let mut s = state();
        s.workspace_mgr.switcher_open = true;
        s.confirm_remove_dialog = Some(crate::state::ConfirmRemoveDialog {
            project_id: knotra_vcs::ProjectId::new(),
            project_name: "svc".to_owned(),
        });
        assert_eq!(top_layer(&s), Some(Layer::WorkspaceMenu));
    }

    #[test]
    fn remove_project_is_the_last_resort() {
        let mut s = state();
        s.confirm_remove_dialog = Some(crate::state::ConfirmRemoveDialog {
            project_id: knotra_vcs::ProjectId::new(),
            project_name: "svc".to_owned(),
        });
        assert_eq!(top_layer(&s), Some(Layer::RemoveProject));
    }
}

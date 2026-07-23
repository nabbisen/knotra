//! Minimal i18n support for knotra.
//!
//! All user-visible strings are routed through this module so that locale
//! support can be expanded in a later phase without touching every view file.
//!
//! Currently supported locales: `en` (English), `ja` (Japanese).

use std::collections::HashMap;

/// Supported UI locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum Locale {
    #[default]
    En,
    Ja,
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Locale::En => write!(f, "English"),
            Locale::Ja => write!(f, "日本語"),
        }
    }
}

/// A translation key.
pub type Key = &'static str;

/// Catalog holds all translations for the active locale.
pub struct Catalog {
    locale: Locale,
    strings: HashMap<Key, &'static str>,
}

impl Catalog {
    pub fn for_locale(locale: Locale) -> Self {
        let strings = match locale {
            Locale::En => en_strings(),
            Locale::Ja => ja_strings(),
        };
        Catalog { locale, strings }
    }

    /// Look up a string by key, falling back to the key itself.
    pub fn t(&self, key: Key) -> &'static str {
        self.strings.get(key).copied().unwrap_or(key)
    }

    pub fn locale(&self) -> Locale {
        self.locale
    }
}

fn en_strings() -> HashMap<Key, &'static str> {
    let mut m = HashMap::new();
    // Navigation
    m.insert("nav.dashboard", "Dashboard");
    m.insert("nav.sync", "Sync");
    m.insert("nav.context", "Context");
    m.insert("nav.freezer", "Freezer");
    m.insert("nav.history", "History");
    m.insert("nav.settings", "Settings");
    // Dashboard header
    m.insert("dashboard.title", "Workspace Dashboard");
    m.insert("dashboard.refresh", "Refresh");
    m.insert("dashboard.bulk_sync", "Bulk Sync ▾");
    m.insert("dashboard.filter", "Filter");
    m.insert("dashboard.group_by", "Group by");
    m.insert("dashboard.search_placeholder", "Search projects…");
    m.insert("dashboard.no_projects", "No projects registered.");
    m.insert("dashboard.add_project", "Add Project");
    m.insert("dashboard.last_updated", "Updated");
    m.insert("dashboard.refreshing_count", "Refreshing…");
    m.insert("dashboard.no_workspace", "No workspace");
    m.insert("dashboard.grouping", "Group");
    m.insert("dashboard.grouping.attention", "Needs help");
    m.insert("dashboard.grouping.project_group", "Project group");
    m.insert("dashboard.grouping.none", "None");
    m.insert("dashboard.sorting", "Sort");
    m.insert("dashboard.sorting.recommended", "Needs help first");
    m.insert("dashboard.sorting.name", "Name A-Z");
    m.insert("dashboard.clear_filters", "Clear filters");
    m.insert("dashboard.all_projects", "All projects");
    m.insert("dashboard.work_area_unknown", "Work area unavailable");
    m.insert("dashboard.resolve", "Choose resolution");
    m.insert(
        "dashboard.load_failed",
        "Project status could not be loaded.",
    );
    m.insert(
        "dashboard.no_workspace_error",
        "There is no workspace to check.",
    );
    m.insert("dashboard.try_again", "Try again");
    m.insert("dashboard.create_workspace", "Create workspace");
    m.insert(
        "dashboard.preference_save_failed",
        "Dashboard preferences could not be saved.",
    );
    m.insert("dashboard.cause.missing_path", "Project folder is missing");
    m.insert("dashboard.cause.conflict", "A resolution choice is needed");
    m.insert(
        "dashboard.cause.conflict_detection_unavailable",
        "Resolution status is unavailable",
    );
    m.insert(
        "dashboard.cause.read_unavailable",
        "Project status is unavailable",
    );
    m.insert(
        "dashboard.cause.detached_context",
        "Work area is not attached",
    );
    m.insert(
        "dashboard.cause.status_unknown",
        "Project status is not known yet",
    );
    m.insert("dashboard.progress.uncommitted", "Unsaved files");
    m.insert("dashboard.progress.untracked", "New files");
    m.insert("dashboard.progress.ahead", "Local commits");
    m.insert("dashboard.progress.behind", "Updates available");
    // Add-project dialog
    m.insert("dialog.add_project.title", "Add Project");
    m.insert("dialog.add_project.name_label", "Display name");
    m.insert("dialog.add_project.path_label", "Repository path");
    m.insert("dialog.add_project.name_hint", "My Service");
    m.insert(
        "dialog.add_project.path_hint",
        "/home/user/repos/my-service",
    );
    m.insert("dialog.add_project.confirm", "Add");
    m.insert("dialog.add_project.cancel", "Cancel");
    m.insert(
        "dialog.add_project.error_empty",
        "Name and path are required.",
    );
    // Status labels
    m.insert("status.healthy", "Synced");
    m.insert("status.behind", "Behind");
    m.insert("status.ahead", "Ahead");
    m.insert("status.dirty", "Uncommitted");
    m.insert("status.conflict", "Conflict");
    m.insert("status.unknown", "Unknown");
    m.insert("status.refreshing", "Refreshing…");
    m.insert("status.error", "Error");
    // Filter chip labels
    m.insert("filter.all", "All");
    m.insert("filter.healthy", "Synced");
    m.insert("filter.behind", "Behind");
    m.insert("filter.ahead", "Ahead");
    m.insert("filter.dirty", "Uncommitted");
    m.insert("filter.conflict", "Conflict");
    m.insert("filter.error", "Error");
    m.insert("filter.all_set", "All set");
    m.insert("filter.behind", "Updates available");
    m.insert("filter.ahead", "Local commits");
    m.insert("filter.dirty", "Unsaved work");
    m.insert("filter.conflict", "Needs a choice");
    m.insert("filter.needs_help", "Needs help");
    // Group labels
    m.insert("group.all", "(All groups)");
    m.insert("group.ungrouped", "(Ungrouped)");
    // Card fields
    m.insert("card.context", "Context");
    m.insert("card.vcs", "VCS");
    m.insert("card.ahead", "Ahead");
    m.insert("card.behind", "Behind");
    m.insert("card.uncommitted", "Uncommitted");
    m.insert("card.untracked", "Untracked");
    m.insert("card.conflict", "Conflict");
    m.insert("card.updated", "Updated");
    // Card actions
    m.insert("card.action.fetch", "Fetch");
    m.insert("card.action.remove", "Remove");
    // Actions
    m.insert("action.fetch", "Fetch");
    m.insert("action.pull", "Pull");
    m.insert("action.switch_context", "Switch Context");
    m.insert("action.open_freezer", "Open Freezer");
    m.insert("action.confirm", "Confirm");
    m.insert("action.cancel", "Cancel");
    m.insert("action.retry", "Retry");
    m.insert("action.copy_log", "Copy Log");
    m.insert("action.close", "Close");
    m.insert("plain.activity.details", "Details");
    m.insert("plain.activity.succeeded", "succeeded");
    m.insert("plain.activity.failed", "failed");
    m.insert("plain.activity.skipped", "skipped");
    m.insert("plain.activity.retry_failed_fetches", "Retry failed checks");
    m.insert("plain.activity.review_retry", "Review retry");
    m.insert(
        "plain.activity.busy",
        "Wait for the current operation to finish.",
    );
    m.insert(
        "plain.activity.none_available",
        "These projects are no longer available in this workspace.",
    );
    m.insert("plain.activity.retrying_fetch", "Retrying failed checks");
    m.insert(
        "plain.activity.retry_context_again",
        "Open Change work area again.",
    );
    m.insert(
        "plain.activity.retry_freeze_again",
        "Validate the release point again.",
    );
    m.insert(
        "plain.activity.retry_refresh_again",
        "Use Refresh to check again.",
    );
    m.insert(
        "plain.activity.log_save_failed",
        "The result is visible, but History could not be saved.",
    );
    m.insert(
        "plain.activity.excluded_workspace",
        "Not in the active workspace",
    );
    m.insert(
        "plain.activity.excluded_missing",
        "Project folder is missing",
    );
    m.insert(
        "plain.activity.excluded_unsupported",
        "Not a supported repository",
    );
    m.insert("plain.activity.excluded_status", "Status is unavailable");
    m.insert("plain.activity.kind_refresh", "Refresh");
    m.insert("plain.activity.kind_fetch", "Check for updates");
    m.insert("plain.activity.kind_smart_pull", "Get latest");
    m.insert("plain.activity.kind_context_switch", "Change work area");
    m.insert("plain.activity.kind_freeze", "Record release point");
    m.insert("plain.activity.kind_freeze_rollback", "Undo release point");
    m.insert(
        "plain.activity.retry_preparing",
        "Refreshing project status for retry...",
    );
    m.insert(
        "plain.activity.retry_prepare_failed",
        "Could not refresh project status.",
    );
    // Keyboard shortcuts hint
    m.insert("shortcut.refresh", "Ctrl+R  Refresh");
    m.insert("shortcut.context", "Ctrl+K  Context");
    m.insert("shortcut.freezer", "Ctrl+T  Freezer");
    m.insert("shortcut.search", "Ctrl+/  Search");
    // Errors
    m.insert("error.read_failed", "Failed to read repository status.");
    m.insert("error.no_repo", "No Git or jj repository found.");
    // Confirm remove
    m.insert("confirm.remove_project", "Remove project from workspace?");
    m.insert("confirm.remove_yes", "Remove");
    m.insert("confirm.remove_no", "Keep");

    // --- Plain-language layer (UX review) -----------------------------------
    // First-level wording for non-technical users. Expert terms (Fetch, Pull,
    // Tag, Conflict, …) remain available inside "Show details" via the keys
    // above, but the primary interface uses goal-oriented language.
    m.insert("tier.needs_attention", "Needs help");
    m.insert(
        "tier.needs_attention.hint",
        "These projects need your choice before continuing.",
    );
    m.insert("tier.active", "In progress");
    m.insert(
        "tier.active.hint",
        "These projects have work or changes waiting.",
    );
    m.insert("tier.clean", "All set");
    m.insert(
        "tier.clean.hint",
        "These projects need no action right now.",
    );

    m.insert("plain.check_now", "Check now");
    m.insert("plain.check_for_updates", "Check for updates");
    m.insert("plain.get_latest", "Get latest safely");
    m.insert("plain.save_release_point", "Save release point");
    m.insert("plain.change_work_area", "Change work area");
    m.insert("plain.show_what_happened", "Show what happened");
    m.insert("plain.show_details", "Show details");
    m.insert("plain.hide_details", "Hide details");
    m.insert("plain.exit_selection", "Exit selection");
    m.insert("plain.selection.enter", "Select");
    m.insert("plain.select_visible_projects", "Select visible projects");
    m.insert("plain.selection.none", "No projects selected");
    m.insert("plain.selection.selected_suffix", "selected");
    m.insert(
        "plain.selection.no_visible_projects",
        "No projects match this view.",
    );
    m.insert(
        "plain.selection.none_fetchable",
        "None of the selected projects can be checked right now.",
    );
    m.insert(
        "plain.selection.choose_one_work_area",
        "Choose one project to change work area.",
    );
    m.insert(
        "plain.fetch.skipped_unavailable",
        "This project cannot be checked right now.",
    );

    m.insert("plain.status.all_set", "All set");
    m.insert("plain.status.unsaved_work", "Unsaved work");
    m.insert("plain.status.needs_choice", "Needs your choice");
    m.insert("plain.status.not_sure", "Not sure yet");
    m.insert("plain.status.checking", "Checking…");
    m.insert("plain.status.behind", "Updates available");
    m.insert("plain.status.ahead", "Unshared changes");

    m.insert("plain.disabled.choose_one", "Choose at least one project.");
    m.insert(
        "plain.disabled.no_upstream",
        "These projects have nowhere to get updates from.",
    );
    m.insert(
        "plain.error.path_missing",
        "We cannot find this project folder.",
    );
    m.insert(
        "plain.error.no_repo",
        "This folder does not look like a project knotra can check.",
    );

    // --- Modal flows (Phase 2-4) -------------------------------------------
    m.insert("plain.project", "Project");
    m.insert("plain.what_will_happen", "What will happen");
    m.insert("plain.note", "Note");
    m.insert("plain.of", "of");
    m.insert("plain.waiting", "Waiting…");
    m.insert("plain.needs_help", "Needs help");
    m.insert("plain.no_next_step", "No next step needed.");

    // Get latest safely (Smart Pull)
    m.insert("plain.get_latest.preparing", "Preparing a safe plan…");
    m.insert(
        "plain.get_latest.preparing_hint",
        "This usually takes a few seconds.",
    );
    m.insert(
        "plain.get_latest.review_heading",
        "Review the plan before anything changes.",
    );
    m.insert("plain.get_latest.start", "Start getting latest");
    m.insert("plain.get_latest.working", "Getting latest…");
    m.insert("plain.get_latest.action_get", "Get latest");
    m.insert("plain.get_latest.action_check", "Check only");
    m.insert("plain.get_latest.action_get_anyway", "Get latest anyway");
    m.insert("plain.get_latest.action_skip", "Skip");
    m.insert("plain.get_latest.check_only", "Check only");
    m.insert("plain.get_latest.get_anyway", "Get anyway");
    m.insert(
        "plain.get_latest.note_unsaved",
        "Has unsaved work — check only by default.",
    );
    m.insert(
        "plain.get_latest.note_save_restore",
        "Will save work, get latest, then restore.",
    );
    m.insert(
        "plain.get_latest.note_needs_choice",
        "Needs your choice — skipped until resolved.",
    );
    m.insert(
        "plain.get_latest.note_no_upstream",
        "No update source is configured.",
    );
    m.insert(
        "plain.get_latest.note_not_selected",
        "Not selected for this run.",
    );
    m.insert(
        "plain.get_latest.note_status_missing",
        "Status is not available — skipped.",
    );
    m.insert(
        "plain.get_latest.note_project_not_found",
        "Project was not found — skipped.",
    );
    m.insert("plain.get_latest.done_row", "Done");
    m.insert("plain.get_latest.needs_help_row", "Needs help");
    m.insert("plain.get_latest.skipped_row", "Skipped");
    m.insert("plain.get_latest.all_done_prefix", "All");
    m.insert(
        "plain.get_latest.all_done_suffix",
        "projects are up to date.",
    );
    m.insert("plain.get_latest.done_count", "done.");
    m.insert("plain.get_latest.needs_help_count", "need help.");
    m.insert("plain.get_latest.skipped_count", "skipped.");
    m.insert(
        "plain.get_latest.review_help_rows",
        "Review the highlighted rows before continuing.",
    );

    // Save release point (Freezer / Tag)
    m.insert("plain.release.name_label", "Release name");
    m.insert("plain.release.name_hint", "v1.2.3");
    m.insert(
        "plain.release.name_invalid",
        "Use letters, numbers, dots, dashes, or underscores.",
    );
    m.insert("plain.release.note_label", "Note for later (optional)");
    m.insert("plain.release.note_hint", "");
    m.insert("plain.release.check_readiness", "Check readiness");
    m.insert("plain.release.checking", "Checking…");
    m.insert("plain.release.ready_check", "Ready check");
    m.insert("plain.release.row_ready", "Ready");
    m.insert("plain.release.row_excluded", "Not included");
    m.insert("plain.release.fix_one", "Fix 1 item before saving.");
    m.insert(
        "plain.release.fix_some",
        "Fix highlighted items before saving.",
    );
    m.insert("plain.release.saving", "Saving release point…");
    m.insert(
        "plain.release.saving_hint",
        "This saves atomically — either all succeed or none do.",
    );
    m.insert("plain.release.outcome_success", "Release point saved.");
    m.insert(
        "plain.release.outcome_undone",
        "We stopped and undid all changes.",
    );
    m.insert(
        "plain.release.outcome_undone_hint",
        "Nothing was saved. Try again after fixing the highlighted items.",
    );
    m.insert(
        "plain.release.outcome_partial",
        "We could not undo everything.",
    );
    m.insert(
        "plain.release.outcome_partial_hint",
        "Some projects may need manual cleanup. Show details for instructions.",
    );
    m.insert("plain.release.outcome_nothing", "Nothing to save.");
    m.insert("plain.release.row_saved", "Saved");
    m.insert("plain.release.row_undone", "Undone");
    m.insert(
        "plain.release.share_offer",
        "Share this release point with the team?",
    );
    m.insert("plain.release.share_action", "Share release point");
    m.insert("plain.release.share_decline", "Not now");
    m.insert("plain.release.sharing", "Sharing release point…");
    m.insert("plain.release.shared_status", "Release point shared");
    m.insert("plain.release.share_failed_status", "Release point sharing");
    m.insert("plain.release.projects_suffix", "project(s).");
    m.insert("plain.release.succeeded_suffix", "succeeded,");
    m.insert("plain.release.failed_suffix", "failed.");
    m.insert(
        "plain.release.blocker_name_used",
        "This release name is already in use. Choose another or remove the old one.",
    );
    m.insert(
        "plain.release.blocker_needs_choice",
        "Needs your choice — resolve it first.",
    );
    m.insert("plain.release.blocker_unsaved", "Has unsaved work.");

    // Change work area (Context Switch)
    m.insert("plain.switch.search_label", "Find work area");
    m.insert("plain.switch.search_hint", "Type to filter");
    m.insert(
        "plain.switch.loading_hint",
        "Choose where this project should go.",
    );
    m.insert("plain.switch.no_project", "Choose one project first.");
    m.insert("plain.switch.no_targets", "No work areas found.");
    m.insert("plain.switch.reason_current", "Already current.");
    m.insert(
        "plain.switch.reason_unavailable",
        "This project cannot be checked right now.",
    );
    m.insert(
        "plain.switch.reason_conflict",
        "Finish the current fix before changing work area.",
    );
    m.insert(
        "plain.switch.reason_dirty",
        "Save or clear unsaved work first.",
    );
    m.insert("plain.switch.kind_local", "Local work area");
    m.insert("plain.switch.kind_shared", "From shared source");
    m.insert("plain.switch.kind_saved_name", "Saved name");
    m.insert("plain.switch.kind_change", "Change");
    m.insert(
        "plain.switch.dirty_hint",
        "This project has unsaved work. Check it before changing work area.",
    );
    m.insert(
        "plain.switch.reason_empty",
        "Enter the name of the work area to switch to.",
    );
    m.insert("plain.switch.working", "Changing work area…");
    m.insert("plain.switch.done_title", "Work area changed.");
    m.insert(
        "plain.switch.failed_title",
        "We could not change the work area.",
    );
    m.insert(
        "plain.switch.failed_hint",
        "Show details for the exact reason and suggested steps.",
    );

    // Conflict resolve panel
    m.insert("plain.resolve.title", "Resolve");
    m.insert(
        "plain.resolve.instruction",
        "Open each file, choose the final version, then mark it done.",
    );
    m.insert("plain.resolve.open_editor", "Open in editor");
    m.insert("plain.resolve.mark_done", "Mark done");
    m.insert("plain.resolve.stop_attempt", "Stop this fix attempt");
    m.insert("plain.resolve.loading", "Checking files…");
    m.insert("plain.resolve.marking", "Marking file done…");
    m.insert("plain.resolve.stopping", "Stopping this fix attempt…");
    m.insert(
        "plain.resolve.working_hint",
        "This usually takes a few seconds.",
    );
    m.insert("plain.resolve.done", "Done.");
    m.insert("plain.resolve.failed", "We could not finish that action.");
    m.insert("plain.resolve.no_files", "No files need your choice now.");
    m.insert(
        "plain.resolve.unsupported",
        "This action is available for Git projects only.",
    );
    m.insert(
        "plain.resolve.stop_unavailable",
        "This fix attempt cannot be stopped here.",
    );
    m.insert(
        "plain.resolve.editor_not_configured",
        "Choose an editor in Settings first.",
    );
    m.insert(
        "plain.resolve.file_outside_project",
        "This file is outside the project folder.",
    );
    m.insert("plain.resolve.file_missing", "We cannot find this file.");

    // Generate notes (Changelog)
    m.insert("plain.changelog.title", "Generate notes");
    m.insert("plain.changelog.since_label", "Since");
    m.insert("plain.changelog.since_hint", "v1.2.0");
    m.insert("plain.changelog.generate", "Generate notes");
    m.insert(
        "plain.changelog.reason_empty",
        "Enter a starting point (e.g. the previous release name).",
    );
    m.insert("plain.changelog.collecting", "Collecting notes…");
    m.insert("plain.changelog.copy", "Copy to clipboard");
    m.insert("plain.changelog.projects_label", "Projects");
    m.insert("plain.changelog.no_projects", "No projects available.");
    m.insert("plain.changelog.summary_commits", "commits");
    m.insert("plain.changelog.summary_with_notes", "projects with notes");
    m.insert("plain.changelog.summary_no_changes", "no changes");
    m.insert("plain.changelog.summary_failed", "could not be checked");
    m.insert("plain.changelog.ready", "Notes are ready.");
    m.insert("plain.changelog.no_changes_found", "No changes found.");
    m.insert(
        "plain.changelog.some_failed",
        "Some projects could not be checked.",
    );
    m.insert(
        "plain.changelog.all_failed",
        "No projects could be checked.",
    );
    m.insert("plain.changelog.no_change_projects", "No changes:");
    m.insert(
        "plain.changelog.project_failed",
        "This project could not be checked.",
    );
    m.insert("plain.changelog.copied_prefix", "Copied notes");
    m.insert(
        "plain.changelog.copied_suffix",
        "characters to the clipboard.",
    );

    // --- Phase 5: guided setup, empty states, undo -------------------------
    m.insert("plain.add_project.title", "Add project folder");
    m.insert("plain.add_project.step1_of2", "Step 1 of 2");
    m.insert("plain.add_project.step2_of2", "Step 2 of 2");
    m.insert(
        "plain.add_project.step1_instruction",
        "Choose the folder that contains your project.",
    );
    m.insert(
        "plain.add_project.step2_instruction",
        "Give this project a name.",
    );
    m.insert("plain.add_project.folder_label", "Project folder");
    m.insert(
        "plain.add_project.folder_hint",
        "/home/user/repos/my-project",
    );
    m.insert("plain.add_project.folder_chosen", "Chosen folder");
    m.insert("plain.add_project.browse", "Choose folder");
    m.insert("plain.add_project.next", "Next");
    m.insert("plain.add_project.back", "Back");
    m.insert("plain.add_project.add", "Add project");
    m.insert("plain.add_project.name_label", "Project name");
    m.insert(
        "plain.add_project.error_no_folder",
        "Choose a project folder first.",
    );
    m.insert(
        "plain.add_project.reason_no_folder",
        "Choose a folder to continue.",
    );
    m.insert(
        "plain.add_project.reason_no_name",
        "Enter a project name to continue.",
    );
    m.insert("plain.empty.welcome_title", "Welcome to knotra");
    m.insert(
        "plain.empty.welcome_body",
        "Add your first project folder. knotra will check it and show what needs your attention.",
    );
    m.insert("plain.empty.add_first", "Add project folder");
    m.insert("plain.empty.all_clean", "🎉 All set");
    m.insert(
        "plain.empty.all_clean_hint",
        "Every project is up to date. Nothing needs your attention right now.",
    );
    m.insert(
        "plain.empty.no_match",
        "No projects match the current filter.",
    );
    m.insert("plain.undo.removed_prefix", "Removed from the list:");
    m.insert("plain.undo.undo", "Undo");
    m.insert("plain.undo.dismiss", "Dismiss");

    // --- Phase 6: accessibility + pre-existing catalog gaps ----------------
    // Workspace tab toolbar
    m.insert("plain.add_workspace", "New workspace");
    m.insert("workspace.rename.short", "Rename");
    m.insert("workspace.delete.short", "Remove");
    m.insert("workspace.create.title", "Create workspace");
    m.insert("workspace.create.confirm", "Create workspace");
    m.insert("workspace.rename.title", "Rename workspace");
    m.insert("workspace.rename.confirm", "Rename workspace");
    m.insert("workspace.name_label", "Name");
    m.insert("workspace.name_hint", "Work projects");
    m.insert("workspace.delete.title", "Remove workspace?");
    m.insert("workspace.delete.body_prefix", "This removes");
    m.insert(
        "workspace.delete.body_suffix",
        "from knotra. Project folders on this computer stay where they are.",
    );
    m.insert(
        "workspace.delete.project_count_suffix",
        "projects in this workspace",
    );
    m.insert("workspace.delete.confirm", "Remove workspace");
    m.insert(
        "workspace.delete.disabled_last",
        "Keep at least one workspace.",
    );
    m.insert("workspace.error.empty_name", "Enter a workspace name.");
    m.insert(
        "workspace.error.duplicate_name",
        "That workspace already exists.",
    );
    m.insert(
        "workspace.error.save_failed",
        "We could not save this workspace.",
    );
    m.insert(
        "workspace.error.delete_failed",
        "We could not remove this workspace.",
    );
    // Confirm remove dialog
    m.insert("plain.remove.title", "Remove this project?");
    m.insert(
        "plain.remove.body",
        "This only removes it from knotra. Your project folder stays on this computer.",
    );
    m.insert("plain.remove.confirm", "Remove from list");
    // Command palette
    m.insert("palette.title", "Command palette");
    m.insert(
        "palette.search_placeholder",
        "Search actions, projects, and workspaces",
    );
    m.insert("palette.no_matches", "No matching actions.");
    m.insert("palette.kind.project", "Project");
    m.insert("palette.kind.workspace", "Workspace");
    m.insert("palette.action.check_all", "Check all projects");
    m.insert(
        "palette.action.changelog_selected",
        "Generate notes for selected",
    );
    m.insert("palette.action.add_project", "Add project folder");
    m.insert("palette.action.remove_project", "Remove selected project");
    m.insert("palette.action.workspace_create", "Create new workspace");
    m.insert("palette.action.workspace_next", "Switch to next workspace");
    m.insert("palette.action.clear_selection", "Clear selection");
    m.insert("palette.action.open_settings", "Open settings");
    m.insert("palette.action.open_history", "Open history");
    m.insert("palette.action.toggle_theme", "Toggle theme");
    m.insert("palette.action.refresh", "Refresh dashboard");
    m.insert("palette.action.shortcuts", "Show keyboard shortcuts");
    m.insert("palette.disabled.no_workspace", "Open a workspace first.");
    m.insert(
        "palette.disabled.no_fetchable_projects",
        "No projects can be checked right now.",
    );
    m.insert(
        "palette.disabled.only_one_workspace",
        "There is no other workspace to switch to.",
    );
    m.insert(
        "palette.disabled.no_selection_to_clear",
        "There is no selection to clear.",
    );
    m.insert(
        "palette.disabled.choose_one_to_remove",
        "Choose one project to remove.",
    );
    m.insert("palette.disabled.already_open", "Already open.");
    m.insert(
        "palette.disabled.unavailable",
        "That action is not available.",
    );
    // History screen
    m.insert("history.title", "What happened");
    m.insert("history.search_hint", "Search history…");
    m.insert("history.empty", "No operations recorded yet.");
    m.insert("history.no_match", "No entries match the search.");
    m.insert("history.expand", "Details");
    m.insert("history.collapse", "Hide");
    m.insert("history.copy_log", "Copy log");
    m.insert("history.commands_header", "Commands");
    m.insert("history.recovery_header", "Recovery steps");
    m.insert("history.rollback_note", "Rolled back");
    // Settings screen
    m.insert("settings.title", "Settings");
    m.insert("settings.section.display", "Display");
    m.insert("settings.section.refresh", "Refresh & performance");
    m.insert("settings.section.tools", "External tools");
    m.insert("settings.section.logs", "Logs");
    m.insert("settings.locale_label", "Language");
    m.insert("settings.theme_label", "Theme");
    m.insert("settings.theme_dark", "Dark");
    m.insert("settings.theme_light", "Light");
    m.insert(
        "settings.refresh_interval_label",
        "Background refresh (seconds; 0 = manual)",
    );
    m.insert("settings.max_concurrent_label", "Max concurrent reads");
    m.insert("settings.editor_label", "External editor path");
    m.insert("settings.editor_hint", "/usr/bin/nvim (optional)");
    m.insert("settings.merge_tool_label", "Merge tool path");
    m.insert("settings.merge_tool_hint", "/usr/bin/meld (optional)");
    m.insert("settings.max_logs_label", "Max operation log entries");
    m.insert(
        "settings.restart_hint",
        "Some changes take effect on next launch.",
    );
    m.insert("settings.save", "Save settings");
    // Topology scan (used in legacy settings panel)
    m.insert("topology.scan", "Scan topology");
    m
}

fn ja_strings() -> HashMap<Key, &'static str> {
    let mut m = HashMap::new();
    // Navigation
    m.insert("nav.dashboard", "ダッシュボード");
    m.insert("nav.sync", "同期");
    m.insert("nav.context", "コンテキスト");
    m.insert("nav.freezer", "フリーザー");
    m.insert("nav.history", "履歴");
    m.insert("nav.settings", "設定");
    // Dashboard header
    m.insert("dashboard.title", "ワークスペース");
    m.insert("dashboard.refresh", "更新");
    m.insert("dashboard.bulk_sync", "一括同期 ▾");
    m.insert("dashboard.filter", "フィルター");
    m.insert("dashboard.group_by", "グループ");
    m.insert("dashboard.search_placeholder", "プロジェクトを検索…");
    m.insert(
        "dashboard.no_projects",
        "プロジェクトが登録されていません。",
    );
    m.insert("dashboard.add_project", "プロジェクトを追加");
    m.insert("dashboard.last_updated", "更新");
    m.insert("dashboard.refreshing_count", "更新中…");
    m.insert("dashboard.no_workspace", "ワークスペースなし");
    m.insert("dashboard.grouping", "グループ");
    m.insert("dashboard.grouping.attention", "対応状況");
    m.insert("dashboard.grouping.project_group", "プロジェクトグループ");
    m.insert("dashboard.grouping.none", "なし");
    m.insert("dashboard.sorting", "並び順");
    m.insert("dashboard.sorting.recommended", "おすすめ");
    m.insert("dashboard.sorting.name", "名前");
    m.insert("dashboard.clear_filters", "絞り込みを解除");
    m.insert("dashboard.all_projects", "すべてのプロジェクト");
    m.insert("dashboard.work_area_unknown", "作業場所を確認できません");
    m.insert("dashboard.resolve", "解決方法を選ぶ");
    m.insert(
        "dashboard.load_failed",
        "プロジェクトの状態を読み込めませんでした。",
    );
    m.insert(
        "dashboard.no_workspace_error",
        "確認するワークスペースがありません。",
    );
    m.insert("dashboard.try_again", "もう一度試す");
    m.insert("dashboard.create_workspace", "ワークスペースを作成");
    m.insert(
        "dashboard.preference_save_failed",
        "ダッシュボード設定を保存できませんでした。",
    );
    m.insert(
        "dashboard.cause.missing_path",
        "プロジェクトのフォルダーが見つかりません",
    );
    m.insert("dashboard.cause.conflict", "解決方法を選ぶ必要があります");
    m.insert(
        "dashboard.cause.conflict_detection_unavailable",
        "解決状況を確認できません",
    );
    m.insert(
        "dashboard.cause.read_unavailable",
        "プロジェクトの状態を確認できません",
    );
    m.insert(
        "dashboard.cause.detached_context",
        "作業場所が接続されていません",
    );
    m.insert(
        "dashboard.cause.status_unknown",
        "プロジェクトの状態はまだ不明です",
    );
    m.insert("dashboard.progress.uncommitted", "未保存のファイル");
    m.insert("dashboard.progress.untracked", "新しいファイル");
    m.insert("dashboard.progress.ahead", "ローカルのコミット");
    m.insert("dashboard.progress.behind", "更新あり");
    // Add-project dialog
    m.insert("dialog.add_project.title", "プロジェクトを追加");
    m.insert("dialog.add_project.name_label", "表示名");
    m.insert("dialog.add_project.path_label", "リポジトリパス");
    m.insert("dialog.add_project.name_hint", "My Service");
    m.insert(
        "dialog.add_project.path_hint",
        "/home/user/repos/my-service",
    );
    m.insert("dialog.add_project.confirm", "追加");
    m.insert("dialog.add_project.cancel", "キャンセル");
    m.insert("dialog.add_project.error_empty", "名前とパスは必須です。");
    // Status labels
    m.insert("status.healthy", "同期済み");
    m.insert("status.behind", "Behind");
    m.insert("status.ahead", "Ahead");
    m.insert("status.dirty", "未コミットあり");
    m.insert("status.conflict", "コンフリクトあり");
    m.insert("status.unknown", "不明");
    m.insert("status.refreshing", "更新中…");
    m.insert("status.error", "エラー");
    // Filter chip labels
    m.insert("filter.all", "すべて");
    m.insert("filter.healthy", "同期済み");
    m.insert("filter.behind", "Behind");
    m.insert("filter.ahead", "Ahead");
    m.insert("filter.dirty", "未コミット");
    m.insert("filter.conflict", "競合");
    m.insert("filter.error", "エラー");
    m.insert("filter.all_set", "問題なし");
    m.insert("filter.behind", "更新あり");
    m.insert("filter.ahead", "ローカルのコミット");
    m.insert("filter.dirty", "作業中の変更");
    m.insert("filter.conflict", "選択が必要");
    m.insert("filter.needs_help", "対応が必要");
    // Group labels
    m.insert("group.all", "(すべて)");
    m.insert("group.ungrouped", "(グループなし)");
    // Card fields
    m.insert("card.context", "コンテキスト");
    m.insert("card.vcs", "VCS");
    m.insert("card.ahead", "Ahead");
    m.insert("card.behind", "Behind");
    m.insert("card.uncommitted", "未コミット");
    m.insert("card.untracked", "未追跡");
    m.insert("card.conflict", "競合");
    m.insert("card.updated", "更新");
    // Card actions
    m.insert("card.action.fetch", "フェッチ");
    m.insert("card.action.remove", "削除");
    // Actions
    m.insert("action.fetch", "フェッチ");
    m.insert("action.pull", "プル");
    m.insert("action.switch_context", "コンテキスト切替");
    m.insert("action.open_freezer", "フリーザーを開く");
    m.insert("action.confirm", "確認");
    m.insert("action.cancel", "キャンセル");
    m.insert("action.retry", "再試行");
    m.insert("action.copy_log", "ログをコピー");
    m.insert("action.close", "閉じる");
    m.insert("plain.activity.details", "詳細");
    m.insert("plain.activity.succeeded", "成功");
    m.insert("plain.activity.failed", "失敗");
    m.insert("plain.activity.skipped", "対象外");
    m.insert(
        "plain.activity.retry_failed_fetches",
        "失敗した取得を再試行",
    );
    m.insert("plain.activity.review_retry", "再試行を確認");
    m.insert(
        "plain.activity.busy",
        "現在の操作が完了するまでお待ちください。",
    );
    m.insert(
        "plain.activity.none_available",
        "これらのプロジェクトは現在のワークスペースで利用できません。",
    );
    m.insert("plain.activity.retrying_fetch", "失敗した取得を再試行中");
    m.insert(
        "plain.activity.retry_context_again",
        "作業場所の変更をもう一度開いてください。",
    );
    m.insert(
        "plain.activity.retry_freeze_again",
        "リリースポイントをもう一度検証してください。",
    );
    m.insert(
        "plain.activity.retry_refresh_again",
        "更新を使ってもう一度確認してください。",
    );
    m.insert(
        "plain.activity.log_save_failed",
        "結果は表示されていますが、履歴を保存できませんでした。",
    );
    m.insert(
        "plain.activity.excluded_workspace",
        "現在のワークスペースにありません",
    );
    m.insert(
        "plain.activity.excluded_missing",
        "プロジェクトフォルダーがありません",
    );
    m.insert(
        "plain.activity.excluded_unsupported",
        "対応しているリポジトリではありません",
    );
    m.insert("plain.activity.excluded_status", "状態を取得できません");
    m.insert("plain.activity.kind_refresh", "更新");
    m.insert("plain.activity.kind_fetch", "取得");
    m.insert("plain.activity.kind_smart_pull", "最新版を取得");
    m.insert("plain.activity.kind_context_switch", "作業場所を変更");
    m.insert("plain.activity.kind_freeze", "リリースポイントを記録");
    m.insert(
        "plain.activity.kind_freeze_rollback",
        "リリースポイントを取り消し",
    );
    m.insert(
        "plain.activity.retry_preparing",
        "再試行のためプロジェクト状態を更新中...",
    );
    m.insert(
        "plain.activity.retry_prepare_failed",
        "プロジェクト状態を更新できませんでした。",
    );
    // Keyboard shortcuts hint
    m.insert("shortcut.refresh", "Ctrl+R  更新");
    m.insert("shortcut.context", "Ctrl+K  コンテキスト");
    m.insert("shortcut.freezer", "Ctrl+T  フリーザー");
    m.insert("shortcut.search", "Ctrl+/  検索");
    // Errors
    m.insert(
        "error.read_failed",
        "リポジトリの状態を読み込めませんでした。",
    );
    m.insert(
        "error.no_repo",
        "Git または jj リポジトリが見つかりません。",
    );
    // Confirm remove
    m.insert(
        "confirm.remove_project",
        "ワークスペースからプロジェクトを削除しますか？",
    );
    m.insert("confirm.remove_yes", "削除");
    m.insert("confirm.remove_no", "キャンセル");

    // --- Plain-language layer (UX review) -----------------------------------
    m.insert("tier.needs_attention", "対応が必要");
    m.insert(
        "tier.needs_attention.hint",
        "続行する前に選択が必要なプロジェクトです。",
    );
    m.insert("tier.active", "作業中");
    m.insert(
        "tier.active.hint",
        "作業中または変更が保留中のプロジェクトです。",
    );
    m.insert("tier.clean", "問題なし");
    m.insert(
        "tier.clean.hint",
        "今すぐ対応が必要なプロジェクトはありません。",
    );

    m.insert("plain.check_now", "今すぐ確認");
    m.insert("plain.check_for_updates", "更新を確認");
    m.insert("plain.get_latest", "安全に最新を取得");
    m.insert("plain.save_release_point", "リリースポイントを保存");
    m.insert("plain.change_work_area", "作業エリアを変更");
    m.insert("plain.show_what_happened", "実行内容を表示");
    m.insert("plain.show_details", "詳細を表示");
    m.insert("plain.hide_details", "詳細を隠す");
    m.insert("plain.exit_selection", "選択を終了");
    m.insert("plain.selection.enter", "選択");
    m.insert(
        "plain.select_visible_projects",
        "表示中のプロジェクトを選択",
    );
    m.insert("plain.selection.none", "プロジェクトが選択されていません");
    m.insert("plain.selection.selected_suffix", "件選択中");
    m.insert(
        "plain.selection.no_visible_projects",
        "この表示に一致するプロジェクトがありません。",
    );
    m.insert(
        "plain.selection.none_fetchable",
        "選択したプロジェクトは今は確認できません。",
    );
    m.insert(
        "plain.selection.choose_one_work_area",
        "作業エリアを変更するプロジェクトを1つ選んでください。",
    );
    m.insert(
        "plain.fetch.skipped_unavailable",
        "このプロジェクトは今は確認できません。",
    );

    m.insert("plain.status.all_set", "問題なし");
    m.insert("plain.status.unsaved_work", "未保存の作業");
    m.insert("plain.status.needs_choice", "選択が必要");
    m.insert("plain.status.not_sure", "確認中");
    m.insert("plain.status.checking", "確認中…");
    m.insert("plain.status.behind", "更新があります");
    m.insert("plain.status.ahead", "未共有の変更");

    m.insert(
        "plain.disabled.choose_one",
        "プロジェクトを1つ以上選んでください。",
    );
    m.insert(
        "plain.disabled.no_upstream",
        "更新の取得元が設定されていません。",
    );
    m.insert(
        "plain.error.path_missing",
        "プロジェクトフォルダーが見つかりません。",
    );
    m.insert(
        "plain.error.no_repo",
        "このフォルダーは knotra が確認できるプロジェクトではないようです。",
    );

    // --- Modal flows (Phase 2-4) -------------------------------------------
    m.insert("plain.project", "プロジェクト");
    m.insert("plain.what_will_happen", "内容");
    m.insert("plain.note", "メモ");
    m.insert("plain.of", "/");
    m.insert("plain.waiting", "待機中…");
    m.insert("plain.needs_help", "対応が必要");
    m.insert("plain.no_next_step", "次のステップは不要です。");

    // Get latest safely
    m.insert("plain.get_latest.preparing", "安全なプランを準備中…");
    m.insert("plain.get_latest.preparing_hint", "通常は数秒かかります。");
    m.insert(
        "plain.get_latest.review_heading",
        "変更前に内容を確認してください。",
    );
    m.insert("plain.get_latest.start", "最新を取得する");
    m.insert("plain.get_latest.working", "最新を取得中…");
    m.insert("plain.get_latest.action_get", "最新を取得");
    m.insert("plain.get_latest.action_check", "確認のみ");
    m.insert("plain.get_latest.action_get_anyway", "そのまま最新を取得");
    m.insert("plain.get_latest.action_skip", "スキップ");
    m.insert("plain.get_latest.check_only", "確認のみ");
    m.insert("plain.get_latest.get_anyway", "そのまま取得");
    m.insert(
        "plain.get_latest.note_unsaved",
        "未保存の作業あり — デフォルトは確認のみ",
    );
    m.insert(
        "plain.get_latest.note_save_restore",
        "作業を保存し、最新を取得後に復元します",
    );
    m.insert(
        "plain.get_latest.note_needs_choice",
        "選択が必要 — 解決後にスキップ解除できます",
    );
    m.insert(
        "plain.get_latest.note_no_upstream",
        "更新元が設定されていません。",
    );
    m.insert(
        "plain.get_latest.note_not_selected",
        "今回は選択されていません。",
    );
    m.insert(
        "plain.get_latest.note_status_missing",
        "状態を確認できないためスキップします。",
    );
    m.insert(
        "plain.get_latest.note_project_not_found",
        "プロジェクトが見つからないためスキップします。",
    );
    m.insert("plain.get_latest.done_row", "完了");
    m.insert("plain.get_latest.needs_help_row", "対応が必要");
    m.insert("plain.get_latest.skipped_row", "スキップ");
    m.insert("plain.get_latest.all_done_prefix", "すべての");
    m.insert(
        "plain.get_latest.all_done_suffix",
        "プロジェクトが最新です。",
    );
    m.insert("plain.get_latest.done_count", "件完了。");
    m.insert(
        "plain.get_latest.needs_help_count",
        "件が対応を必要としています。",
    );
    m.insert("plain.get_latest.skipped_count", "件スキップ。");
    m.insert(
        "plain.get_latest.review_help_rows",
        "続行前に強調表示された行を確認してください。",
    );

    // Save release point
    m.insert("plain.release.name_label", "リリース名");
    m.insert("plain.release.name_hint", "v1.2.3");
    m.insert(
        "plain.release.name_invalid",
        "英数字、ドット、ハイフン、アンダースコアのみ使用できます。",
    );
    m.insert("plain.release.note_label", "後のためのメモ（任意）");
    m.insert("plain.release.note_hint", "");
    m.insert("plain.release.check_readiness", "準備確認");
    m.insert("plain.release.checking", "確認中…");
    m.insert("plain.release.ready_check", "準備確認");
    m.insert("plain.release.row_ready", "準備完了");
    m.insert("plain.release.row_excluded", "対象外");
    m.insert("plain.release.fix_one", "保存前に1件修正してください。");
    m.insert(
        "plain.release.fix_some",
        "強調表示された項目を修正してから保存してください。",
    );
    m.insert("plain.release.saving", "リリースポイントを保存中…");
    m.insert(
        "plain.release.saving_hint",
        "すべて成功するか、何も保存しないかのどちらかです。",
    );
    m.insert(
        "plain.release.outcome_success",
        "リリースポイントを保存しました。",
    );
    m.insert(
        "plain.release.outcome_undone",
        "処理を停止し、すべての変更を元に戻しました。",
    );
    m.insert(
        "plain.release.outcome_undone_hint",
        "何も保存されませんでした。問題を修正してから再試行してください。",
    );
    m.insert(
        "plain.release.outcome_partial",
        "すべての変更を元に戻せませんでした。",
    );
    m.insert(
        "plain.release.outcome_partial_hint",
        "手動でのクリーンアップが必要な場合があります。詳細を表示してください。",
    );
    m.insert(
        "plain.release.outcome_nothing",
        "保存するものがありません。",
    );
    m.insert("plain.release.row_saved", "保存済み");
    m.insert("plain.release.row_undone", "元に戻しました");
    m.insert(
        "plain.release.share_offer",
        "このリリースポイントをチームと共有しますか？",
    );
    m.insert("plain.release.share_action", "リリースポイントを共有");
    m.insert("plain.release.share_decline", "今はしない");
    m.insert("plain.release.sharing", "リリースポイントを共有しています…");
    m.insert(
        "plain.release.shared_status",
        "リリースポイントを共有しました",
    );
    m.insert(
        "plain.release.share_failed_status",
        "リリースポイントの共有",
    );
    m.insert("plain.release.projects_suffix", "件。");
    m.insert("plain.release.succeeded_suffix", "件成功、");
    m.insert("plain.release.failed_suffix", "件失敗。");
    m.insert(
        "plain.release.blocker_name_used",
        "このリリース名はすでに使用されています。別の名前を選ぶか古いものを削除してください。",
    );
    m.insert(
        "plain.release.blocker_needs_choice",
        "選択が必要 — 先に解決してください。",
    );
    m.insert("plain.release.blocker_unsaved", "未保存の作業があります。");

    // Change work area
    m.insert("plain.switch.search_label", "作業エリアを検索");
    m.insert("plain.switch.search_hint", "入力して絞り込み");
    m.insert(
        "plain.switch.loading_hint",
        "このプロジェクトの切り替え先を選んでください。",
    );
    m.insert(
        "plain.switch.no_project",
        "先にプロジェクトを1つ選んでください。",
    );
    m.insert("plain.switch.no_targets", "作業エリアが見つかりません。");
    m.insert("plain.switch.reason_current", "現在の作業エリアです。");
    m.insert(
        "plain.switch.reason_unavailable",
        "このプロジェクトは今は確認できません。",
    );
    m.insert(
        "plain.switch.reason_conflict",
        "現在の修正を完了してから作業エリアを変更してください。",
    );
    m.insert(
        "plain.switch.reason_dirty",
        "未保存の作業を保存または片付けてください。",
    );
    m.insert("plain.switch.kind_local", "ローカル作業エリア");
    m.insert("plain.switch.kind_shared", "共有元から");
    m.insert("plain.switch.kind_saved_name", "保存済みの名前");
    m.insert("plain.switch.kind_change", "変更");
    m.insert(
        "plain.switch.dirty_hint",
        "このプロジェクトには未保存の作業があります。作業エリアを変更する前に確認してください。",
    );
    m.insert(
        "plain.switch.reason_empty",
        "切り替え先の作業エリア名を入力してください。",
    );
    m.insert("plain.switch.working", "作業エリアを変更中…");
    m.insert("plain.switch.done_title", "作業エリアを変更しました。");
    m.insert(
        "plain.switch.failed_title",
        "作業エリアを変更できませんでした。",
    );
    m.insert(
        "plain.switch.failed_hint",
        "詳細を表示して原因と対処法を確認してください。",
    );

    // Conflict resolve panel
    m.insert("plain.resolve.title", "解決");
    m.insert(
        "plain.resolve.instruction",
        "各ファイルを開いて最終バージョンを選び、完了としてマークしてください。",
    );
    m.insert("plain.resolve.open_editor", "エディタで開く");
    m.insert("plain.resolve.mark_done", "完了としてマーク");
    m.insert("plain.resolve.stop_attempt", "この修正を中断");
    m.insert("plain.resolve.loading", "ファイルを確認中…");
    m.insert("plain.resolve.marking", "ファイルを完了としてマーク中…");
    m.insert("plain.resolve.stopping", "この修正を中断中…");
    m.insert("plain.resolve.working_hint", "通常は数秒かかります。");
    m.insert("plain.resolve.done", "完了しました。");
    m.insert("plain.resolve.failed", "この操作を完了できませんでした。");
    m.insert(
        "plain.resolve.no_files",
        "対応が必要なファイルはありません。",
    );
    m.insert(
        "plain.resolve.unsupported",
        "この操作は Git プロジェクトでのみ利用できます。",
    );
    m.insert(
        "plain.resolve.stop_unavailable",
        "この修正はここでは中断できません。",
    );
    m.insert(
        "plain.resolve.editor_not_configured",
        "先に設定でエディタを選んでください。",
    );
    m.insert(
        "plain.resolve.file_outside_project",
        "このファイルはプロジェクトフォルダーの外にあります。",
    );
    m.insert(
        "plain.resolve.file_missing",
        "このファイルが見つかりません。",
    );

    // Generate notes
    m.insert("plain.changelog.title", "ノートを生成");
    m.insert("plain.changelog.since_label", "開始地点");
    m.insert("plain.changelog.since_hint", "v1.2.0");
    m.insert("plain.changelog.generate", "ノートを生成");
    m.insert(
        "plain.changelog.reason_empty",
        "開始地点を入力してください（例：前回のリリース名）。",
    );
    m.insert("plain.changelog.collecting", "収集中…");
    m.insert("plain.changelog.copy", "クリップボードにコピー");
    m.insert("plain.changelog.projects_label", "プロジェクト");
    m.insert(
        "plain.changelog.no_projects",
        "利用できるプロジェクトがありません。",
    );
    m.insert("plain.changelog.summary_commits", "件のコミット");
    m.insert("plain.changelog.summary_with_notes", "件にノートあり");
    m.insert("plain.changelog.summary_no_changes", "件は変更なし");
    m.insert("plain.changelog.summary_failed", "件は確認できませんでした");
    m.insert("plain.changelog.ready", "ノートの準備ができました。");
    m.insert(
        "plain.changelog.no_changes_found",
        "変更は見つかりませんでした。",
    );
    m.insert(
        "plain.changelog.some_failed",
        "一部のプロジェクトを確認できませんでした。",
    );
    m.insert(
        "plain.changelog.all_failed",
        "プロジェクトを確認できませんでした。",
    );
    m.insert("plain.changelog.no_change_projects", "変更なし:");
    m.insert(
        "plain.changelog.project_failed",
        "このプロジェクトを確認できませんでした。",
    );
    m.insert("plain.changelog.copied_prefix", "ノート");
    m.insert(
        "plain.changelog.copied_suffix",
        "文字をクリップボードにコピーしました。",
    );

    // --- Phase 5 (ja) ------------------------------------------------------
    m.insert("plain.add_project.title", "プロジェクトフォルダーを追加");
    m.insert("plain.add_project.step1_of2", "ステップ 1 / 2");
    m.insert("plain.add_project.step2_of2", "ステップ 2 / 2");
    m.insert(
        "plain.add_project.step1_instruction",
        "プロジェクトが入っているフォルダーを選んでください。",
    );
    m.insert(
        "plain.add_project.step2_instruction",
        "このプロジェクトに名前をつけてください。",
    );
    m.insert("plain.add_project.folder_label", "プロジェクトフォルダー");
    m.insert(
        "plain.add_project.folder_hint",
        "/home/user/repos/my-project",
    );
    m.insert("plain.add_project.folder_chosen", "選択したフォルダー");
    m.insert("plain.add_project.browse", "フォルダーを選択");
    m.insert("plain.add_project.next", "次へ");
    m.insert("plain.add_project.back", "戻る");
    m.insert("plain.add_project.add", "プロジェクトを追加");
    m.insert("plain.add_project.name_label", "プロジェクト名");
    m.insert(
        "plain.add_project.error_no_folder",
        "先にプロジェクトフォルダーを選んでください。",
    );
    m.insert(
        "plain.add_project.reason_no_folder",
        "続けるにはフォルダーを選んでください。",
    );
    m.insert(
        "plain.add_project.reason_no_name",
        "続けるにはプロジェクト名を入力してください。",
    );
    m.insert("plain.empty.welcome_title", "knotra へようこそ");
    m.insert("plain.empty.welcome_body",    "最初のプロジェクトフォルダーを追加してください。knotra が確認して、対応が必要なものをお知らせします。");
    m.insert("plain.empty.add_first", "プロジェクトフォルダーを追加");
    m.insert("plain.empty.all_clean", "🎉 問題なし");
    m.insert(
        "plain.empty.all_clean_hint",
        "すべてのプロジェクトが最新です。今すぐ対応が必要なものはありません。",
    );
    m.insert(
        "plain.empty.no_match",
        "現在のフィルターに一致するプロジェクトはありません。",
    );
    m.insert("plain.undo.removed_prefix", "リストから削除しました：");
    m.insert("plain.undo.undo", "元に戻す");
    m.insert("plain.undo.dismiss", "閉じる");

    // --- Phase 6 (ja) ------------------------------------------------------
    m.insert("plain.add_workspace", "新しいワークスペース");
    m.insert("workspace.rename.short", "名前を変更");
    m.insert("workspace.delete.short", "削除");
    m.insert("workspace.create.title", "ワークスペースを作成");
    m.insert("workspace.create.confirm", "ワークスペースを作成");
    m.insert("workspace.rename.title", "ワークスペース名を変更");
    m.insert("workspace.rename.confirm", "名前を変更");
    m.insert("workspace.name_label", "名前");
    m.insert("workspace.name_hint", "仕事用プロジェクト");
    m.insert("workspace.delete.title", "ワークスペースを削除しますか？");
    m.insert("workspace.delete.body_prefix", "削除対象:");
    m.insert(
        "workspace.delete.body_suffix",
        "knotra の一覧から削除します。プロジェクトフォルダーはこのコンピューターに残ります。",
    );
    m.insert(
        "workspace.delete.project_count_suffix",
        "件のプロジェクトがあります",
    );
    m.insert("workspace.delete.confirm", "ワークスペースを削除");
    m.insert(
        "workspace.delete.disabled_last",
        "少なくとも 1 つのワークスペースを残してください。",
    );
    m.insert(
        "workspace.error.empty_name",
        "ワークスペース名を入力してください。",
    );
    m.insert(
        "workspace.error.duplicate_name",
        "同じ名前のワークスペースがすでにあります。",
    );
    m.insert(
        "workspace.error.save_failed",
        "このワークスペースを保存できませんでした。",
    );
    m.insert(
        "workspace.error.delete_failed",
        "このワークスペースを削除できませんでした。",
    );
    m.insert("plain.remove.title", "このプロジェクトを削除しますか？");
    m.insert("plain.remove.body",          "knotra のリストから削除するだけです。プロジェクトフォルダーはこのコンピューターに残ります。");
    m.insert("plain.remove.confirm", "リストから削除");
    // Command palette
    m.insert("palette.title", "コマンドパレット");
    m.insert(
        "palette.search_placeholder",
        "操作、プロジェクト、ワークスペースを検索",
    );
    m.insert("palette.no_matches", "一致する操作はありません。");
    m.insert("palette.kind.project", "プロジェクト");
    m.insert("palette.kind.workspace", "ワークスペース");
    m.insert("palette.action.check_all", "すべてのプロジェクトを確認");
    m.insert(
        "palette.action.changelog_selected",
        "選択項目のノートを生成",
    );
    m.insert("palette.action.add_project", "プロジェクトフォルダーを追加");
    m.insert(
        "palette.action.remove_project",
        "選択したプロジェクトを削除",
    );
    m.insert(
        "palette.action.workspace_create",
        "新しいワークスペースを作成",
    );
    m.insert(
        "palette.action.workspace_next",
        "次のワークスペースへ切り替え",
    );
    m.insert("palette.action.clear_selection", "選択を解除");
    m.insert("palette.action.open_settings", "設定を開く");
    m.insert("palette.action.open_history", "履歴を開く");
    m.insert("palette.action.toggle_theme", "テーマを切り替え");
    m.insert("palette.action.refresh", "ダッシュボードを更新");
    m.insert("palette.action.shortcuts", "キーボードショートカットを表示");
    m.insert(
        "palette.disabled.no_workspace",
        "先にワークスペースを開いてください。",
    );
    m.insert(
        "palette.disabled.no_fetchable_projects",
        "今は確認できるプロジェクトがありません。",
    );
    m.insert(
        "palette.disabled.only_one_workspace",
        "切り替え先のワークスペースがありません。",
    );
    m.insert(
        "palette.disabled.no_selection_to_clear",
        "解除する選択がありません。",
    );
    m.insert(
        "palette.disabled.choose_one_to_remove",
        "削除するプロジェクトを1つ選んでください。",
    );
    m.insert("palette.disabled.already_open", "すでに開いています。");
    m.insert("palette.disabled.unavailable", "この操作は利用できません。");
    m.insert("history.title", "実行内容");
    m.insert("history.search_hint", "履歴を検索…");
    m.insert("history.empty", "まだ操作が記録されていません。");
    m.insert("history.no_match", "検索に一致する項目はありません。");
    m.insert("history.expand", "詳細");
    m.insert("history.collapse", "閉じる");
    m.insert("history.copy_log", "ログをコピー");
    m.insert("history.commands_header", "コマンド");
    m.insert("history.recovery_header", "回復手順");
    m.insert("history.rollback_note", "ロールバック済み");
    m.insert("settings.title", "設定");
    m.insert("settings.section.display", "表示");
    m.insert("settings.section.refresh", "更新とパフォーマンス");
    m.insert("settings.section.tools", "外部ツール");
    m.insert("settings.section.logs", "ログ");
    m.insert("settings.locale_label", "言語");
    m.insert("settings.theme_label", "テーマ");
    m.insert("settings.theme_dark", "ダーク");
    m.insert("settings.theme_light", "ライト");
    m.insert(
        "settings.refresh_interval_label",
        "バックグラウンド更新（秒; 0 = 手動のみ）",
    );
    m.insert("settings.max_concurrent_label", "最大同時読み込み数");
    m.insert("settings.editor_label", "外部エディタのパス");
    m.insert("settings.editor_hint", "/usr/bin/nvim （任意）");
    m.insert("settings.merge_tool_label", "マージツールのパス");
    m.insert("settings.merge_tool_hint", "/usr/bin/meld （任意）");
    m.insert("settings.max_logs_label", "操作ログの最大保持数");
    m.insert(
        "settings.restart_hint",
        "一部の変更は次回起動時に有効になります。",
    );
    m.insert("settings.save", "設定を保存");
    m.insert("topology.scan", "トポロジをスキャン");
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// First-level (plain-language) keys must not leak developer jargon.
    /// Expert terms remain available behind "Show details" via the technical
    /// keys (status.*, card.*, action.*), but the plain.* and tier.* layers
    /// are what non-technical users read first.
    const FIRST_LEVEL_PREFIXES: &[&str] =
        &["plain.", "tier.", "workspace.", "dashboard.", "filter."];

    /// Words that must never appear in first-level English wording.
    const FORBIDDEN_EN: &[&str] = &[
        "fetch",
        "pull",
        "tag",
        "branch",
        "conflict",
        "uncommitted",
        "detached",
        "upstream",
        "rollback",
        "execute",
        "cli",
        "stash",
        "merge",
        "commit",
        "repo",
    ];

    #[test]
    fn first_level_wording_has_no_developer_jargon() {
        let en = en_strings();
        for (key, value) in en.iter() {
            if !FIRST_LEVEL_PREFIXES.iter().any(|p| key.starts_with(p)) {
                continue;
            }
            let lower = value.to_lowercase();
            for bad in FORBIDDEN_EN {
                if *bad == "commit" && matches!(*key, "filter.ahead" | "dashboard.progress.ahead") {
                    continue;
                }
                assert!(
                    !lower
                        .split(|c: char| !c.is_alphanumeric())
                        .any(|w| w == *bad),
                    "first-level key `{key}` = {value:?} contains forbidden \
                     developer term `{bad}`; move expert wording behind \
                     \"Show details\""
                );
            }
        }
    }

    /// Every first-level key defined in English must also exist in Japanese.
    #[test]
    fn plain_keys_are_localised_in_both_catalogs() {
        let en = en_strings();
        let ja = ja_strings();
        for key in en.keys() {
            if FIRST_LEVEL_PREFIXES.iter().any(|p| key.starts_with(p)) {
                assert!(
                    ja.contains_key(key),
                    "first-level key `{key}` is missing from the Japanese catalog"
                );
            }
        }
    }

    #[test]
    fn japanese_dashboard_filters_do_not_retain_english_direction_labels() {
        let ja = ja_strings();
        for key in ["filter.behind", "filter.ahead"] {
            let value = ja.get(key).expect("Japanese filter translation");
            assert!(!value.contains("Behind"), "{key} retained English Behind");
            assert!(!value.contains("Ahead"), "{key} retained English Ahead");
        }
    }
}

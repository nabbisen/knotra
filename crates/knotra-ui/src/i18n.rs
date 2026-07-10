//! Minimal i18n support for knotra.
//!
//! All user-visible strings are routed through this module so that locale
//! support can be expanded in a later phase without touching every view file.
//!
//! Currently supported locales: `en` (English), `ja` (Japanese).

use std::collections::HashMap;

/// Supported UI locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[derive(Default)]
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

    pub fn locale(&self) -> Locale { self.locale }
}

fn en_strings() -> HashMap<Key, &'static str> {
    let mut m = HashMap::new();
    // Navigation
    m.insert("nav.dashboard",  "Dashboard");
    m.insert("nav.sync",       "Sync");
    m.insert("nav.context",    "Context");
    m.insert("nav.freezer",    "Freezer");
    m.insert("nav.history",    "History");
    m.insert("nav.settings",   "Settings");
    // Dashboard header
    m.insert("dashboard.title",            "Workspace Dashboard");
    m.insert("dashboard.refresh",          "Refresh");
    m.insert("dashboard.bulk_sync",        "Bulk Sync ▾");
    m.insert("dashboard.filter",           "Filter");
    m.insert("dashboard.group_by",         "Group by");
    m.insert("dashboard.search_placeholder","Search projects…");
    m.insert("dashboard.no_projects",      "No projects registered.");
    m.insert("dashboard.add_project",      "Add Project");
    m.insert("dashboard.last_updated",     "Updated");
    m.insert("dashboard.refreshing_count", "Refreshing…");
    // Add-project dialog
    m.insert("dialog.add_project.title",       "Add Project");
    m.insert("dialog.add_project.name_label",  "Display name");
    m.insert("dialog.add_project.path_label",  "Repository path");
    m.insert("dialog.add_project.name_hint",   "My Service");
    m.insert("dialog.add_project.path_hint",   "/home/user/repos/my-service");
    m.insert("dialog.add_project.confirm",     "Add");
    m.insert("dialog.add_project.cancel",      "Cancel");
    m.insert("dialog.add_project.error_empty", "Name and path are required.");
    // Status labels
    m.insert("status.healthy",    "Synced");
    m.insert("status.behind",     "Behind");
    m.insert("status.ahead",      "Ahead");
    m.insert("status.dirty",      "Uncommitted");
    m.insert("status.conflict",   "Conflict");
    m.insert("status.unknown",    "Unknown");
    m.insert("status.refreshing", "Refreshing…");
    m.insert("status.error",      "Error");
    // Filter chip labels
    m.insert("filter.all",      "All");
    m.insert("filter.healthy",  "Synced");
    m.insert("filter.behind",   "Behind");
    m.insert("filter.ahead",    "Ahead");
    m.insert("filter.dirty",    "Uncommitted");
    m.insert("filter.conflict", "Conflict");
    m.insert("filter.error",    "Error");
    // Group labels
    m.insert("group.all",      "(All groups)");
    m.insert("group.ungrouped","(Ungrouped)");
    // Card fields
    m.insert("card.context",     "Context");
    m.insert("card.vcs",         "VCS");
    m.insert("card.ahead",       "Ahead");
    m.insert("card.behind",      "Behind");
    m.insert("card.uncommitted", "Uncommitted");
    m.insert("card.untracked",   "Untracked");
    m.insert("card.conflict",    "Conflict");
    m.insert("card.updated",     "Updated");
    // Card actions
    m.insert("card.action.fetch",   "Fetch");
    m.insert("card.action.remove",  "Remove");
    // Actions
    m.insert("action.fetch",          "Fetch");
    m.insert("action.pull",           "Pull");
    m.insert("action.switch_context", "Switch Context");
    m.insert("action.open_freezer",   "Open Freezer");
    m.insert("action.confirm",        "Confirm");
    m.insert("action.cancel",         "Cancel");
    m.insert("action.retry",          "Retry");
    m.insert("action.copy_log",       "Copy Log");
    m.insert("action.close",          "Close");
    // Keyboard shortcuts hint
    m.insert("shortcut.refresh",      "Ctrl+R  Refresh");
    m.insert("shortcut.context",      "Ctrl+K  Context");
    m.insert("shortcut.freezer",      "Ctrl+T  Freezer");
    m.insert("shortcut.search",       "Ctrl+/  Search");
    // Errors
    m.insert("error.read_failed", "Failed to read repository status.");
    m.insert("error.no_repo",     "No Git or jj repository found.");
    // Confirm remove
    m.insert("confirm.remove_project",   "Remove project from workspace?");
    m.insert("confirm.remove_yes",       "Remove");
    m.insert("confirm.remove_no",        "Keep");

    // --- Plain-language layer (UX review) -----------------------------------
    // First-level wording for non-technical users. Expert terms (Fetch, Pull,
    // Tag, Conflict, …) remain available inside "Show details" via the keys
    // above, but the primary interface uses goal-oriented language.
    m.insert("tier.needs_attention",       "Needs help");
    m.insert("tier.needs_attention.hint",  "These projects need your choice before continuing.");
    m.insert("tier.active",                 "In progress");
    m.insert("tier.active.hint",            "These projects have work or changes waiting.");
    m.insert("tier.clean",                  "All set");
    m.insert("tier.clean.hint",             "These projects need no action right now.");

    m.insert("plain.check_now",             "Check now");
    m.insert("plain.check_for_updates",     "Check for updates");
    m.insert("plain.get_latest",            "Get latest safely");
    m.insert("plain.save_release_point",    "Save release point");
    m.insert("plain.change_work_area",      "Change work area");
    m.insert("plain.show_what_happened",    "Show what happened");
    m.insert("plain.show_details",          "Show details");
    m.insert("plain.hide_details",          "Hide details");
    m.insert("plain.exit_selection",        "Exit selection");

    m.insert("plain.status.all_set",        "All set");
    m.insert("plain.status.unsaved_work",   "Unsaved work");
    m.insert("plain.status.needs_choice",   "Needs your choice");
    m.insert("plain.status.not_sure",       "Not sure yet");
    m.insert("plain.status.checking",       "Checking…");
    m.insert("plain.status.behind",         "Updates available");
    m.insert("plain.status.ahead",          "Unshared changes");

    m.insert("plain.disabled.choose_one",   "Choose at least one project.");
    m.insert("plain.disabled.no_upstream",  "These projects have nowhere to get updates from.");
    m.insert("plain.error.path_missing",    "We cannot find this project folder.");
    m.insert("plain.error.no_repo",         "This folder does not look like a project knotra can check.");
    m
}

fn ja_strings() -> HashMap<Key, &'static str> {
    let mut m = HashMap::new();
    // Navigation
    m.insert("nav.dashboard",  "ダッシュボード");
    m.insert("nav.sync",       "同期");
    m.insert("nav.context",    "コンテキスト");
    m.insert("nav.freezer",    "フリーザー");
    m.insert("nav.history",    "履歴");
    m.insert("nav.settings",   "設定");
    // Dashboard header
    m.insert("dashboard.title",             "ワークスペース");
    m.insert("dashboard.refresh",           "更新");
    m.insert("dashboard.bulk_sync",         "一括同期 ▾");
    m.insert("dashboard.filter",            "フィルター");
    m.insert("dashboard.group_by",          "グループ");
    m.insert("dashboard.search_placeholder","プロジェクトを検索…");
    m.insert("dashboard.no_projects",       "プロジェクトが登録されていません。");
    m.insert("dashboard.add_project",       "プロジェクトを追加");
    m.insert("dashboard.last_updated",      "更新");
    m.insert("dashboard.refreshing_count",  "更新中…");
    // Add-project dialog
    m.insert("dialog.add_project.title",       "プロジェクトを追加");
    m.insert("dialog.add_project.name_label",  "表示名");
    m.insert("dialog.add_project.path_label",  "リポジトリパス");
    m.insert("dialog.add_project.name_hint",   "My Service");
    m.insert("dialog.add_project.path_hint",   "/home/user/repos/my-service");
    m.insert("dialog.add_project.confirm",     "追加");
    m.insert("dialog.add_project.cancel",      "キャンセル");
    m.insert("dialog.add_project.error_empty", "名前とパスは必須です。");
    // Status labels
    m.insert("status.healthy",    "同期済み");
    m.insert("status.behind",     "Behind");
    m.insert("status.ahead",      "Ahead");
    m.insert("status.dirty",      "未コミットあり");
    m.insert("status.conflict",   "コンフリクトあり");
    m.insert("status.unknown",    "不明");
    m.insert("status.refreshing", "更新中…");
    m.insert("status.error",      "エラー");
    // Filter chip labels
    m.insert("filter.all",      "すべて");
    m.insert("filter.healthy",  "同期済み");
    m.insert("filter.behind",   "Behind");
    m.insert("filter.ahead",    "Ahead");
    m.insert("filter.dirty",    "未コミット");
    m.insert("filter.conflict", "競合");
    m.insert("filter.error",    "エラー");
    // Group labels
    m.insert("group.all",       "(すべて)");
    m.insert("group.ungrouped", "(グループなし)");
    // Card fields
    m.insert("card.context",     "コンテキスト");
    m.insert("card.vcs",         "VCS");
    m.insert("card.ahead",       "Ahead");
    m.insert("card.behind",      "Behind");
    m.insert("card.uncommitted", "未コミット");
    m.insert("card.untracked",   "未追跡");
    m.insert("card.conflict",    "競合");
    m.insert("card.updated",     "更新");
    // Card actions
    m.insert("card.action.fetch",   "フェッチ");
    m.insert("card.action.remove",  "削除");
    // Actions
    m.insert("action.fetch",          "フェッチ");
    m.insert("action.pull",           "プル");
    m.insert("action.switch_context", "コンテキスト切替");
    m.insert("action.open_freezer",   "フリーザーを開く");
    m.insert("action.confirm",        "確認");
    m.insert("action.cancel",         "キャンセル");
    m.insert("action.retry",          "再試行");
    m.insert("action.copy_log",       "ログをコピー");
    m.insert("action.close",          "閉じる");
    // Keyboard shortcuts hint
    m.insert("shortcut.refresh",      "Ctrl+R  更新");
    m.insert("shortcut.context",      "Ctrl+K  コンテキスト");
    m.insert("shortcut.freezer",      "Ctrl+T  フリーザー");
    m.insert("shortcut.search",       "Ctrl+/  検索");
    // Errors
    m.insert("error.read_failed", "リポジトリの状態を読み込めませんでした。");
    m.insert("error.no_repo",     "Git または jj リポジトリが見つかりません。");
    // Confirm remove
    m.insert("confirm.remove_project",  "ワークスペースからプロジェクトを削除しますか？");
    m.insert("confirm.remove_yes",      "削除");
    m.insert("confirm.remove_no",       "キャンセル");

    // --- Plain-language layer (UX review) -----------------------------------
    m.insert("tier.needs_attention",       "対応が必要");
    m.insert("tier.needs_attention.hint",  "続行する前に選択が必要なプロジェクトです。");
    m.insert("tier.active",                 "作業中");
    m.insert("tier.active.hint",            "作業中または変更が保留中のプロジェクトです。");
    m.insert("tier.clean",                  "問題なし");
    m.insert("tier.clean.hint",             "今すぐ対応が必要なプロジェクトはありません。");

    m.insert("plain.check_now",             "今すぐ確認");
    m.insert("plain.check_for_updates",     "更新を確認");
    m.insert("plain.get_latest",            "安全に最新を取得");
    m.insert("plain.save_release_point",    "リリースポイントを保存");
    m.insert("plain.change_work_area",      "作業エリアを変更");
    m.insert("plain.show_what_happened",    "実行内容を表示");
    m.insert("plain.show_details",          "詳細を表示");
    m.insert("plain.hide_details",          "詳細を隠す");
    m.insert("plain.exit_selection",        "選択を終了");

    m.insert("plain.status.all_set",        "問題なし");
    m.insert("plain.status.unsaved_work",   "未保存の作業");
    m.insert("plain.status.needs_choice",   "選択が必要");
    m.insert("plain.status.not_sure",       "確認中");
    m.insert("plain.status.checking",       "確認中…");
    m.insert("plain.status.behind",         "更新があります");
    m.insert("plain.status.ahead",          "未共有の変更");

    m.insert("plain.disabled.choose_one",   "プロジェクトを1つ以上選んでください。");
    m.insert("plain.disabled.no_upstream",  "更新の取得元が設定されていません。");
    m.insert("plain.error.path_missing",    "プロジェクトフォルダーが見つかりません。");
    m.insert("plain.error.no_repo",         "このフォルダーは knotra が確認できるプロジェクトではないようです。");
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// First-level (plain-language) keys must not leak developer jargon.
    /// Expert terms remain available behind "Show details" via the technical
    /// keys (status.*, card.*, action.*), but the plain.* and tier.* layers
    /// are what non-technical users read first.
    const FIRST_LEVEL_PREFIXES: &[&str] = &["plain.", "tier."];

    /// Words that must never appear in first-level English wording.
    const FORBIDDEN_EN: &[&str] = &[
        "fetch", "pull", "tag", "branch", "conflict", "uncommitted",
        "detached", "upstream", "rollback", "execute", "cli", "stash",
        "merge", "commit", "repo",
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
                assert!(
                    !lower.split(|c: char| !c.is_alphanumeric()).any(|w| w == *bad),
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
}

//! Settings view — all user-configurable preferences.
//!
//! RFC-038 Stage 3: originally six text/number fields on `validated_field`
//! (Stage 2), the form laid out per D4 — two columns at Standard/Wide
//! width, stacked to one at Compact, reusing `dashboard`'s existing
//! `WidthMode` (`view/dashboard/width_mode.rs`) rather than inventing a
//! second responsive mechanism for one screen.
//!
//! RFC-073 D11–D13: four of those six became selects (`REFRESH_AUTO_STEPS`/
//! `SIMULTANEOUS_READS_STEPS`/`LOG_SIZE_STEPS`/`FS_DEBOUNCE_STEPS`, all
//! `config.rs`) — "nothing can be typed half-way, so no unintended value is
//! ever applied or saved" — leaving `validated_field` to the two path
//! fields (editor, merge tool) that remain genuinely free text. D11 also
//! moves what most people never touch (Simultaneous reads, Operation log
//! size, File-system monitoring) under a closed-by-default "Advanced"
//! disclosure (`view_advanced_section`); D4's two-column `field_grid` below
//! is unchanged by either.
//!
//! Per Handoff 050 §1: `validated_field` deliberately has no width
//! parameter (Stage 2's own decision). In a two-column grid the **cell**
//! decides width, so numeric and path fields become equal width within
//! their column; that is the point of D4, not a casualty of it.

use iced::{
    Alignment, Element, Length, Padding,
    widget::{Space, button, column, container, row, scrollable, text},
};
use knotra_ui::ThemeChoice;
use knotra_ui::i18n::Locale;
use knotra_ui::widget::{
    BUTTON_HEIGHT, Tokens, WIDE_CONTENT_WIDTH, centered_button_label, centered_button_row, icon,
    secondary_maybe, select, strong, style, validated_field,
};

use crate::{
    message::{Message, SettingsMessage, WorkspaceMessage},
    state::AppState,
    view::dashboard::WidthMode,
};

/// Matches `OverlayWidth::Large` (`knotra-ui`'s dialog-width vocabulary,
/// RFC-034) — reusing the app's existing "large bounded content" number
/// rather than inventing a new one for this form (D4: "a bounded form, not
/// a full-width column").
///
/// RFC-071 D7: well inside the shared `WIDE_CONTENT_WIDTH` ceiling
/// Dashboard and History also use, so Settings satisfies "the same
/// maximum" without needing a second `Length::Fixed(WIDE_CONTENT_WIDTH)`
/// wrapper of its own — one that would sit outside this already-narrower
/// cap and never do anything. The `const` assertion below keeps that
/// relationship checked at compile time rather than assumed.
const FORM_MAX_WIDTH: f32 = 680.0;

const _: () = assert!(FORM_MAX_WIDTH <= WIDE_CONTENT_WIDTH);

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = view_header(state);
    let body = view_body(state);

    column![header, scrollable(body).height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

fn view_header(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    // RFC-034 R13: per-screen back navigation removed — Dashboard/History are
    // reached through the persistent shell now, not a screen-owned button.
    row![strong(
        text(state.t("settings.title")).size(snora::design::style::text::heading_size(tokens))
    )]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding(Padding::new(12.0))
    .into()
}

fn view_body(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let mode = state.width_mode;

    // RFC-073 D11: Settings shows what most people use; Simultaneous
    // reads, Operation log size and File-system monitoring move under
    // "Advanced" (`view_advanced_section`), closed by default.
    let form = column![
        section_header(tokens, state.t("settings.section.display")),
        view_display_section(state),
        section_header(tokens, state.t("settings.section.refresh")),
        view_refresh_auto_field(state),
        section_header(tokens, state.t("settings.section.tools")),
        field_grid(
            mode,
            vec![view_editor_field(state), view_merge_tool_field(state)]
        ),
        section_header(tokens, state.t("settings.section.workspaces")),
        view_workspaces_section(state),
        view_advanced_section(state),
        view_save_status(state),
    ]
    .spacing(16)
    .width(Length::Fill)
    .max_width(FORM_MAX_WIDTH);

    // RFC-071 D7: left-aligned, not centred — the same change Dashboard
    // and History's wide band make, replacing RFC-035 R8's "content
    // centred". `form`'s own `FORM_MAX_WIDTH` already governs what width
    // actually renders (see the `const` assertion above).
    container(form).padding(24).align_left(Length::Fill).into()
}

// ---------------------------------------------------------------------------
// Two-column form grid (D4)
// ---------------------------------------------------------------------------

/// Arranges 1-2 field cells side by side at Standard/Wide width, stacked at
/// Compact. `validated_field`'s own `Length::Fill` (on both the input and
/// the group) means cells placed in a `row!` split the available width
/// evenly — the grid's column, not the field, decides how wide each one is.
fn field_grid<'a>(mode: WidthMode, cells: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    match mode {
        WidthMode::Compact => column(cells).spacing(16).into(),
        WidthMode::Standard | WidthMode::Wide => row(cells).spacing(16).into(),
    }
}

// ---------------------------------------------------------------------------
// Display section — unchanged from before Stage 3 (no numeric validation,
// not a `validated_field` candidate)
// ---------------------------------------------------------------------------

/// RFC-070 D3, extended by Amendment A1: both the language and the theme
/// choice are one `select` each, showing the current choice directly — no
/// separate "Active: …" note, and no button disabled *because* it is the
/// current choice (A1's own rule: a current choice is chosen, not
/// unavailable; a disabled button means something else since RFC-071 A1/A5
/// drew unavailable controls in grey). Neither select carries a focus
/// target: Settings has no RFC-066 `FocusTarget` wiring at all today (its
/// buttons never did either, confirmed by grep before this change), so
/// `is_focused: false` is the honest unwired state, the same choice other
/// not-yet-wired controls elsewhere in the app make explicitly rather than
/// silently.
fn view_display_section(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;

    // RFC-070 Amendment A1 §1: "English"/"日本語" stay literal endonyms, not
    // `state.t()` calls — unchanged reasoning from the buttons this
    // replaces (a language names itself, never translated).
    let locale_options = vec![
        (Locale::En, "English".to_owned()),
        (Locale::Ja, "日本語".to_owned()),
    ];
    let locale_row = row![
        text(state.t("settings.locale_label"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        Space::new().width(Length::Fill),
        select::pick_list(
            tokens,
            locale_options,
            Some(state.config.locale),
            false,
            |value| Message::Settings(SettingsMessage::LocaleChanged(value)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // RFC-070 D3: Light, Dark, High contrast light, High contrast dark —
    // the RFC's own order.
    let theme_options = vec![
        (
            ThemeChoice::Light,
            state.t("settings.theme_light").to_owned(),
        ),
        (ThemeChoice::Dark, state.t("settings.theme_dark").to_owned()),
        (
            ThemeChoice::HighContrastLight,
            state.t("settings.theme_high_contrast_light").to_owned(),
        ),
        (
            ThemeChoice::HighContrastDark,
            state.t("settings.theme_high_contrast_dark").to_owned(),
        ),
    ];
    let theme_row = row![
        text(state.t("settings.theme_label"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        Space::new().width(Length::Fill),
        select::pick_list(
            tokens,
            theme_options,
            Some(state.config.theme),
            false,
            |value| Message::Settings(SettingsMessage::ThemeChanged(value)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // RFC-071 D11/D12: percent labels ("115 %") are numbers, not catalog
    // text — the RFC's own words for this, matching the Zoom in/out
    // shortcuts' already-numeric nature.
    let zoom_options: Vec<(u16, String)> = crate::config::ZOOM_STEPS
        .iter()
        .map(|&percent| (percent, format!("{percent} %")))
        .collect();
    let zoom_row = row![
        text(state.t("settings.zoom_label"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        Space::new().width(Length::Fill),
        select::pick_list(
            tokens,
            zoom_options,
            Some(state.config.zoom_percent),
            false,
            |value| Message::Settings(SettingsMessage::ZoomChanged(value)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // RFC-071 Amendment A3 D14: explains what the setting does — read to
    // understand before changing it, the same category `fs_watch_hint`
    // below is in.
    let zoom_help = text(state.t("settings.zoom_help"))
        .size(snora::design::style::text::body_size(tokens))
        .line_height(snora::design::style::text::body_line_height(tokens));

    column![locale_row, theme_row, zoom_row, zoom_help]
        .spacing(8)
        .into()
}

// ---------------------------------------------------------------------------
// Workspaces section (RFC-073 D4) — creation, rename and removal move here
// from the header switcher's menu, built from the existing dialogs and
// texts (`workspace_mgr.rs`/`workspace_manager.rs` are unchanged; only
// these buttons' entry points are new). The palette's own workspace
// actions stay exactly as they were.
// ---------------------------------------------------------------------------

fn view_workspaces_section(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let can_delete = state.all_workspaces.len() > 1;

    let mut list = column![].spacing(8);
    for ws in &state.all_workspaces {
        let rename_btn = secondary_maybe(
            tokens,
            state.t("workspace.rename.short"),
            Some(Message::Workspace(
                WorkspaceMessage::RenameWorkspaceDialogOpenedFor(ws.id.clone()),
            )),
        );

        let remove_btn = secondary_maybe(
            tokens,
            state.t("workspace.delete.short"),
            can_delete.then_some(Message::Workspace(
                WorkspaceMessage::DeleteWorkspaceRequestedFor(ws.id.clone()),
            )),
        );

        // RFC-073 D4: "Remove ... stays unavailable for the last
        // workspace, with its existing reason" — the same
        // `workspace.delete.disabled_last` text the delete dialog itself
        // falls back to when the last workspace is somehow targeted.
        let remove: Element<'_, Message> = if can_delete {
            remove_btn
        } else {
            column![
                remove_btn,
                text(state.t("workspace.delete.disabled_last"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
            ]
            .spacing(4)
            .into()
        };

        list = list.push(
            row![
                text(ws.name.clone())
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                Space::new().width(Length::Fill),
                rename_btn,
                remove,
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    let add_workspace = secondary_maybe(
        tokens,
        state.t("plain.add_workspace"),
        Some(Message::Workspace(
            WorkspaceMessage::CreateWorkspaceDialogOpened,
        )),
    );

    column![list, add_workspace].spacing(12).into()
}

// ---------------------------------------------------------------------------
// Stepped settings (RFC-038 D1/Stage 2's `validated_field`s, replaced by
// RFC-073 D12's selects below — `view_editor_field`/`view_merge_tool_field`
// further down are the two that stayed on `validated_field`)
// ---------------------------------------------------------------------------

/// RFC-073 D12: a select, not a typed field — "Off" or one of four named
/// intervals, D16's own labels, mapped to `config.rs`'s `REFRESH_AUTO_STEPS`
/// seconds values.
fn view_refresh_auto_field(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let options: Vec<(u32, String)> = crate::config::REFRESH_AUTO_STEPS
        .iter()
        .map(|&secs| (secs, refresh_auto_choice_label(state, secs).to_owned()))
        .collect();
    row![
        text(state.t("settings.refresh_auto_label"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        Space::new().width(Length::Fill),
        select::pick_list(
            tokens,
            options,
            Some(state.config.refresh_interval_secs),
            false,
            |value| Message::Settings(SettingsMessage::RefreshAutoChanged(value)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// The catalog label for one of [`crate::config::REFRESH_AUTO_STEPS`]'s
/// five values — an exhaustive match, not a formula, since these five are
/// named choices (D16), not plain numbers.
fn refresh_auto_choice_label(state: &AppState, secs: u32) -> &str {
    match secs {
        0 => state.t("settings.choice.off"),
        30 => state.t("settings.choice.every_30s"),
        60 => state.t("settings.choice.every_1m"),
        300 => state.t("settings.choice.every_5m"),
        900 => state.t("settings.choice.every_15m"),
        _ => unreachable!("REFRESH_AUTO_STEPS only contains these five values"),
    }
}

/// RFC-073 D12: a select of plain numbers — "a plain number needs no
/// catalog text" — over `config.rs`'s `SIMULTANEOUS_READS_STEPS`.
fn view_max_concurrent_field(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let options: Vec<(usize, String)> = crate::config::SIMULTANEOUS_READS_STEPS
        .iter()
        .map(|&n| (n, n.to_string()))
        .collect();
    row![
        text(state.t("settings.max_concurrent_label"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        Space::new().width(Length::Fill),
        select::pick_list(
            tokens,
            options,
            Some(state.config.max_concurrent_reads),
            false,
            |value| Message::Settings(SettingsMessage::MaxConcurrentChanged(value)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// RFC-073 D12: a select of plain numbers over `config.rs`'s
/// `LOG_SIZE_STEPS`. D13: the one setting that needs a restart to take
/// full effect — `config.rs`'s own doc comment on `max_log_entries` has
/// the reason — so `settings.needs_restart` sits under it.
fn view_max_logs_field(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let options: Vec<(usize, String)> = crate::config::LOG_SIZE_STEPS
        .iter()
        .map(|&n| (n, n.to_string()))
        .collect();
    let field = row![
        text(state.t("settings.max_logs_label"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        Space::new().width(Length::Fill),
        select::pick_list(
            tokens,
            options,
            Some(state.config.max_log_entries),
            false,
            |value| Message::Settings(SettingsMessage::MaxLogEntriesChanged(value)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    column![
        field,
        text(state.t("settings.needs_restart"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
    ]
    .spacing(4)
    .into()
}

fn view_editor_field(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    validated_field(
        tokens,
        state.t("settings.editor_label"),
        state.t("settings.editor_hint"),
        &state.settings_edit.external_editor,
        None,
        |s| Message::Settings(SettingsMessage::ExternalEditorChanged(s)),
        None,
    )
}

fn view_merge_tool_field(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    validated_field(
        tokens,
        state.t("settings.merge_tool_label"),
        state.t("settings.merge_tool_hint"),
        &state.settings_edit.external_merge_tool,
        None,
        |s| Message::Settings(SettingsMessage::ExternalMergeToolChanged(s)),
        None,
    )
}

/// RFC-073 D12: a select over `config.rs`'s `FS_DEBOUNCE_STEPS` — each
/// option's own label carries the existing `settings.unit_seconds` suffix
/// baked in (`"2 seconds"`), since a `select` has no separate unit slot the
/// way `validated_field`'s own `unit` parameter did.
fn view_fs_debounce_field(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let unit = state.t("settings.unit_seconds");
    let options: Vec<(u32, String)> = crate::config::FS_DEBOUNCE_STEPS
        .iter()
        .map(|&secs| (secs, format!("{secs} {unit}")))
        .collect();
    row![
        text(state.t("settings.fs_watch_interval_label"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        Space::new().width(Length::Fill),
        select::pick_list(
            tokens,
            options,
            Some(state.config.fs_debounce_secs),
            false,
            |value| Message::Settings(SettingsMessage::FsDebounceSecs(value)),
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

// ---------------------------------------------------------------------------
// Advanced (RFC-073 D11) — Simultaneous reads, Operation log size and
// File-system monitoring, closed by default and never saved open.
// ---------------------------------------------------------------------------

/// A disclosure button (D11/D4's own idiom — `dashboard/section.rs`'s group
/// toggle) that shows `settings.section.advanced` alone when closed, and
/// its three settings when open: Simultaneous reads directly beneath it
/// (self-labeled, the same way `view_display_section`'s rows need no
/// sub-header of their own), then "Logs" and "File-system Monitoring"
/// keeping their own existing headers.
fn view_advanced_section(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let open = state.settings_advanced_open;

    // Right reads as "click to open"; down as "already open, contents
    // below" — the same disclosure idiom `dashboard/section.rs`'s group
    // toggle and `history.rs`'s entry toggle use.
    let chevron = if open {
        icon::chevron_down()
    } else {
        icon::chevron_right()
    };

    // RFC-073 D8: `BUTTON_HEIGHT` — a frequent, full-width target, the same
    // rule every other pressable row on this screen follows.
    let header = button(centered_button_row(
        row![
            text(state.t("settings.section.advanced"))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
            icon::icon_element(&chevron),
        ]
        .spacing(6),
    ))
    .height(Length::Fixed(BUTTON_HEIGHT))
    .width(Length::Fill)
    .on_press(Message::Settings(SettingsMessage::AdvancedToggled))
    .style({
        let t = tokens.clone();
        move |_theme, status| style::with_focus_ring(&t, false, style::ghost(&t, status))
    });

    if !open {
        return header.into();
    }

    column![
        header,
        view_max_concurrent_field(state),
        section_header(tokens, state.t("settings.section.logs")),
        view_max_logs_field(state),
        section_header(tokens, state.t("settings.section.fs_watch")),
        view_fs_watch_section(state),
    ]
    .spacing(16)
    .into()
}

// ---------------------------------------------------------------------------
// FS Watch section — enable toggle and hint stay plain (not fields); the
// debounce interval is the one select in this section.
// ---------------------------------------------------------------------------

fn view_fs_watch_section(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    // RFC-073 Amendment A2: `secondary` — a toggle, not the section's sole
    // action; its current state is the label's own text, not the fill.
    // RFC-073 D8: `BUTTON_HEIGHT`, matching the Workspaces section's own
    // buttons on this same screen.
    let t = tokens.clone();
    column![
        labeled_row(
            tokens,
            state.t("settings.fs_watch_enable_label"),
            button(centered_button_label(text(
                if state.config.fs_watch_enabled {
                    state.t("settings.fs_watch_enabled")
                } else {
                    state.t("settings.fs_watch_disabled")
                }
            )))
            .height(Length::Fixed(BUTTON_HEIGHT))
            .padding([0, 12])
            .on_press(Message::Settings(SettingsMessage::FsWatchEnabledChanged(
                !state.config.fs_watch_enabled
            )))
            .style(move |_theme, status| {
                style::with_focus_ring(&t, false, style::secondary(&t, status))
            })
            .into(),
        ),
        // RFC-038 Stage 1 §9: this sentence names `.git/HEAD` and "index" —
        // moved into the catalog unchanged rather than reworded, per the
        // handoff's explicit instruction. `settings.*` is not among
        // `FIRST_LEVEL_PREFIXES`, so `first_level_wording_has_no_developer_jargon`
        // does not and will not police it either way.
        // RFC-071 Amendment A3 D14: explains what this setting does — read
        // to understand before enabling it.
        text(state.t("settings.fs_watch_hint"))
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        view_fs_debounce_field(state),
    ]
    .spacing(12)
    .into()
}

// ---------------------------------------------------------------------------
// Save status — RFC-071 Amendment A7: every setting is saved when it
// changes, so there is no button here any more, only a failure message
// (`persist_config`'s own doc comment, `app/misc.rs`) when the most recent
// save did not succeed. A successful save shows nothing — a message on
// every keystroke would be noise, not news.
// ---------------------------------------------------------------------------

fn view_save_status(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;

    // RFC-071 Amendment A3 D14: an error, read to understand and fix a
    // problem.
    match state.settings_save_msg {
        Some(ref msg) => text(msg.as_str())
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens))
            .into(),
        None => Space::new().into(),
    }
}

fn section_header<'a>(tokens: &Tokens, label: &'a str) -> Element<'a, Message> {
    text(label)
        .size(snora::design::style::text::body_size(tokens))
        .line_height(snora::design::style::text::body_line_height(tokens))
        .into()
}

fn labeled_row<'a>(
    tokens: &Tokens,
    label: &'a str,
    widget: Element<'a, Message>,
) -> Element<'a, Message> {
    // RFC-071 Amendment A3 D14: a field label — explicit move category.
    row![
        text(label)
            .size(snora::design::style::text::body_size(tokens))
            .line_height(snora::design::style::text::body_line_height(tokens)),
        Space::new().width(Length::Fill),
        widget,
    ]
    .align_y(Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::refresh_auto_choice_label;
    use crate::{config::AppConfig, state::AppState};

    /// RFC-073 D12: every one of `REFRESH_AUTO_STEPS`' five values has its
    /// own named choice — `refresh_auto_choice_label`'s match is an
    /// `unreachable!()` away from panicking on the sixth, so this proves
    /// the five that exist today all resolve, rather than trusting the
    /// match arms by eye.
    #[test]
    fn refresh_auto_choice_label_covers_every_step() {
        let state = AppState::new(AppConfig::default());
        for &secs in &crate::config::REFRESH_AUTO_STEPS {
            let label = refresh_auto_choice_label(&state, secs);
            assert!(!label.is_empty(), "secs={secs}");
        }
    }
}

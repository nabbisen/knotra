//! Settings view — all user-configurable preferences.

use iced::{
    Alignment, Element, Length, Padding,
    widget::{Space, button, column, row, scrollable, text, text_input},
};
use knotra_ui::i18n::Locale;

use crate::{
    message::{Message, SettingsMessage, TopologyMessage},
    state::AppState,
};

pub fn view(state: &AppState) -> Element<'_, Message> {
    let header = view_header(state);
    let body = view_body(state);

    column![header, scrollable(body).height(Length::Fill)]
        .height(Length::Fill)
        .into()
}

fn view_header(state: &AppState) -> Element<'_, Message> {
    // RFC-034 R13: per-screen back navigation removed — Dashboard/History are
    // reached through the persistent shell now, not a screen-owned button.
    row![text(state.t("settings.title")).size(20)]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding(Padding::new(12.0))
        .into()
}

fn view_body(state: &AppState) -> Element<'_, Message> {
    let edit = &state.settings_edit;

    // ------------------------------------------------------------------ //
    // Display
    // ------------------------------------------------------------------ //

    let locale_row = row![
        text(state.t("settings.locale_label")).size(13),
        Space::new().width(Length::Fill),
        button(text("English")).on_press_maybe(if state.config.locale != Locale::En {
            Some(Message::Settings(SettingsMessage::LocaleChanged(
                Locale::En,
            )))
        } else {
            None
        }),
        button(text("日本語")).on_press_maybe(if state.config.locale != Locale::Ja {
            Some(Message::Settings(SettingsMessage::LocaleChanged(
                Locale::Ja,
            )))
        } else {
            None
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let theme_row = row![
        text(state.t("settings.theme_label")).size(13),
        Space::new().width(Length::Fill),
        button(text(state.t("settings.theme_dark"))).on_press_maybe(if !state.config.dark_theme {
            Some(Message::Settings(SettingsMessage::ThemeChanged(true)))
        } else {
            None
        }),
        button(text(state.t("settings.theme_light"))).on_press_maybe(if state.config.dark_theme {
            Some(Message::Settings(SettingsMessage::ThemeChanged(false)))
        } else {
            None
        }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // RFC-038 Stage 1 §2: was a hand-rolled `match` returning a
    // pre-baked "Active: {locale}" string — a second, unaudited
    // localisation mechanism alongside the catalog, so a Japanese user got
    // the value translated but not the "Active:" label. `Locale`'s own
    // `Display` impl (i18n.rs) is left as the source of each language's
    // name — "English"/"日本語" are endonyms, not translatable content, the
    // same reason the locale-switch buttons above say "English"/"日本語"
    // unconditionally rather than through `state.t()`.
    let active_locale_note = format!(
        "{} {}",
        state.t("settings.active_prefix"),
        state.config.locale
    );
    let active_theme_note = format!(
        "{} {}",
        state.t("settings.active_prefix"),
        if state.config.dark_theme {
            state.t("settings.theme_dark")
        } else {
            state.t("settings.theme_light")
        }
    );

    // ------------------------------------------------------------------ //
    // Refresh & Performance
    // ------------------------------------------------------------------ //

    let refresh_input: iced::widget::TextInput<'_, Message> =
        text_input("60", &edit.refresh_interval_secs)
            .on_input(|s| {
                let n = s.parse::<u32>().unwrap_or(0);
                Message::Settings(SettingsMessage::RefreshIntervalChanged(n))
            })
            .width(80);

    let max_concurrent_input = text_input("8", &edit.max_concurrent_reads)
        .on_input(|s| {
            if let Some(n) = s.parse::<usize>().ok().filter(|&n| n > 0) {
                Message::Settings(SettingsMessage::MaxConcurrentChanged(n))
            } else {
                Message::Settings(SettingsMessage::MaxConcurrentChanged(1))
            }
        })
        .width(80);

    let max_logs_input = text_input("200", &edit.max_log_entries)
        .on_input(|s| {
            if let Some(n) = s.parse::<usize>().ok().filter(|&n| n > 0) {
                Message::Settings(SettingsMessage::MaxLogEntriesChanged(n))
            } else {
                Message::Settings(SettingsMessage::MaxLogEntriesChanged(10))
            }
        })
        .width(80);

    // ------------------------------------------------------------------ //
    // External Tools
    // ------------------------------------------------------------------ //

    let editor_input = text_input(state.t("settings.editor_hint"), &edit.external_editor)
        .on_input(|s| Message::Settings(SettingsMessage::ExternalEditorChanged(s)))
        .width(350);

    let merge_input = text_input(
        state.t("settings.merge_tool_hint"),
        &edit.external_merge_tool,
    )
    .on_input(|s| Message::Settings(SettingsMessage::ExternalMergeToolChanged(s)))
    .width(350);

    // ------------------------------------------------------------------ //
    // Save
    // ------------------------------------------------------------------ //

    let save_btn = button(text(state.t("settings.save")))
        .on_press(Message::Settings(SettingsMessage::SaveRequested));

    let save_msg: Element<'_, Message> = if let Some(ref msg) = state.settings_save_msg {
        text(msg.as_str()).size(13).into()
    } else {
        Space::new().into()
    };

    // ------------------------------------------------------------------ //
    // Compose layout
    // ------------------------------------------------------------------ //

    column![
        // Display section
        section_header(state.t("settings.section.display")),
        locale_row,
        text(active_locale_note).size(11),
        theme_row,
        text(active_theme_note).size(11),
        // Refresh section
        section_header(state.t("settings.section.refresh")),
        labeled_row(
            state.t("settings.refresh_interval_label"),
            refresh_input.into()
        ),
        labeled_row(
            state.t("settings.max_concurrent_label"),
            max_concurrent_input.into()
        ),
        // External tools section
        section_header(state.t("settings.section.tools")),
        text(state.t("settings.editor_label")).size(13),
        editor_input,
        text(state.t("settings.merge_tool_label")).size(13),
        merge_input,
        // Logs section
        section_header(state.t("settings.section.logs")),
        labeled_row(state.t("settings.max_logs_label"), max_logs_input.into()),
        // FS Watch section
        section_header(state.t("settings.section.fs_watch")),
        labeled_row(
            state.t("settings.fs_watch_enable_label"),
            button(text(if state.config.fs_watch_enabled {
                state.t("settings.fs_watch_enabled")
            } else {
                state.t("settings.fs_watch_disabled")
            }))
            .on_press(Message::Settings(SettingsMessage::FsWatchEnabledChanged(
                !state.config.fs_watch_enabled
            )))
            .into(),
        ),
        // RFC-038 Stage 1 §9: this sentence names `.git/HEAD` and "index" —
        // moved into the catalog unchanged rather than reworded, per the
        // handoff's explicit instruction. `settings.*` is not among
        // `FIRST_LEVEL_PREFIXES`, so `first_level_wording_has_no_developer_jargon`
        // does not and will not police it either way.
        text(state.t("settings.fs_watch_hint")).size(11),
        labeled_row(
            state.t("settings.fs_watch_interval_label"),
            text_input("2", &state.settings_edit.fs_debounce_secs)
                .on_input(|s| {
                    let n = s.parse::<u32>().unwrap_or(2);
                    Message::Settings(SettingsMessage::FsDebounceSecs(n))
                })
                .width(80)
                .into(),
        ),
        // Topology scan button
        section_header(state.t("settings.section.topology")),
        row![
            button(text(state.t("topology.scan")).size(12))
                .on_press(Message::Topology(TopologyMessage::ScanRequested)),
            text(match &state.topology.phase {
                crate::state::topology::TopologyPhase::Idle =>
                    state.t("settings.topology_not_scanned"),
                crate::state::topology::TopologyPhase::Scanning =>
                    state.t("settings.topology_scanning"),
                crate::state::topology::TopologyPhase::Ready(_) =>
                    state.t("settings.topology_scan_complete"),
                crate::state::topology::TopologyPhase::Error(_) =>
                    state.t("settings.topology_scan_error"),
            })
            .size(12),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center),
        // Save row
        row![save_btn, save_msg]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .padding([8, 0]),
        text(state.t("settings.restart_hint")).size(11),
    ]
    .spacing(8)
    .padding(24)
    .into()
}

fn section_header(label: &str) -> Element<'_, Message> {
    text(label).size(15).into()
}

fn labeled_row<'a>(label: &'a str, widget: Element<'a, Message>) -> Element<'a, Message> {
    row![
        text(label).size(13),
        Space::new().width(Length::Fill),
        widget,
    ]
    .align_y(Alignment::Center)
    .into()
}

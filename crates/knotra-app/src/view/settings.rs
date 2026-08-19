//! Settings view — all user-configurable preferences.
//!
//! RFC-038 Stage 3: the six text/number fields are migrated onto
//! `validated_field` (Stage 2), and the form is laid out per D4 — two
//! columns at Standard/Wide width, stacked to one at Compact, reusing
//! `dashboard`'s existing `WidthMode` (`view/dashboard/width_mode.rs`)
//! rather than inventing a second responsive mechanism for one screen.
//!
//! Per Handoff 050 §1: `validated_field` deliberately has no width
//! parameter (Stage 2's own decision, re-affirmed here after measuring the
//! six fields' current widths — 80px for the four numeric ones, 350px for
//! the two path ones). In a two-column grid the **cell** decides width, so
//! numeric and path fields become equal width within their column; that is
//! the point of D4, not a casualty of it.

use iced::{
    Alignment, Element, Length, Padding,
    widget::{Space, button, column, container, row, scrollable, text},
};
use knotra_ui::i18n::Locale;
use knotra_ui::widget::{Tokens, validated_field};

use crate::{
    message::{Message, SettingsMessage},
    state::AppState,
    view::dashboard::WidthMode,
};

/// Matches `OverlayWidth::Large` (`knotra-ui`'s dialog-width vocabulary,
/// RFC-034) — reusing the app's existing "large bounded content" number
/// rather than inventing a new one for this form (D4: "a bounded form, not
/// a full-width column").
const FORM_MAX_WIDTH: f32 = 680.0;

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
    row![text(state.t("settings.title")).size(snora::design::style::text::heading_size(tokens))]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding(Padding::new(12.0))
        .into()
}

fn view_body(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let mode = state.width_mode;

    let form = column![
        section_header(tokens, state.t("settings.section.display")),
        view_display_section(state),
        section_header(tokens, state.t("settings.section.refresh")),
        field_grid(
            mode,
            vec![
                view_refresh_interval_field(state),
                view_max_concurrent_field(state),
            ],
        ),
        section_header(tokens, state.t("settings.section.tools")),
        field_grid(
            mode,
            vec![view_editor_field(state), view_merge_tool_field(state)]
        ),
        section_header(tokens, state.t("settings.section.logs")),
        view_max_logs_field(state),
        section_header(tokens, state.t("settings.section.fs_watch")),
        view_fs_watch_section(state),
        view_save_row(state),
        text(state.t("settings.restart_hint"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    ]
    .spacing(16)
    .width(Length::Fill)
    .max_width(FORM_MAX_WIDTH);

    container(form).padding(24).center_x(Length::Fill).into()
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

fn view_display_section(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let locale_row = row![
        text(state.t("settings.locale_label"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
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
        text(state.t("settings.theme_label"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
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

    column![
        locale_row,
        text(active_locale_note)
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
        theme_row,
        text(active_theme_note)
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    ]
    .spacing(8)
    .into()
}

// ---------------------------------------------------------------------------
// Validated fields (RFC-038 D1/Stage 2, migrated here in Stage 3)
// ---------------------------------------------------------------------------

fn view_refresh_interval_field(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let value = &state.settings_edit.refresh_interval_secs;
    let error = u32_field_error(value).map(|key| state.t(key));
    validated_field(
        tokens,
        state.t("settings.refresh_interval_label"),
        "60",
        value,
        Some(state.t("settings.unit_seconds")),
        |s| Message::Settings(SettingsMessage::RefreshIntervalChanged(s)),
        error,
    )
}

fn view_max_concurrent_field(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let value = &state.settings_edit.max_concurrent_reads;
    let error = positive_usize_field_error(value).map(|key| state.t(key));
    validated_field(
        tokens,
        state.t("settings.max_concurrent_label"),
        "8",
        value,
        None,
        |s| Message::Settings(SettingsMessage::MaxConcurrentChanged(s)),
        error,
    )
}

fn view_max_logs_field(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let value = &state.settings_edit.max_log_entries;
    let error = positive_usize_field_error(value).map(|key| state.t(key));
    validated_field(
        tokens,
        state.t("settings.max_logs_label"),
        "200",
        value,
        None,
        |s| Message::Settings(SettingsMessage::MaxLogEntriesChanged(s)),
        error,
    )
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

fn view_fs_debounce_field(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let value = &state.settings_edit.fs_debounce_secs;
    let error = u32_field_error(value).map(|key| state.t(key));
    validated_field(
        tokens,
        state.t("settings.fs_watch_interval_label"),
        "2",
        value,
        Some(state.t("settings.unit_seconds")),
        |s| Message::Settings(SettingsMessage::FsDebounceSecs(s)),
        error,
    )
}

// ---------------------------------------------------------------------------
// Field validity — pure, testable without an `AppState` (RFC-038 §6, by
// analogy with RFC-042 R3: proven to fail on a planted violation before
// being trusted; see the review request).
// ---------------------------------------------------------------------------

/// The error key for a field that accepts any `u32`, including 0 — refresh
/// interval and FS-watch debounce (`config.rs` documents "0 = manual only"
/// for the former; the latter's 0 means immediate, the same shape).
/// `None` when `s` parses.
fn u32_field_error(s: &str) -> Option<&'static str> {
    s.parse::<u32>()
        .is_err()
        .then_some("settings.error.invalid_number")
}

/// The error key for a field that must be a `usize` greater than 0 — max
/// concurrent reads, max log entries. Both `0`s would mean "nothing" (no
/// reads possible / no logs kept), which is never a meaningful setting, so
/// `0` is rejected the same as unparseable text. `None` when valid.
fn positive_usize_field_error(s: &str) -> Option<&'static str> {
    let valid = s.parse::<usize>().is_ok_and(|n| n > 0);
    (!valid).then_some("settings.error.invalid_positive_number")
}

// ---------------------------------------------------------------------------
// FS Watch section — enable toggle and hint stay plain (not fields); the
// debounce interval is the one `validated_field` in this section.
// ---------------------------------------------------------------------------

fn view_fs_watch_section(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    column![
        labeled_row(
            tokens,
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
        text(state.t("settings.fs_watch_hint"))
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
        view_fs_debounce_field(state),
    ]
    .spacing(12)
    .into()
}

// ---------------------------------------------------------------------------
// Save row — unchanged from before Stage 3
// ---------------------------------------------------------------------------

fn view_save_row(state: &AppState) -> Element<'_, Message> {
    let tokens = &state.theme.tokens;
    let save_btn = button(text(state.t("settings.save")))
        .on_press(Message::Settings(SettingsMessage::SaveRequested));

    let save_msg: Element<'_, Message> = if let Some(ref msg) = state.settings_save_msg {
        text(msg.as_str())
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens))
            .into()
    } else {
        Space::new().into()
    };

    row![save_btn, save_msg]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([8, 0])
        .into()
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
    row![
        text(label)
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
        Space::new().width(Length::Fill),
        widget,
    ]
    .align_y(Alignment::Center)
    .into()
}

#[cfg(test)]
mod tests {
    use super::{positive_usize_field_error, u32_field_error};

    #[test]
    fn u32_field_accepts_zero_and_any_parseable_value() {
        assert_eq!(u32_field_error("0"), None);
        assert_eq!(u32_field_error("60"), None);
        assert_eq!(
            u32_field_error("abc"),
            Some("settings.error.invalid_number")
        );
        assert_eq!(u32_field_error(""), Some("settings.error.invalid_number"));
        assert_eq!(u32_field_error("-1"), Some("settings.error.invalid_number"));
    }

    #[test]
    fn positive_usize_field_rejects_zero_and_unparseable() {
        assert_eq!(positive_usize_field_error("8"), None);
        assert_eq!(positive_usize_field_error("1"), None);
        assert_eq!(
            positive_usize_field_error("0"),
            Some("settings.error.invalid_positive_number")
        );
        assert_eq!(
            positive_usize_field_error("abc"),
            Some("settings.error.invalid_positive_number")
        );
        assert_eq!(
            positive_usize_field_error(""),
            Some("settings.error.invalid_positive_number")
        );
    }
}

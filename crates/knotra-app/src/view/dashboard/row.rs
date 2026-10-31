//! A single dashboard project row: identity, status/progress summary, and
//! the tier-specific action.

use iced::widget::{Space, button, column, container, rich_text, row, span, text};
use iced::{Alignment, Element, Length};
use knotra_ui::widget::{
    BUTTON_HEIGHT, STRONG_FONT, Tokens, centered_button_label, checkbox, status_badge, style,
};
use knotra_vcs::model::project::Project;

use crate::{
    message::{ConflictOpsMessage, DetailPanelMessage, Message, SelectionMessage},
    state::{
        AppState,
        dashboard::{
            DashboardCause, DashboardEntry, DashboardTier, ProgressKind, badge_for_all_set,
            badge_for_cause, badge_for_progress,
        },
        focus::FocusTarget,
    },
};

use super::width_mode::WidthMode;

/// Bounded row-track widths (R9), sized for the 1000-1279px standard-width
/// breakpoint (`.git-exclude/reviewed/062-...md` "Standard, 1000-1279px:
/// bounded three-track project rows"): identity, then status/progress, then
/// the tier action, each a fixed width rather than `FillPortion` so the
/// three columns line up down the entire list. Measured against a 1000px
/// window (`SIDEBAR_WIDTH` 180px leaves ~820px of content, minus the
/// dashboard body's 24px horizontal padding, the row's own 16px padding, and
/// 16px of inter-column spacing, leaves ~764px before a scrollbar) — the sum
/// here (700px of tracks + 16px of gaps = 716px) keeps a margin at that
/// floor. Stage 4 centres and may revisit these; they must not grow
/// indefinitely at wide widths (the audit's explicit warning), so this stage
/// keeps them fixed rather than picking something that only works at one
/// size.
const IDENTITY_TRACK_WIDTH: f32 = 280.0;
const MIDDLE_TRACK_WIDTH: f32 = 320.0;
const ACTION_TRACK_WIDTH: f32 = 100.0;

/// Stable per-row `FocusTarget` keys, shared between `dashboard/mod.rs`'s
/// `focus_order` and this module's [`is_focused`] — one expression per
/// target rather than the `format!` string duplicated in each place
/// (Handoff 025 §7.5, same discipline as `toolbar.rs`/`section.rs`).
pub(super) fn checkbox_key(id: &knotra_vcs::ProjectId) -> String {
    format!("dashboard.row.{id}.checkbox")
}

pub(super) fn name_key(id: &knotra_vcs::ProjectId) -> String {
    format!("dashboard.row.{id}.name")
}

pub(super) fn action_key(id: &knotra_vcs::ProjectId) -> String {
    format!("dashboard.row.{id}.action")
}

fn is_focused(state: &AppState, key: &str) -> bool {
    state.dashboard_focus.as_ref() == Some(&FocusTarget::control_dynamic(key.to_owned()))
}

/// RFC-071 D4: the project-name button's own text colour — `accent` on
/// hover, press, or keyboard focus; `text_primary` at rest. Kept in step
/// with `name_button`'s span, which applies the same rule statically for
/// `focused` (see that function's own doc comment for why hover alone
/// cannot drive the span's underline the same way).
fn project_name_style(
    tokens: &Tokens,
    status: iced::widget::button::Status,
    focused: bool,
) -> iced::widget::button::Style {
    use iced::widget::button::Status;

    let text_primary = snora::design::style::color::to_iced_color(tokens.palette.text_primary);
    let accent = snora::design::style::color::to_iced_color(tokens.palette.accent);
    let text_color = if focused || matches!(status, Status::Hovered | Status::Pressed) {
        accent
    } else {
        text_primary
    };
    iced::widget::button::Style {
        background: None,
        text_color,
        ..iced::widget::button::Style::default()
    }
}

pub(super) fn view_project_row<'a>(
    state: &'a AppState,
    entry: DashboardEntry<'a>,
    mode: WidthMode,
) -> Element<'a, Message> {
    let tokens = &state.theme.tokens;
    let project = entry.project;

    let checkbox_element = selection_checkbox(state, tokens, project);
    let name = name_button(state, tokens, project);
    let action = row_action(state, tokens, &entry, project);
    let work_area = entry
        .status
        .and_then(|status| status.context.as_ref())
        .map(|context| context.label.as_str())
        .unwrap_or(state.t("dashboard.work_area_unknown"));

    match mode {
        WidthMode::Compact => {
            view_compact_row(state, &entry, checkbox_element, name, action, work_area)
        }
        WidthMode::Standard | WidthMode::Wide => {
            view_standard_row(state, &entry, checkbox_element, name, action, work_area)
        }
    }
}

/// The selection checkbox, when selection mode is on — shared between the
/// standard and compact layouts, which place it differently but never
/// change its behaviour or focus target.
fn selection_checkbox<'a>(
    state: &'a AppState,
    tokens: &Tokens,
    project: &'a Project,
) -> Option<Element<'a, Message>> {
    if !state.selection_mode {
        return None;
    }
    let id = project.id.clone();
    let is_checked = state.selection.contains(&project.id);
    Some(checkbox(
        tokens,
        format!(
            "{} — {}",
            state.t("plain.selection.select_project"),
            project.name
        ),
        is_checked,
        move |_checked| Message::Selection(SelectionMessage::Toggled(id.clone())),
        is_focused(state, &checkbox_key(&project.id)),
    ))
}

/// RFC-071 D4: a project name is `text_primary` at rest — blue is for
/// what acts, and a name's own click target/message are unchanged, but it
/// no longer looks like a link by default. On hover or keyboard focus it
/// becomes `accent`, with an underline, so what is clickable is visible
/// when it matters (never only on hover — A1 D13 item 2).
///
/// **Hover and focus are not symmetric here, and that is reported, not
/// silently accepted**: a plain `iced::widget::Text`'s colour has no
/// per-`button::Status` hook of its own, so hover's colour change is driven
/// entirely by the button's own `.style(...)` closure (which iced already
/// re-evaluates every frame from real pointer state) — the name's `Span`
/// leaves its colour unset there, inheriting the button's. The underline
/// has no such ambient hook: `iced::widget::text::Style` carries only a
/// colour, and `Span::underline` is static content built once per `view()`
/// call, with no per-`Status` variant at all. Keyboard focus already is
/// app state the view rebuilds on, so the span can set its own colour and
/// underline statically whenever `focused` is true; mouse hover is not
/// tracked as app state anywhere, and adding live pointer-position
/// tracking to get a reactive underline on hover alone was judged
/// disproportionate to this one cosmetic detail.
fn name_button<'a>(
    state: &'a AppState,
    tokens: &Tokens,
    project: &'a Project,
) -> Element<'a, Message> {
    let focused = is_focused(state, &name_key(&project.id));
    let t = tokens.clone();

    let mut name_span: iced::advanced::text::Span<'_, ()> =
        span(project.name.as_str()).font(STRONG_FONT);
    if focused {
        name_span = name_span
            .color(snora::design::style::color::to_iced_color(
                tokens.palette.accent,
            ))
            .underline(true);
    }
    let label = rich_text![name_span]
        .size(snora::design::style::text::body_size(tokens))
        .line_height(snora::design::style::text::body_line_height(tokens));

    // RFC-071 A1 D13 item 2: a small "›" follows the name, in
    // `text_secondary`, always visible — so what a name does is
    // recognisable without hovering, not only on hover or focus.
    let cue = knotra_ui::widget::icon::colored_icon_element(
        &knotra_ui::widget::icon::chevron_right(),
        snora::design::style::text::body_small_size(tokens).0,
        snora::design::style::color::to_iced_color(tokens.palette.text_secondary),
    );

    let content = row![label, cue].spacing(2).align_y(Alignment::Center);

    button(content)
        .on_press(Message::DetailPanel(DetailPanelMessage::Opened(
            project.id.clone(),
        )))
        .style(move |_theme, status| {
            style::with_focus_ring(&t, focused, project_name_style(&t, status, focused))
        })
        .into()
}

/// The tier-specific row action — shared between layouts. `Space::new()` for
/// InProgress/AllSet (no action slot, per RFC-032 R10's tier density).
fn row_action<'a>(
    state: &'a AppState,
    tokens: &Tokens,
    entry: &DashboardEntry<'a>,
    project: &'a Project,
) -> Element<'a, Message> {
    if entry.tier != DashboardTier::NeedsHelp {
        return Space::new().into();
    }
    let focused = is_focused(state, &action_key(&project.id));
    if entry.cause == Some(DashboardCause::Conflict) {
        // The row's one mutating action (RFC-032 R10): opens the
        // conflict-resolution overlay, so it carries `secondary`'s
        // stronger weight.
        //
        // RFC-035 R13/Handoff 030 §6.2: no busy-reason text here anymore.
        // With multiple needs-help/conflict rows on screen, each printing
        // "Busy" while the interlock is held is the same duplication R13
        // targets in the selection bar, just repeated per row instead of
        // per action. The button still disables correctly (`on_press_maybe`
        // stays gated on `!is_busy()`); it just stops re-explaining why,
        // the same way row.rs's other actions never explained "busy" either.
        row_action_button(
            tokens,
            state.t("dashboard.resolve"),
            style::secondary,
            (!state.operation_interlock.is_busy()).then_some(Message::ConflictOps(
                ConflictOpsMessage::OpenRequested(Some(project.id.clone())),
            )),
            None,
            focused,
        )
    } else {
        // Dispatches the same `DetailPanel::Opened` message the project
        // name does — a link-weight affordance to the same destination,
        // not a distinct mutating action, so `ghost` matches the name's
        // own treatment rather than `secondary`.
        row_action_button(
            tokens,
            state.t("plain.show_details"),
            style::ghost,
            Some(Message::DetailPanel(DetailPanelMessage::Opened(
                project.id.clone(),
            ))),
            None,
            focused,
        )
    }
}

/// RFC-071 D3: the badge for a row's own status line — picks the one
/// `badge_for_*` function that matches what the row's status text already
/// shows (`cause_label` for `NeedsHelp`, the count line for `InProgress`,
/// `work_area` for `AllSet`), so the badge is never out of step with the
/// words beside it. The `None`/fallback cases mirror the status line's own
/// fallback text exactly: `cause_label` reads a missing cause the same way
/// `Some(StatusUnknown)` does, and the count line falls back to
/// `plain.status.unsaved_work` when `relevant_count` is `None`.
fn entry_badge(entry: &DashboardEntry<'_>) -> (knotra_ui::widget::Tone, snora::Icon) {
    match entry.tier {
        DashboardTier::NeedsHelp => {
            badge_for_cause(entry.cause.unwrap_or(DashboardCause::StatusUnknown))
        }
        DashboardTier::InProgress => badge_for_progress(
            entry
                .relevant_count
                .map(|c| c.kind)
                .unwrap_or(ProgressKind::Uncommitted),
        ),
        DashboardTier::AllSet => badge_for_all_set(),
    }
}

/// The `body` role's own rendered line height in pixels (`size ×
/// line_height` ratio) — D3's "the line height of the text it precedes",
/// computed rather than guessed, so the badge and the text beside it share
/// one vertical rhythm.
fn body_badge_diameter(tokens: &Tokens) -> f32 {
    tokens.typography.body.size * tokens.typography.body.line_height
}

/// Standard/wide layout (R9): bounded three-track row, unchanged from Stage 3.
fn view_standard_row<'a>(
    state: &'a AppState,
    entry: &DashboardEntry<'a>,
    checkbox_element: Option<Element<'a, Message>>,
    name: Element<'a, Message>,
    action: Element<'a, Message>,
    work_area: &'a str,
) -> Element<'a, Message> {
    let tokens = &state.theme.tokens;
    let mut identity = row![].spacing(4).align_y(Alignment::Center);
    if let Some(checkbox_element) = checkbox_element {
        identity = identity.push(checkbox_element);
    }
    let mut identity_details = column![name].spacing(2);
    if entry.tier == DashboardTier::NeedsHelp {
        let vcs = entry
            .status
            .map(|status| status.identity.vcs_kind.to_string())
            .unwrap_or_else(|| state.t("status.unknown").to_owned());
        identity_details = identity_details.push(
            text(vcs)
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens)),
        );
    }
    identity = identity.push(identity_details);

    // RFC-071 D1: a project's status line is `body`, same as D9's density
    // adjustments expect — the VCS sub-line above it (metadata) keeps
    // `body_small`. RFC-071 D3: the status line carries a tone badge before
    // its words — never colour alone, the badge's symbol and the words both
    // carry the meaning.
    let (tone, glyph) = entry_badge(entry);
    let diameter = body_badge_diameter(tokens);
    let middle: Element<'_, Message> = match entry.tier {
        DashboardTier::NeedsHelp => row![
            status_badge(tokens, tone, &glyph, diameter),
            text(cause_label(state, entry.cause))
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into(),
        DashboardTier::InProgress => {
            let count = entry
                .relevant_count
                .map(|count| format!("{}: {}", progress_label(state, count.kind), count.value))
                .unwrap_or_else(|| state.t("plain.status.unsaved_work").to_owned());
            column![
                text(work_area)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
                row![
                    status_badge(tokens, tone, &glyph, diameter),
                    text(count)
                        .size(snora::design::style::text::body_size(tokens))
                        .line_height(snora::design::style::text::body_line_height(tokens)),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            ]
            .spacing(2)
            .into()
        }
        DashboardTier::AllSet => row![
            status_badge(tokens, tone, &glyph, diameter),
            text(work_area)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into(),
    };

    container(
        row![
            container(identity).width(Length::Fixed(IDENTITY_TRACK_WIDTH)),
            container(middle).width(Length::Fixed(MIDDLE_TRACK_WIDTH)),
            container(action).width(Length::Fixed(ACTION_TRACK_WIDTH)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([7, 8])
    .into()
}

/// Compact layout (R8, 800-999px): two lines. Line one is identity and the
/// row action (pushed to the far edge by a `Fill` spacer, same idiom the
/// toolbar/shell use for pinning a control to the opposite end of a row);
/// line two is a single combined status/reason line. Tier density is
/// unchanged from the standard layout (RFC-032 R10) — this only rearranges
/// the same content onto two lines instead of three tracks.
fn view_compact_row<'a>(
    state: &'a AppState,
    entry: &DashboardEntry<'a>,
    checkbox_element: Option<Element<'a, Message>>,
    name: Element<'a, Message>,
    action: Element<'a, Message>,
    work_area: &'a str,
) -> Element<'a, Message> {
    let mut line_one = row![].spacing(4).align_y(Alignment::Center);
    if let Some(checkbox_element) = checkbox_element {
        line_one = line_one.push(checkbox_element);
    }
    line_one = line_one.push(name);
    line_one = line_one.push(Space::new().width(Length::Fill));
    line_one = line_one.push(action);

    let tokens = &state.theme.tokens;
    let line_two = compact_status_line(state, entry, work_area);
    let (tone, glyph) = entry_badge(entry);

    container(
        column![
            line_one,
            row![
                status_badge(tokens, tone, &glyph, body_badge_diameter(tokens)),
                text(line_two)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        ]
        .spacing(2),
    )
    .width(Length::Fill)
    .padding([7, 8])
    .into()
}

/// The compact layout's single status/reason line, folding what the
/// standard layout spreads across the identity column's VCS sub-line and
/// the middle track into one line per tier — same content, not more or
/// less (RFC-032 R10).
fn compact_status_line(state: &AppState, entry: &DashboardEntry<'_>, work_area: &str) -> String {
    match entry.tier {
        DashboardTier::NeedsHelp => {
            let vcs = entry
                .status
                .map(|status| status.identity.vcs_kind.to_string())
                .unwrap_or_else(|| state.t("status.unknown").to_owned());
            format!("{vcs} · {}", cause_label(state, entry.cause))
        }
        DashboardTier::InProgress => {
            let count = entry
                .relevant_count
                .map(|count| format!("{}: {}", progress_label(state, count.kind), count.value))
                .unwrap_or_else(|| state.t("plain.status.unsaved_work").to_owned());
            format!("{work_area} · {count}")
        }
        DashboardTier::AllSet => work_area.to_owned(),
    }
}

/// The row's tier-specific action (R6): `secondary` for the row's one
/// mutating action, `ghost` for a link-weight affordance — the caller
/// chooses which via `style_fn`, one of `style::secondary`/`style::ghost`'s
/// plain-function signatures (same shape `select_mode_button` in
/// `toolbar.rs` closes over, but parametrized here since a row's action
/// varies by cause). Replaces `guided_button`'s disabled-reason-text
/// composition on top of a real semantic style plus a working focus ring,
/// keyed to the row's `dashboard.row.{id}.action` target.
fn row_action_button<'a>(
    tokens: &Tokens,
    label: &'a str,
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
    on_press: Option<Message>,
    reason: Option<&'a str>,
    is_focused: bool,
) -> Element<'a, Message> {
    let t = tokens.clone();
    let show_reason = on_press.is_none();

    let btn = button(centered_button_label(
        text(label)
            .size(snora::design::style::text::body_small_size(tokens))
            .line_height(snora::design::style::text::body_small_line_height(tokens)),
    ))
    .height(BUTTON_HEIGHT)
    .padding([0, 12])
    .on_press_maybe(on_press)
    .style(move |_theme, status| style::with_focus_ring(&t, is_focused, style_fn(&t, status)));

    match reason {
        Some(r) if show_reason => column![
            btn,
            text(r)
                .size(snora::design::style::text::body_small_size(tokens))
                .line_height(snora::design::style::text::body_small_line_height(tokens))
        ]
        .spacing(4)
        .into(),
        _ => btn.into(),
    }
}

fn cause_label(state: &AppState, cause: Option<DashboardCause>) -> &'static str {
    match cause {
        Some(DashboardCause::MissingPath) => state.t("dashboard.cause.missing_path"),
        Some(DashboardCause::Conflict) => state.t("dashboard.cause.conflict"),
        Some(DashboardCause::ConflictDetectionUnavailable) => {
            state.t("dashboard.cause.conflict_detection_unavailable")
        }
        Some(DashboardCause::ReadUnavailable) => state.t("dashboard.cause.read_unavailable"),
        Some(DashboardCause::DetachedContext) => state.t("dashboard.cause.detached_context"),
        Some(DashboardCause::StatusUnknown) | None => state.t("dashboard.cause.status_unknown"),
    }
}

fn progress_label(state: &AppState, kind: ProgressKind) -> &'static str {
    match kind {
        ProgressKind::Uncommitted => state.t("dashboard.progress.uncommitted"),
        ProgressKind::Untracked => state.t("dashboard.progress.untracked"),
        ProgressKind::Ahead => state.t("dashboard.progress.ahead"),
        ProgressKind::Behind => state.t("dashboard.progress.behind"),
    }
}

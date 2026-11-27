//! A single dashboard project row: identity, status/progress summary, and
//! the tier-specific action.

use iced::widget::{Space, column, container, row, text};
use iced::{Alignment, Element, Length};
#[cfg(test)]
use knotra_ui::widget::centered_button_label;
use knotra_ui::widget::{
    Tokens, body_badge_diameter, centered_button_row, checkbox_control_unlabeled, name_link,
    status_badge,
};
use knotra_vcs::model::project::Project;

use crate::{
    message::{ConflictOpsMessage, DetailPanelMessage, Message, SelectionMessage},
    state::{
        AppState,
        dashboard::{
            DashboardCause, DashboardEntry, DashboardTier, ProgressKind, badge_for_all_set,
            badge_for_cause, badge_for_progress, workspace_has_mixed_vcs_kinds,
        },
    },
    view::ids,
};

use super::width_mode::WidthMode;

/// Bounded row-track widths (R9), sized for the 1000-1279px standard-width
/// breakpoint (`.git-exclude/reviewed/062-...md` "Standard, 1000-1279px:
/// bounded three-track project rows"): identity, then status/progress, then
/// the tier action, each a fixed width rather than `FillPortion` so the
/// three columns line up down the entire list. Originally measured against
/// a 1000px window (a now-removed 180px sidebar from an earlier shell
/// layout left ~820px of content, minus the dashboard body's 24px
/// horizontal padding, the row's own 16px padding, and 16px of
/// inter-column spacing, leaving ~764px before a scrollbar), with the
/// original sum (700px of tracks + 16px of gaps = 716px) keeping a margin
/// at that floor. The sidebar itself, and the `SIDEBAR_WIDTH` constant
/// that named it, are long gone from the shell (RFC-071 D8a removes the
/// leftover constant); this measurement stays historical rather than
/// rewritten to describe a shell that no longer exists. RFC-071 D8 then
/// grew `ACTION_TRACK_WIDTH` from 100px to 160px (see its own doc comment
/// below) — the sum is now 760px of tracks + 16px of gaps = 776px,
/// already past the old 764px sidebar-era floor; the floor itself no
/// longer applies (there is no sidebar to leave ~764px of content), and a
/// real build confirms today's three tracks still fit cleanly in a real
/// 1000px-wide window (the screenshots behind `ACTION_TRACK_WIDTH`'s own
/// measurement), so this is not a live regression — only the historical
/// arithmetic's own margin claim, which this note corrects rather than
/// leaves silently wrong. Stage 4 centres and may revisit these; they must
/// not grow indefinitely at wide widths (the audit's explicit warning), so
/// this stage keeps them fixed rather than picking something that only
/// works at one size.
const IDENTITY_TRACK_WIDTH: f32 = 280.0;
const MIDDLE_TRACK_WIDTH: f32 = 320.0;

/// RFC-071 D8: grown from 100px to fit `row_action_button`'s label at
/// `body` (16px) — Amendment A3 D14 moved that label off `body_small`
/// (14px) without revisiting this width, so the longest pair in either
/// language, "Choose resolution" / 「解決方法を選ぶ」 (`dashboard.resolve`,
/// the longer of the two possible action labels — confirmed by reading
/// `crates/knotra-ui/src/i18n.rs`), wrapped onto two lines at the old
/// value. Measured the same way Task 098 found its own button-padding
/// defect: a real build, a real NeedsHelp/conflict row at the Standard
/// band (~1040px window), screenshotted in both languages at each
/// candidate width. 100/140/150px wrapped "Choose resolution"; 160px was
/// the smallest round value (matching this file's own `IDENTITY_`/
/// `MIDDLE_TRACK_WIDTH` convention of clean round numbers) where both
/// languages rendered on one line with visible margin from the track's own
/// edge, at 160/180/200px alike — the button is content-sized, not
/// track-stretched, so 160px is the point past which a wider track buys
/// nothing.
const ACTION_TRACK_WIDTH: f32 = 160.0;

pub(super) fn view_project_row<'a>(
    state: &'a AppState,
    entry: DashboardEntry<'a>,
    mode: WidthMode,
) -> Element<'a, Message> {
    let tokens = &state.theme.tokens;
    let project = entry.project;

    let checkbox_element = selection_checkbox(state, tokens, project);
    let name = name_button(tokens, project);
    let action = row_action(tokens, state, &entry, project);
    // RFC-073 Amendment A6 §2: an unborn HEAD's own `context.label` is just
    // the branch name — on its own indistinguishable from an ordinary
    // branch with commits — so this composes "<branch> · No commits yet"
    // here, the one place both layouts read `work_area` from.
    let work_area = entry
        .status
        .and_then(|status| status.context.as_ref())
        .map(|context| {
            if context.unborn {
                format!(
                    "{} · {}",
                    context.label,
                    state.t("dashboard.no_commits_yet")
                )
            } else {
                context.label.clone()
            }
        })
        .unwrap_or_else(|| state.t("dashboard.work_area_unknown").to_owned());

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
    let accessible_label = format!(
        "{} — {}",
        state.t("plain.selection.select_project"),
        project.name
    );
    let box_widget = checkbox_control_unlabeled(
        tokens,
        is_checked,
        move |_checked| Message::Selection(SelectionMessage::Toggled(id.clone())),
        ids::row::checkbox(&project.id),
    );
    Some(
        iced::widget::tooltip(
            box_widget,
            knotra_ui::widget::overlay::raised_card(
                tokens,
                // RFC-071 Amendment A3 D14: a tooltip is a transient hint,
                // not primary content read to act — `body_small`, the same
                // size the old `checkbox`'s own accessible-label tooltip
                // used (`knotra_ui::widget::checkbox`).
                text(accessible_label)
                    .size(snora::design::style::text::body_small_size(tokens))
                    .line_height(snora::design::style::text::body_small_line_height(tokens)),
            ),
            iced::widget::tooltip::Position::Bottom,
        )
        .into(),
    )
}

/// RFC-075 stage 2 §3: the project-name link, on `knotra_ui::widget::
/// name_link` — see that primitive's own doc comment for the one "same
/// look" difference this stage reports (the underline-on-focus, lost
/// along with knotra's own manually-drawn ring).
fn name_button<'a>(tokens: &Tokens, project: &'a Project) -> Element<'a, Message> {
    name_link(
        tokens,
        project.name.clone(),
        Message::DetailPanel(DetailPanelMessage::Opened(project.id.clone())),
        ids::row::name(&project.id),
    )
}

/// RFC-074 Stage 3 D3: all five of the handoff's own conditions, in one
/// place, shared with [`crate::state::sync::would_pull`] — the same
/// "would this project's plan entry resolve to Pull" fact `build_plan`
/// uses — so a row's own button and the plan it opens can never disagree
/// about which projects qualify. "Not missing" and "not in selection
/// mode" are checked directly here rather than inferred from
/// `entry.tier` (a `path_missing` project is always `NeedsHelp`, so this
/// is belt-and-suspenders, the same style `palette.rs`'s own `workspace_
/// has_fetchable_project` and `any_upstream` checks already use).
pub(super) fn would_get_latest(
    state: &AppState,
    project_id: &knotra_vcs::ProjectId,
    status: Option<&knotra_vcs::ProjectStatus>,
) -> bool {
    if state.selection_mode || state.missing_projects.contains(project_id) {
        return false;
    }
    status.is_some_and(|status| status.remote.behind > 0 && crate::state::sync::would_pull(status))
}

/// The tier-specific row action — shared between layouts. `Space::new()`
/// when nothing qualifies (per RFC-032 R10's tier density: a row carries
/// at most one action, never more).
fn row_action<'a>(
    tokens: &Tokens,
    state: &'a AppState,
    entry: &DashboardEntry<'a>,
    project: &'a Project,
) -> Element<'a, Message> {
    if entry.tier != DashboardTier::NeedsHelp {
        // RFC-074 Stage 3 D3: the one other row action, "Get latest
        // safely" — never alongside the NeedsHelp actions below (a row
        // never shows two actions), since `would_get_latest` requires no
        // conflict and a clean work area, both of which put a project in
        // `NeedsHelp`/`InProgress`'s dirty tiers instead, never here.
        return if would_get_latest(state, &project.id, entry.status) {
            row_action_control(
                tokens,
                state.t("plain.get_latest"),
                (!state.operation_interlock.is_busy()).then_some(Message::Sync(
                    crate::message::SyncMessage::RowPullRequested(project.id.clone()),
                )),
                knotra_ui::widget::style::secondary,
                ids::row::get_latest(&project.id),
            )
        } else {
            Space::new().into()
        };
    }
    let id = ids::row::action(&project.id);
    if entry.cause == Some(DashboardCause::Conflict) {
        // The row's one mutating action (RFC-032 R10): opens the
        // conflict-resolution overlay, so it carries `secondary`'s
        // stronger weight.
        //
        // RFC-035 R13/Handoff 030 §6.2: no busy-reason text here anymore.
        // With multiple needs-help/conflict rows on screen, each printing
        // "Busy" while the interlock is held is the same duplication R13
        // targets in the selection bar, just repeated per row instead of
        // per action. The control still disables correctly (`on_press_maybe`
        // stays gated on `!is_busy()`); it just stops re-explaining why,
        // the same way row.rs's other actions never explained "busy" either.
        row_action_control(
            tokens,
            state.t("dashboard.resolve"),
            (!state.operation_interlock.is_busy()).then_some(Message::ConflictOps(
                ConflictOpsMessage::OpenRequested(Some(project.id.clone())),
            )),
            knotra_ui::widget::style::secondary,
            id,
        )
    } else {
        // Dispatches the same `DetailPanel::Opened` message the project
        // name does — a link-weight affordance to the same destination,
        // not a distinct mutating action, so `ghost` matches the name's
        // own treatment rather than `secondary`.
        row_action_control(
            tokens,
            state.t("plain.show_details"),
            Some(Message::DetailPanel(DetailPanelMessage::Opened(
                project.id.clone(),
            ))),
            knotra_ui::widget::style::ghost,
            id,
        )
    }
}

/// Amendment A1 §3 (Review 299 F3): `knotra_ui::widget::secondary_control_
/// maybe`/`ghost_control` both build on `buttons.rs`'s shared
/// `make_control`, whose `[0, 18]` padding is right for a *standalone*
/// button but is not what [`ACTION_TRACK_WIDTH`] was measured against —
/// the pre-stage-2 `row_action_button` used `[0, 12]`, and the 160px track
/// was sized around *that* box, not an 18px one. The extra 6px of padding
/// on each side was enough to visibly shift the label right and push the
/// box's own right edge onto the track's border (seen directly in C2, at
/// x 636-796 — the track's own right edge is x 796). Rebuilt here, on the
/// same `snora::design::control` the generic vocabulary itself uses, with
/// the one byte-for-byte difference that actually matters: the padding.
fn row_action_control<'a, M: Clone + 'a>(
    tokens: &Tokens,
    label: &'a str,
    on_press: Option<M>,
    style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
    id: iced::widget::Id,
) -> iced::Element<'a, M> {
    let t = tokens.clone();
    snora::design::control(
        tokens,
        centered_button_row(iced::widget::row![
            text(label)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens))
        ]),
    )
    .height(Length::Fixed(knotra_ui::widget::BUTTON_HEIGHT))
    .id(id)
    .padding([0.0, 12.0])
    .on_press_maybe(on_press)
    .style(move |_theme, status| style_fn(&t, status))
    .into()
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

/// Standard/wide layout (R9): bounded three-track row, unchanged from Stage 3.
fn view_standard_row<'a>(
    state: &'a AppState,
    entry: &DashboardEntry<'a>,
    checkbox_element: Option<Element<'a, Message>>,
    name: Element<'a, Message>,
    action: Element<'a, Message>,
    work_area: String,
) -> Element<'a, Message> {
    let tokens = &state.theme.tokens;
    let mut identity = row![].spacing(4).align_y(Alignment::Center);
    if let Some(checkbox_element) = checkbox_element {
        identity = identity.push(checkbox_element);
    }
    let mut identity_details = column![name].spacing(2);
    // RFC-073 D3: the kind label tells something only when the workspace
    // holds both Git and Jujutsu projects.
    if entry.tier == DashboardTier::NeedsHelp
        && workspace_has_mixed_vcs_kinds(state.workspace_status.as_ref())
    {
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
            // RFC-071 Amendment A3 D9: one status line, joining the two
            // existing catalog texts with " · " — no new wording — the
            // same one-line shape NeedsHelp and AllSet already use below,
            // instead of `work_area` and `count` on two separate lines.
            row![
                status_badge(tokens, tone, &glyph, diameter),
                text(format!("{work_area} · {count}"))
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens)),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
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
    work_area: String,
) -> Element<'a, Message> {
    let mut line_one = row![].spacing(4).align_y(Alignment::Center);
    if let Some(checkbox_element) = checkbox_element {
        line_one = line_one.push(checkbox_element);
    }
    line_one = line_one.push(name);
    line_one = line_one.push(Space::new().width(Length::Fill));
    line_one = line_one.push(action);

    let tokens = &state.theme.tokens;
    let line_two = compact_status_line(state, entry, &work_area);
    let (tone, glyph) = entry_badge(entry);

    container(
        column![
            line_one,
            row![
                status_badge(tokens, tone, &glyph, body_badge_diameter(tokens)),
                // RFC-071 Amendment A3 D9: `Length::Fill` so this line
                // wraps instead of rendering unbounded past the window's
                // edge — a `Row` with no `Fill` child sizes itself to its
                // children's natural width, and `text` only wraps when its
                // own layout width is bounded.
                text(line_two)
                    .size(snora::design::style::text::body_size(tokens))
                    .line_height(snora::design::style::text::body_line_height(tokens))
                    .width(Length::Fill),
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
        // RFC-073 D3: the kind label tells something only when the
        // workspace holds both Git and Jujutsu projects.
        DashboardTier::NeedsHelp
            if workspace_has_mixed_vcs_kinds(state.workspace_status.as_ref()) =>
        {
            let vcs = entry
                .status
                .map(|status| status.identity.vcs_kind.to_string())
                .unwrap_or_else(|| state.t("status.unknown").to_owned());
            format!("{vcs} · {}", cause_label(state, entry.cause))
        }
        DashboardTier::NeedsHelp => cause_label(state, entry.cause).to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use knotra_ui::test_support::{Harness, pixel_diff};
    use knotra_ui::theme::KnotraTheme;

    #[derive(Debug, Clone, PartialEq)]
    enum Msg {
        Pressed,
    }

    /// The exact pre-stage-2 construction (`row_action_button`, deleted by
    /// RFC-075 §3), reproduced here **only** for this one comparison, the
    /// same discipline `knotra-ui`'s own `name_link` test uses: never
    /// called by any live view, just a frozen record of what the look at
    /// rest *was*.
    fn old_row_action_button<'a>(
        tokens: &Tokens,
        label: &'a str,
        style_fn: fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style,
        on_press: Option<Msg>,
    ) -> iced::Element<'a, Msg> {
        let t = tokens.clone();
        iced::widget::button(centered_button_label(
            text(label)
                .size(snora::design::style::text::body_size(tokens))
                .line_height(snora::design::style::text::body_line_height(tokens)),
        ))
        .height(Length::Fixed(knotra_ui::widget::BUTTON_HEIGHT))
        .padding([0, 12])
        .on_press_maybe(on_press)
        .style(move |_theme, status| style_fn(&t, status))
        .into()
    }

    /// Amendment A1 §3: the row action's look at rest, restored — compared
    /// byte-for-byte against the frozen pre-stage-2 construction, for both
    /// labels (`style::secondary`'s "Choose resolution"/「解決方法を選ぶ」,
    /// `style::ghost`'s "Show details"/「詳細を表示」) in English and
    /// Japanese, in all four themes.
    ///
    /// **Not also rendered in both layouts**: `row_action` itself builds
    /// this one `Element` exactly once per row (`view_project_row`, above)
    /// and hands the *same* value to whichever of `view_standard_row`/
    /// `view_compact_row` draws it — there is only one construction to
    /// compare, not two; the layouts differ in where they place it, never
    /// in what it is.
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "renders and reads back pixels — the pinned face (Noto Sans) is installed \
                  only on the Linux CI runner (Task 108 A1 §4, `label_overflow_guard.rs`'s own \
                  precedent); the pixels this compares would be some other font's on \
                  macOS/Windows"
    )]
    fn row_action_draws_identically_to_the_old_button_at_rest_in_every_theme_and_locale() {
        let size = iced_test::core::Size {
            width: 200.0,
            height: 60.0,
        };
        let icon_font = Some(knotra_ui::widget::icon::FONT_BYTES);

        type StyleFn = fn(&Tokens, iced::widget::button::Status) -> iced::widget::button::Style;
        let labels: [(&str, &str, StyleFn); 4] = [
            (
                "Choose resolution",
                "en",
                knotra_ui::widget::style::secondary,
            ),
            ("解決方法を選ぶ", "ja", knotra_ui::widget::style::secondary),
            ("Show details", "en", knotra_ui::widget::style::ghost),
            ("詳細を表示", "ja", knotra_ui::widget::style::ghost),
        ];

        for (name, theme) in [
            ("light", KnotraTheme::light()),
            ("dark", KnotraTheme::dark()),
            ("high_contrast_light", KnotraTheme::high_contrast_light()),
            ("high_contrast_dark", KnotraTheme::high_contrast_dark()),
        ] {
            let t = &theme.tokens;
            for (label, locale, style_fn) in labels {
                let old: iced::Element<'static, Msg> =
                    old_row_action_button(t, label, style_fn, Some(Msg::Pressed));
                let new: iced::Element<'static, Msg> = row_action_control(
                    t,
                    label,
                    Some(Msg::Pressed),
                    style_fn,
                    iced::widget::Id::new("action"),
                );

                let mut h_old = Harness::new(old, size, icon_font);
                let (old_rgba, ow, oh) = h_old.screenshot(&theme.base);
                let mut h_new = Harness::new(new, size, icon_font);
                let (new_rgba, nw, nh) = h_new.screenshot(&theme.base);

                if let Err(diff) = pixel_diff(&old_rgba, (ow, oh), &new_rgba, (nw, nh)) {
                    panic!("{name}/{locale}/\"{label}\": not identical at rest: {diff}");
                }
            }
        }
    }
}

#[cfg(test)]
mod row_get_latest_tests {
    use super::*;
    use crate::config::AppConfig;
    use knotra_vcs::{
        ConflictStatus, Project, ProjectId, ProjectStatus, RemoteStatus, RepositoryIdentity,
        VcsKind, WorkingTreeStatus, Workspace, WorkspaceStatus,
    };

    fn state_with_project(
        behind: u32,
        uncommitted: u32,
        has_conflict: bool,
        upstream: Option<&str>,
        missing: bool,
        selection_mode: bool,
    ) -> (AppState, ProjectId) {
        let mut state = AppState::new(AppConfig::default());
        let project = Project::new("svc", "/tmp");
        let id = project.id.clone();
        state.workspace = Some(Workspace {
            projects: vec![project],
            ..Workspace::new("Main")
        });
        state.workspace_status = Some(WorkspaceStatus {
            projects: vec![ProjectStatus {
                project_id: id.clone(),
                identity: RepositoryIdentity {
                    path: "/tmp".into(),
                    vcs_kind: VcsKind::Git,
                },
                context: None,
                remote: RemoteStatus {
                    ahead: 0,
                    behind,
                    upstream: upstream.map(str::to_owned),
                },
                working_tree: WorkingTreeStatus {
                    uncommitted_count: uncommitted,
                    untracked_count: 0,
                },
                conflict: ConflictStatus {
                    has_conflict,
                    conflict_count: None,
                    detection_unavailable: false,
                },
                refreshed_at: jiff::Timestamp::now(),
                read_error: None,
            }],
            last_refresh: None,
        });
        if missing {
            state.missing_projects.insert(id.clone());
        }
        state.selection_mode = selection_mode;
        (state, id)
    }

    /// Handoff §2 row 1/2: one row per failing condition, and one that
    /// passes everything — "the button and the plan agree" is checked in
    /// the same case, each time the button would show: a one-project plan
    /// for exactly that project must resolve to `Pull`, via the same
    /// shared `would_pull` the button itself calls.
    #[test]
    fn would_get_latest_holds_only_when_every_condition_holds() {
        struct Case {
            name: &'static str,
            behind: u32,
            uncommitted: u32,
            has_conflict: bool,
            upstream: Option<&'static str>,
            missing: bool,
            selection_mode: bool,
            expected: bool,
        }
        let cases = [
            Case {
                name: "passes everything",
                behind: 2,
                uncommitted: 0,
                has_conflict: false,
                upstream: Some("origin/main"),
                missing: false,
                selection_mode: false,
                expected: true,
            },
            Case {
                name: "unsaved work",
                behind: 2,
                uncommitted: 1,
                has_conflict: false,
                upstream: Some("origin/main"),
                missing: false,
                selection_mode: false,
                expected: false,
            },
            Case {
                name: "no shared source",
                behind: 0,
                uncommitted: 0,
                has_conflict: false,
                upstream: None,
                missing: false,
                selection_mode: false,
                expected: false,
            },
            Case {
                name: "a needed choice",
                behind: 2,
                uncommitted: 0,
                has_conflict: true,
                upstream: Some("origin/main"),
                missing: false,
                selection_mode: false,
                expected: false,
            },
            Case {
                name: "missing",
                behind: 2,
                uncommitted: 0,
                has_conflict: false,
                upstream: Some("origin/main"),
                missing: true,
                selection_mode: false,
                expected: false,
            },
            Case {
                name: "selection mode",
                behind: 2,
                uncommitted: 0,
                has_conflict: false,
                upstream: Some("origin/main"),
                missing: false,
                selection_mode: true,
                expected: false,
            },
            // Beyond the handoff's own listed 5: "not behind" (up to
            // date) — part of D3's own condition list ("its project
            // shows 'Updates available'"), added for completeness.
            Case {
                name: "not behind (up to date)",
                behind: 0,
                uncommitted: 0,
                has_conflict: false,
                upstream: Some("origin/main"),
                missing: false,
                selection_mode: false,
                expected: false,
            },
        ];

        for case in cases {
            let (state, id) = state_with_project(
                case.behind,
                case.uncommitted,
                case.has_conflict,
                case.upstream,
                case.missing,
                case.selection_mode,
            );
            let status = state.workspace_status.as_ref().unwrap().projects.first();
            let got = would_get_latest(&state, &id, status);
            assert_eq!(got, case.expected, "case {:?}", case.name);

            if got {
                let ws = state.workspace.as_ref().unwrap();
                let mut sync_state = crate::state::sync::SyncCenterState::default();
                sync_state.project_selection.insert(id.clone(), true);
                let plan = sync_state.build_plan(&ws.projects, state.workspace_status.as_ref());
                assert_eq!(
                    plan.entries[0].disposition,
                    knotra_vcs::SmartPullDisposition::Pull,
                    "case {:?}: the button and the plan must agree",
                    case.name
                );
            }
        }
    }

    /// Handoff §2 row 4: a row that needs help keeps only its own action
    /// — never alongside "Get latest safely", even rendered through the
    /// real dashboard view, not just the pure predicate.
    #[test]
    fn needs_help_row_never_shows_two_actions() {
        let (state, _id) = state_with_project(0, 0, true, Some("origin/main"), false, false);
        let mut ui = crate::headless::simulator(
            crate::view::dashboard::view(&state, crate::view::dashboard::WidthMode::Standard),
            iced_test::core::Size::new(1280.0, 800.0),
        );
        assert!(ui.find(state.t("dashboard.resolve")).is_ok());
        assert!(ui.find(state.t("plain.get_latest")).is_err());
    }

    /// Handoff §2 row 5: "the label never cut", in either band — this
    /// row's own control, not `label_overflow_guard.rs`'s shared matrix
    /// (which would have two identically-labelled "Get latest safely"
    /// controls on screen at once — this row's own and the toolbar's,
    /// `ui.find` cannot tell apart; this test renders the row alone,
    /// through `view_project_row` directly, the same isolation
    /// `label_overflow_guard.rs`'s own detail-panel test already uses for
    /// the identical reason, see that test's own doc comment).
    ///
    /// **Seen failing first**, confirmed directly — but the defect was in
    /// this test, not the product: its own first version iterated
    /// `WidthMode::Compact`/`Standard` independent of the logical width,
    /// so it paired `Standard` (three fixed-width tracks, ~776px minimum)
    /// with a 150%-zoom 800px window (logical ≈533px) — a combination
    /// the real app never produces (`view.rs`'s own `state.width_mode` is
    /// always *derived from* the logical width, the same discipline
    /// `label_overflow_guard.rs`'s own matrix already follows and this
    /// one did not, at first). That combination failed loudly (the label
    /// not found at its usual bounds at all, not merely narrowed).
    /// Deriving `mode` from `logical.width` via `WidthMode::from_width`
    /// instead — matching the real app and the existing guard — passes
    /// cleanly against `ACTION_TRACK_WIDTH`'s own existing 160px,
    /// unchanged: 「安全に最新を取得」's natural width did fit after all.
    #[test]
    #[cfg_attr(
        not(target_os = "linux"),
        ignore = "the pinned faces (Noto Sans, Noto Sans CJK JP) are installed only on the \
                  Linux CI runner (Task 108 A1 §4) — the widths this guard measures would be \
                  some other font's on macOS/Windows"
    )]
    fn row_get_latest_label_never_cut_in_either_band() {
        use crate::state::dashboard::RelevantCount;
        use crate::view::dashboard::WidthMode;

        const REFERENCE_SIZE: iced_test::core::Size = iced_test::core::Size {
            width: 4000.0,
            height: 2000.0,
        };

        let mut checked = 0usize;
        let mut failures: Vec<String> = Vec::new();

        for &locale in &[knotra_ui::i18n::Locale::En, knotra_ui::i18n::Locale::Ja] {
            for &zoom_percent in &[100.0_f32, 150.0_f32] {
                for &width in &[800.0_f32, 1280.0_f32] {
                    let (mut state, _id) =
                        state_with_project(2, 0, false, Some("origin/main"), false, false);
                    state.catalog = knotra_ui::i18n::Catalog::for_locale(locale);
                    let project = &state.workspace.as_ref().unwrap().projects[0];
                    let status = state.workspace_status.as_ref().unwrap().projects.first();
                    let entry = DashboardEntry {
                        project,
                        status,
                        tier: DashboardTier::InProgress,
                        cause: None,
                        relevant_count: Some(RelevantCount {
                            kind: ProgressKind::Behind,
                            value: 2,
                        }),
                    };
                    let label = state.t("plain.get_latest").to_owned();

                    let logical = crate::headless::logical_size_at_zoom(
                        iced_test::core::Size::new(width, 900.0),
                        zoom_percent,
                    );
                    // The real app derives the band from the logical
                    // width (`view.rs`'s own `state.width_mode`), never
                    // picks one independent of it — Standard's three
                    // fixed-width tracks are not meant to fit a Compact-
                    // band width at all, so pairing them freely would
                    // check a combination the real app never produces.
                    let mode = WidthMode::from_width(logical.width);
                    let context = format!(
                        "locale={locale:?} zoom={zoom_percent}% width={width} mode={mode:?}"
                    );

                    let mut ui = crate::headless::simulator(
                        view_project_row(&state, entry.clone(), mode),
                        logical,
                    );
                    let mut reference = crate::headless::simulator(
                        view_project_row(&state, entry, mode),
                        REFERENCE_SIZE,
                    );

                    let actual = ui.find(label.as_str());
                    let natural = reference.find(label.as_str());
                    match (actual, natural) {
                        (Ok(a), Ok(n)) => {
                            let (ab, nb) = (a.bounds(), n.bounds());
                            if (ab.width - nb.width).abs() > 0.5 {
                                failures.push(format!(
                                    "{context}: width {:.2} != natural {:.2} (clamped)",
                                    ab.width, nb.width
                                ));
                            }
                            if ab.height != nb.height {
                                failures.push(format!(
                                    "{context}: height {:.2} != natural {:.2}",
                                    ab.height, nb.height
                                ));
                            }
                            if ab.x + ab.width > logical.width + 0.5 {
                                failures.push(format!(
                                    "{context}: right edge {:.2} exceeds the viewport's {:.2}",
                                    ab.x + ab.width,
                                    logical.width
                                ));
                            }
                        }
                        (Err(e), _) => failures.push(format!("{context}: not found: {e}")),
                        (_, Err(e)) => {
                            failures.push(format!("{context}: not found at reference size: {e}"))
                        }
                    }
                    checked += 1;
                }
            }
        }

        assert!(checked > 0, "the matrix must check at least one case");
        eprintln!("row.rs: checked {checked} row get-latest label cases");
        assert!(
            failures.is_empty(),
            "{} of {checked} cases clamped, wrapped or ran off-screen:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}

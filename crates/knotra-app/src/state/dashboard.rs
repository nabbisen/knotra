//! Pure dashboard classification, filtering, grouping, and ordering.

use std::{cmp::Ordering, collections::HashSet};

use knotra_ui::widget::{Tone, status_symbol as symbol};
use knotra_vcs::{ProjectId, WorkspaceStatus, model::project::Project};

use crate::{
    config::{DashboardGrouping, DashboardSort},
    message::StatusFilter,
    state::FilterState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DashboardTier {
    NeedsHelp,
    InProgress,
    AllSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardCause {
    MissingPath,
    Conflict,
    ConflictDetectionUnavailable,
    ReadUnavailable,
    DetachedContext,
    StatusUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProgressKind {
    Uncommitted,
    Untracked,
    Ahead,
    Behind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelevantCount {
    pub kind: ProgressKind,
    pub value: u32,
}

#[derive(Debug, Clone)]
pub struct DashboardEntry<'a> {
    pub project: &'a Project,
    pub status: Option<&'a knotra_vcs::ProjectStatus>,
    pub tier: DashboardTier,
    pub cause: Option<DashboardCause>,
    pub relevant_count: Option<RelevantCount>,
}

// --- RFC-071 D3 / Amendment A4: tone and symbol ----------------------------
//
// A badge goes on each row's status line and on each chip — never on a
// group heading, which is structure, not a status (A4). Every function here
// is exhaustive over its own enum, with no `_` arm, so a new `StatusFilter`,
// `DashboardCause` or `ProgressKind` variant fails to compile without a
// tone and a symbol, rather than silently rendering with neither.

/// A chip's badge. Every `StatusFilter` variant is one of D3's five rows:
/// `NeedsHelp`/`Conflict` both read "Needs help"/"Needs a choice" and share
/// `danger`; the rest are each their own row.
#[must_use]
pub fn badge_for_filter(filter: &StatusFilter) -> (Tone, snora::Icon) {
    match filter {
        StatusFilter::NeedsHelp | StatusFilter::Conflict => (Tone::Danger, symbol::alert()),
        StatusFilter::Dirty => (Tone::Warning, symbol::unsaved()),
        StatusFilter::Behind => (Tone::Info, symbol::updates_available()),
        StatusFilter::Ahead => (Tone::Info, symbol::local_commits()),
        StatusFilter::AllSet => (Tone::Success, symbol::all_set()),
    }
}

/// A needs-help row's badge. Every `DashboardCause` variant is a reason a
/// project needs help — none of them is "all set" or "in progress" — so
/// every one shares D3's "Needs help, Needs a choice, unreadable" row:
/// `danger`, alert.
#[must_use]
pub fn badge_for_cause(cause: DashboardCause) -> (Tone, snora::Icon) {
    match cause {
        DashboardCause::MissingPath
        | DashboardCause::Conflict
        | DashboardCause::ConflictDetectionUnavailable
        | DashboardCause::ReadUnavailable
        | DashboardCause::DetachedContext
        | DashboardCause::StatusUnknown => (Tone::Danger, symbol::alert()),
    }
}

/// An in-progress row's badge. `Uncommitted`/`Untracked` are both "work not
/// yet saved to the repository" (Review 250 §3 confirms `Untracked` here) —
/// D3's "Unsaved work" row, `warning`. `Ahead`/`Behind` are D3's "Local
/// commits"/"Updates available" rows, both `info` but with different
/// arrows.
#[must_use]
pub fn badge_for_progress(kind: ProgressKind) -> (Tone, snora::Icon) {
    match kind {
        ProgressKind::Uncommitted | ProgressKind::Untracked => (Tone::Warning, symbol::unsaved()),
        ProgressKind::Ahead => (Tone::Info, symbol::local_commits()),
        ProgressKind::Behind => (Tone::Info, symbol::updates_available()),
    }
}

/// An all-set row's badge. D3's "All set" row: `success`, check. A plain
/// function rather than a one-armed match over a unit case, since there is
/// no enum to be exhaustive over here.
#[must_use]
pub fn badge_for_all_set() -> (Tone, snora::Icon) {
    (Tone::Success, symbol::all_set())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardSectionKey {
    Tier(DashboardTier),
    ProjectGroup(Option<String>),
    Flat,
}

#[derive(Debug, Clone)]
pub struct DashboardSection<'a> {
    pub key: DashboardSectionKey,
    pub collapsed: bool,
    pub entries: Vec<DashboardEntry<'a>>,
}

#[derive(Debug, Clone, Default)]
pub struct DashboardDisplay<'a> {
    pub sections: Vec<DashboardSection<'a>>,
    pub ordered_selectable_ids: Vec<ProjectId>,
}

#[derive(Debug, Clone, Copy)]
pub struct DashboardDisplayOptions {
    pub grouping: DashboardGrouping,
    pub sort: DashboardSort,
    pub in_progress_collapsed: bool,
    pub all_set_collapsed: bool,
}

/// Every project, classified, before `FilterState` (search text, active
/// group, status-filter toggles) narrows the list. Shared by
/// [`build_dashboard_display`] and [`count_for_filter`] — RFC-071 D5's chip
/// counts read from this same unfiltered set, so toggling one chip never
/// moves another chip's own number.
fn classify_all<'a>(
    projects: &'a [Project],
    workspace_status: Option<&'a WorkspaceStatus>,
    missing_projects: &HashSet<ProjectId>,
) -> Vec<DashboardEntry<'a>> {
    let statuses = workspace_status
        .map(|status| status.projects.as_slice())
        .unwrap_or_default();
    projects
        .iter()
        .map(|project| {
            let status = statuses
                .iter()
                .find(|status| status.project_id == project.id);
            classify(project, status, missing_projects.contains(&project.id))
        })
        .collect()
}

/// RFC-071 D5: a chip's own count — how many of the full, unfiltered
/// classified set match `filter`.
#[must_use]
pub fn count_for_filter(
    projects: &[Project],
    workspace_status: Option<&WorkspaceStatus>,
    missing_projects: &HashSet<ProjectId>,
    filter: &StatusFilter,
) -> usize {
    classify_all(projects, workspace_status, missing_projects)
        .iter()
        .filter(|entry| matches_status_filter(entry, filter))
        .count()
}

pub fn build_dashboard_display<'a>(
    projects: &'a [Project],
    workspace_status: Option<&'a WorkspaceStatus>,
    missing_projects: &HashSet<ProjectId>,
    filter: &FilterState,
    options: DashboardDisplayOptions,
) -> DashboardDisplay<'a> {
    let mut entries: Vec<_> = classify_all(projects, workspace_status, missing_projects)
        .into_iter()
        .filter(|entry| matches_filter(entry, filter))
        .collect();

    sort_entries(&mut entries, options.sort);

    let sections = match options.grouping {
        DashboardGrouping::Attention => attention_sections(
            entries,
            options.in_progress_collapsed,
            options.all_set_collapsed,
        ),
        DashboardGrouping::ProjectGroup => project_group_sections(entries),
        DashboardGrouping::None => (!entries.is_empty())
            .then_some(DashboardSection {
                key: DashboardSectionKey::Flat,
                collapsed: false,
                entries,
            })
            .into_iter()
            .collect(),
    };
    let ordered_selectable_ids = sections
        .iter()
        .filter(|section| !section.collapsed)
        .flat_map(|section| section.entries.iter())
        .map(|entry| entry.project.id.clone())
        .collect();

    DashboardDisplay {
        sections,
        ordered_selectable_ids,
    }
}

fn classify<'a>(
    project: &'a Project,
    status: Option<&'a knotra_vcs::ProjectStatus>,
    path_missing: bool,
) -> DashboardEntry<'a> {
    let (tier, cause, relevant_count) = if path_missing {
        (
            DashboardTier::NeedsHelp,
            Some(DashboardCause::MissingPath),
            None,
        )
    } else if let Some(status) = status {
        if status.conflict.has_conflict {
            (
                DashboardTier::NeedsHelp,
                Some(DashboardCause::Conflict),
                None,
            )
        } else if status.conflict.detection_unavailable {
            (
                DashboardTier::NeedsHelp,
                Some(DashboardCause::ConflictDetectionUnavailable),
                None,
            )
        } else if status.read_error.is_some() {
            (
                DashboardTier::NeedsHelp,
                Some(DashboardCause::ReadUnavailable),
                None,
            )
        } else if status
            .context
            .as_ref()
            .is_some_and(|context| context.is_detached)
        {
            (
                DashboardTier::NeedsHelp,
                Some(DashboardCause::DetachedContext),
                None,
            )
        } else if let Some(relevant_count) = relevant_count(status) {
            (DashboardTier::InProgress, None, Some(relevant_count))
        } else {
            (DashboardTier::AllSet, None, None)
        }
    } else {
        (
            DashboardTier::NeedsHelp,
            Some(DashboardCause::StatusUnknown),
            None,
        )
    };

    DashboardEntry {
        project,
        status,
        tier,
        cause,
        relevant_count,
    }
}

fn relevant_count(status: &knotra_vcs::ProjectStatus) -> Option<RelevantCount> {
    [
        (
            ProgressKind::Uncommitted,
            status.working_tree.uncommitted_count,
        ),
        (ProgressKind::Untracked, status.working_tree.untracked_count),
        (ProgressKind::Ahead, status.remote.ahead),
        (ProgressKind::Behind, status.remote.behind),
    ]
    .into_iter()
    .find(|(_, value)| *value > 0)
    .map(|(kind, value)| RelevantCount { kind, value })
}

fn matches_filter(entry: &DashboardEntry<'_>, filter: &FilterState) -> bool {
    if !filter.search_text.is_empty()
        && !entry
            .project
            .name
            .to_lowercase()
            .contains(&filter.search_text.to_lowercase())
    {
        return false;
    }
    if let Some(group) = &filter.active_group
        && entry.project.group.as_deref() != Some(group.as_str())
    {
        return false;
    }
    filter.status_filters.is_empty()
        || filter
            .status_filters
            .iter()
            .any(|status_filter| matches_status_filter(entry, status_filter))
}

fn matches_status_filter(entry: &DashboardEntry<'_>, filter: &StatusFilter) -> bool {
    let usable_status_facts = !matches!(
        entry.cause,
        Some(
            DashboardCause::MissingPath
                | DashboardCause::ReadUnavailable
                | DashboardCause::StatusUnknown
        )
    );
    match filter {
        StatusFilter::AllSet => entry.tier == DashboardTier::AllSet,
        StatusFilter::Behind => {
            usable_status_facts && entry.status.is_some_and(|status| status.remote.behind > 0)
        }
        StatusFilter::Ahead => {
            usable_status_facts && entry.status.is_some_and(|status| status.remote.ahead > 0)
        }
        StatusFilter::Dirty => {
            usable_status_facts
                && entry
                    .status
                    .is_some_and(|status| status.working_tree.is_dirty())
        }
        StatusFilter::Conflict => {
            usable_status_facts
                && entry
                    .status
                    .is_some_and(|status| status.conflict.has_conflict)
        }
        StatusFilter::NeedsHelp => entry.tier == DashboardTier::NeedsHelp,
    }
}

fn attention_sections<'a>(
    entries: Vec<DashboardEntry<'a>>,
    in_progress_collapsed: bool,
    all_set_collapsed: bool,
) -> Vec<DashboardSection<'a>> {
    [
        (DashboardTier::NeedsHelp, false),
        (DashboardTier::InProgress, in_progress_collapsed),
        (DashboardTier::AllSet, all_set_collapsed),
    ]
    .into_iter()
    .filter_map(|(tier, collapsed)| {
        let tier_entries: Vec<_> = entries
            .iter()
            .filter(|entry| entry.tier == tier)
            .cloned()
            .collect();
        (!tier_entries.is_empty()).then_some(DashboardSection {
            key: DashboardSectionKey::Tier(tier),
            collapsed,
            entries: tier_entries,
        })
    })
    .collect()
}

fn project_group_sections<'a>(entries: Vec<DashboardEntry<'a>>) -> Vec<DashboardSection<'a>> {
    let mut names: Vec<String> = entries
        .iter()
        .filter_map(|entry| entry.project.group.clone())
        .collect();
    names.sort_by(|left, right| {
        left.to_lowercase()
            .cmp(&right.to_lowercase())
            .then_with(|| left.cmp(right))
    });
    names.dedup();

    let mut sections: Vec<_> = names
        .into_iter()
        .map(|name| DashboardSection {
            key: DashboardSectionKey::ProjectGroup(Some(name.clone())),
            collapsed: false,
            entries: entries
                .iter()
                .filter(|entry| entry.project.group.as_deref() == Some(name.as_str()))
                .cloned()
                .collect(),
        })
        .collect();
    let ungrouped: Vec<_> = entries
        .into_iter()
        .filter(|entry| entry.project.group.is_none())
        .collect();
    if !ungrouped.is_empty() {
        sections.push(DashboardSection {
            key: DashboardSectionKey::ProjectGroup(None),
            collapsed: false,
            entries: ungrouped,
        });
    }
    sections
}

fn sort_entries(entries: &mut [DashboardEntry<'_>], sort: DashboardSort) {
    entries.sort_by(|left, right| match sort {
        DashboardSort::NameAscending => compare_identity(left, right),
        DashboardSort::Recommended => left
            .tier
            .cmp(&right.tier)
            .then_with(|| match left.tier {
                DashboardTier::NeedsHelp => cause_rank(left.cause).cmp(&cause_rank(right.cause)),
                DashboardTier::InProgress => compare_progress(left, right),
                DashboardTier::AllSet => Ordering::Equal,
            })
            .then_with(|| compare_identity(left, right)),
    });
}

fn compare_progress(left: &DashboardEntry<'_>, right: &DashboardEntry<'_>) -> Ordering {
    match (left.relevant_count, right.relevant_count) {
        (Some(left), Some(right)) => left
            .kind
            .cmp(&right.kind)
            .then_with(|| right.value.cmp(&left.value)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn cause_rank(cause: Option<DashboardCause>) -> u8 {
    match cause {
        Some(DashboardCause::Conflict) => 0,
        Some(DashboardCause::MissingPath) => 1,
        Some(DashboardCause::ConflictDetectionUnavailable) => 2,
        Some(DashboardCause::ReadUnavailable) => 3,
        Some(DashboardCause::DetachedContext) => 4,
        Some(DashboardCause::StatusUnknown) | None => 5,
    }
}

fn compare_identity(left: &DashboardEntry<'_>, right: &DashboardEntry<'_>) -> Ordering {
    left.project
        .name
        .to_lowercase()
        .cmp(&right.project.name.to_lowercase())
        .then_with(|| {
            left.project
                .id
                .to_string()
                .cmp(&right.project.id.to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use knotra_vcs::model::status::{
        ConflictStatus, RemoteStatus, RepositoryIdentity, VcsContext, VcsKind, WorkingTreeStatus,
    };
    use uuid::Uuid;

    fn project(number: u128, name: &str, group: Option<&str>) -> Project {
        let mut project = Project::new(name, format!("/project/{number}"));
        project.id = ProjectId(Uuid::from_u128(number));
        project.group = group.map(str::to_owned);
        project
    }

    fn status(project: &Project) -> knotra_vcs::ProjectStatus {
        knotra_vcs::ProjectStatus {
            project_id: project.id.clone(),
            identity: RepositoryIdentity {
                path: project.path.clone(),
                vcs_kind: VcsKind::Git,
            },
            context: Some(VcsContext {
                label: "feature/work".to_owned(),
                branch: Some("feature/work".to_owned()),
                jj_change_id: None,
                jj_bookmark: None,
                is_detached: false,
            }),
            remote: RemoteStatus::default(),
            working_tree: WorkingTreeStatus::default(),
            conflict: ConflictStatus::default(),
            refreshed_at: Utc::now(),
            read_error: None,
        }
    }

    fn entry_for<'a>(
        project: &'a Project,
        status: Option<&'a knotra_vcs::ProjectStatus>,
        missing: bool,
    ) -> DashboardEntry<'a> {
        classify(project, status, missing)
    }

    #[test]
    fn classification_uses_the_specified_priority_and_typed_detached_fact() {
        let project = project(1, "api", None);
        let mut status = status(&project);
        status.conflict.has_conflict = true;
        status.conflict.detection_unavailable = true;
        status.read_error = Some("read failed".to_owned());
        status.context.as_mut().expect("context").is_detached = true;
        status.working_tree.uncommitted_count = 2;
        status.remote.ahead = 3;
        status.remote.behind = 4;

        assert_eq!(
            entry_for(&project, Some(&status), true).cause,
            Some(DashboardCause::MissingPath)
        );
        assert_eq!(
            entry_for(&project, Some(&status), false).cause,
            Some(DashboardCause::Conflict)
        );
        status.conflict.has_conflict = false;
        assert_eq!(
            entry_for(&project, Some(&status), false).cause,
            Some(DashboardCause::ConflictDetectionUnavailable)
        );
        status.conflict.detection_unavailable = false;
        assert_eq!(
            entry_for(&project, Some(&status), false).cause,
            Some(DashboardCause::ReadUnavailable)
        );
        status.read_error = None;
        assert_eq!(
            entry_for(&project, Some(&status), false).cause,
            Some(DashboardCause::DetachedContext)
        );

        status.identity.vcs_kind = VcsKind::Jujutsu;
        status.context.as_mut().expect("context").branch = None;
        status.context.as_mut().expect("context").jj_change_id = Some("abc123".to_owned());
        assert_eq!(
            entry_for(&project, Some(&status), false).cause,
            Some(DashboardCause::DetachedContext)
        );
    }

    #[test]
    fn display_labels_do_not_create_in_progress_or_help_tiers() {
        let project = project(1, "api", None);
        for label in ["main", "master", "trunk", "feature/work", "bookmark"] {
            let mut status = status(&project);
            status.context.as_mut().expect("context").label = label.to_owned();
            status.context.as_mut().expect("context").branch = Some(label.to_owned());
            assert_eq!(
                entry_for(&project, Some(&status), false).tier,
                DashboardTier::AllSet
            );
        }
        assert_eq!(
            entry_for(&project, None, false).cause,
            Some(DashboardCause::StatusUnknown)
        );
    }

    #[test]
    fn fact_filters_follow_the_complete_typed_truth_table() {
        let project = project(1, "api", None);
        let mut observed = status(&project);
        observed.remote.behind = 1;
        observed.remote.ahead = 1;
        observed.working_tree.uncommitted_count = 1;

        let mut cases = Vec::new();
        cases.push((
            entry_for(&project, Some(&observed), true),
            [false, false, false, false, false, true],
        ));

        let mut missing_with_stale_conflict = observed.clone();
        missing_with_stale_conflict.conflict.has_conflict = true;
        let missing_conflict_entry = entry_for(&project, Some(&missing_with_stale_conflict), true);
        assert_eq!(missing_conflict_entry.tier, DashboardTier::NeedsHelp);
        assert_eq!(
            missing_conflict_entry.cause,
            Some(DashboardCause::MissingPath)
        );
        cases.push((
            missing_conflict_entry,
            [false, false, false, false, false, true],
        ));

        let mut conflict = observed.clone();
        conflict.conflict.has_conflict = true;
        cases.push((
            entry_for(&project, Some(&conflict), false),
            [false, true, true, true, true, true],
        ));

        let mut detection = observed.clone();
        detection.conflict.detection_unavailable = true;
        cases.push((
            entry_for(&project, Some(&detection), false),
            [false, true, true, true, false, true],
        ));

        let mut read = observed.clone();
        read.read_error = Some("failed".to_owned());
        cases.push((
            entry_for(&project, Some(&read), false),
            [false, false, false, false, false, true],
        ));

        let mut detached = observed.clone();
        detached.context.as_mut().expect("context").is_detached = true;
        cases.push((
            entry_for(&project, Some(&detached), false),
            [false, true, true, true, false, true],
        ));
        cases.push((
            entry_for(&project, None, false),
            [false, false, false, false, false, true],
        ));
        cases.push((
            entry_for(&project, Some(&observed), false),
            [false, true, true, true, false, false],
        ));

        let clean = status(&project);
        cases.push((
            entry_for(&project, Some(&clean), false),
            [true, false, false, false, false, false],
        ));

        let filters = [
            StatusFilter::AllSet,
            StatusFilter::Behind,
            StatusFilter::Ahead,
            StatusFilter::Dirty,
            StatusFilter::Conflict,
            StatusFilter::NeedsHelp,
        ];
        for (entry, expected) in cases {
            for (filter, expected) in filters.iter().zip(expected) {
                assert_eq!(
                    matches_status_filter(&entry, filter),
                    expected,
                    "{filter:?}"
                );
            }
        }
    }

    #[test]
    fn relevant_count_and_recommended_order_use_kind_before_value() {
        let projects = vec![
            project(1, "one changed", None),
            project(2, "many behind", None),
            project(3, "five changed", None),
            project(4, "ahead", None),
        ];
        let mut statuses: Vec<_> = projects.iter().map(status).collect();
        statuses[0].working_tree.uncommitted_count = 1;
        statuses[0].remote.behind = 200;
        statuses[1].remote.behind = 100;
        statuses[2].working_tree.uncommitted_count = 5;
        statuses[3].remote.ahead = 2;
        let workspace_status = WorkspaceStatus {
            projects: statuses,
            last_refresh: None,
        };
        let display = build_dashboard_display(
            &projects,
            Some(&workspace_status),
            &HashSet::new(),
            &FilterState::default(),
            DashboardDisplayOptions {
                grouping: DashboardGrouping::None,
                sort: DashboardSort::Recommended,
                in_progress_collapsed: false,
                all_set_collapsed: false,
            },
        );
        assert_eq!(
            display.ordered_selectable_ids,
            vec![
                projects[2].id.clone(),
                projects[0].id.clone(),
                projects[3].id.clone(),
                projects[1].id.clone(),
            ]
        );
        assert_eq!(
            display.sections[0].entries[1].relevant_count,
            Some(RelevantCount {
                kind: ProgressKind::Uncommitted,
                value: 1,
            })
        );
    }

    #[test]
    fn project_groups_are_case_stable_with_ungrouped_last() {
        let projects = vec![
            project(1, "z", None),
            project(2, "b", Some("alpha")),
            project(3, "a", Some("Alpha")),
            project(4, "c", Some("Beta")),
        ];
        let statuses = WorkspaceStatus {
            projects: projects.iter().map(status).collect(),
            last_refresh: None,
        };
        let display = build_dashboard_display(
            &projects,
            Some(&statuses),
            &HashSet::new(),
            &FilterState::default(),
            DashboardDisplayOptions {
                grouping: DashboardGrouping::ProjectGroup,
                sort: DashboardSort::NameAscending,
                in_progress_collapsed: false,
                all_set_collapsed: true,
            },
        );
        let keys: Vec<_> = display
            .sections
            .iter()
            .map(|section| section.key.clone())
            .collect();
        assert_eq!(
            keys,
            vec![
                DashboardSectionKey::ProjectGroup(Some("Alpha".to_owned())),
                DashboardSectionKey::ProjectGroup(Some("alpha".to_owned())),
                DashboardSectionKey::ProjectGroup(Some("Beta".to_owned())),
                DashboardSectionKey::ProjectGroup(None),
            ]
        );
    }

    #[test]
    fn selectable_ids_equal_rendered_rows_for_all_display_combinations() {
        let projects = vec![
            project(1, "help", Some("A")),
            project(2, "work", Some("A")),
            project(3, "set", None),
        ];
        let mut statuses: Vec<_> = projects.iter().map(status).collect();
        statuses[0].conflict.has_conflict = true;
        statuses[1].working_tree.untracked_count = 2;
        let workspace_status = WorkspaceStatus {
            projects: statuses,
            last_refresh: None,
        };

        for grouping in [
            DashboardGrouping::Attention,
            DashboardGrouping::ProjectGroup,
            DashboardGrouping::None,
        ] {
            for sort in [DashboardSort::Recommended, DashboardSort::NameAscending] {
                for in_progress_collapsed in [false, true] {
                    for all_set_collapsed in [false, true] {
                        let display = build_dashboard_display(
                            &projects,
                            Some(&workspace_status),
                            &HashSet::new(),
                            &FilterState::default(),
                            DashboardDisplayOptions {
                                grouping,
                                sort,
                                in_progress_collapsed,
                                all_set_collapsed,
                            },
                        );
                        let rendered: Vec<_> = display
                            .sections
                            .iter()
                            .filter(|section| !section.collapsed)
                            .flat_map(|section| section.entries.iter())
                            .map(|entry| entry.project.id.clone())
                            .collect();
                        assert_eq!(display.ordered_selectable_ids, rendered);
                        let unique: HashSet<_> = display
                            .sections
                            .iter()
                            .flat_map(|section| section.entries.iter())
                            .map(|entry| entry.project.id.clone())
                            .collect();
                        assert_eq!(unique.len(), projects.len());
                        assert_eq!(
                            display
                                .sections
                                .iter()
                                .map(|section| section.entries.len())
                                .sum::<usize>(),
                            projects.len()
                        );
                        if grouping != DashboardGrouping::Attention {
                            assert_eq!(display.ordered_selectable_ids.len(), projects.len());
                        }
                    }
                }
            }
        }
    }

    /// RFC-071 D3/Amendment A4: every chip's tone, matching the table's
    /// five rows — `NeedsHelp`/`Conflict` share `danger`.
    #[test]
    fn badge_for_filter_matches_d3s_table() {
        assert_eq!(badge_for_filter(&StatusFilter::NeedsHelp).0, Tone::Danger);
        assert_eq!(badge_for_filter(&StatusFilter::Conflict).0, Tone::Danger);
        assert_eq!(badge_for_filter(&StatusFilter::Dirty).0, Tone::Warning);
        assert_eq!(badge_for_filter(&StatusFilter::Behind).0, Tone::Info);
        assert_eq!(badge_for_filter(&StatusFilter::Ahead).0, Tone::Info);
        assert_eq!(badge_for_filter(&StatusFilter::AllSet).0, Tone::Success);
    }

    /// RFC-071 D3: every `DashboardCause` is `danger` — none of them is
    /// "all set" or "in progress".
    #[test]
    fn badge_for_cause_is_always_danger() {
        for cause in [
            DashboardCause::MissingPath,
            DashboardCause::Conflict,
            DashboardCause::ConflictDetectionUnavailable,
            DashboardCause::ReadUnavailable,
            DashboardCause::DetachedContext,
            DashboardCause::StatusUnknown,
        ] {
            assert_eq!(badge_for_cause(cause).0, Tone::Danger, "{cause:?}");
        }
    }

    /// RFC-071 D3 / Review 250 §3: `Uncommitted`/`Untracked` are both
    /// "Unsaved work" (`warning`); `Ahead`/`Behind` are both `info`.
    #[test]
    fn badge_for_progress_matches_d3s_table() {
        assert_eq!(
            badge_for_progress(ProgressKind::Uncommitted).0,
            Tone::Warning
        );
        assert_eq!(badge_for_progress(ProgressKind::Untracked).0, Tone::Warning);
        assert_eq!(badge_for_progress(ProgressKind::Ahead).0, Tone::Info);
        assert_eq!(badge_for_progress(ProgressKind::Behind).0, Tone::Info);
    }

    #[test]
    fn badge_for_all_set_is_success() {
        assert_eq!(badge_for_all_set().0, Tone::Success);
    }

    /// RFC-071 D5: a chip's count reads from the full classified set,
    /// independent of the active search text or other status-filter
    /// toggles — toggling `NeedsHelp` must not move `AllSet`'s own number.
    #[test]
    fn count_for_filter_is_independent_of_active_filters() {
        let needs_help = project(1, "needs-help", None);
        let mut needs_help_status = status(&needs_help);
        needs_help_status.conflict.has_conflict = true;

        let all_set = project(2, "all-set", None);
        let all_set_status = status(&all_set);

        let projects = [needs_help.clone(), all_set.clone()];
        let ws_status = knotra_vcs::WorkspaceStatus {
            projects: vec![needs_help_status, all_set_status],
            ..Default::default()
        };
        let missing = HashSet::new();

        assert_eq!(
            count_for_filter(
                &projects,
                Some(&ws_status),
                &missing,
                &StatusFilter::NeedsHelp
            ),
            1
        );
        assert_eq!(
            count_for_filter(&projects, Some(&ws_status), &missing, &StatusFilter::AllSet),
            1
        );
        assert_eq!(
            count_for_filter(&projects, Some(&ws_status), &missing, &StatusFilter::Dirty),
            0
        );
    }
}

//! VCS adapter implementations and the unified `VcsAdapter` interface.

pub mod adapter;
pub(crate) mod git;
pub(crate) mod jj;

/// RFC-073 Amendment A6 §2: `status_digest()` fails both for a HEAD that
/// names a branch with no commits yet (`peel_to_commit` has nothing to
/// peel to) and for a HEAD that is genuinely unreadable — the two must not
/// be shown the same way, since the first is an ordinary, workable
/// project and the second is a project that needs help. Given
/// `repository_info().head` (endringer 0.33.4), read after `status_digest`
/// has already failed, this returns the branch name for the first case
/// and an explanatory reason for the second — shared by `git.rs` and
/// `jj.rs` so the two backends cannot independently decide this
/// distinction differently.
pub(crate) fn unborn_branch_or_read_error(
    head: Option<endringer_core::types::HeadState>,
) -> Result<String, &'static str> {
    match head {
        Some(endringer_core::types::HeadState::Unborn {
            branch: Some(branch),
        }) => Ok(branch),
        _ => Err("HEAD is unreadable"),
    }
}

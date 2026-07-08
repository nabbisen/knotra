//! Git backend for endringer, powered by [`gix`].
//!
//! Exposes [`GitBackend`], which implements
//! [`endringer_backend_core::backend::VcsBackend`].

pub(crate) mod blame;
pub(crate) mod object;
pub(crate) mod branch;
pub(crate) mod commit;
pub(crate) mod diff;
pub(crate) mod tag;
pub(crate) mod graph;
pub(crate) mod stash;
pub(crate) mod status;
pub(crate) mod submodule;
pub(crate) mod util;
pub(crate) mod worktree;

mod backend;
pub use backend::GitBackend;

#[cfg(test)]
mod tests;

//! Core types and VCS backend trait for endringer.
//!
//! This crate is an implementation detail. Most users depend on the
//! [`endringer`] facade crate, which re-exports everything from here.
//!
//! Depend on `endringer-core` directly only when implementing a custom
//! [`VcsBackend`].

pub mod backend;
pub mod types;

//! Jujutsu backend for endringer.
//!
//! Reads jj's underlying git object store via gix — no `jj` binary required.

mod backend;
pub use backend::JjBackend;

#[cfg(test)]
mod tests;

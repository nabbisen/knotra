//! Domain model types exposed by `endringer` to the GUI layer.
//!
//! All types here are VCS-agnostic at the surface. The GUI should depend
//! only on this module and never import VCS-specific internals.

pub mod operation;
pub mod project;
pub mod status;
pub mod workspace;

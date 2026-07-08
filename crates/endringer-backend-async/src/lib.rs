//! Async façade for `endringer-backend-git` and `endringer-backend-jj`.
//!
//! Ported from endringer 0.19.2.

pub mod repository;
pub mod async_api;

pub use async_api::AsyncRepository;
pub use repository::Repository;

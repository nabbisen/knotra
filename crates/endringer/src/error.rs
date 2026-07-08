//! Error types for `endringer`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EndringerError {
    #[error("repository not found at path: {path}")]
    RepositoryNotFound { path: String },

    #[error("VCS read error for {repo}: {source}")]
    VcsReadError {
        repo: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("VCS write error for {repo}: command `{cmd}` failed with status {code}: {stderr}")]
    VcsWriteError {
        repo: String,
        cmd: String,
        code: i32,
        stderr: String,
    },

    #[error("command execution failed: {0}")]
    CommandExecutionError(String),

    #[error("validation error: {0}")]
    ValidationError(String),

    #[error("transaction error: {0}")]
    TransactionError(String),

    #[error("rollback error: {0}")]
    RollbackError(String),

    #[error("unsupported VCS kind: {0}")]
    UnsupportedVcs(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, EndringerError>;

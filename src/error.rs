use thiserror::Error;

/// Error types for Dojo
#[derive(Error, Debug, Clone)]
pub enum DojoError {
    #[error("Failed to open repository: {0}")]
    RepoOpen(String),

    #[error("Failed to load commits: {0}")]
    CommitLoad(String),

    #[error("Failed to load files: {0}")]
    FileLoad(String),

    #[error("Failed to load diff: {0}")]
    DiffLoad(String),

    #[error("Invalid commit ID: {0}")]
    InvalidCommitId(String),

    #[error("Invalid file path: {0}")]
    InvalidPath(String),

    #[error("Repository not loaded")]
    RepoNotLoaded,

    #[error("Config error: {0}")]
    Config(String),

    #[error("Failed to update description: {0}")]
    DescriptionUpdate(String),
}

impl From<anyhow::Error> for DojoError {
    fn from(err: anyhow::Error) -> Self {
        DojoError::RepoOpen(err.to_string())
    }
}

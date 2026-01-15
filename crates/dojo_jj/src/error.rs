use std::sync::Arc;
use thiserror::Error;

/// Error loading/refreshing a repository
#[derive(Error, Debug, Clone)]
pub enum RefreshError {
    #[error("Worker channel closed")]
    ChannelClosed,

    #[error("{0}")]
    Jj(Arc<anyhow::Error>),
}

impl From<anyhow::Error> for RefreshError {
    fn from(err: anyhow::Error) -> Self {
        RefreshError::Jj(Arc::new(err))
    }
}

/// Error loading files for a commit
#[derive(Error, Debug, Clone)]
pub enum FilesError {
    #[error("Repository not loaded")]
    RepoNotLoaded,

    #[error("Worker channel closed")]
    ChannelClosed,

    #[error("{0}")]
    Jj(Arc<anyhow::Error>),
}

impl From<anyhow::Error> for FilesError {
    fn from(err: anyhow::Error) -> Self {
        FilesError::Jj(Arc::new(err))
    }
}

/// Error loading diffs for a commit
#[derive(Error, Debug, Clone)]
pub enum DiffsError {
    #[error("Repository not loaded")]
    RepoNotLoaded,

    #[error("Worker channel closed")]
    ChannelClosed,

    #[error("{0}")]
    Jj(Arc<anyhow::Error>),
}

impl From<anyhow::Error> for DiffsError {
    fn from(err: anyhow::Error) -> Self {
        DiffsError::Jj(Arc::new(err))
    }
}

/// Error loading stats for commits
#[derive(Error, Debug, Clone)]
pub enum StatsError {
    #[error("Repository not loaded")]
    RepoNotLoaded,

    #[error("Worker channel closed")]
    ChannelClosed,
}

/// Error updating a commit description
#[derive(Error, Debug, Clone)]
pub enum UpdateDescriptionError {
    #[error("Worker channel closed")]
    ChannelClosed,

    #[error("{0}")]
    Jj(Arc<anyhow::Error>),
}

impl From<anyhow::Error> for UpdateDescriptionError {
    fn from(err: anyhow::Error) -> Self {
        UpdateDescriptionError::Jj(Arc::new(err))
    }
}

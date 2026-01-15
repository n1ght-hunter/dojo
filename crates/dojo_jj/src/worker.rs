//! Background worker for workspace/repository operations.
//!
//! This module provides a dedicated worker thread for all jj-lib operations,
//! keeping the UI thread responsive. Communication is bidirectional:
//! - Commands are sent from UI to worker via mpsc channel
//! - Events are sent from worker to UI via Iced subscription

use std::path::PathBuf;

use futures::SinkExt;
use iced::Subscription;
use iced::stream;
use tokio::sync::{mpsc, oneshot};

use crate::error::{DiffsError, FilesError, RefreshError, StatsError, UpdateDescriptionError};
use crate::{CommitInfo, FileChange, FileDiff, FileStats, RepoHandle};

/// Commands sent from UI to worker.
#[derive(Debug)]
pub enum WorkspaceCommand {
    /// Snapshot working copy and reload commits
    Refresh {
        respond: Option<oneshot::Sender<Result<Vec<CommitInfo>, RefreshError>>>,
    },
    /// Load file changes for a commit
    LoadFiles {
        commit_id: String,
        respond: Option<oneshot::Sender<Result<Vec<FileChange>, FilesError>>>,
    },
    /// Load diffs for a commit
    LoadDiffs {
        commit_id: String,
        respond: Option<oneshot::Sender<Result<Vec<FileDiff>, DiffsError>>>,
    },
    /// Load stats for multiple commits
    LoadStats {
        commit_ids: Vec<String>,
        respond: Option<oneshot::Sender<Result<Vec<(String, FileStats)>, StatsError>>>,
    },
    /// Update commit description
    UpdateDescription {
        commit_id: String,
        description: String,
        respond: Option<oneshot::Sender<Result<(), UpdateDescriptionError>>>,
    },
    /// Shutdown the worker
    Shutdown,
}

/// Events sent from worker to UI via subscription
#[derive(Debug, Clone)]
pub enum WorkspaceEvent {
    /// Worker ready, here's the command channel
    Ready(mpsc::Sender<WorkspaceCommand>),
    /// Repository loaded with commits
    Loaded(Result<Vec<CommitInfo>, RefreshError>),
    /// File stats loaded for commits
    StatsLoaded(Result<Vec<(String, FileStats)>, StatsError>),
    /// Files loaded for a commit
    FilesLoaded(Result<Vec<FileChange>, FilesError>),
    /// Diffs loaded for a commit
    DiffsLoaded(Result<Vec<FileDiff>, DiffsError>),
    /// Description updated
    DescriptionUpdated(Result<(), UpdateDescriptionError>),
}

// ============ Task-based API helpers ============

/// Refresh repo and return commits directly via Future
pub async fn refresh(tx: &mpsc::Sender<WorkspaceCommand>) -> Result<Vec<CommitInfo>, RefreshError> {
    let (respond_tx, respond_rx) = oneshot::channel();
    tx.send(WorkspaceCommand::Refresh {
        respond: Some(respond_tx),
    })
    .await
    .map_err(|_| RefreshError::ChannelClosed)?;
    respond_rx.await.unwrap()
}

/// Load files for a commit directly via Future
pub async fn load_files(
    tx: &mpsc::Sender<WorkspaceCommand>,
    commit_id: String,
) -> Result<Vec<FileChange>, FilesError> {
    let (respond_tx, respond_rx) = oneshot::channel();
    tx.send(WorkspaceCommand::LoadFiles {
        commit_id,
        respond: Some(respond_tx),
    })
    .await
    .map_err(|_| FilesError::ChannelClosed)?;
    respond_rx.await.unwrap()
}

/// Load diffs for a commit directly via Future
pub async fn load_diffs(
    tx: &mpsc::Sender<WorkspaceCommand>,
    commit_id: String,
) -> Result<Vec<FileDiff>, DiffsError> {
    let (respond_tx, respond_rx) = oneshot::channel();
    tx.send(WorkspaceCommand::LoadDiffs {
        commit_id,
        respond: Some(respond_tx),
    })
    .await
    .map_err(|_| DiffsError::ChannelClosed)?;
    respond_rx.await.unwrap()
}

/// Load stats for commits directly via Future
pub async fn load_stats(
    tx: &mpsc::Sender<WorkspaceCommand>,
    commit_ids: Vec<String>,
) -> Result<Vec<(String, FileStats)>, StatsError> {
    let (respond_tx, respond_rx) = oneshot::channel();
    tx.send(WorkspaceCommand::LoadStats {
        commit_ids,
        respond: Some(respond_tx),
    })
    .await
    .map_err(|_| StatsError::ChannelClosed)?;
    respond_rx.await.unwrap()
}

/// Update description directly via Future
pub async fn update_description(
    tx: &mpsc::Sender<WorkspaceCommand>,
    commit_id: String,
    description: String,
) -> Result<(), UpdateDescriptionError> {
    let (respond_tx, respond_rx) = oneshot::channel();
    tx.send(WorkspaceCommand::UpdateDescription {
        commit_id,
        description,
        respond: Some(respond_tx),
    })
    .await
    .map_err(|_| UpdateDescriptionError::ChannelClosed)?;
    respond_rx.await.unwrap()
}

/// Background worker that owns the RepoHandle
struct WorkspaceWorker {
    handle: Option<RepoHandle>,
    path: PathBuf,
}

impl WorkspaceWorker {
    fn new(path: PathBuf) -> Self {
        Self { handle: None, path }
    }

    /// Handle a command and return an optional event (only if no respond channel)
    async fn handle_command(&mut self, cmd: WorkspaceCommand) -> Option<WorkspaceEvent> {
        match cmd {
            WorkspaceCommand::Refresh { respond } => {
                let result = self.do_refresh().await;
                if let Some(tx) = respond {
                    let _ = tx.send(result);
                    None
                } else {
                    Some(WorkspaceEvent::Loaded(result))
                }
            }
            WorkspaceCommand::LoadFiles { commit_id, respond } => {
                let result = self.do_load_files(&commit_id).await;
                if let Some(tx) = respond {
                    let _ = tx.send(result);
                    None
                } else {
                    Some(WorkspaceEvent::FilesLoaded(result))
                }
            }
            WorkspaceCommand::LoadDiffs { commit_id, respond } => {
                let result = self.do_load_diffs(&commit_id).await;
                if let Some(tx) = respond {
                    let _ = tx.send(result);
                    None
                } else {
                    Some(WorkspaceEvent::DiffsLoaded(result))
                }
            }
            WorkspaceCommand::LoadStats {
                commit_ids,
                respond,
            } => {
                let result = self.do_load_stats(&commit_ids).await;
                if let Some(tx) = respond {
                    let _ = tx.send(result);
                    None
                } else {
                    Some(WorkspaceEvent::StatsLoaded(result))
                }
            }
            WorkspaceCommand::UpdateDescription {
                commit_id,
                description,
                respond,
            } => {
                let result = self.do_update_description(&commit_id, &description).await;
                if let Some(tx) = respond {
                    let _ = tx.send(result);
                    None
                } else {
                    Some(WorkspaceEvent::DescriptionUpdated(result))
                }
            }
            WorkspaceCommand::Shutdown => None,
        }
    }

    async fn do_refresh(&mut self) -> Result<Vec<CommitInfo>, RefreshError> {
        // Snapshot working copy first
        RepoHandle::snapshot_working_copy(&self.path).await?;

        // Open/reopen the handle
        let handle = RepoHandle::open(&self.path)?;
        let commits = handle.log(100)?;

        self.handle = Some(handle);
        Ok(commits)
    }

    async fn do_load_files(&self, commit_id: &str) -> Result<Vec<FileChange>, FilesError> {
        let handle = self.handle.as_ref().ok_or(FilesError::RepoNotLoaded)?;
        Ok(handle.get_changed_files(commit_id).await?)
    }

    async fn do_load_diffs(&self, commit_id: &str) -> Result<Vec<FileDiff>, DiffsError> {
        let handle = self.handle.as_ref().ok_or(DiffsError::RepoNotLoaded)?;

        let files = handle
            .get_changed_files(commit_id)
            .await
            .map_err(|e| DiffsError::Jj(std::sync::Arc::new(e)))?;

        let mut diffs = Vec::new();
        for file in files {
            match handle.get_file_diff(commit_id, &file.path).await {
                Ok(diff) => diffs.push(diff),
                Err(_) => continue,
            }
        }
        Ok(diffs)
    }

    async fn do_load_stats(
        &self,
        commit_ids: &[String],
    ) -> Result<Vec<(String, FileStats)>, StatsError> {
        let handle = self.handle.as_ref().ok_or(StatsError::RepoNotLoaded)?;
        Ok(handle.get_batch_stats(commit_ids).await)
    }

    async fn do_update_description(
        &mut self,
        commit_id: &str,
        description: &str,
    ) -> Result<(), UpdateDescriptionError> {
        // Need a fresh mutable handle for updates
        let mut handle = RepoHandle::open(&self.path)?;
        handle.update_description(commit_id, description)?;

        // Refresh our handle after mutation
        self.handle = Some(RepoHandle::open(&self.path)?);
        Ok(())
    }
}

/// Create a subscription for a workspace worker
pub fn subscription(repo_path: PathBuf) -> Subscription<WorkspaceEvent> {
    struct Worker(PathBuf);

    impl Worker {
        fn stream(self) -> impl futures::Stream<Item = WorkspaceEvent> {
            stream::channel(
                32,
                move |mut output: futures::channel::mpsc::Sender<WorkspaceEvent>| async move {
                    let (tx, mut rx) = mpsc::channel::<WorkspaceCommand>(32);

                    // Send the command channel back to the app
                    let _ = output.send(WorkspaceEvent::Ready(tx)).await;

                    let mut worker = WorkspaceWorker::new(self.0);

                    // Initial load
                    let _ = output
                        .send(WorkspaceEvent::Loaded(worker.do_refresh().await))
                        .await;

                    // Process commands
                    while let Some(cmd) = rx.recv().await {
                        if matches!(cmd, WorkspaceCommand::Shutdown) {
                            break;
                        }
                        if let Some(event) = worker.handle_command(cmd).await {
                            let _ = output.send(event).await;
                        }
                    }
                },
            )
        }
    }

    Subscription::run_with(repo_path.clone(), |path: &PathBuf| {
        Worker(path.clone()).stream()
    })
}

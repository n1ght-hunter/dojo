use std::path::PathBuf;

use iced::{Subscription, Task};
use tokio::sync::mpsc;

use crate::components::{right_panel, sidebar};
use crate::screens::LogScreen;
use crate::state_wrapper::StateMut;
use dojo_jj::{
    DiffsError, FileChange, FileDiff, FilesError, RefreshError, StatsError, UpdateDescriptionError,
    WorkspaceCommand, WorkspaceEvent, worker,
};

/// Display error for the UI
#[derive(Debug, Clone)]
pub enum RepoError {
    Refresh(RefreshError),
    Files(FilesError),
    Diffs(DiffsError),
    Stats(StatsError),
    UpdateDescription(UpdateDescriptionError),
}

impl std::fmt::Display for RepoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoError::Refresh(e) => write!(f, "{e}"),
            RepoError::Files(e) => write!(f, "{e}"),
            RepoError::Diffs(e) => write!(f, "{e}"),
            RepoError::Stats(e) => write!(f, "{e}"),
            RepoError::UpdateDescription(e) => write!(f, "{e}"),
        }
    }
}

/// State for a single repository
pub struct RepoState {
    pub path: PathBuf,
    pub name: String,
    /// Command channel to the worker
    pub command_tx: Option<mpsc::Sender<WorkspaceCommand>>,
    pub log_screen: LogScreen,
    pub sidebar: sidebar::State,
    pub right_panel: right_panel::State,
    pub files: Vec<FileChange>,
    pub diffs: Vec<FileDiff>,
    pub loading: bool,
    pub error: Option<RepoError>,
}

/// Messages for a single repository
#[derive(Debug, Clone)]
pub enum Message {
    /// Worker events from subscription
    Worker(WorkspaceEvent),

    // User interactions
    SelectCommit(usize),

    // Sub-component messages
    Sidebar(sidebar::Message),
    RightPanel(right_panel::Message),
}

impl RepoState {
    /// Create a new repo state for the given path
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        Self {
            path,
            name,
            command_tx: None,
            log_screen: LogScreen::new(),
            sidebar: sidebar::State::default(),
            right_panel: right_panel::State::new(),
            files: Vec::new(),
            diffs: Vec::new(),
            loading: true,
            error: None,
        }
    }

    /// Returns the subscription for this repository's worker.
    pub fn subscription(&self) -> Subscription<Message> {
        worker::subscription(self.path.clone()).map(Message::Worker)
    }

    /// Send a refresh command to the worker
    pub fn refresh(&self) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.try_send(WorkspaceCommand::Refresh { respond: None });
        }
    }
}

/// Update repo state based on message
pub fn update(mut state: StateMut<'_, RepoState>, message: Message) -> Task<Message> {
    match message {
        Message::Worker(event) => handle_worker_event(state, event),

        Message::SelectCommit(index) => {
            state.log_screen.select(index);
            right_panel::clear(&mut state.right_panel);
            state.files.clear();
            state.diffs.clear();

            if let Some(commit) = state.log_screen.selected_commit() {
                let commit_id = commit.commit_id.clone();
                if let Some(tx) = &state.command_tx {
                    let _ = tx.try_send(WorkspaceCommand::LoadFiles {
                        commit_id: commit_id.clone(),
                        respond: None,
                    });
                    let _ = tx.try_send(WorkspaceCommand::LoadDiffs {
                        commit_id,
                        respond: None,
                    });
                }
            }
            Task::none()
        }

        Message::Sidebar(msg) => {
            sidebar::update(state.reborrow().map(|s| &mut s.sidebar), msg);
            Task::none()
        }

        Message::RightPanel(msg) => {
            let selected_commit = state.log_screen.selected_commit().cloned();

            // Handle special cases that need repo_state access
            match &msg {
                right_panel::Message::Summary(summary_msg) => match summary_msg {
                    crate::components::summary::Message::ExpandAll => {
                        let files = state.files.clone();
                        right_panel::expand_all(&mut state.right_panel, &files);
                        return Task::none();
                    }
                    crate::components::summary::Message::DescriptionEditor(editor_msg) => {
                        if matches!(
                            editor_msg,
                            crate::components::description_editor::Message::Save
                        ) {
                            // Save description to repository via worker
                            if let Some(ref commit) = selected_commit {
                                let new_description = state.right_panel.get_description_draft();
                                let commit_id = commit.commit_id.clone();

                                if let Some(tx) = &state.command_tx {
                                    let _ = tx.try_send(WorkspaceCommand::UpdateDescription {
                                        commit_id,
                                        description: new_description,
                                        respond: None,
                                    });
                                }
                            }
                            return Task::none();
                        }
                    }
                    _ => {}
                },
                _ => {}
            }

            right_panel::update(
                state.reborrow().map(|s| &mut s.right_panel),
                msg,
                selected_commit.as_ref(),
            );
            Task::none()
        }
    }
}

/// Handle worker events
fn handle_worker_event(mut state: StateMut<'_, RepoState>, event: WorkspaceEvent) -> Task<Message> {
    match event {
        WorkspaceEvent::Ready(tx) => {
            state.command_tx = Some(tx);
            Task::none()
        }

        WorkspaceEvent::Loaded(result) => {
            state.loading = false;
            match result {
                Ok(commits) => {
                    state.error = None;

                    // Collect commit IDs for stats loading
                    let commit_ids: Vec<String> =
                        commits.iter().map(|c| c.commit_id.clone()).collect();

                    state.log_screen.set_commits(commits);

                    // Request stats for all commits
                    if let Some(tx) = &state.command_tx {
                        let _ = tx.try_send(WorkspaceCommand::LoadStats {
                            commit_ids,
                            respond: None,
                        });
                    }

                    // Load files/diffs for first selected commit
                    if let Some(commit) = state.log_screen.selected_commit() {
                        let commit_id = commit.commit_id.clone();
                        if let Some(tx) = &state.command_tx {
                            let _ = tx.try_send(WorkspaceCommand::LoadFiles {
                                commit_id: commit_id.clone(),
                                respond: None,
                            });
                            let _ = tx.try_send(WorkspaceCommand::LoadDiffs {
                                commit_id,
                                respond: None,
                            });
                        }
                    }
                }
                Err(e) => {
                    state.error = Some(RepoError::Refresh(e));
                }
            }
            Task::none()
        }

        WorkspaceEvent::StatsLoaded(result) => {
            match result {
                Ok(stats) => {
                    state.log_screen.update_stats(stats);
                }
                Err(e) => {
                    state.error = Some(RepoError::Stats(e));
                }
            }
            Task::none()
        }

        WorkspaceEvent::FilesLoaded(result) => {
            match result {
                Ok(files) => state.files = files,
                Err(e) => state.error = Some(RepoError::Files(e)),
            }
            Task::none()
        }

        WorkspaceEvent::DiffsLoaded(result) => {
            match result {
                Ok(diffs) => {
                    state.diffs = diffs;
                }
                Err(e) => state.error = Some(RepoError::Diffs(e)),
            }
            Task::none()
        }

        WorkspaceEvent::DescriptionUpdated(result) => {
            match result {
                Ok(()) => {
                    right_panel::description_saved(&mut state.right_panel);
                    // Request a refresh to get updated commits
                    if let Some(tx) = &state.command_tx {
                        let _ = tx.try_send(WorkspaceCommand::Refresh { respond: None });
                    }
                }
                Err(e) => {
                    state.error = Some(RepoError::UpdateDescription(e));
                }
            }
            Task::none()
        }
    }
}

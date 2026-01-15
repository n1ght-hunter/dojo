use std::path::PathBuf;
use std::sync::Arc;

use iced::Task;

use crate::components::{right_panel, sidebar};
use crate::error::DojoError;
use crate::jj::{CommitInfo, FileChange, FileDiff, FileStats, RepoHandle};
use crate::screens::LogScreen;
use crate::state_wrapper::StateMut;

/// State for a single repository
pub struct RepoState {
    pub path: PathBuf,
    pub name: String,
    pub handle: Option<Arc<RepoHandle>>,
    pub log_screen: LogScreen,
    pub sidebar: sidebar::State,
    pub right_panel: right_panel::State,
    pub files: Vec<FileChange>,
    pub diffs: Vec<FileDiff>,
    pub loading: bool,
    pub error: Option<DojoError>,
}

/// Messages for a single repository
#[derive(Debug, Clone)]
pub enum Message {
    // Data loading
    Loaded(Result<(Arc<RepoHandle>, Vec<CommitInfo>), DojoError>),
    FilesLoaded(Result<Vec<FileChange>, DojoError>),
    DiffsLoaded(Result<Vec<FileDiff>, DojoError>),
    StatsLoaded(Result<Vec<(String, FileStats)>, DojoError>),

    // User interactions
    SelectCommit(usize),

    // Description editing
    DescriptionSaved(Result<(), DojoError>),

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
            handle: None,
            log_screen: LogScreen::new(),
            sidebar: sidebar::State::default(),
            right_panel: right_panel::State::new(),
            files: Vec::new(),
            diffs: Vec::new(),
            loading: true,
            error: None,
        }
    }

    /// Load the repository asynchronously
    pub fn load(path: PathBuf) -> Task<Message> {
        Task::perform(
            async move {
                let handle =
                    RepoHandle::open(&path).map_err(|e| DojoError::RepoOpen(e.to_string()))?;
                let commits = handle
                    .log(100)
                    .map_err(|e| DojoError::CommitLoad(e.to_string()))?;
                Ok((Arc::new(handle), commits))
            },
            Message::Loaded,
        )
    }
}

/// Update repo state based on message
pub fn update(mut state: StateMut<'_, RepoState>, message: Message) -> Task<Message> {
    match message {
        Message::Loaded(result) => {
            state.loading = false;
            match result {
                Ok((handle, commits)) => {
                    state.handle = Some(handle.clone());
                    state.error = None;

                    // Collect commit IDs for stats loading
                    let commit_ids: Vec<String> =
                        commits.iter().map(|c| c.commit_id.clone()).collect();

                    state.log_screen.set_commits(commits);

                    // Load files for first selected commit and stats for all commits
                    let mut tasks = vec![load_stats(handle, commit_ids)];

                    if let Some(commit) = state.log_screen.selected_commit() {
                        let commit_id = commit.commit_id.clone();
                        let handle = state.handle.clone();
                        tasks.push(load_commit_data(handle, commit_id));
                    }

                    return Task::batch(tasks);
                }
                Err(e) => {
                    state.error = Some(e);
                }
            }
            Task::none()
        }

        Message::SelectCommit(index) => {
            state.log_screen.select(index);
            right_panel::clear(&mut state.right_panel);
            state.files.clear();
            state.diffs.clear();

            if let Some(commit) = state.log_screen.selected_commit() {
                let commit_id = commit.commit_id.clone();
                let handle = state.handle.clone();
                return load_commit_data(handle, commit_id);
            }
            Task::none()
        }

        Message::FilesLoaded(result) => {
            match result {
                Ok(files) => state.files = files,
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }

        Message::DiffsLoaded(result) => {
            match result {
                Ok(diffs) => {
                    state.diffs = diffs;
                }
                Err(e) => state.error = Some(e),
            }
            Task::none()
        }

        Message::StatsLoaded(result) => {
            match result {
                Ok(stats) => {
                    // Update commits with their stats
                    state.log_screen.update_stats(stats);
                }
                Err(e) => state.error = Some(e),
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
                            // Save description to repository
                            if let Some(ref commit) = selected_commit {
                                let new_description = state.right_panel.get_description_draft();
                                let commit_id = commit.commit_id.clone();
                                let handle = state.handle.clone();
                                let path = state.path.clone();

                                return Task::perform(
                                    async move {
                                        save_description(
                                            handle,
                                            &path,
                                            &commit_id,
                                            &new_description,
                                        )
                                        .await
                                    },
                                    Message::DescriptionSaved,
                                );
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

        Message::DescriptionSaved(result) => {
            match result {
                Ok(()) => {
                    right_panel::description_saved(&mut state.right_panel);
                    // Reload commits to get updated description
                    let path = state.path.clone();
                    return RepoState::load(path);
                }
                Err(e) => {
                    state.error = Some(e);
                }
            }
            Task::none()
        }
    }
}

/// Load files and diffs for a commit
fn load_commit_data(handle: Option<Arc<RepoHandle>>, commit_id: String) -> Task<Message> {
    let handle2 = handle.clone();
    let commit_id2 = commit_id.clone();

    Task::batch([
        Task::perform(
            async move { load_files(handle, &commit_id).await },
            Message::FilesLoaded,
        ),
        Task::perform(
            async move { load_diffs(handle2, &commit_id2).await },
            Message::DiffsLoaded,
        ),
    ])
}

/// Load stats for all commits
fn load_stats(handle: Arc<RepoHandle>, commit_ids: Vec<String>) -> Task<Message> {
    Task::perform(
        async move { load_batch_stats(handle, commit_ids).await },
        Message::StatsLoaded,
    )
}

// Helper functions for async loading
async fn load_files(
    handle: Option<Arc<RepoHandle>>,
    commit_id: &str,
) -> Result<Vec<FileChange>, DojoError> {
    let handle = handle.ok_or(DojoError::RepoNotLoaded)?;
    handle
        .get_changed_files(commit_id)
        .await
        .map_err(|e| DojoError::FileLoad(e.to_string()))
}

async fn load_diffs(
    handle: Option<Arc<RepoHandle>>,
    commit_id: &str,
) -> Result<Vec<FileDiff>, DojoError> {
    let handle = handle.ok_or(DojoError::RepoNotLoaded)?;

    let files = handle
        .get_changed_files(commit_id)
        .await
        .map_err(|e| DojoError::FileLoad(e.to_string()))?;

    let mut diffs = Vec::new();
    for file in files {
        match handle.get_file_diff(commit_id, &file.path).await {
            Ok(diff) => diffs.push(diff),
            Err(_) => continue, // Skip files we can't diff
        }
    }
    Ok(diffs)
}

async fn load_batch_stats(
    handle: Arc<RepoHandle>,
    commit_ids: Vec<String>,
) -> Result<Vec<(String, FileStats)>, DojoError> {
    Ok(handle.get_batch_stats(&commit_ids).await)
}

async fn save_description(
    _handle: Option<Arc<RepoHandle>>,
    path: &std::path::Path,
    commit_id: &str,
    new_description: &str,
) -> Result<(), DojoError> {
    // Need to open a fresh handle with mutable access
    let mut handle =
        RepoHandle::open(path).map_err(|e| DojoError::DescriptionUpdate(e.to_string()))?;

    handle
        .update_description(commit_id, new_description)
        .map_err(|e| DojoError::DescriptionUpdate(e.to_string()))
}

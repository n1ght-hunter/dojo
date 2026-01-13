use std::path::PathBuf;
use std::sync::Arc;

use iced::Task;

use crate::error::DojoError;
use crate::jj::{CommitInfo, FileChange, FileDiff, RepoHandle};
use crate::screens::LogScreen;
use crate::widgets::{right_panel, sidebar, RightPanel, Sidebar};

/// State for a single repository
pub struct RepoState {
    pub path: PathBuf,
    pub name: String,
    pub handle: Option<Arc<RepoHandle>>,
    pub log_screen: LogScreen,
    pub sidebar: Sidebar,
    pub right_panel: RightPanel,
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
            handle: None,
            log_screen: LogScreen::new(),
            sidebar: Sidebar::new(),
            right_panel: RightPanel::new(),
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

    /// Update repo state based on message
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(result) => {
                self.loading = false;
                match result {
                    Ok((handle, commits)) => {
                        self.handle = Some(handle);
                        self.error = None;
                        self.log_screen.set_commits(commits);

                        // Load files for first selected commit
                        if let Some(commit) = self.log_screen.selected_commit() {
                            let commit_id = commit.commit_id.clone();
                            return self.load_commit_data(commit_id);
                        }
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
                Task::none()
            }

            Message::SelectCommit(index) => {
                self.log_screen.select(index);
                self.right_panel.clear();
                self.files.clear();
                self.diffs.clear();

                if let Some(commit) = self.log_screen.selected_commit() {
                    let commit_id = commit.commit_id.clone();
                    return self.load_commit_data(commit_id);
                }
                Task::none()
            }

            Message::FilesLoaded(result) => {
                match result {
                    Ok(files) => self.files = files,
                    Err(e) => self.error = Some(e),
                }
                Task::none()
            }

            Message::DiffsLoaded(result) => {
                match result {
                    Ok(diffs) => {
                        self.diffs = diffs;
                    }
                    Err(e) => self.error = Some(e),
                }
                Task::none()
            }

            Message::Sidebar(msg) => {
                self.sidebar.update(msg);
                Task::none()
            }

            Message::RightPanel(msg) => {
                // Handle ExpandAll specially since it needs access to files
                if let right_panel::Message::Summary(crate::widgets::summary::Message::ExpandAll) =
                    &msg
                {
                    self.right_panel.expand_all(&self.files);
                } else {
                    self.right_panel.update(msg);
                }
                Task::none()
            }
        }
    }

    /// Load files and diffs for a commit
    fn load_commit_data(&self, commit_id: String) -> Task<Message> {
        let handle = self.handle.clone();
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

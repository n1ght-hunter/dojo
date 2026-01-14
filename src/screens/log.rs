use iced::widget::{container, scrollable, text};
use iced::{Element, Fill};

use crate::components::GraphColumn;
use crate::jj::{CommitInfo, FileStats};

/// The commit log screen showing the list of commits
pub struct LogScreen {
    commits: Vec<CommitInfo>,
    selected_index: Option<usize>,
    graph: GraphColumn,
}

impl LogScreen {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
            selected_index: None,
            graph: GraphColumn::new(),
        }
    }

    pub fn set_commits(&mut self, commits: Vec<CommitInfo>) {
        self.graph.compute(&commits);
        self.commits = commits;
        // Select the first commit by default if available
        if !self.commits.is_empty() && self.selected_index.is_none() {
            self.selected_index = Some(0);
            self.graph.set_selected(self.selected_index);
        }
    }

    pub fn selected_commit(&self) -> Option<&CommitInfo> {
        self.selected_index.and_then(|i| self.commits.get(i))
    }

    pub fn select(&mut self, index: usize) {
        if index < self.commits.len() {
            self.selected_index = Some(index);
            self.graph.set_selected(self.selected_index);
        }
    }

    /// Update commits with their file statistics
    pub fn update_stats(&mut self, stats: Vec<(String, FileStats)>) {
        for (commit_id, file_stats) in stats {
            if let Some(commit) = self.commits.iter_mut().find(|c| c.commit_id == commit_id) {
                commit.file_stats = Some(file_stats);
            }
        }
        // Recompute graph to update display
        self.graph.compute(&self.commits);
    }

    /// View returns Element<usize> where the usize is the selected commit index
    pub fn view(&self) -> Element<'_, usize> {
        if self.commits.is_empty() {
            return container(text("No commits found")).padding(20).into();
        }

        // Render everything in the graph canvas
        // Canvas now returns usize directly (clicked row index)
        let content = self.graph.view(&self.commits);

        container(scrollable(content).height(Fill).width(Fill))
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }
}

impl Default for LogScreen {
    fn default() -> Self {
        Self::new()
    }
}

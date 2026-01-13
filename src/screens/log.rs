use iced::widget::{Row, column, container, row, scrollable, text};
use iced::{Element, Fill, Length};

use crate::Message;
use crate::jj::CommitInfo;
use crate::widgets::GraphColumn;

/// Row height must match the graph widget
const ROW_HEIGHT: f32 = 30.0;

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
        }
    }

    pub fn selected_commit(&self) -> Option<&CommitInfo> {
        self.selected_index.and_then(|i| self.commits.get(i))
    }

    pub fn select(&mut self, index: usize) {
        if index < self.commits.len() {
            self.selected_index = Some(index);
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.commits.is_empty() {
            return container(text("No commits found")).padding(20).into();
        }

        // Build commit info rows (without graph - graph is separate canvas)
        let commit_rows: Vec<Element<'_, Message>> = self
            .commits
            .iter()
            .enumerate()
            .map(|(index, commit)| self.commit_row(index, commit))
            .collect();

        let commit_list = column(commit_rows);

        // Create a row with graph on left and commit info on right
        let content: Row<'_, Message> = row![
            self.graph.view(),
            scrollable(commit_list).height(Fill).width(Fill),
        ]
        .spacing(5);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn commit_row<'a>(&'a self, index: usize, commit: &'a CommitInfo) -> Element<'a, Message> {
        let is_selected = self.selected_index == Some(index);

        // Short commit id (first 8 chars)
        let short_id: String = if commit.commit_id.len() > 8 {
            commit.commit_id[..8].to_string()
        } else {
            commit.commit_id.clone()
        };

        // Short change id (first 8 chars)
        let short_change_id: String = if commit.change_id.len() > 8 {
            commit.change_id[..8].to_string()
        } else {
            commit.change_id.clone()
        };

        // First line of description
        let first_line: String = commit
            .description
            .lines()
            .next()
            .unwrap_or("(no description)")
            .to_string();

        // Format timestamp
        let time_str: String = commit.timestamp.format("%Y-%m-%d %H:%M").to_string();

        // Clone author for ownership
        let author = commit.author.clone();

        let content = row![
            text(short_change_id).size(12).color([0.69, 0.58, 0.98]), // Purple for change id
            text(short_id).size(12).color([0.38, 0.45, 0.64]),        // Gray for commit id
            text(author).size(12).width(Length::Fixed(120.0)),
            text(time_str).size(12).color([0.38, 0.45, 0.64]),
            text(first_line).size(12),
        ]
        .spacing(10)
        .height(Length::Fixed(ROW_HEIGHT));

        let row_container = container(content)
            .width(Fill)
            .center_y(Length::Fixed(ROW_HEIGHT));

        if is_selected {
            row_container
                .style(|_theme| container::Style {
                    background: Some(iced::Color::from_rgb(0.26, 0.28, 0.35).into()),
                    ..Default::default()
                })
                .into()
        } else {
            row_container.into()
        }
    }
}

impl Default for LogScreen {
    fn default() -> Self {
        Self::new()
    }
}

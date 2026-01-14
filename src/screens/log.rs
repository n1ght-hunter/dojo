use iced::widget::{Row, column, container, mouse_area, row, scrollable, text};
use iced::{Element, Fill, Length};

use crate::components::GraphColumn;
use crate::jj::CommitInfo;

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

    /// View returns Element<usize> where the usize is the selected commit index
    pub fn view(&self) -> Element<'_, usize> {
        if self.commits.is_empty() {
            return container(text("No commits found")).padding(20).into();
        }

        // Build commit info rows
        let commit_rows: Vec<Element<'_, usize>> = self
            .commits
            .iter()
            .enumerate()
            .map(|(index, commit)| self.commit_row(index, commit))
            .collect();

        let commit_list = column(commit_rows);

        // Create a row with graph on left and commit info on right
        let content: Row<'_, usize> = row![
            self.graph.view().map(|_: ()| 0usize), // Graph doesn't emit messages
            scrollable(commit_list).height(Fill).width(Fill),
        ]
        .spacing(5);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn commit_row(&self, index: usize, commit: &CommitInfo) -> Element<'_, usize> {
        let is_selected = self.selected_index == Some(index);

        // First line of description
        let first_line = commit
            .description
            .lines()
            .next()
            .unwrap_or("(no description)")
            .to_string();

        // Format timestamp relative or absolute
        let time_str = commit.timestamp.format("%Y-%m-%d").to_string();

        // Clone author for ownership
        let author = commit.author.clone();

        // Build the row content
        let mut content_row = Row::new().spacing(8);

        // Description (main content)
        content_row = content_row.push(text(first_line).size(12).width(Fill));

        // Bookmark badges (inline, like Sublime Merge)
        for bookmark in &commit.bookmarks {
            content_row = content_row.push(bookmark_badge(bookmark.clone()));
        }

        // Parent count badge for merge commits
        if commit.parent_ids.len() > 1 {
            content_row = content_row.push(parent_count_badge(commit.parent_ids.len()));
        }

        // Author
        content_row = content_row.push(
            text(author)
                .size(11)
                .style(text::primary)
                .width(Length::Fixed(100.0)),
        );

        // Timestamp
        content_row = content_row.push(
            text(time_str)
                .size(11)
                .style(text::default)
                .width(Length::Fixed(80.0)),
        );

        let content = content_row.height(Length::Fixed(ROW_HEIGHT));

        let row_container = container(content)
            .width(Fill)
            .padding([0, 8])
            .center_y(Length::Fixed(ROW_HEIGHT));

        let styled_row: Element<'_, usize> = if is_selected {
            row_container.style(container::rounded_box).into()
        } else {
            row_container.into()
        };

        // Make the row clickable - returns the index
        mouse_area(styled_row).on_press(index).into()
    }
}

impl Default for LogScreen {
    fn default() -> Self {
        Self::new()
    }
}

/// Bookmark badge
fn bookmark_badge(name: String) -> Element<'static, usize> {
    container(text(name).size(10))
        .padding([2, 6])
        .style(container::rounded_box)
        .into()
}

/// Parent count badge for merge commits
fn parent_count_badge(count: usize) -> Element<'static, usize> {
    container(text(format!("[{}]", count)).size(10).style(text::primary))
        .padding([2, 4])
        .style(container::bordered_box)
        .into()
}

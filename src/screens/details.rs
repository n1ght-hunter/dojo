use iced::widget::{column, container, row, text};
use iced::{Element, Fill};

use crate::Message;
use crate::components::DiffViewer;
use crate::jj::{CommitInfo, FileDiff};

/// Details pane showing information about a selected commit and its diffs
pub struct DetailsPane {
    diff_viewer: DiffViewer,
}

impl DetailsPane {
    pub fn new() -> Self {
        Self {
            diff_viewer: DiffViewer::new(),
        }
    }

    pub fn set_diffs(&mut self, diffs: Vec<FileDiff>) {
        self.diff_viewer.set_diffs(diffs);
    }

    pub fn clear_diffs(&mut self) {
        self.diff_viewer.clear();
    }

    pub fn view(&self, commit: Option<&CommitInfo>) -> Element<'_, Message> {
        match commit {
            None => container(text("Select a commit to view details").size(14))
                .padding(10)
                .width(Fill)
                .height(Fill)
                .into(),
            Some(commit) => self.commit_details(commit),
        }
    }

    fn commit_details(&self, commit: &CommitInfo) -> Element<'_, Message> {
        // Clone data to avoid lifetime issues
        let change_id = format!("Change: {}", &commit.change_id);
        let commit_id = format!("Commit: {}", &commit.commit_id);
        let author_line = format!("Author: {}", &commit.author);
        let time_line = format!("Date: {}", commit.timestamp.format("%Y-%m-%d %H:%M:%S %z"));
        let is_working_copy = commit.is_working_copy;

        let header = column![
            text(change_id).size(14).color([0.69, 0.58, 0.98]), // Purple
            text(commit_id).size(12).color([0.38, 0.45, 0.64]), // Gray
        ]
        .spacing(2);

        let working_copy_indicator: Element<'_, Message> = if is_working_copy {
            row![
                text("●").size(12).color([0.31, 0.98, 0.48]), // Green
                text(" Working Copy").size(12).color([0.31, 0.98, 0.48]),
            ]
            .spacing(2)
            .into()
        } else {
            text("").into()
        };

        let metadata = row![
            column![
                text(author_line).size(12),
                text(time_line).size(12).color([0.38, 0.45, 0.64]),
            ]
            .spacing(2),
            working_copy_indicator,
        ]
        .spacing(20);

        // Description (short, single line)
        let description_text = if commit.description.trim().is_empty() {
            "(no description)".to_string()
        } else {
            let first_line = commit.description.lines().next().unwrap_or("").trim();
            if first_line.len() > 80 {
                format!("{}...", &first_line[..77])
            } else {
                first_line.to_string()
            }
        };

        let is_empty_desc = commit.description.trim().is_empty();
        let description = text(description_text).size(12).color(if is_empty_desc {
            [0.38, 0.45, 0.64]
        } else {
            [0.97, 0.97, 0.95]
        });

        // Commit info section (compact)
        let info_section = container(
            column![header, metadata, description,]
                .spacing(6)
                .padding(10),
        )
        .width(Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Color::from_rgb(0.12, 0.13, 0.17).into()),
            ..Default::default()
        });

        // Diff section takes remaining space
        let diff_section = container(self.diff_viewer.view())
            .width(Fill)
            .height(Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Color::from_rgb(0.16, 0.16, 0.21).into()),
                ..Default::default()
            });

        column![info_section, diff_section].spacing(0).into()
    }
}

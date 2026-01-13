use iced::widget::{Column, column, container, row, scrollable, text};
use iced::{Element, Fill, Length};

use crate::Message;
use crate::jj::CommitInfo;

/// A simple horizontal divider line
fn divider<'a>() -> Element<'a, Message> {
    container(text(""))
        .width(Fill)
        .height(Length::Fixed(1.0))
        .style(|_theme| container::Style {
            background: Some(iced::Color::from_rgb(0.38, 0.45, 0.64).into()),
            ..Default::default()
        })
        .into()
}

/// Details pane showing information about a selected commit
pub struct DetailsPane;

impl DetailsPane {
    pub fn view(commit: Option<&CommitInfo>) -> Element<'_, Message> {
        match commit {
            None => container(text("Select a commit to view details").size(14))
                .padding(10)
                .width(Fill)
                .into(),
            Some(commit) => Self::commit_details(commit),
        }
    }

    fn commit_details(commit: &CommitInfo) -> Element<'_, Message> {
        // Header with change ID and commit ID
        let change_id = format!("Change: {}", &commit.change_id);
        let commit_id = format!("Commit: {}", &commit.commit_id);

        let header = column![
            text(change_id).size(14).color([0.69, 0.58, 0.98]), // Purple
            text(commit_id).size(12).color([0.38, 0.45, 0.64]), // Gray
        ]
        .spacing(2);

        // Author and timestamp
        let author_line = format!("Author: {}", commit.author);
        let time_line = format!("Date: {}", commit.timestamp.format("%Y-%m-%d %H:%M:%S %z"));

        let metadata = column![
            text(author_line).size(12),
            text(time_line).size(12).color([0.38, 0.45, 0.64]),
        ]
        .spacing(2);

        // Parent commits
        let parents: Column<'_, Message> = if commit.parent_ids.is_empty() {
            column![text("Parents: (none)").size(12).color([0.38, 0.45, 0.64])]
        } else {
            let parent_ids: Vec<Element<'_, Message>> = commit
                .parent_ids
                .iter()
                .map(|id| {
                    let short_id = if id.len() > 12 {
                        format!("  {}", &id[..12])
                    } else {
                        format!("  {}", id)
                    };
                    text(short_id).size(11).color([0.38, 0.45, 0.64]).into()
                })
                .collect();

            column![
                text("Parents:").size(12),
                Column::with_children(parent_ids).spacing(1),
            ]
        };

        // Description
        let description = if commit.description.trim().is_empty() {
            text("(no description)").size(12).color([0.38, 0.45, 0.64])
        } else {
            text(commit.description.trim()).size(12)
        };

        // Working copy indicator
        let working_copy_indicator: Element<'_, Message> = if commit.is_working_copy {
            row![
                text("●").size(12).color([0.31, 0.98, 0.48]), // Green
                text(" Working Copy").size(12).color([0.31, 0.98, 0.48]),
            ]
            .spacing(2)
            .into()
        } else {
            text("").into()
        };

        let content = column![
            header,
            divider(),
            metadata,
            parents,
            divider(),
            text("Description:").size(12),
            scrollable(description).height(Length::Fill),
            working_copy_indicator,
        ]
        .spacing(8)
        .padding(10);

        container(content)
            .width(Fill)
            .height(Length::Fixed(200.0))
            .style(|_theme| container::Style {
                background: Some(iced::Color::from_rgb(0.12, 0.13, 0.17).into()),
                ..Default::default()
            })
            .into()
    }
}

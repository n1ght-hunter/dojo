use iced::widget::{Column, button, column, container, row, scrollable, text};
use iced::{Element, Fill, Length, Theme};

use crate::components::description_editor::{self, DescriptionEditor};
use crate::components::diff::view_file_diff_content;
use crate::jj::{ChangeKind, CommitInfo, FileChange, FileDiff};
use crate::settings::DiffSettings;

/// Messages for summary interactions
#[derive(Debug, Clone)]
pub enum Message {
    ToggleFile(String),
    CollapseAll,
    ExpandAll,
    // Description editing
    StartEditDescription,
    DescriptionEditor(description_editor::Message),
}

/// Render the commit summary view with collapsible file diffs
pub fn view<'a>(
    commit: Option<&'a CommitInfo>,
    files: &'a [FileChange],
    diffs: &'a [FileDiff],
    expanded_files: &'a std::collections::HashSet<String>,
    description_editor: &'a DescriptionEditor,
    width: f32,
    settings: &'a DiffSettings,
    theme: &'a Theme,
) -> Element<'a, Message> {
    match commit {
        None => container(text("No commit selected").size(14).style(text::default))
            .padding(20)
            .width(Fill)
            .height(Fill)
            .style(container::bordered_box)
            .into(),

        Some(commit) => {
            let content = column![
                metadata_section(commit),
                message_section(commit, description_editor),
                files_section(files, diffs, expanded_files, width, settings, theme),
            ]
            .spacing(0);

            scrollable(content).width(Fill).height(Fill).into()
        }
    }
}

fn metadata_section(commit: &CommitInfo) -> Element<'_, Message> {
    let short_commit_id = &commit.commit_id[..12.min(commit.commit_id.len())];
    let short_change_id = &commit.change_id[..12.min(commit.change_id.len())];

    let mut rows: Column<'_, Message> = Column::new().spacing(4);

    rows = rows.push(metadata_row("Commit Hash", short_commit_id.to_string()));
    rows = rows.push(metadata_row("Change ID", short_change_id.to_string()));
    rows = rows.push(metadata_row("Author", commit.author.clone()));
    rows = rows.push(metadata_row(
        "Date",
        commit.timestamp.format("%a, %d %b %Y %H:%M").to_string(),
    ));

    if !commit.parent_ids.is_empty() {
        let parents: Vec<String> = commit
            .parent_ids
            .iter()
            .map(|p| p[..8.min(p.len())].to_string())
            .collect();
        rows = rows.push(metadata_row("Parents", parents.join(", ")));
    }

    if !commit.bookmarks.is_empty() {
        rows = rows.push(bookmarks_row(&commit.bookmarks));
    }

    container(rows.padding(12))
        .width(Fill)
        .style(container::bordered_box)
        .into()
}

fn metadata_row(label: &str, value: String) -> Element<'_, Message> {
    row![
        text(label)
            .size(11)
            .style(text::primary)
            .width(Length::Fixed(100.0)),
        text(value).size(11).style(text::default),
    ]
    .spacing(8)
    .into()
}

fn bookmarks_row(bookmarks: &[String]) -> Element<'_, Message> {
    let mut bookmark_row = row![
        text("Bookmarks")
            .size(11)
            .style(text::primary)
            .width(Length::Fixed(100.0)),
    ]
    .spacing(8);

    for bookmark in bookmarks {
        bookmark_row = bookmark_row.push(bookmark_badge(bookmark.clone()));
    }

    bookmark_row.into()
}

fn bookmark_badge(name: String) -> Element<'static, Message> {
    container(text(name).size(10))
        .padding([2, 6])
        .style(container::rounded_box)
        .into()
}

fn message_section<'a>(
    commit: &'a CommitInfo,
    description_editor: &'a DescriptionEditor,
) -> Element<'a, Message> {
    if description_editor.editing {
        // Edit mode: delegate to DescriptionEditor component
        description_editor.view().map(Message::DescriptionEditor)
    } else {
        // View mode: show description with edit button
        let message = if commit.description.trim().is_empty() {
            "(no description)".to_string()
        } else {
            commit.description.clone()
        };

        let edit_button = button(text("Edit").size(11))
            .on_press(Message::StartEditDescription)
            .padding([4, 12])
            .style(button::text);

        let header = row![
            text("Description").size(11).style(text::primary),
            container(text("")).width(Fill),
            edit_button,
        ]
        .spacing(8);

        let message_text = text(message).size(12);

        container(column![header, message_text].spacing(8))
            .padding(12)
            .width(Fill)
            .style(container::bordered_box)
            .into()
    }
}

fn files_section<'a>(
    files: &'a [FileChange],
    diffs: &'a [FileDiff],
    expanded_files: &'a std::collections::HashSet<String>,
    width: f32,
    settings: &'a DiffSettings,
    theme: &'a Theme,
) -> Element<'a, Message> {
    if files.is_empty() {
        return container(text("No files changed").size(11).style(text::default))
            .padding(12)
            .width(Fill)
            .into();
    }

    // Stats
    let added = files.iter().filter(|f| f.kind == ChangeKind::Added).count();
    let modified = files
        .iter()
        .filter(|f| f.kind == ChangeKind::Modified)
        .count();
    let deleted = files
        .iter()
        .filter(|f| f.kind == ChangeKind::Deleted)
        .count();

    let stats_text = format!(
        "{} file{}: {} added, {} modified, {} deleted",
        files.len(),
        if files.len() == 1 { "" } else { "s" },
        added,
        modified,
        deleted
    );

    // Show "Expand all" if none expanded, "Collapse all" if any expanded
    let any_expanded = !expanded_files.is_empty();
    let toggle_button = if any_expanded {
        button(text("Collapse all").size(10))
            .on_press(Message::CollapseAll)
            .padding([2, 8])
            .style(button::text)
    } else {
        button(text("Expand all").size(10))
            .on_press(Message::ExpandAll)
            .padding([2, 8])
            .style(button::text)
    };

    let header = row![
        text("Files").size(11).style(text::primary),
        text(stats_text).size(10).style(text::default),
        container(text("")).width(Fill),
        toggle_button,
    ]
    .spacing(8);

    let mut content = column![
        container(header)
            .padding([8, 12])
            .width(Fill)
            .style(container::bordered_box),
    ]
    .spacing(0);

    // File dropdowns
    for file in files {
        let is_expanded = expanded_files.contains(&file.path);
        let diff = diffs.iter().find(|d| d.path == file.path);
        content = content.push(file_dropdown(
            file,
            is_expanded,
            diff,
            width,
            settings,
            theme,
        ));
    }

    content.into()
}

fn file_dropdown<'a>(
    file: &'a FileChange,
    is_expanded: bool,
    diff: Option<&'a FileDiff>,
    width: f32,
    settings: &'a DiffSettings,
    theme: &'a Theme,
) -> Element<'a, Message> {
    let toggle_icon = if is_expanded { "▼" } else { "▶" };

    let indicator_text = match file.kind {
        ChangeKind::Added => text("+").size(11).style(text::success),
        ChangeKind::Modified => text("~").size(11),
        ChangeKind::Deleted => text("-").size(11).style(text::danger),
    };

    let path = file.path.clone();

    // Calculate diff stats if available
    let stats = if let Some(d) = diff {
        let (adds, removes) = d.lines.iter().fold((0, 0), |(a, r), line| match line {
            crate::jj::DiffLine::Added(_) => (a + 1, r),
            crate::jj::DiffLine::Removed(_) => (a, r + 1),
            _ => (a, r),
        });
        row![
            text(format!("-{}", removes)).size(10).style(text::danger),
            text(" ").size(10),
            text(format!("+{}", adds)).size(10).style(text::success),
        ]
        .spacing(2)
    } else {
        row![]
    };

    let header_row = row![
        button(
            row![
                text(toggle_icon).size(9),
                container(indicator_text).width(Length::Fixed(16.0)),
                text(path).size(11),
            ]
            .spacing(4),
        )
        .on_press(Message::ToggleFile(file.path.clone()))
        .padding([6, 12])
        .style(button::text),
        container(text("")).width(Fill),
        container(stats).padding([6, 0]),
        button(text("···").size(14))
            .padding([6, 12])
            .style(button::text),
        // Space for scrollbar
        container(text("")).width(Length::Fixed(12.0)),
    ]
    .spacing(0);

    let header_container = container(header_row)
        .width(Fill)
        .style(container::bordered_box);

    if is_expanded {
        if let Some(diff) = diff {
            let diff_content: Element<'_, Message> =
                view_file_diff_content(diff, width, settings, theme);
            column![header_container, diff_content].spacing(0).into()
        } else {
            let loading = container(text("Loading diff...").size(10).style(text::default))
                .padding([8, 12])
                .width(Fill);
            column![header_container, loading].spacing(0).into()
        }
    } else {
        header_container.into()
    }
}

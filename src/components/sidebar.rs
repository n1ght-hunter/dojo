use iced::widget::{column, container, scrollable, text};
use iced::{Element, Fill};

use crate::state_wrapper::StateMut;

/// Messages for sidebar interactions
#[derive(Debug, Clone)]
pub enum Message {
    SelectBookmark(String),
}

/// State for the sidebar component
#[derive(Debug, Default)]
pub struct State {
    // TODO: Add actual bookmark/remote/tag data from jj
}

pub fn update(_state: StateMut<'_, State>, message: Message) {
    match message {
        Message::SelectBookmark(_name) => {
            // TODO: Navigate to bookmark
        }
    }
}

pub fn view(_state: &State) -> Element<'static, Message> {
    let content = column![
        text("BOOKMARKS").size(10).style(text::primary),
        bookmark_item("main", true),
        section_header("REMOTES"),
        remote_header("origin"),
        bookmark_item("  main", false),
        section_header("TAGS"),
        text("(no tags)").size(11).style(text::default),
    ]
    .spacing(4)
    .padding(10);

    scrollable(content).width(Fill).height(Fill).into()
}

fn section_header(label: &str) -> Element<'static, Message> {
    text(label.to_string()).size(10).style(text::primary).into()
}

fn remote_header(name: &str) -> Element<'static, Message> {
    text(name.to_string()).size(11).style(text::default).into()
}

fn bookmark_item(name: &str, is_current: bool) -> Element<'static, Message> {
    let prefix = if is_current { "● " } else { "  " };

    let label = text(format!("{}{}", prefix, name)).size(11);
    let styled_label = if is_current {
        label.style(text::success)
    } else {
        label.style(text::default)
    };

    container(styled_label).width(Fill).padding([2, 0]).into()
}

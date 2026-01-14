use iced::widget::{column, container, scrollable, text};
use iced::{Element, Fill};

/// Messages for sidebar interactions
#[derive(Debug, Clone)]
pub enum Message {
    SelectBookmark(String),
}

/// Sidebar showing bookmarks (jj's term for what git calls branches), remotes, and tags
pub struct Sidebar {
    // TODO: Add actual bookmark/remote/tag data from jj
}

impl Sidebar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SelectBookmark(_name) => {
                // TODO: Navigate to bookmark
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content = column![
            text("BOOKMARKS").size(10).style(text::primary),
            Self::bookmark_item("main", true),
            Self::section_header("REMOTES"),
            Self::remote_header("origin"),
            Self::bookmark_item("  main", false),
            Self::section_header("TAGS"),
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
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Element, Fill};

/// Messages for sidebar interactions
#[derive(Debug, Clone)]
pub enum Message {
    Toggle,
    SelectBookmark(String),
}

/// Sidebar showing bookmarks (jj's term for what git calls branches), remotes, and tags
pub struct Sidebar {
    pub is_open: bool,
    // TODO: Add actual bookmark/remote/tag data from jj
}

impl Sidebar {
    pub fn new() -> Self {
        Self { is_open: true }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Toggle => {
                self.is_open = !self.is_open;
            }
            Message::SelectBookmark(_name) => {
                // TODO: Navigate to bookmark
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        if !self.is_open {
            // Collapsed state - just show expand button
            return container(
                button(text("▶").size(12))
                    .on_press(Message::Toggle)
                    .padding([4, 8])
                    .style(button::secondary),
            )
            .width(30)
            .height(Fill)
            .into();
        }

        // Header with collapse button
        let header = row![
            button(text("◀").size(10))
                .on_press(Message::Toggle)
                .padding([2, 6])
                .style(button::secondary),
            text("BOOKMARKS").size(10).style(text::primary),
        ]
        .spacing(8);

        let content = column![
            header,
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

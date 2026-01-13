use std::collections::HashSet;

use iced::widget::{button, column, container, responsive, row, rule, scrollable, text};
use iced::{Border, Element, Fill, Length, Theme};

use crate::jj::{CommitInfo, FileChange, FileDiff};
use crate::settings::DiffSettings;
use crate::widgets::{diff, summary};

/// Tab types for the right panel
#[derive(Debug, Clone, PartialEq)]
pub enum Tab {
    Summary,
    File(String),
}

/// Messages for right panel interactions
#[derive(Debug, Clone)]
pub enum Message {
    SwitchTab(Tab),
    Summary(summary::Message),
}

/// Right panel state
pub struct RightPanel {
    pub active_tab: Tab,
    pub expanded_files: HashSet<String>,
}

impl RightPanel {
    pub fn new() -> Self {
        Self {
            active_tab: Tab::Summary,
            expanded_files: HashSet::new(),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::SwitchTab(tab) => {
                self.active_tab = tab;
            }
            Message::Summary(msg) => match msg {
                summary::Message::ToggleFile(path) => {
                    if self.expanded_files.contains(&path) {
                        self.expanded_files.remove(&path);
                    } else {
                        self.expanded_files.insert(path);
                    }
                }
                summary::Message::CollapseAll => {
                    self.expanded_files.clear();
                }
                summary::Message::ExpandAll => {
                    // Handled by repo_state since it has access to files
                }
            },
        }
    }

    pub fn expand_all(&mut self, files: &[FileChange]) {
        for file in files {
            self.expanded_files.insert(file.path.clone());
        }
    }

    pub fn clear(&mut self) {
        self.active_tab = Tab::Summary;
        self.expanded_files.clear();
    }

    pub fn view<'a>(
        &'a self,
        commit: Option<&'a CommitInfo>,
        files: &'a [FileChange],
        diffs: &'a [FileDiff],
        settings: &'a DiffSettings,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let tab_bar = self.view_tab_bar(files);

        // Use responsive to get actual width for diff layout decisions
        let content = responsive(move |size| {
            self.view_content(commit, files, diffs, size.width, settings, theme)
        });

        column![tab_bar, content].spacing(0).into()
    }

    fn view_tab_bar<'a>(&'a self, files: &'a [FileChange]) -> Element<'a, Message> {
        // Summary tab (always first)
        let summary_active = self.active_tab == Tab::Summary;
        let summary_tab = tab_button("Summary", summary_active, Message::SwitchTab(Tab::Summary));

        // File tabs in horizontal scrollable with small gap
        let mut file_tabs = row![].spacing(2);
        for file in files {
            let is_active = self.active_tab == Tab::File(file.path.clone());
            let short_name = file.path.split('/').last().unwrap_or(&file.path);
            file_tabs = file_tabs.push(tab_button(
                short_name,
                is_active,
                Message::SwitchTab(Tab::File(file.path.clone())),
            ));
        }

        // Wrap file tabs in horizontal scrollable (no visible scrollbar)
        let scrollable_tabs = scrollable(file_tabs)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(0).scroller_width(0),
            ))
            .width(Fill);

        let divider = rule::vertical(1);
        let tabs_row = row![summary_tab, divider, scrollable_tabs].spacing(2);

        container(tabs_row)
            .width(Fill)
            .height(Length::Fixed(32.0))
            .style(container::bordered_box)
            .into()
    }

    fn view_content<'a>(
        &'a self,
        commit: Option<&'a CommitInfo>,
        files: &'a [FileChange],
        diffs: &'a [FileDiff],
        width: f32,
        settings: &'a DiffSettings,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        match &self.active_tab {
            Tab::Summary => {
                summary::view(commit, files, diffs, &self.expanded_files, width, settings, theme)
                    .map(Message::Summary)
            }
            Tab::File(path) => {
                if let Some(file_diff) = diffs.iter().find(|d| &d.path == path) {
                    scrollable(diff::view_file_diff_content::<Message>(
                        file_diff, width, settings, theme,
                    ))
                    .width(Fill)
                    .height(Fill)
                    .into()
                } else {
                    container(text("Loading diff...").size(12))
                        .padding(12)
                        .width(Fill)
                        .height(Fill)
                        .into()
                }
            }
        }
    }
}

impl Default for RightPanel {
    fn default() -> Self {
        Self::new()
    }
}

fn tab_button(label: &str, is_active: bool, msg: Message) -> Element<'_, Message> {
    button(text(label).size(11))
        .on_press(msg)
        .padding([8, 12])
        .style(move |theme: &Theme, status| {
            let base = if is_active {
                button::primary(theme, status)
            } else {
                button::secondary(theme, status)
            };
            button::Style {
                border: Border {
                    radius: 0.0.into(),
                    ..base.border
                },
                ..base
            }
        })
        .into()
}

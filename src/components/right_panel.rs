use std::collections::HashSet;

use iced::widget::{button, column, container, row, rule, scrollable, text};
use iced::{Border, Element, Fill, Length, Subscription, Theme};

use crate::components::{description_editor, diff, summary};
use crate::jj::{CommitInfo, FileChange, FileDiff};
use crate::state_wrapper::StateMut;

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

/// Context data needed by RightPanel (non-owned references)
pub struct Context<'a> {
    pub commit: Option<&'a CommitInfo>,
    pub files: &'a [FileChange],
    pub diffs: &'a [FileDiff],
    pub theme: &'a Theme,
}

/// Right panel state
pub struct State {
    pub active_tab: Tab,
    pub expanded_files: HashSet<String>,
    pub description_editor: description_editor::State,
}

impl State {
    pub fn new() -> Self {
        Self {
            active_tab: Tab::Summary,
            expanded_files: HashSet::new(),
            description_editor: description_editor::State::new(),
        }
    }

    /// Get the current description draft text (for saving)
    pub fn get_description_draft(&self) -> String {
        self.description_editor.get_text()
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

pub fn update(mut state: StateMut<'_, State>, message: Message, commit: Option<&CommitInfo>) {
    match message {
        Message::SwitchTab(tab) => {
            state.active_tab = tab;
        }
        Message::Summary(msg) => match msg {
            summary::Message::ToggleFile(path) => {
                if state.expanded_files.contains(&path) {
                    state.expanded_files.remove(&path);
                } else {
                    state.expanded_files.insert(path);
                }
            }
            summary::Message::CollapseAll => {
                state.expanded_files.clear();
            }
            summary::Message::ExpandAll => {
                // Handled by repo_state since it has access to files
            }
            summary::Message::StartEditDescription => {
                if let Some(commit) = commit {
                    state.description_editor.start_editing(&commit.description);
                }
            }
            summary::Message::DescriptionEditor(editor_msg) => {
                // Handle save/cancel specially, delegate rest to editor
                match &editor_msg {
                    description_editor::Message::Cancel => {
                        state.description_editor.cancel();
                    }
                    description_editor::Message::Save => {
                        // Handled by repo_state to perform the actual save
                    }
                    _ => {
                        description_editor::update(
                            state.reborrow().map(|s| &mut s.description_editor),
                            editor_msg,
                        );
                    }
                }
            }
        },
    }
}

pub fn expand_all(state: &mut State, files: &[FileChange]) {
    for file in files {
        state.expanded_files.insert(file.path.clone());
    }
}

pub fn clear(state: &mut State) {
    state.active_tab = Tab::Summary;
    state.expanded_files.clear();
    state.description_editor.cancel();
}

/// Called after description is saved successfully
pub fn description_saved(state: &mut State) {
    state.description_editor.cancel();
}

/// Subscription for keyboard shortcuts when editing
pub fn subscription(state: &State) -> Subscription<Message> {
    description_editor::subscription(&state.description_editor)
        .map(|msg| Message::Summary(summary::Message::DescriptionEditor(msg)))
}

pub fn view<'a>(
    state: &'a State,
    settings: &'a crate::settings::Settings,
    ctx: Context<'a>,
) -> Element<'a, Message> {
    let tab_bar = view_tab_bar(state, ctx.files);

    let content = match &state.active_tab {
        Tab::Summary => summary::view(
            ctx.commit,
            ctx.files,
            ctx.diffs,
            &state.expanded_files,
            &state.description_editor,
            0.0, // Width not needed for summary layout currently
            &settings.diff,
            ctx.theme,
        )
        .map(Message::Summary),
        Tab::File(path) => {
            if let Some(file_diff) = ctx.diffs.iter().find(|d| &d.path == path) {
                scrollable(diff::view_file_diff_content::<Message>(
                    file_diff,
                    0.0, // Width not critical
                    &settings.diff,
                    ctx.theme,
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
    };

    column![tab_bar, content].spacing(0).into()
}

fn view_tab_bar<'a>(state: &'a State, files: &'a [FileChange]) -> Element<'a, Message> {
    // Summary tab (always first)
    let summary_active = state.active_tab == Tab::Summary;
    let summary_tab = tab_button("Summary", summary_active, Message::SwitchTab(Tab::Summary));

    // File tabs in horizontal scrollable with small gap
    let mut file_tabs = row![].spacing(2);
    for file in files {
        let is_active = state.active_tab == Tab::File(file.path.clone());
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

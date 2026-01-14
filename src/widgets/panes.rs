use iced::widget::{container, pane_grid};
use iced::{Element, Fill, Size, Subscription, Theme};

use crate::repo_state::RepoState;
use crate::settings::Settings;
use crate::widgets::sidebar;

/// Types of panes in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneType {
    Sidebar,
    CommitList,
    RightPanel,
}

/// Messages for pane interactions
#[derive(Debug, Clone)]
pub enum Message {
    Resized(pane_grid::ResizeEvent),
    ToggleSidebar,
    WindowResized(Size),
}

/// Subscription for window resize events
pub fn subscription() -> Subscription<Message> {
    iced::window::resize_events().map(|(_id, size)| Message::WindowResized(size))
}

/// State for the pane grid
pub struct PaneState {
    panes: pane_grid::State<PaneType>,
    sidebar_pane: Option<pane_grid::Pane>,
    commit_pane: pane_grid::Pane,
    sidebar_split: Option<pane_grid::Split>,
    commit_split: Option<pane_grid::Split>,
    // Store actual pixel widths so they stay consistent on window resize
    sidebar_width: f32,
    commit_width: f32,
    sidebar_open: bool,
    window_width: f32,
}

impl PaneState {
    pub fn new() -> Self {
        // Create pane_grid: Sidebar | CommitList | RightPanel
        let (mut panes, sidebar_pane) = pane_grid::State::new(PaneType::Sidebar);

        // Split sidebar to create commit list
        let (commit_pane, sidebar_split) = panes
            .split(
                pane_grid::Axis::Vertical,
                sidebar_pane,
                PaneType::CommitList,
            )
            .expect("Failed to create commit list pane");

        // Split commit list to create right panel
        let (_right_pane, commit_split) = panes
            .split(pane_grid::Axis::Vertical, commit_pane, PaneType::RightPanel)
            .expect("Failed to create right panel pane");

        // Set initial ratios: sidebar ~15%, commit ~35%, right ~50%
        panes.resize(sidebar_split, 0.15);
        panes.resize(commit_split, 0.40);

        Self {
            panes,
            sidebar_pane: Some(sidebar_pane),
            commit_pane,
            sidebar_split: Some(sidebar_split),
            commit_split: Some(commit_split),
            sidebar_width: 180.0, // 15% of 1200
            commit_width: 340.0,  // ~40% of remaining 1020
            sidebar_open: true,
            window_width: 1200.0,
        }
    }

    pub fn sidebar_open(&self) -> bool {
        self.sidebar_open
    }

    pub fn update(&mut self, message: Message, settings: &Settings) {
        match message {
            Message::WindowResized(size) => {
                let old_width = self.window_width;
                self.window_width = size.width;

                // Skip if width hasn't changed
                if (old_width - size.width).abs() < 1.0 {
                    return;
                }

                // Recalculate ratios from stored pixel widths to keep them consistent
                if let Some(split) = self.sidebar_split {
                    // Clamp sidebar width to settings bounds
                    let clamped_width = self.sidebar_width.clamp(
                        settings.sidebar.min_width as f32,
                        settings.sidebar.max_width as f32,
                    );
                    self.sidebar_width = clamped_width;

                    let ratio = clamped_width / self.window_width;
                    self.panes.resize(split, ratio);
                }

                if let Some(split) = self.commit_split {
                    let sidebar_width = if self.sidebar_open {
                        self.sidebar_width
                    } else {
                        0.0
                    };
                    let remaining_width = self.window_width - sidebar_width;

                    // Clamp commit width to min/max
                    let clamped_width = self.commit_width.clamp(
                        settings.commit_list.min_width as f32,
                        settings.commit_list.max_width as f32,
                    );
                    self.commit_width = clamped_width;

                    let ratio = clamped_width / remaining_width;
                    self.panes.resize(split, ratio.clamp(0.1, 0.9));
                }
            }

            Message::ToggleSidebar => {
                if self.sidebar_open {
                    // Close sidebar: remove the pane
                    if let Some(sidebar_pane) = self.sidebar_pane.take() {
                        self.panes.close(sidebar_pane);
                        self.sidebar_split = None;
                    }
                    self.sidebar_open = false;
                } else {
                    // Open sidebar: split commit pane to add sidebar on the left
                    if let Some((new_pane, split)) = self.panes.split(
                        pane_grid::Axis::Vertical,
                        self.commit_pane,
                        PaneType::Sidebar,
                    ) {
                        // split() puts new pane on the right, we need sidebar on the left
                        // Swap the pane contents so sidebar is on the left
                        self.panes.swap(new_pane, self.commit_pane);
                        self.sidebar_pane = Some(self.commit_pane);
                        self.commit_pane = new_pane;
                        self.sidebar_split = Some(split);

                        // Use stored width, clamped to settings
                        let clamped_width = self.sidebar_width.clamp(
                            settings.sidebar.min_width as f32,
                            settings.sidebar.max_width as f32,
                        );
                        self.sidebar_width = clamped_width;
                        let ratio = clamped_width / self.window_width;
                        self.panes.resize(split, ratio);
                    }
                    self.sidebar_open = true;
                }
            }

            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                // Determine which split is being resized and apply constraints
                if Some(split) == self.sidebar_split {
                    // Calculate pixel width from ratio
                    let width = self.window_width * ratio;

                    // Clamp to min/max settings
                    let clamped_width = width.clamp(
                        settings.sidebar.min_width as f32,
                        settings.sidebar.max_width as f32,
                    );
                    self.sidebar_width = clamped_width;

                    let clamped_ratio = clamped_width / self.window_width;
                    self.panes.resize(split, clamped_ratio);
                } else if Some(split) == self.commit_split {
                    // Calculate available width (after sidebar)
                    let sidebar_width = if self.sidebar_open {
                        self.sidebar_width
                    } else {
                        0.0
                    };
                    let remaining_width = self.window_width - sidebar_width;

                    // Calculate commit width from ratio
                    let width = remaining_width * ratio;

                    // Clamp to min/max settings
                    let clamped_width = width.clamp(
                        settings.commit_list.min_width as f32,
                        settings.commit_list.max_width as f32,
                    );
                    self.commit_width = clamped_width;

                    let clamped_ratio = clamped_width / remaining_width;
                    self.panes.resize(split, clamped_ratio);
                } else {
                    self.panes.resize(split, ratio);
                }
            }
        }
    }

    pub fn view<'a, M: 'a + Clone>(
        &'a self,
        repo: &'a RepoState,
        settings: &'a Settings,
        theme: &'a Theme,
        map_sidebar: impl Fn(sidebar::Message) -> M + 'a + Clone,
        map_commit: impl Fn(usize) -> M + 'a + Clone,
        map_right_panel: impl Fn(crate::widgets::right_panel::Message) -> M + 'a + Clone,
        map_resize: impl Fn(Message) -> M + 'a,
    ) -> Element<'a, M> {
        let map_sidebar_clone = map_sidebar.clone();
        let map_commit_clone = map_commit.clone();
        let map_right_panel_clone = map_right_panel.clone();

        let pane_content =
            pane_grid::PaneGrid::new(&self.panes, move |_pane, pane_type, _is_maximized| {
                let content: Element<'_, M> = match pane_type {
                    PaneType::Sidebar => {
                        container(repo.sidebar.view().map(map_sidebar_clone.clone()))
                            .style(container::bordered_box)
                            .width(Fill)
                            .height(Fill)
                            .into()
                    }
                    PaneType::CommitList => {
                        container(repo.log_screen.view().map(map_commit_clone.clone()))
                            .style(container::bordered_box)
                            .width(Fill)
                            .height(Fill)
                            .into()
                    }
                    PaneType::RightPanel => repo
                        .right_panel
                        .view(
                            repo.log_screen.selected_commit(),
                            &repo.files,
                            &repo.diffs,
                            &settings.diff,
                            theme,
                        )
                        .map(map_right_panel_clone.clone()),
                };

                pane_grid::Content::new(content)
            })
            .on_resize(10, move |event| map_resize(Message::Resized(event)))
            .width(Fill)
            .height(Fill);

        pane_content.into()
    }
}

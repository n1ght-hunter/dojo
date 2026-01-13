mod error;
mod jj;
mod repo_state;
mod screens;
mod widgets;

use std::path::PathBuf;

use iced::widget::{center, column, container, pane_grid, text};
use iced::{Element, Fill, Task, Theme};

use repo_state::RepoState;
use widgets::{sidebar, tab_bar, toolbar};

fn main() -> iced::Result {
    iced::application(boot, update, view)
        .title("Dojo")
        .theme(theme)
        .run()
}

struct App {
    repos: Vec<RepoState>,
    active_repo: usize,
    panes: pane_grid::State<PaneType>,
    loading: bool,
    error: Option<String>,
}

/// Types of panes in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneType {
    Sidebar,
    CommitList,
    RightPanel,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Tab bar
    TabBar(tab_bar::Message),
    // Toolbar (placeholder)
    Toolbar(toolbar::Message),
    // Route to specific repo
    Repo(usize, repo_state::Message),
    // Pane management
    PaneResized(pane_grid::ResizeEvent),
    // File dialog result for opening new repo
    RepoPathSelected(Option<PathBuf>),
}

fn boot() -> (App, Task<Message>) {
    let repo_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Create Sublime Merge-style layout:
    // ┌──────────┬─────────────────┬──────────────────────────┐
    // │ Sidebar  │  Commit List    │      Right Panel         │
    // │ (bookmarks│  (with graph)   │  (summary/diff tabs)    │
    // │  remotes │                 │                          │
    // │  tags)   │                 │                          │
    // └──────────┴─────────────────┴──────────────────────────┘

    let (mut panes, sidebar_pane) = pane_grid::State::new(PaneType::Sidebar);

    let (commit_pane, sidebar_split) = panes
        .split(
            pane_grid::Axis::Vertical,
            sidebar_pane,
            PaneType::CommitList,
        )
        .expect("Failed to create commit list pane");

    let (_right_pane, commit_split) = panes
        .split(pane_grid::Axis::Vertical, commit_pane, PaneType::RightPanel)
        .expect("Failed to create right panel pane");

    // Adjust ratios: Sidebar ~15%, CommitList ~35%, RightPanel ~50%
    panes.resize(sidebar_split, 0.15);
    panes.resize(commit_split, 0.40);

    // Create initial repo state
    let repo = RepoState::new(repo_path.clone());

    let app = App {
        repos: vec![repo],
        active_repo: 0,
        panes,
        loading: true,
        error: None,
    };

    // Load the repository
    let task = RepoState::load(repo_path).map(|msg| Message::Repo(0, msg));

    (app, task)
}

fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::TabBar(msg) => {
            match msg {
                tab_bar::Message::Select(index) => {
                    if index < app.repos.len() {
                        app.active_repo = index;
                    }
                }
                tab_bar::Message::Close(index) => {
                    if app.repos.len() > 1 && index < app.repos.len() {
                        app.repos.remove(index);
                        if app.active_repo >= app.repos.len() {
                            app.active_repo = app.repos.len().saturating_sub(1);
                        }
                    }
                }
                tab_bar::Message::OpenNew => {
                    // TODO: Open file dialog to select new repo
                    // For now, just a placeholder
                }
            }
            Task::none()
        }

        Message::Toolbar(_msg) => {
            // Placeholder - toolbar not yet functional
            Task::none()
        }

        Message::Repo(index, msg) => {
            if index < app.repos.len() {
                // Handle loading state
                if let repo_state::Message::Loaded(result) = &msg {
                    app.loading = false;
                    if let Err(e) = result {
                        app.error = Some(e.to_string());
                    } else {
                        app.error = None;
                    }
                }

                app.repos[index]
                    .update(msg)
                    .map(move |m| Message::Repo(index, m))
            } else {
                Task::none()
            }
        }

        Message::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
            app.panes.resize(split, ratio);
            Task::none()
        }

        Message::RepoPathSelected(path) => {
            if let Some(path) = path {
                let index = app.repos.len();
                app.repos.push(RepoState::new(path.clone()));
                app.active_repo = index;
                return RepoState::load(path).map(move |msg| Message::Repo(index, msg));
            }
            Task::none()
        }
    }
}

fn view(app: &App) -> Element<'_, Message> {
    // Show loading or error state
    if app.loading {
        return center(text("Loading repository...").size(20))
            .width(Fill)
            .height(Fill)
            .into();
    }

    if let Some(ref err) = app.error {
        let content = column![
            text("Error loading repository").size(20),
            text(err).size(14).style(text::danger),
        ]
        .spacing(10);

        return center(content).width(Fill).height(Fill).into();
    }

    // Build the main layout
    let repo_names: Vec<String> = app.repos.iter().map(|r| r.name.clone()).collect();

    // Tab bar (hidden when only 1 repo)
    let tab_bar_element: Option<Element<'_, Message>> = tab_bar::view(&repo_names, app.active_repo)
        .map(|e| e.map(Message::TabBar));

    // Toolbar
    let toolbar_element: Element<'_, Message> = toolbar::view(None).map(Message::Toolbar);

    // Main content with panes
    let active_repo = &app.repos[app.active_repo];
    let pane_content = pane_grid::PaneGrid::new(&app.panes, |_pane, pane_type, _is_maximized| {
        let content: Element<'_, Message> = match pane_type {
            PaneType::Sidebar => {
                let sidebar_view = active_repo
                    .sidebar
                    .view()
                    .map(|m: sidebar::Message| repo_state::Message::Sidebar(m))
                    .map(|m| Message::Repo(app.active_repo, m));

                if active_repo.sidebar.is_open {
                    container(sidebar_view)
                        .style(container::bordered_box)
                        .width(Fill)
                        .height(Fill)
                        .into()
                } else {
                    container(sidebar_view).width(30).height(Fill).into()
                }
            }
            PaneType::CommitList => container(
                active_repo
                    .log_screen
                    .view()
                    .map(repo_state::Message::SelectCommit)
                    .map(|m| Message::Repo(app.active_repo, m)),
            )
            .style(container::bordered_box)
            .width(Fill)
            .height(Fill)
            .into(),
            PaneType::RightPanel => active_repo
                .right_panel
                .view(
                    active_repo.log_screen.selected_commit(),
                    &active_repo.files,
                    &active_repo.diffs,
                )
                .map(repo_state::Message::RightPanel)
                .map(|m| Message::Repo(app.active_repo, m)),
        };

        pane_grid::Content::new(content)
    })
    .on_resize(10, Message::PaneResized)
    .width(Fill)
    .height(Fill);

    // Assemble final layout
    let mut layout = column![].spacing(0);

    if let Some(tabs) = tab_bar_element {
        layout = layout.push(tabs);
    }

    layout = layout.push(toolbar_element);
    layout = layout.push(pane_content);

    container(layout).width(Fill).height(Fill).into()
}

fn theme(_app: &App) -> Theme {
    Theme::Dracula
}

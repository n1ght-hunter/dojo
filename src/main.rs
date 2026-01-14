mod error;
mod jj;
mod repo_state;
mod screens;
mod settings;
mod widgets;

use std::path::PathBuf;

use iced::widget::{center, column, container, text};
use iced::{Element, Fill, Subscription, Task, Theme};

use repo_state::RepoState;
use settings::Settings;
use widgets::{PaneState, panes, tab_bar, toolbar};

fn main() -> iced::Result {
    iced::application(boot, update, view)
        .title("Dojo")
        .theme(theme)
        .subscription(subscription)
        .run()
}

struct App {
    repos: Vec<RepoState>,
    active_repo: usize,
    panes: PaneState,
    loading: bool,
    error: Option<String>,
    settings: Settings,
    theme: Theme,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Tab bar
    TabBar(tab_bar::Message),
    // Toolbar
    Toolbar(toolbar::Message),
    // Route to specific repo
    Repo(usize, repo_state::Message),
    // Pane management
    Panes(panes::Message),
    // File dialog result for opening new repo
    RepoPathSelected(Option<PathBuf>),
}

fn subscription(_app: &App) -> Subscription<Message> {
    panes::subscription().map(Message::Panes)
}

fn boot() -> (App, Task<Message>) {
    let repo_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Create initial repo state
    let repo = RepoState::new(repo_path.clone());

    let app = App {
        repos: vec![repo],
        active_repo: 0,
        panes: PaneState::new(),
        loading: true,
        error: None,
        settings: Settings::default(),
        theme: Theme::Dracula,
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
                }
            }
            Task::none()
        }

        Message::Toolbar(msg) => {
            match msg {
                toolbar::Message::ToggleSidebar => {
                    app.panes
                        .update(panes::Message::ToggleSidebar, &app.settings);
                }
                _ => {}
            }
            Task::none()
        }

        Message::Repo(index, msg) => {
            if index < app.repos.len() {
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

        Message::Panes(msg) => {
            app.panes.update(msg, &app.settings);
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
    let tab_bar_element: Option<Element<'_, Message>> =
        tab_bar::view(&repo_names, app.active_repo).map(|e| e.map(Message::TabBar));

    // Toolbar
    let toolbar_element: Element<'_, Message> =
        toolbar::view(None, app.panes.sidebar_open()).map(Message::Toolbar);

    // Main content
    let active_repo = &app.repos[app.active_repo];
    let active_index = app.active_repo;

    // Pane grid
    let pane_content = app.panes.view(
        active_repo,
        &app.settings,
        &app.theme,
        move |m| Message::Repo(active_index, repo_state::Message::Sidebar(m)),
        move |m| Message::Repo(active_index, repo_state::Message::SelectCommit(m)),
        move |m| Message::Repo(active_index, repo_state::Message::RightPanel(m)),
        Message::Panes,
    );

    // Assemble final layout
    let mut layout = column![].spacing(0);

    if let Some(tabs) = tab_bar_element {
        layout = layout.push(tabs);
    }

    layout = layout.push(toolbar_element);
    layout = layout.push(pane_content);

    container(layout).width(Fill).height(Fill).into()
}

fn theme(app: &App) -> Theme {
    app.theme.clone()
}

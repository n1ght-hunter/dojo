mod components;
mod repo_state;
mod screens;
mod settings;
mod state_wrapper;
use std::path::PathBuf;

use iced::widget::{center, column, container, text};
use iced::window;
use iced::{Element, Event, Fill, Subscription, Task, Theme};

use components::{PaneState, panes, right_panel, tab_bar, toolbar};
use dojo_jj::WorkspaceEvent;
use repo_state::RepoState;
use settings::Settings;
use state_wrapper::StateMut;

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
    // Window focus - triggers snapshot and refresh
    WindowFocused,
}

fn subscription(app: &App) -> Subscription<Message> {
    let pane_sub = panes::subscription().map(Message::Panes);

    // Listen for window focus events to trigger snapshot/refresh
    let focus_sub = iced::event::listen_with(|event, _status, _id| {
        if let Event::Window(window::Event::Focused) = event {
            Some(Message::WindowFocused)
        } else {
            None
        }
    });

    // Worker subscriptions for each repo
    let repo_subs = app.repos.iter().enumerate().map(|(i, repo)| {
        repo.subscription()
            .with(i)
            .map(|(i, msg)| Message::Repo(i, msg))
    });

    // Get subscription from active repo's right panel (for keyboard shortcuts)
    let right_panel_sub = app.repos.get(app.active_repo).map(|repo| {
        let active_index = app.active_repo;
        right_panel::subscription(&repo.right_panel)
            .with(active_index)
            .map(|(index, msg)| Message::Repo(index, repo_state::Message::RightPanel(msg)))
    });

    let mut subs: Vec<Subscription<Message>> = vec![pane_sub, focus_sub];
    subs.extend(repo_subs);
    if let Some(rp_sub) = right_panel_sub {
        subs.push(rp_sub);
    }

    Subscription::batch(subs)
}

fn boot() -> (App, Task<Message>) {
    let repo_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Create initial repo state
    let repo = RepoState::new(repo_path);

    let app = App {
        repos: vec![repo],
        active_repo: 0,
        panes: PaneState::new(),
        loading: true,
        error: None,
        settings: Settings::default(),
        theme: Theme::Dracula,
    };

    // Worker subscription handles initial loading
    (app, Task::none())
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
                // Check for worker Loaded event to update app-level loading state
                if let repo_state::Message::Worker(WorkspaceEvent::Loaded(result)) = &msg {
                    app.loading = false;
                    if let Err(e) = result {
                        app.error = Some(e.to_string());
                    } else {
                        app.error = None;
                    }
                }

                // Clone tx before mutable borrow
                let tx = app.repos[index].command_tx.clone();
                let mut state = StateMut::new(&mut app.repos[index], &mut app.settings);
                if let Some(ref tx) = tx {
                    state = state.with_worker(tx);
                }
                repo_state::update(state, msg).map(move |m| Message::Repo(index, m))
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
                app.repos.push(RepoState::new(path));
                app.active_repo = index;
                // Worker subscription will handle loading automatically
            }
            Task::none()
        }

        Message::WindowFocused => {
            // Trigger refresh via worker to capture any file changes
            if let Some(repo) = app.repos.get(app.active_repo) {
                repo.refresh();
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

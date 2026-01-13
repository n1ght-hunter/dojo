mod jj;
mod screens;
mod widgets;

use std::path::PathBuf;

use iced::widget::{center, column, container, text};
use iced::{Element, Fill, Task, Theme};

use jj::{CommitInfo, RepoHandle};
use screens::{DetailsPane, LogScreen};

fn main() -> iced::Result {
    iced::application(boot, update, view)
        .title("Dojo")
        .theme(theme)
        .run()
}

struct App {
    repo_path: PathBuf,
    state: AppState,
    log_screen: LogScreen,
}

enum AppState {
    Loading,
    Loaded,
    Error(String),
}

#[derive(Debug, Clone)]
enum Message {
    RepoLoaded(Result<Vec<CommitInfo>, String>),
    SelectCommit(usize),
}

fn boot() -> (App, Task<Message>) {
    let repo_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let path = repo_path.clone();

    let app = App {
        repo_path,
        state: AppState::Loading,
        log_screen: LogScreen::new(),
    };

    // Load repository asynchronously
    let task = Task::perform(async move { load_repo(&path) }, Message::RepoLoaded);

    (app, task)
}

fn update(state: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::RepoLoaded(result) => {
            match result {
                Ok(commits) => {
                    state.log_screen.set_commits(commits);
                    state.state = AppState::Loaded;
                }
                Err(e) => {
                    state.state = AppState::Error(e);
                }
            }
            Task::none()
        }
        Message::SelectCommit(index) => {
            state.log_screen.select(index);
            Task::none()
        }
    }
}

fn view(state: &App) -> Element<'_, Message> {
    match &state.state {
        AppState::Loading => center(text("Loading repository...").size(20))
            .width(Fill)
            .height(Fill)
            .into(),
        AppState::Error(err) => {
            let content = column![
                text("Error loading repository").size(20),
                text(err).size(14).color([1.0, 0.33, 0.33]),
            ]
            .spacing(10);

            center(content).width(Fill).height(Fill).into()
        }
        AppState::Loaded => {
            // Main layout: log screen on top, details pane on bottom
            let selected = state.log_screen.selected_commit();

            let content = column![state.log_screen.view(), DetailsPane::view(selected),];

            container(content).width(Fill).height(Fill).into()
        }
    }
}

fn theme(_state: &App) -> Theme {
    Theme::Dracula
}

fn load_repo(path: &PathBuf) -> Result<Vec<CommitInfo>, String> {
    let handle = RepoHandle::open(path).map_err(|e| e.to_string())?;
    handle.log(100).map_err(|e| e.to_string())
}

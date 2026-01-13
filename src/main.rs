use iced::{Element, Theme};

fn main() -> iced::Result {
    iced::application(App::default, update, view)
        .title("Dojo")
        .theme(theme)
        .run()
}

#[derive(Default)]
struct App {}

#[derive(Debug, Clone)]
enum Message {}

fn update(_state: &mut App, message: Message) {
    match message {}
}

fn view(_state: &App) -> Element<'_, Message> {
    iced::widget::text("Dojo - JJ GUI").into()
}

fn theme(_state: &App) -> Theme {
    Theme::Dracula
}

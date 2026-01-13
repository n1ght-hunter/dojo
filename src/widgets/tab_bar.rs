use iced::widget::{button, container, row, text, Row};
use iced::{Element, Fill, Length};

/// Messages for tab bar interactions
#[derive(Debug, Clone)]
pub enum Message {
    Select(usize),
    Close(usize),
    OpenNew,
}

/// Render the tab bar for multiple repositories
/// Hidden when only one repo is open
pub fn view<'a>(repo_names: &[String], active: usize) -> Option<Element<'a, Message>> {
    // Hide tab bar when only one repo
    if repo_names.len() <= 1 {
        return None;
    }

    let mut tabs: Row<'a, Message> = Row::new().spacing(0);

    for (i, name) in repo_names.iter().enumerate() {
        let is_active = i == active;
        tabs = tabs.push(tab_button(i, name.clone(), is_active));
    }

    // Add new repo button
    tabs = tabs.push(new_repo_button());

    // Spacer to push tabs left
    tabs = tabs.push(container(text("")).width(Fill));

    Some(
        container(tabs)
            .width(Fill)
            .height(Length::Fixed(32.0))
            .style(container::bordered_box)
            .into(),
    )
}

fn tab_button(index: usize, name: String, is_active: bool) -> Element<'static, Message> {
    let tab_content = row![
        button(text(name).size(12))
            .on_press(Message::Select(index))
            .padding([8, 12])
            .style(if is_active {
                button::primary
            } else {
                button::secondary
            }),
        button(text("×").size(12))
            .on_press(Message::Close(index))
            .padding([8, 4])
            .style(button::text),
    ]
    .spacing(0);

    container(tab_content).into()
}

fn new_repo_button() -> Element<'static, Message> {
    button(text("+").size(14))
        .on_press(Message::OpenNew)
        .padding([8, 12])
        .style(button::text)
        .into()
}

use iced::widget::{button, container, row, text, Space};
use iced::{Element, Fill, Length};

/// Messages for toolbar interactions (placeholder - not yet implemented)
#[derive(Debug, Clone)]
pub enum Message {
    Back,
    Forward,
    Menu,
    Terminal,
    Search,
    More,
    Pull,
    Push,
}

/// Render the toolbar (placeholder buttons - visual only for now)
pub fn view<'a>(current_bookmark: Option<&'a str>) -> Element<'a, Message> {
    let bookmark_name = current_bookmark.unwrap_or("main");

    let left_section = row![
        toolbar_button("☐", Message::Menu),
        toolbar_button("◀", Message::Back),
        toolbar_button("▶", Message::Forward),
    ]
    .spacing(2);

    let center_section = row![
        toolbar_button("≡", Message::Menu),
        toolbar_button(">_", Message::Terminal),
        text(bookmark_name).size(12).style(text::primary),
    ]
    .spacing(8);

    let right_section = row![
        toolbar_button("🔍", Message::Search),
        toolbar_button("···", Message::More),
        toolbar_button("↓ Pull", Message::Pull),
        toolbar_button("↑ Push", Message::Push),
    ]
    .spacing(4);

    container(
        row![
            left_section,
            Space::new().width(Length::Fixed(20.0)),
            center_section,
            Space::new().width(Fill),
            right_section,
        ]
        .padding([6, 12])
        .spacing(8),
    )
    .width(Fill)
    .height(Length::Fixed(36.0))
    .style(container::bordered_box)
    .into()
}

fn toolbar_button(label: &str, msg: Message) -> Element<'static, Message> {
    button(text(label.to_string()).size(12))
        .on_press(msg)
        .padding([4, 8])
        .style(button::secondary)
        .into()
}

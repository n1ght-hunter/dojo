use iced::widget::{container, rich_text, row, text, text::Span, Column};
use iced::{color, Background, Border, Element, Fill, Length, Theme};

use crate::jj::{DiffLine, FileDiff};

/// Render a file's diff content (inline unified view)
pub fn view_file_diff_content<'a, M: 'a>(diff: &'a FileDiff) -> Element<'a, M> {
    let mut lines: Column<'_, M> = Column::new().spacing(0);

    let mut old_line = 1u32;
    let mut new_line = 1u32;

    for diff_line in &diff.lines {
        match diff_line {
            DiffLine::Hunk {
                old_start,
                old_count,
                new_start,
                new_count,
            } => {
                old_line = *old_start;
                new_line = *new_start;
                lines = lines.push(hunk_line::<M>(*old_start, *old_count, *new_start, *new_count));
            }
            DiffLine::Context(content) => {
                lines = lines.push(context_line::<M>(old_line, new_line, content.clone()));
                old_line += 1;
                new_line += 1;
            }
            DiffLine::Added(content) => {
                lines = lines.push(added_line::<M>(new_line, content.clone()));
                new_line += 1;
            }
            DiffLine::Removed(content) => {
                lines = lines.push(removed_line::<M>(old_line, content.clone()));
                old_line += 1;
            }
        }
    }

    container(lines).width(Fill).into()
}

fn hunk_line<'a, M: 'a>(old_start: u32, old_count: u32, new_start: u32, new_count: u32) -> Element<'a, M> {
    let hunk_text = format!(
        "@@ -{},{} +{},{} @@",
        old_start, old_count, new_start, new_count
    );

    container(
        row![
            gutter::<M>("".to_string(), "".to_string(), LineKind::Hunk),
            container(text(hunk_text).size(11).style(text::primary))
                .padding([2, 8])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .style(hunk_style)
    .into()
}

fn context_line<'a, M: 'a>(old_num: u32, new_num: u32, content: String) -> Element<'a, M> {
    row![
        gutter::<M>(format!("{}", old_num), format!("{}", new_num), LineKind::Context),
        container(text(format!(" {}", content)).size(11))
            .padding([2, 4])
            .width(Fill),
    ]
    .spacing(0)
    .into()
}

fn added_line<'a, M: 'a>(new_num: u32, content: String) -> Element<'a, M> {
    let spans: Vec<Span<'a, (), _>> = vec![
        Span::new("+").color(color!(0x50FA7B)),
        Span::new(content).color(color!(0x50FA7B)),
    ];

    container(
        row![
            gutter::<M>("".to_string(), format!("{}", new_num), LineKind::Added),
            container(rich_text(spans).size(11))
                .padding([2, 4])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .style(added_line_style)
    .into()
}

fn removed_line<'a, M: 'a>(old_num: u32, content: String) -> Element<'a, M> {
    let spans: Vec<Span<'a, (), _>> = vec![
        Span::new("-").color(color!(0xFF5555)),
        Span::new(content).color(color!(0xFF5555)),
    ];

    container(
        row![
            gutter::<M>(format!("{}", old_num), "".to_string(), LineKind::Removed),
            container(rich_text(spans).size(11))
                .padding([2, 4])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .style(removed_line_style)
    .into()
}

#[derive(Clone, Copy)]
enum LineKind {
    Context,
    Added,
    Removed,
    Hunk,
}

fn gutter<'a, M: 'a>(old_num: String, new_num: String, kind: LineKind) -> Element<'a, M> {
    let old_display = if old_num.is_empty() {
        "    ".to_string()
    } else {
        format!("{:>4}", old_num)
    };
    let new_display = if new_num.is_empty() {
        "    ".to_string()
    } else {
        format!("{:>4}", new_num)
    };

    let style = match kind {
        LineKind::Context | LineKind::Hunk => gutter_style,
        LineKind::Added => added_gutter_style,
        LineKind::Removed => removed_gutter_style,
    };

    container(
        row![
            container(text(old_display).size(10).style(text::default))
                .width(Length::Fixed(36.0))
                .padding([2, 4]),
            container(text(new_display).size(10).style(text::default))
                .width(Length::Fixed(36.0))
                .padding([2, 4]),
        ]
        .spacing(0),
    )
    .width(Length::Fixed(72.0))
    .style(style)
    .into()
}

// Styles
fn gutter_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x21222C))),
        border: Border::default(),
        ..Default::default()
    }
}

fn hunk_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x44475A))),
        ..Default::default()
    }
}

fn added_line_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x50FA7B, 0.12))),
        ..Default::default()
    }
}

fn removed_line_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0xFF5555, 0.12))),
        ..Default::default()
    }
}

fn added_gutter_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0x50FA7B, 0.20))),
        ..Default::default()
    }
}

fn removed_gutter_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(color!(0xFF5555, 0.20))),
        ..Default::default()
    }
}

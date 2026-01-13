use iced::widget::{container, rich_text, row, text, text::Span, Column, Row};
use iced::{Background, Border, Color, Element, Fill, Length, Theme};

use crate::jj::{ChangeKind, DiffLine, DiffSegment, FileDiff};
use crate::settings::DiffSettings;

/// Colors for diff rendering, derived from theme
struct DiffColors {
    added_bg: Color,
    added_highlight: Color,
    added_text: Color,
    removed_bg: Color,
    removed_highlight: Color,
    removed_text: Color,
    gutter_bg: Color,
    hunk_bg: Color,
}

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color {
        a: alpha,
        ..color
    }
}

fn get_diff_colors(theme: &Theme) -> DiffColors {
    let palette = theme.extended_palette();

    DiffColors {
        added_bg: with_alpha(palette.success.weak.color, 0.15),
        added_highlight: with_alpha(palette.success.strong.color, 0.35),
        added_text: palette.success.base.color,
        removed_bg: with_alpha(palette.danger.weak.color, 0.15),
        removed_highlight: with_alpha(palette.danger.strong.color, 0.35),
        removed_text: palette.danger.base.color,
        gutter_bg: with_alpha(palette.background.weak.color, 0.5),
        hunk_bg: with_alpha(palette.primary.weak.color, 0.2),
    }
}

/// Render a file's diff content with responsive layout
pub fn view_file_diff_content<'a, M: 'a>(
    diff: &'a FileDiff,
    width: f32,
    settings: &DiffSettings,
    theme: &Theme,
) -> Element<'a, M> {
    let colors = get_diff_colors(theme);

    // Single column for new/deleted files
    if diff.kind == ChangeKind::Added {
        return view_single_column(diff, &colors, true);
    }
    if diff.kind == ChangeKind::Deleted {
        return view_single_column(diff, &colors, false);
    }

    // Side-by-side or unified based on width
    if width >= settings.side_by_side_threshold as f32 {
        view_side_by_side(diff, &colors)
    } else {
        view_unified(diff, &colors)
    }
}

/// Unified diff view (single column with + and -)
fn view_unified<'a, M: 'a>(diff: &FileDiff, colors: &DiffColors) -> Element<'a, M> {
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
                lines = lines.push(hunk_line::<M>(*old_start, *old_count, *new_start, *new_count, colors));
            }
            DiffLine::Context(content) => {
                lines = lines.push(context_line::<M>(old_line, new_line, content.clone(), colors));
                old_line += 1;
                new_line += 1;
            }
            DiffLine::Added(segments) => {
                lines = lines.push(added_line::<M>(new_line, segments, colors));
                new_line += 1;
            }
            DiffLine::Removed(segments) => {
                lines = lines.push(removed_line::<M>(old_line, segments, colors));
                old_line += 1;
            }
        }
    }

    container(lines).width(Fill).into()
}

/// Side-by-side diff view
fn view_side_by_side<'a, M: 'a>(diff: &FileDiff, colors: &DiffColors) -> Element<'a, M> {
    let mut left_lines: Column<'_, M> = Column::new().spacing(0);
    let mut right_lines: Column<'_, M> = Column::new().spacing(0);

    let mut old_line = 1u32;
    let mut new_line = 1u32;

    // Process diff lines and pair them for side-by-side
    let mut i = 0;
    let lines = &diff.lines;

    while i < lines.len() {
        match &lines[i] {
            DiffLine::Hunk {
                old_start,
                old_count,
                new_start,
                new_count,
            } => {
                old_line = *old_start;
                new_line = *new_start;
                let hunk_text = format!(
                    "@@ -{},{} +{},{} @@",
                    old_start, old_count, new_start, new_count
                );
                left_lines = left_lines.push(side_hunk_line::<M>(hunk_text.clone(), colors));
                right_lines = right_lines.push(side_hunk_line::<M>(hunk_text, colors));
                i += 1;
            }
            DiffLine::Context(content) => {
                left_lines = left_lines.push(side_context_line::<M>(old_line, content, colors));
                right_lines = right_lines.push(side_context_line::<M>(new_line, content, colors));
                old_line += 1;
                new_line += 1;
                i += 1;
            }
            DiffLine::Removed(segments) => {
                // Check if next line is Added (paired change)
                if i + 1 < lines.len() {
                    if let DiffLine::Added(add_segments) = &lines[i + 1] {
                        // Paired: removed on left, added on right
                        left_lines = left_lines.push(side_removed_line::<M>(old_line, segments, colors));
                        right_lines = right_lines.push(side_added_line::<M>(new_line, add_segments, colors));
                        old_line += 1;
                        new_line += 1;
                        i += 2;
                        continue;
                    }
                }
                // Unpaired removed: left side only, blank on right
                left_lines = left_lines.push(side_removed_line::<M>(old_line, segments, colors));
                right_lines = right_lines.push(side_empty_line::<M>(colors));
                old_line += 1;
                i += 1;
            }
            DiffLine::Added(segments) => {
                // Unpaired added: blank on left, added on right
                left_lines = left_lines.push(side_empty_line::<M>(colors));
                right_lines = right_lines.push(side_added_line::<M>(new_line, segments, colors));
                new_line += 1;
                i += 1;
            }
        }
    }

    Row::new()
        .push(container(left_lines).width(Fill))
        .push(container(right_lines).width(Fill))
        .spacing(1)
        .into()
}

/// Single column view for new/deleted files
fn view_single_column<'a, M: 'a>(diff: &FileDiff, colors: &DiffColors, is_new: bool) -> Element<'a, M> {
    let mut lines: Column<'_, M> = Column::new().spacing(0);
    let mut line_num = 1u32;

    for diff_line in &diff.lines {
        match diff_line {
            DiffLine::Hunk { new_start, .. } if is_new => {
                line_num = *new_start;
            }
            DiffLine::Hunk { old_start, .. } => {
                line_num = *old_start;
            }
            DiffLine::Context(content) => {
                lines = lines.push(single_context_line::<M>(line_num, content, colors));
                line_num += 1;
            }
            DiffLine::Added(segments) if is_new => {
                lines = lines.push(single_added_line::<M>(line_num, segments, colors));
                line_num += 1;
            }
            DiffLine::Removed(segments) if !is_new => {
                lines = lines.push(single_removed_line::<M>(line_num, segments, colors));
                line_num += 1;
            }
            _ => {}
        }
    }

    container(lines).width(Fill).into()
}

// === Unified view line helpers ===

fn hunk_line<'a, M: 'a>(old_start: u32, old_count: u32, new_start: u32, new_count: u32, colors: &DiffColors) -> Element<'a, M> {
    let hunk_text = format!(
        "@@ -{},{} +{},{} @@",
        old_start, old_count, new_start, new_count
    );
    let hunk_bg = colors.hunk_bg;
    let gutter_bg = colors.gutter_bg;

    container(
        row![
            gutter::<M>("".to_string(), "".to_string(), gutter_bg),
            container(text(hunk_text).size(11).style(text::primary))
                .padding([2, 8])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(hunk_bg)),
        ..Default::default()
    })
    .into()
}

fn context_line<'a, M: 'a>(old_num: u32, new_num: u32, content: String, colors: &DiffColors) -> Element<'a, M> {
    let gutter_bg = colors.gutter_bg;

    row![
        gutter::<M>(format!("{}", old_num), format!("{}", new_num), gutter_bg),
        container(text(format!(" {}", content)).size(11))
            .padding([2, 4])
            .width(Fill),
    ]
    .spacing(0)
    .into()
}

fn added_line<'a, M: 'a>(new_num: u32, segments: &[DiffSegment], colors: &DiffColors) -> Element<'a, M> {
    let text_color = colors.added_text;
    let bg_color = colors.added_bg;
    let highlight_color = colors.added_highlight;
    let gutter_highlight = colors.added_highlight;

    let mut spans: Vec<Span<'a, (), _>> = vec![Span::new("+").color(text_color)];
    for seg in segments {
        let span = Span::new(seg.text.clone()).color(text_color);
        if seg.highlighted {
            spans.push(span.background(highlight_color));
        } else {
            spans.push(span);
        }
    }

    container(
        row![
            gutter::<M>("".to_string(), format!("{}", new_num), gutter_highlight),
            container(rich_text(spans).size(11))
                .padding([2, 4])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(bg_color)),
        ..Default::default()
    })
    .into()
}

fn removed_line<'a, M: 'a>(old_num: u32, segments: &[DiffSegment], colors: &DiffColors) -> Element<'a, M> {
    let text_color = colors.removed_text;
    let bg_color = colors.removed_bg;
    let highlight_color = colors.removed_highlight;
    let gutter_highlight = colors.removed_highlight;

    let mut spans: Vec<Span<'a, (), _>> = vec![Span::new("-").color(text_color)];
    for seg in segments {
        let span = Span::new(seg.text.clone()).color(text_color);
        if seg.highlighted {
            spans.push(span.background(highlight_color));
        } else {
            spans.push(span);
        }
    }

    container(
        row![
            gutter::<M>(format!("{}", old_num), "".to_string(), gutter_highlight),
            container(rich_text(spans).size(11))
                .padding([2, 4])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(bg_color)),
        ..Default::default()
    })
    .into()
}

fn gutter<'a, M: 'a>(old_num: String, new_num: String, bg: Color) -> Element<'a, M> {
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
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(bg)),
        border: Border::default(),
        ..Default::default()
    })
    .into()
}

// === Side-by-side view line helpers ===

fn side_hunk_line<'a, M: 'a>(hunk_text: String, colors: &DiffColors) -> Element<'a, M> {
    let hunk_bg = colors.hunk_bg;

    container(text(hunk_text).size(11).style(text::primary))
        .padding([2, 8])
        .width(Fill)
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(hunk_bg)),
            ..Default::default()
        })
        .into()
}

fn side_context_line<'a, M: 'a>(line_num: u32, content: &str, colors: &DiffColors) -> Element<'a, M> {
    let gutter_bg = colors.gutter_bg;

    row![
        side_gutter::<M>(line_num, gutter_bg),
        container(text(format!(" {}", content)).size(11))
            .padding([2, 4])
            .width(Fill),
    ]
    .spacing(0)
    .into()
}

fn side_added_line<'a, M: 'a>(line_num: u32, segments: &[DiffSegment], colors: &DiffColors) -> Element<'a, M> {
    let text_color = colors.added_text;
    let bg_color = colors.added_bg;
    let highlight_color = colors.added_highlight;
    let gutter_highlight = colors.added_highlight;

    let mut spans: Vec<Span<'a, (), _>> = Vec::new();
    for seg in segments {
        let span = Span::new(seg.text.clone()).color(text_color);
        if seg.highlighted {
            spans.push(span.background(highlight_color));
        } else {
            spans.push(span);
        }
    }

    container(
        row![
            side_gutter::<M>(line_num, gutter_highlight),
            container(rich_text(spans).size(11))
                .padding([2, 4])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(bg_color)),
        ..Default::default()
    })
    .into()
}

fn side_removed_line<'a, M: 'a>(line_num: u32, segments: &[DiffSegment], colors: &DiffColors) -> Element<'a, M> {
    let text_color = colors.removed_text;
    let bg_color = colors.removed_bg;
    let highlight_color = colors.removed_highlight;
    let gutter_highlight = colors.removed_highlight;

    let mut spans: Vec<Span<'a, (), _>> = Vec::new();
    for seg in segments {
        let span = Span::new(seg.text.clone()).color(text_color);
        if seg.highlighted {
            spans.push(span.background(highlight_color));
        } else {
            spans.push(span);
        }
    }

    container(
        row![
            side_gutter::<M>(line_num, gutter_highlight),
            container(rich_text(spans).size(11))
                .padding([2, 4])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(bg_color)),
        ..Default::default()
    })
    .into()
}

fn side_empty_line<'a, M: 'a>(colors: &DiffColors) -> Element<'a, M> {
    let gutter_bg = colors.gutter_bg;

    container(
        row![
            side_gutter::<M>(0, gutter_bg),
            container(text("").size(11))
                .padding([2, 4])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .into()
}

fn side_gutter<'a, M: 'a>(line_num: u32, bg: Color) -> Element<'a, M> {
    let display = if line_num == 0 {
        "    ".to_string()
    } else {
        format!("{:>4}", line_num)
    };

    container(text(display).size(10).style(text::default))
        .width(Length::Fixed(36.0))
        .padding([2, 4])
        .style(move |_theme: &Theme| container::Style {
            background: Some(Background::Color(bg)),
            border: Border::default(),
            ..Default::default()
        })
        .into()
}

// === Single column view line helpers ===

fn single_context_line<'a, M: 'a>(line_num: u32, content: &str, colors: &DiffColors) -> Element<'a, M> {
    let gutter_bg = colors.gutter_bg;

    row![
        side_gutter::<M>(line_num, gutter_bg),
        container(text(format!(" {}", content)).size(11))
            .padding([2, 4])
            .width(Fill),
    ]
    .spacing(0)
    .into()
}

fn single_added_line<'a, M: 'a>(line_num: u32, segments: &[DiffSegment], colors: &DiffColors) -> Element<'a, M> {
    let text_color = colors.added_text;
    let bg_color = colors.added_bg;
    let highlight_color = colors.added_highlight;
    let gutter_highlight = colors.added_highlight;

    let mut spans: Vec<Span<'a, (), _>> = vec![Span::new("+").color(text_color)];
    for seg in segments {
        let span = Span::new(seg.text.clone()).color(text_color);
        if seg.highlighted {
            spans.push(span.background(highlight_color));
        } else {
            spans.push(span);
        }
    }

    container(
        row![
            side_gutter::<M>(line_num, gutter_highlight),
            container(rich_text(spans).size(11))
                .padding([2, 4])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(bg_color)),
        ..Default::default()
    })
    .into()
}

fn single_removed_line<'a, M: 'a>(line_num: u32, segments: &[DiffSegment], colors: &DiffColors) -> Element<'a, M> {
    let text_color = colors.removed_text;
    let bg_color = colors.removed_bg;
    let highlight_color = colors.removed_highlight;
    let gutter_highlight = colors.removed_highlight;

    let mut spans: Vec<Span<'a, (), _>> = vec![Span::new("-").color(text_color)];
    for seg in segments {
        let span = Span::new(seg.text.clone()).color(text_color);
        if seg.highlighted {
            spans.push(span.background(highlight_color));
        } else {
            spans.push(span);
        }
    }

    container(
        row![
            side_gutter::<M>(line_num, gutter_highlight),
            container(rich_text(spans).size(11))
                .padding([2, 4])
                .width(Fill),
        ]
        .spacing(0),
    )
    .width(Fill)
    .style(move |_theme: &Theme| container::Style {
        background: Some(Background::Color(bg_color)),
        ..Default::default()
    })
    .into()
}

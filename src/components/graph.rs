use iced::mouse;
use iced::widget::canvas::{self, Canvas, Event, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Font, Length, Point, Rectangle, Theme};
use std::collections::HashMap;

use dojo_jj::CommitInfo;

/// Column spacing for graph lines
const COLUMN_WIDTH: f32 = 18.0;
/// Vertical spacing between commits (two-line layout)
const ROW_HEIGHT: f32 = 50.0;
/// Node radius
const NODE_RADIUS: f32 = 6.0;
/// Font size for text
const FONT_SIZE: f32 = 12.0;
/// Smaller font size for secondary text
const FONT_SIZE_SMALL: f32 = 11.0;
/// Font size for stats
const FONT_SIZE_STATS: f32 = 10.0;

/// Vertical offset for line 1 (change_id, author, date)
const LINE1_Y_OFFSET: f32 = 16.0;
/// Vertical offset for line 2 (description, stats)
const LINE2_Y_OFFSET: f32 = 36.0;
/// Node Y offset (aligned with line 1)
const NODE_Y_OFFSET: f32 = 16.0;

/// State for the graph canvas (tracks hover)
#[derive(Default, Debug, Clone)]
pub struct GraphState {
    hovered_row: Option<usize>,
}

/// A line in the graph with source and target coordinates
/// Following GG's model: each line knows its full extent
#[derive(Debug, Clone)]
struct GraphLine {
    source_col: usize,
    source_row: usize,
    target_col: usize,
    target_row: usize, // Row where line ends (may be beyond displayed commits)
    indirect: bool,
}

impl GraphLine {
    /// Check if this line passes through the given row
    fn passes_row(&self, row: usize) -> bool {
        row >= self.source_row && row < self.target_row
    }

    /// Check if this line is vertical (same column)
    fn is_vertical(&self) -> bool {
        self.source_col == self.target_col
    }
}

/// A row in the graph display
#[derive(Debug, Clone)]
struct GraphRow {
    column: usize,
    is_working_copy: bool,
    is_immutable: bool,
    change_id: String,
    description: String,
    author: String,
    timestamp: String,
    bookmarks: Vec<String>,
    // File statistics (optional, loaded asynchronously)
    file_count: Option<usize>,
    lines_added: Option<usize>,
    lines_removed: Option<usize>,
}

/// Graph and commit info canvas
#[derive(Debug, Clone)]
pub struct GraphColumn {
    rows: Vec<GraphRow>,
    lines: Vec<GraphLine>,
    max_column: usize,
    selected_index: Option<usize>,
}

impl GraphColumn {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            lines: Vec::new(),
            max_column: 0,
            selected_index: None,
        }
    }

    /// Compute the graph layout from commits
    /// Based on GG's algorithm: track stems, create lines with full coordinates
    pub fn compute(&mut self, commits: &[CommitInfo]) {
        self.rows.clear();
        self.lines.clear();
        self.max_column = 0;

        if commits.is_empty() {
            return;
        }

        let num_rows = commits.len();

        // Build commit_id -> row index map
        let commit_to_row: HashMap<&str, usize> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.commit_id.as_str(), i))
            .collect();

        // Stems: ongoing vertical lines waiting for a commit
        // Each stem tracks: source coordinates, target commit, column
        struct Stem {
            source_col: usize,
            source_row: usize,
            target_id: String,
            indirect: bool,
        }

        let mut stems: Vec<Option<Stem>> = Vec::new();

        for (row_idx, commit) in commits.iter().enumerate() {
            // Find stem targeting this commit
            let targeting_stem = stems.iter().position(|s| {
                s.as_ref()
                    .map(|stem| stem.target_id == commit.commit_id)
                    .unwrap_or(false)
            });

            let column = if let Some(slot) = targeting_stem {
                slot
            } else {
                // Find first empty slot or append
                stems.iter().position(|s| s.is_none()).unwrap_or_else(|| {
                    stems.push(None);
                    stems.len() - 1
                })
            };

            // Ensure stems vector is large enough
            while stems.len() <= column {
                stems.push(None);
            }

            // If there was a stem targeting this commit, create a line from it
            if let Some(terminated) = stems[column].take() {
                self.lines.push(GraphLine {
                    source_col: terminated.source_col,
                    source_row: terminated.source_row,
                    target_col: column,
                    target_row: row_idx,
                    indirect: terminated.indirect,
                });
            }

            // Process parent edges - create stems for parents
            // Use parent_edges which includes the is_indirect flag for elided revisions
            for (i, parent_edge) in commit.parent_edges.iter().enumerate() {
                let parent_id = &parent_edge.commit_id;
                let indirect = parent_edge.is_indirect;

                // Check if parent already has a stem
                let existing_stem = stems.iter().position(|s| {
                    s.as_ref()
                        .map(|stem| &stem.target_id == parent_id)
                        .unwrap_or(false)
                });

                if let Some(stem_col) = existing_stem {
                    // Parent already has a stem - create intersection line to it
                    // The line goes from this node down to the next row in that column
                    if stem_col != column {
                        // Find where this parent actually is (or use last row if not displayed)
                        let parent_row = commit_to_row
                            .get(parent_id.as_str())
                            .copied()
                            .unwrap_or(num_rows);

                        self.lines.push(GraphLine {
                            source_col: column,
                            source_row: row_idx,
                            target_col: stem_col,
                            target_row: parent_row,
                            indirect,
                        });
                    }
                } else {
                    // Create new stem for this parent
                    let stem_col =
                        if i == 0 && stems.get(column).map(|s| s.is_none()).unwrap_or(true) {
                            // First parent: use same column if available
                            column
                        } else {
                            // Additional parents: append new column
                            stems.push(None);
                            stems.len() - 1
                        };

                    while stems.len() <= stem_col {
                        stems.push(None);
                    }

                    stems[stem_col] = Some(Stem {
                        source_col: column,
                        source_row: row_idx,
                        target_id: parent_id.clone(),
                        indirect,
                    });

                    // If stem goes to different column, it's a merge line
                    // The vertical part will be drawn by the stem continuing down
                }
            }

            // Track max column
            self.max_column = self.max_column.max(column);
            for (i, stem) in stems.iter().enumerate() {
                if stem.is_some() {
                    self.max_column = self.max_column.max(i);
                }
            }

            // Extract file stats from commit if available
            let (file_count, lines_added, lines_removed) =
                if let Some(ref stats) = commit.file_stats {
                    (
                        Some(stats.file_count),
                        Some(stats.lines_added),
                        Some(stats.lines_removed),
                    )
                } else {
                    (None, None, None)
                };

            self.rows.push(GraphRow {
                column,
                is_working_copy: commit.is_working_copy,
                is_immutable: commit.is_immutable,
                change_id: commit.change_id.chars().take(8).collect(),
                description: commit.description.lines().next().unwrap_or("").to_string(),
                author: commit.author.clone(),
                timestamp: commit.timestamp.format("%Y-%m-%d %H:%M").to_string(),
                bookmarks: commit.bookmarks.clone(),
                file_count,
                lines_added,
                lines_removed,
            });
        }

        // Create lines for any remaining stems (going to commits not displayed)
        for (col, stem_opt) in stems.iter().enumerate() {
            if let Some(stem) = stem_opt {
                self.lines.push(GraphLine {
                    source_col: stem.source_col,
                    source_row: stem.source_row,
                    target_col: col,
                    target_row: num_rows, // Goes to bottom
                    indirect: stem.indirect,
                });
            }
        }
    }

    fn graph_width(&self) -> f32 {
        (self.max_column as f32 + 2.0) * COLUMN_WIDTH
    }

    pub fn set_selected(&mut self, index: Option<usize>) {
        self.selected_index = index;
    }

    pub fn view(&self, _commits: &[CommitInfo]) -> Element<'_, usize> {
        let height = (self.rows.len() as f32) * ROW_HEIGHT;

        Canvas::new(GraphRenderer {
            graph: self,
            selected_index: self.selected_index,
        })
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .into()
    }
}

impl Default for GraphColumn {
    fn default() -> Self {
        Self::new()
    }
}

struct GraphRenderer<'a> {
    graph: &'a GraphColumn,
    selected_index: Option<usize>,
}

fn get_lane_colors(theme: &Theme) -> Vec<Color> {
    let palette = theme.extended_palette();
    vec![
        palette.primary.strong.color,
        palette.secondary.strong.color,
        palette.success.strong.color,
        palette.danger.strong.color,
        palette.primary.weak.color,
        palette.secondary.weak.color,
    ]
}

/// Get the row index at a given y position
fn row_at_position(y: f32, num_rows: usize) -> Option<usize> {
    if y < 0.0 {
        return None;
    }
    let row = (y / ROW_HEIGHT) as usize;
    if row < num_rows { Some(row) } else { None }
}

impl canvas::Program<usize> for GraphRenderer<'_> {
    type State = GraphState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<usize>> {
        let cursor_position = cursor.position_in(bounds);

        match event {
            Event::Mouse(mouse_event) => match mouse_event {
                mouse::Event::CursorMoved { .. } => {
                    // Update hovered row
                    let new_hovered = cursor_position
                        .and_then(|pos| row_at_position(pos.y, self.graph.rows.len()));

                    if new_hovered != state.hovered_row {
                        state.hovered_row = new_hovered;
                        // Request redraw when hover changes
                        return Some(canvas::Action::request_redraw());
                    }
                    None
                }
                mouse::Event::ButtonPressed(mouse::Button::Left) => {
                    // Handle click - select the row
                    if let Some(pos) = cursor_position {
                        if let Some(row_idx) = row_at_position(pos.y, self.graph.rows.len()) {
                            return Some(canvas::Action::publish(row_idx));
                        }
                    }
                    None
                }
                mouse::Event::CursorLeft => {
                    state.hovered_row = None;
                    Some(canvas::Action::request_redraw())
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = theme.extended_palette();
        let lane_colors = get_lane_colors(theme);
        let text_color = palette.background.base.text;
        let secondary_text = palette.background.weak.text;

        let text_start_x = self.graph.graph_width().max(COLUMN_WIDTH * 3.0);

        // Draw row backgrounds first (selection and hover highlights)
        for (row_idx, _row) in self.graph.rows.iter().enumerate() {
            let row_top = row_idx as f32 * ROW_HEIGHT;

            let is_selected = self.selected_index == Some(row_idx);
            let is_hovered = state.hovered_row == Some(row_idx) && !is_selected;

            if is_selected {
                // Selected row - primary color background
                let bg_color = Color {
                    a: 0.3,
                    ..palette.primary.weak.color
                };
                let bg_rect = Path::rectangle(
                    Point::new(0.0, row_top),
                    iced::Size::new(bounds.width, ROW_HEIGHT),
                );
                frame.fill(&bg_rect, bg_color);
            } else if is_hovered {
                // Hovered row - subtle background
                let bg_color = Color {
                    a: 0.15,
                    ..palette.background.weak.text
                };
                let bg_rect = Path::rectangle(
                    Point::new(0.0, row_top),
                    iced::Size::new(bounds.width, ROW_HEIGHT),
                );
                frame.fill(&bg_rect, bg_color);
            }
        }

        // Draw all lines
        for line in &self.graph.lines {
            let source_x = (line.source_col as f32 + 0.5) * COLUMN_WIDTH;
            // Below node center (adjusted for new row height)
            let source_y =
                (line.source_row as f32) * ROW_HEIGHT + NODE_Y_OFFSET + NODE_RADIUS + 4.0;
            let target_x = (line.target_col as f32 + 0.5) * COLUMN_WIDTH;
            // Above node center (adjusted for new row height)
            let target_y =
                (line.target_row as f32) * ROW_HEIGHT + NODE_Y_OFFSET - NODE_RADIUS - 4.0;

            let color = lane_colors[line.target_col % lane_colors.len()];
            let stroke = Stroke::default().with_color(color).with_width(2.0);

            if line.is_vertical() {
                if line.indirect {
                    // Indirect line (elided revisions) - draw dashed with ~ symbol
                    // Draw short segment from source
                    let gap_start = source_y + 4.0;
                    let gap_end = source_y + 16.0;

                    let path1 = Path::line(
                        Point::new(source_x, source_y),
                        Point::new(source_x, gap_start),
                    );
                    frame.stroke(&path1, stroke.clone());

                    // Draw ~ symbol for elided revisions
                    let tilde = Text {
                        content: "~".to_string(),
                        position: Point::new(source_x - 4.0, gap_start),
                        color,
                        size: FONT_SIZE.into(),
                        font: Font::MONOSPACE,
                        ..Text::default()
                    };
                    frame.fill_text(tilde);

                    // Continue line after gap
                    let path2 = Path::line(
                        Point::new(target_x, gap_end),
                        Point::new(target_x, target_y),
                    );
                    frame.stroke(&path2, stroke);
                } else {
                    // Straight vertical line
                    let path = Path::line(
                        Point::new(source_x, source_y),
                        Point::new(target_x, target_y),
                    );
                    frame.stroke(&path, stroke);
                }
            } else {
                // Curved line (merge/fork)
                let c1 = line.source_col;
                let c2 = line.target_col;
                let mid_y = source_y + 12.0;
                let radius = 6.0;
                let dir = if c2 > c1 { 1.0 } else { -1.0 };

                let path = Path::new(|builder| {
                    builder.move_to(Point::new(source_x, source_y));

                    // Short vertical segment
                    builder.line_to(Point::new(source_x, mid_y - radius));

                    // Arc to horizontal
                    builder.quadratic_curve_to(
                        Point::new(source_x, mid_y),
                        Point::new(source_x + radius * dir, mid_y),
                    );

                    // Horizontal line
                    builder.line_to(Point::new(target_x - radius * dir, mid_y));

                    // Arc to vertical
                    builder.quadratic_curve_to(
                        Point::new(target_x, mid_y),
                        Point::new(target_x, mid_y + radius),
                    );

                    // Vertical line to target
                    builder.line_to(Point::new(target_x, target_y));
                });
                frame.stroke(&path, stroke);

                // If indirect, draw ~ symbol at the start of the curve
                if line.indirect {
                    let tilde = Text {
                        content: "~".to_string(),
                        position: Point::new(source_x - 4.0, source_y + 2.0),
                        color,
                        size: FONT_SIZE.into(),
                        font: Font::MONOSPACE,
                        ..Text::default()
                    };
                    frame.fill_text(tilde);
                }
            }
        }

        // Draw nodes and text on top
        for (row_idx, row) in self.graph.rows.iter().enumerate() {
            let row_top = row_idx as f32 * ROW_HEIGHT;
            let node_y = row_top + NODE_Y_OFFSET;
            let node_x = (row.column as f32 + 0.5) * COLUMN_WIDTH;

            let node_color = if row.is_working_copy {
                palette.success.strong.color
            } else {
                lane_colors[row.column % lane_colors.len()]
            };

            let node = Path::circle(Point::new(node_x, node_y), NODE_RADIUS);

            if row.is_immutable {
                frame.fill(&node, node_color);
            } else {
                frame.fill(&node, palette.background.base.color);
                let outline = Stroke::default().with_color(node_color).with_width(2.0);
                frame.stroke(&node, outline);

                if row.is_working_copy {
                    let inner = Path::circle(Point::new(node_x, node_y), NODE_RADIUS * 0.5);
                    frame.fill(&inner, palette.success.strong.color);
                }
            }

            // ===== LINE 1: change_id, bookmarks, author, date =====
            let line1_y = row_top + LINE1_Y_OFFSET - FONT_SIZE * 0.4;
            let mut text_x = text_start_x;

            // Change ID (colored by lane)
            let change_id_text = Text {
                content: row.change_id.clone(),
                position: Point::new(text_x, line1_y),
                color: node_color,
                size: FONT_SIZE.into(),
                font: Font::MONOSPACE,
                ..Text::default()
            };
            frame.fill_text(change_id_text);
            text_x += 75.0;

            // Bookmarks (if any)
            if !row.bookmarks.is_empty() {
                let bookmark_str = row.bookmarks.join(", ");
                let bookmark_text = Text {
                    content: bookmark_str.clone(),
                    position: Point::new(text_x, line1_y),
                    color: palette.primary.strong.color,
                    size: FONT_SIZE.into(),
                    font: Font::default(),
                    ..Text::default()
                };
                frame.fill_text(bookmark_text);
                text_x += (bookmark_str.chars().count() as f32 * 7.0).min(150.0) + 10.0;
            }

            // Author name (after bookmarks)
            let author_text = Text {
                content: row.author.clone(),
                position: Point::new(text_x, line1_y),
                color: secondary_text,
                size: FONT_SIZE_SMALL.into(),
                font: Font::default(),
                ..Text::default()
            };
            frame.fill_text(author_text);

            // Date (right-aligned on line 1)
            let date_x = bounds.width - 120.0;
            let date_text = Text {
                content: row.timestamp.clone(),
                position: Point::new(date_x, line1_y),
                color: secondary_text,
                size: FONT_SIZE_SMALL.into(),
                font: Font::default(),
                ..Text::default()
            };
            frame.fill_text(date_text);

            // ===== LINE 2: description, file stats =====
            let line2_y = row_top + LINE2_Y_OFFSET - FONT_SIZE * 0.4;

            // Description (left side, starting at text_start_x)
            let desc_x = text_start_x;
            let max_desc_width = bounds.width - desc_x - 150.0; // Leave room for stats
            let max_chars = (max_desc_width / 7.0) as usize;

            let (desc, desc_color) = if row.description.is_empty() {
                ("(no description)".to_string(), secondary_text)
            } else {
                let char_count = row.description.chars().count();
                let truncated = if char_count > max_chars && max_chars > 3 {
                    let t: String = row.description.chars().take(max_chars - 3).collect();
                    format!("{}...", t)
                } else {
                    row.description.clone()
                };
                (truncated, text_color)
            };

            let desc_text = Text {
                content: desc,
                position: Point::new(desc_x, line2_y),
                color: desc_color,
                size: FONT_SIZE.into(),
                font: Font::default(),
                ..Text::default()
            };
            frame.fill_text(desc_text);

            // File stats (right-aligned on line 2)
            if let (Some(file_count), Some(removed), Some(added)) =
                (row.file_count, row.lines_removed, row.lines_added)
            {
                let stats_x = bounds.width - 120.0;

                // "N files" text
                let files_str = format!(
                    "{} file{}",
                    file_count,
                    if file_count == 1 { "" } else { "s" }
                );
                let files_text = Text {
                    content: files_str,
                    position: Point::new(stats_x, line2_y),
                    color: secondary_text,
                    size: FONT_SIZE_STATS.into(),
                    font: Font::default(),
                    ..Text::default()
                };
                frame.fill_text(files_text);

                // "-X" removed (red/danger color)
                let removed_text = Text {
                    content: format!("-{}", removed),
                    position: Point::new(stats_x + 50.0, line2_y),
                    color: palette.danger.base.color,
                    size: FONT_SIZE_STATS.into(),
                    font: Font::MONOSPACE,
                    ..Text::default()
                };
                frame.fill_text(removed_text);

                // "+Y" added (green/success color)
                let added_text = Text {
                    content: format!("+{}", added),
                    position: Point::new(stats_x + 80.0, line2_y),
                    color: palette.success.base.color,
                    size: FONT_SIZE_STATS.into(),
                    font: Font::MONOSPACE,
                    ..Text::default()
                };
                frame.fill_text(added_text);
            }
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if let Some(pos) = cursor.position_in(bounds) {
            if row_at_position(pos.y, self.graph.rows.len()).is_some() {
                return mouse::Interaction::Pointer;
            }
        }
        mouse::Interaction::default()
    }
}

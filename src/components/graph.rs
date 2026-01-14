use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Font, Length, Point, Rectangle, Theme};
use std::collections::HashMap;

use crate::jj::CommitInfo;

/// Column spacing for graph lines
const COLUMN_WIDTH: f32 = 18.0;
/// Vertical spacing between commits
const ROW_HEIGHT: f32 = 30.0;
/// Node radius
const NODE_RADIUS: f32 = 6.0;
/// Font size for text
const FONT_SIZE: f32 = 12.0;

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
}

/// Graph and commit info canvas
#[derive(Debug, Clone)]
pub struct GraphColumn {
    rows: Vec<GraphRow>,
    lines: Vec<GraphLine>,
    max_column: usize,
}

impl GraphColumn {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            lines: Vec::new(),
            max_column: 0,
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
            for (i, parent_id) in commit.parent_ids.iter().enumerate() {
                let indirect = false;

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

            self.rows.push(GraphRow {
                column,
                is_working_copy: commit.is_working_copy,
                is_immutable: commit.is_immutable,
                change_id: commit.change_id.chars().take(8).collect(),
                description: commit
                    .description
                    .lines()
                    .next()
                    .unwrap_or("(no description)")
                    .to_string(),
                author: commit.author.clone(),
                timestamp: commit.timestamp.format("%Y-%m-%d %H:%M").to_string(),
                bookmarks: commit.bookmarks.clone(),
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

    pub fn view<M: 'static>(&self, _commits: &[CommitInfo]) -> Element<'_, M> {
        let height = (self.rows.len() as f32) * ROW_HEIGHT;

        Canvas::new(GraphRenderer { graph: self })
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

impl<M> canvas::Program<M> for GraphRenderer<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let palette = theme.extended_palette();
        let lane_colors = get_lane_colors(theme);
        let text_color = palette.background.base.text;
        let secondary_text = palette.background.weak.text;

        let text_start_x = self.graph.graph_width().max(COLUMN_WIDTH * 3.0);

        // Draw all lines
        for line in &self.graph.lines {
            let source_x = (line.source_col as f32 + 0.5) * COLUMN_WIDTH;
            let source_y = (line.source_row as f32) * ROW_HEIGHT + 21.0; // Below node center
            let target_x = (line.target_col as f32 + 0.5) * COLUMN_WIDTH;
            let target_y = (line.target_row as f32) * ROW_HEIGHT + 9.0; // Above node center (or bottom)

            let color = lane_colors[line.target_col % lane_colors.len()];
            let stroke = Stroke::default()
                .with_color(color)
                .with_width(if line.indirect { 1.5 } else { 2.0 });

            if line.is_vertical() {
                // Straight vertical line
                let path = Path::line(
                    Point::new(source_x, source_y),
                    Point::new(target_x, target_y),
                );
                frame.stroke(&path, stroke);
            } else {
                // Curved line (merge/fork)
                let c1 = line.source_col;
                let c2 = line.target_col;
                let mid_y = source_y + 9.0;
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
            }
        }

        // Draw nodes and text on top
        for (row_idx, row) in self.graph.rows.iter().enumerate() {
            let y = (row_idx as f32 + 0.5) * ROW_HEIGHT;
            let node_x = (row.column as f32 + 0.5) * COLUMN_WIDTH;

            let node_color = if row.is_working_copy {
                palette.success.strong.color
            } else {
                lane_colors[row.column % lane_colors.len()]
            };

            let node = Path::circle(Point::new(node_x, y), NODE_RADIUS);

            if row.is_immutable {
                frame.fill(&node, node_color);
            } else {
                frame.fill(&node, palette.background.base.color);
                let outline = Stroke::default().with_color(node_color).with_width(2.0);
                frame.stroke(&node, outline);

                if row.is_working_copy {
                    let inner = Path::circle(Point::new(node_x, y), NODE_RADIUS * 0.5);
                    frame.fill(&inner, palette.success.strong.color);
                }
            }

            // Draw text
            let mut text_x = text_start_x;
            let text_y = y - FONT_SIZE * 0.4;

            let change_id_text = Text {
                content: row.change_id.clone(),
                position: Point::new(text_x, text_y),
                color: node_color,
                size: FONT_SIZE.into(),
                font: Font::MONOSPACE,
                ..Text::default()
            };
            frame.fill_text(change_id_text);
            text_x += 75.0;

            if !row.bookmarks.is_empty() {
                let bookmark_str = row.bookmarks.join(", ");
                let bookmark_text = Text {
                    content: bookmark_str.clone(),
                    position: Point::new(text_x, text_y),
                    color: palette.primary.strong.color,
                    size: FONT_SIZE.into(),
                    font: Font::default(),
                    ..Text::default()
                };
                frame.fill_text(bookmark_text);
                text_x += (bookmark_str.chars().count() as f32 * 7.0).min(150.0) + 10.0;
            }

            let max_desc_width = bounds.width - text_x - 220.0;
            let max_chars = (max_desc_width / 7.0) as usize;
            let char_count = row.description.chars().count();
            let desc = if char_count > max_chars && max_chars > 3 {
                let truncated: String = row.description.chars().take(max_chars - 3).collect();
                format!("{}...", truncated)
            } else {
                row.description.clone()
            };

            let desc_text = Text {
                content: desc,
                position: Point::new(text_x, text_y),
                color: text_color,
                size: FONT_SIZE.into(),
                font: Font::default(),
                ..Text::default()
            };
            frame.fill_text(desc_text);

            let right_x = bounds.width - 200.0;

            let author_text = Text {
                content: row.author.clone(),
                position: Point::new(right_x, text_y),
                color: secondary_text,
                size: (FONT_SIZE - 1.0).into(),
                font: Font::default(),
                ..Text::default()
            };
            frame.fill_text(author_text);

            let date_text = Text {
                content: row.timestamp.clone(),
                position: Point::new(right_x + 100.0, text_y),
                color: secondary_text,
                size: (FONT_SIZE - 1.0).into(),
                font: Font::default(),
                ..Text::default()
            };
            frame.fill_text(date_text);
        }

        vec![frame.into_geometry()]
    }
}

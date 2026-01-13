use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Theme};

use crate::jj::CommitInfo;

/// Column spacing for graph lines
const COLUMN_WIDTH: f32 = 20.0;
/// Vertical spacing between commits
const ROW_HEIGHT: f32 = 30.0;
/// Node radius
const NODE_RADIUS: f32 = 5.0;

/// Graph column state for rendering
#[derive(Debug, Clone)]
pub struct GraphColumn {
    /// Computed graph data for each commit
    graph_data: Vec<GraphRow>,
}

#[derive(Debug, Clone)]
struct GraphRow {
    /// Column position of this commit's node
    node_column: usize,
    /// Lines to draw: (from_column, to_column, color_index)
    lines: Vec<GraphLine>,
    /// Whether this is the working copy commit
    is_working_copy: bool,
}

#[derive(Debug, Clone)]
struct GraphLine {
    from_column: usize,
    to_column: usize,
    color_index: usize,
    line_type: LineType,
}

#[derive(Debug, Clone)]
enum LineType {
    /// Straight vertical line
    Vertical,
    /// Curve from node to parent
    ToParent,
    /// Continuation from previous row
    Continuation,
}

impl GraphColumn {
    pub fn new() -> Self {
        Self {
            graph_data: Vec::new(),
        }
    }

    /// Compute the graph layout from commits
    pub fn compute(&mut self, commits: &[CommitInfo]) {
        self.graph_data.clear();

        if commits.is_empty() {
            return;
        }

        // Build a map from commit_id to row index
        let mut commit_to_row: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for (i, commit) in commits.iter().enumerate() {
            commit_to_row.insert(&commit.commit_id, i);
        }

        // Track active lanes: lane_index -> commit_id being tracked
        let mut active_lanes: Vec<Option<String>> = Vec::new();

        for (_row_idx, commit) in commits.iter().enumerate() {
            let mut row = GraphRow {
                node_column: 0,
                lines: Vec::new(),
                is_working_copy: commit.is_working_copy,
            };

            // Find or create a lane for this commit
            let node_column = if let Some(col) = active_lanes.iter().position(|lane| {
                lane.as_ref()
                    .map(|id| id == &commit.commit_id)
                    .unwrap_or(false)
            }) {
                // Found an existing lane tracking this commit
                col
            } else {
                // Need a new lane - find first empty or create new
                if let Some(col) = active_lanes.iter().position(|lane| lane.is_none()) {
                    col
                } else {
                    active_lanes.push(None);
                    active_lanes.len() - 1
                }
            };

            row.node_column = node_column;

            // Draw continuation lines for all active lanes
            for (col, lane) in active_lanes.iter().enumerate() {
                if lane.is_some() && col != node_column {
                    row.lines.push(GraphLine {
                        from_column: col,
                        to_column: col,
                        color_index: col,
                        line_type: LineType::Continuation,
                    });
                }
            }

            // Clear the current lane (node will be drawn here)
            active_lanes[node_column] = None;

            // Handle parents
            for (parent_idx, parent_id) in commit.parent_ids.iter().enumerate() {
                // Check if this parent appears later in our commit list
                let parent_row = commit_to_row.get(parent_id.as_str());

                if parent_row.is_some() {
                    // Find or create a lane for this parent
                    let parent_column = if parent_idx == 0 {
                        // First parent takes the same column
                        node_column
                    } else {
                        // Additional parents need new lanes
                        if let Some(col) = active_lanes.iter().position(|lane| lane.is_none()) {
                            col
                        } else {
                            active_lanes.push(None);
                            active_lanes.len() - 1
                        }
                    };

                    // Mark this lane as tracking the parent
                    if parent_column < active_lanes.len() {
                        active_lanes[parent_column] = Some(parent_id.clone());
                    }

                    // Draw line from node to parent lane
                    row.lines.push(GraphLine {
                        from_column: node_column,
                        to_column: parent_column,
                        color_index: parent_column,
                        line_type: if node_column == parent_column {
                            LineType::Vertical
                        } else {
                            LineType::ToParent
                        },
                    });
                }
            }

            // Compact lanes by removing trailing empty ones
            while active_lanes.last().map(|l| l.is_none()).unwrap_or(false) {
                active_lanes.pop();
            }

            self.graph_data.push(row);
        }
    }

    /// Get the width needed for the graph
    pub fn width(&self) -> f32 {
        let max_column = self
            .graph_data
            .iter()
            .flat_map(|row| {
                let node_col = row.node_column;
                let line_cols = row.lines.iter().map(|l| l.from_column.max(l.to_column));
                std::iter::once(node_col).chain(line_cols)
            })
            .max()
            .unwrap_or(0);

        (max_column as f32 + 2.0) * COLUMN_WIDTH
    }

    /// Render the graph as a canvas element
    pub fn view<M: 'static>(&self) -> Element<'_, M> {
        let height = (self.graph_data.len() as f32) * ROW_HEIGHT;
        let width = self.width();

        Canvas::new(GraphRenderer { graph: self })
            .width(Length::Fixed(width))
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

/// Get lane colors from the theme palette
fn get_lane_colors(theme: &Theme) -> Vec<Color> {
    let palette = theme.extended_palette();
    vec![
        palette.primary.strong.color,   // Primary (pink/purple in Dracula)
        palette.secondary.strong.color, // Secondary
        palette.success.strong.color,   // Green
        palette.danger.strong.color,    // Red
        palette.primary.weak.color,     // Lighter primary
        palette.secondary.weak.color,   // Lighter secondary
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

        for (row_idx, row) in self.graph.graph_data.iter().enumerate() {
            let y = (row_idx as f32 + 0.5) * ROW_HEIGHT;

            // Draw lines first (so nodes are on top)
            for line in &row.lines {
                let color = lane_colors[line.color_index % lane_colors.len()];
                let stroke = Stroke::default().with_color(color).with_width(2.0);

                let from_x = (line.from_column as f32 + 0.5) * COLUMN_WIDTH;
                let to_x = (line.to_column as f32 + 0.5) * COLUMN_WIDTH;

                match line.line_type {
                    LineType::Vertical => {
                        // Draw line from node down
                        let path =
                            Path::line(Point::new(from_x, y), Point::new(to_x, y + ROW_HEIGHT));
                        frame.stroke(&path, stroke);
                    }
                    LineType::ToParent => {
                        // Draw curved line to parent column
                        let path = Path::new(|builder| {
                            builder.move_to(Point::new(from_x, y));
                            builder.quadratic_curve_to(
                                Point::new(from_x, y + ROW_HEIGHT * 0.5),
                                Point::new(to_x, y + ROW_HEIGHT),
                            );
                        });
                        frame.stroke(&path, stroke);
                    }
                    LineType::Continuation => {
                        // Draw straight vertical continuation
                        let path = Path::line(
                            Point::new(from_x, y - ROW_HEIGHT * 0.5),
                            Point::new(to_x, y + ROW_HEIGHT * 0.5),
                        );
                        frame.stroke(&path, stroke);
                    }
                }
            }

            // Draw the node
            let node_x = (row.node_column as f32 + 0.5) * COLUMN_WIDTH;
            let node_color = if row.is_working_copy {
                palette.success.strong.color // Green for working copy
            } else {
                lane_colors[row.node_column % lane_colors.len()]
            };

            let node = Path::circle(Point::new(node_x, y), NODE_RADIUS);
            frame.fill(&node, node_color);

            // Draw outline for working copy
            if row.is_working_copy {
                let outline = Stroke::default()
                    .with_color(palette.background.base.text)
                    .with_width(2.0);
                frame.stroke(&node, outline);
            }
        }

        vec![frame.into_geometry()]
    }
}

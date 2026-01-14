use iced::Rectangle;

/// A fixed reference line for the measurement of coordinates.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Axis {
    /// The horizontal axis: —
    Horizontal,
    /// The vertical axis: |
    Vertical,
}

impl Axis {
    /// Splits the provided [`Rectangle`] on the current [`Axis`] with the
    /// given `ratio` and `spacing`.
    pub fn split(
        &self,
        rectangle: &Rectangle,
        ratio: f32,
        spacing: f32,
        min_size_a: f32,
        min_size_b: f32,
    ) -> (Rectangle, Rectangle, f32) {
        self.split_with_constraints(
            rectangle, ratio, spacing, min_size_a, None, min_size_b, None,
        )
    }

    /// Splits the provided [`Rectangle`] on the current [`Axis`] with the
    /// given `ratio`, `spacing`, and min/max constraints for each side.
    pub fn split_with_constraints(
        &self,
        rectangle: &Rectangle,
        ratio: f32,
        spacing: f32,
        min_size_a: f32,
        max_size_a: Option<f32>,
        min_size_b: f32,
        max_size_b: Option<f32>,
    ) -> (Rectangle, Rectangle, f32) {
        match self {
            Axis::Horizontal => {
                let total = rectangle.height;
                let mut size_a = (total * ratio - spacing / 2.0).round();

                // Apply min constraint for A
                size_a = size_a.max(min_size_a);
                // Apply max constraint for A
                if let Some(max_a) = max_size_a {
                    size_a = size_a.min(max_a);
                }
                // Ensure B has room for its min
                size_a = size_a.min(total - min_size_b - spacing);

                let mut size_b = total - size_a - spacing;

                // Apply min constraint for B
                size_b = size_b.max(min_size_b);
                // Apply max constraint for B
                if let Some(max_b) = max_size_b {
                    size_b = size_b.min(max_b);
                }
                // Recalculate A if B was clamped to max
                if let Some(max_b) = max_size_b {
                    if size_b <= max_b {
                        size_a = total - size_b - spacing;
                        size_a = size_a.max(min_size_a);
                    }
                }

                let ratio = (size_a + spacing / 2.0) / total;

                (
                    Rectangle {
                        height: size_a,
                        ..*rectangle
                    },
                    Rectangle {
                        y: rectangle.y + size_a + spacing,
                        height: size_b,
                        ..*rectangle
                    },
                    ratio,
                )
            }
            Axis::Vertical => {
                let total = rectangle.width;
                let mut size_a = (total * ratio - spacing / 2.0).round();

                // Apply min constraint for A
                size_a = size_a.max(min_size_a);
                // Apply max constraint for A
                if let Some(max_a) = max_size_a {
                    size_a = size_a.min(max_a);
                }
                // Ensure B has room for its min
                size_a = size_a.min(total - min_size_b - spacing);

                let mut size_b = total - size_a - spacing;

                // Apply min constraint for B
                size_b = size_b.max(min_size_b);
                // Apply max constraint for B
                if let Some(max_b) = max_size_b {
                    size_b = size_b.min(max_b);
                }
                // Recalculate A if B was clamped to max
                if let Some(max_b) = max_size_b {
                    if size_b <= max_b {
                        size_a = total - size_b - spacing;
                        size_a = size_a.max(min_size_a);
                    }
                }

                let ratio = (size_a + spacing / 2.0) / total;

                (
                    Rectangle {
                        width: size_a,
                        ..*rectangle
                    },
                    Rectangle {
                        x: rectangle.x + size_a + spacing,
                        width: size_b,
                        ..*rectangle
                    },
                    ratio,
                )
            }
        }
    }

    /// Calculates the bounds of the split line in a [`Rectangle`] region.
    pub fn split_line_bounds(&self, rectangle: Rectangle, ratio: f32, spacing: f32) -> Rectangle {
        match self {
            Axis::Horizontal => Rectangle {
                x: rectangle.x,
                y: (rectangle.y + rectangle.height * ratio - spacing / 2.0).round(),
                width: rectangle.width,
                height: spacing,
            },
            Axis::Vertical => Rectangle {
                x: (rectangle.x + rectangle.width * ratio - spacing / 2.0).round(),
                y: rectangle.y,
                width: spacing,
                height: rectangle.height,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    enum Case {
        Horizontal {
            overall_height: f32,
            spacing: f32,
            top_height: f32,
            bottom_y: f32,
            bottom_height: f32,
        },
        Vertical {
            overall_width: f32,
            spacing: f32,
            left_width: f32,
            right_x: f32,
            right_width: f32,
        },
    }

    #[test]
    fn split() {
        let cases = vec![
            // Even height, even spacing
            Case::Horizontal {
                overall_height: 10.0,
                spacing: 2.0,
                top_height: 4.0,
                bottom_y: 6.0,
                bottom_height: 4.0,
            },
            // Odd height, even spacing
            Case::Horizontal {
                overall_height: 9.0,
                spacing: 2.0,
                top_height: 4.0,
                bottom_y: 6.0,
                bottom_height: 3.0,
            },
            // Even height, odd spacing
            Case::Horizontal {
                overall_height: 10.0,
                spacing: 1.0,
                top_height: 5.0,
                bottom_y: 6.0,
                bottom_height: 4.0,
            },
            // Odd height, odd spacing
            Case::Horizontal {
                overall_height: 9.0,
                spacing: 1.0,
                top_height: 4.0,
                bottom_y: 5.0,
                bottom_height: 4.0,
            },
            // Even width, even spacing
            Case::Vertical {
                overall_width: 10.0,
                spacing: 2.0,
                left_width: 4.0,
                right_x: 6.0,
                right_width: 4.0,
            },
            // Odd width, even spacing
            Case::Vertical {
                overall_width: 9.0,
                spacing: 2.0,
                left_width: 4.0,
                right_x: 6.0,
                right_width: 3.0,
            },
            // Even width, odd spacing
            Case::Vertical {
                overall_width: 10.0,
                spacing: 1.0,
                left_width: 5.0,
                right_x: 6.0,
                right_width: 4.0,
            },
            // Odd width, odd spacing
            Case::Vertical {
                overall_width: 9.0,
                spacing: 1.0,
                left_width: 4.0,
                right_x: 5.0,
                right_width: 4.0,
            },
        ];
        for case in cases {
            match case {
                Case::Horizontal {
                    overall_height,
                    spacing,
                    top_height,
                    bottom_y,
                    bottom_height,
                } => {
                    let a = Axis::Horizontal;
                    let r = Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: 10.0,
                        height: overall_height,
                    };
                    let (top, bottom, _ratio) = a.split(&r, 0.5, spacing, 0.0, 0.0);
                    assert_eq!(
                        top,
                        Rectangle {
                            height: top_height,
                            ..r
                        }
                    );
                    assert_eq!(
                        bottom,
                        Rectangle {
                            y: bottom_y,
                            height: bottom_height,
                            ..r
                        }
                    );
                }
                Case::Vertical {
                    overall_width,
                    spacing,
                    left_width,
                    right_x,
                    right_width,
                } => {
                    let a = Axis::Vertical;
                    let r = Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: overall_width,
                        height: 10.0,
                    };
                    let (left, right, _ratio) = a.split(&r, 0.5, spacing, 0.0, 0.0);
                    assert_eq!(
                        left,
                        Rectangle {
                            width: left_width,
                            ..r
                        }
                    );
                    assert_eq!(
                        right,
                        Rectangle {
                            x: right_x,
                            width: right_width,
                            ..r
                        }
                    );
                }
            }
        }
    }
}

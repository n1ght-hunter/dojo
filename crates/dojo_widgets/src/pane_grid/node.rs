use super::{Axis, Pane, PaneConstraints, Split};
use iced::{Rectangle, Size};

use std::collections::BTreeMap;

/// A layout node of a [`PaneGrid`].
///
/// [`PaneGrid`]: super::PaneGrid
#[derive(Debug, Clone)]
pub enum Node {
    /// The region of this [`Node`] is split into two.
    Split {
        /// The [`Split`] of this [`Node`].
        id: Split,

        /// The direction of the split.
        axis: Axis,

        /// The ratio of the split in [0.0, 1.0].
        ratio: f32,

        /// The left/top [`Node`] of the split.
        a: Box<Node>,

        /// The right/bottom [`Node`] of the split.
        b: Box<Node>,
    },
    /// The region of this [`Node`] is taken by a [`Pane`].
    Pane(Pane),
}

#[derive(Debug)]
enum Count {
    Split {
        horizontal: usize,
        vertical: usize,
        a: Box<Count>,
        b: Box<Count>,
    },
    Pane,
}

impl Count {
    fn horizontal(&self) -> usize {
        match self {
            Count::Split { horizontal, .. } => *horizontal,
            Count::Pane => 0,
        }
    }

    fn vertical(&self) -> usize {
        match self {
            Count::Split { vertical, .. } => *vertical,
            Count::Pane => 0,
        }
    }
}

impl Node {
    /// Returns an iterator over each [`Split`] in this [`Node`].
    pub fn splits(&self) -> impl Iterator<Item = &Split> {
        let mut unvisited_nodes = vec![self];

        std::iter::from_fn(move || {
            while let Some(node) = unvisited_nodes.pop() {
                if let Node::Split { id, a, b, .. } = node {
                    unvisited_nodes.push(a);
                    unvisited_nodes.push(b);

                    return Some(id);
                }
            }

            None
        })
    }

    fn count(&self) -> Count {
        match self {
            Node::Split { a, b, axis, .. } => {
                let a = a.count();
                let b = b.count();

                let (horizontal, vertical) = match axis {
                    Axis::Horizontal => (
                        1 + a.horizontal() + b.horizontal(),
                        a.vertical().max(b.vertical()),
                    ),
                    Axis::Vertical => (
                        a.horizontal().max(b.horizontal()),
                        1 + a.vertical() + b.vertical(),
                    ),
                };

                Count::Split {
                    horizontal,
                    vertical,
                    a: Box::new(a),
                    b: Box::new(b),
                }
            }
            Node::Pane(_) => Count::Pane,
        }
    }

    /// Returns the rectangular region for each [`Pane`] in the [`Node`] given
    /// the spacing between panes and the total available space.
    pub fn pane_regions(
        &self,
        spacing: f32,
        min_size: f32,
        bounds: Size,
    ) -> BTreeMap<Pane, Rectangle> {
        let mut regions = BTreeMap::new();
        let count = self.count();

        self.compute_regions(
            spacing,
            min_size,
            &Rectangle {
                x: 0.0,
                y: 0.0,
                width: bounds.width,
                height: bounds.height,
            },
            &count,
            &mut regions,
        );

        regions
    }

    /// Returns the rectangular region for each [`Pane`] in the [`Node`] given
    /// the spacing between panes, total available space, and per-pane constraints.
    pub fn pane_regions_with_constraints(
        &self,
        spacing: f32,
        min_size: f32,
        bounds: Size,
        constraints: &BTreeMap<Pane, PaneConstraints>,
    ) -> BTreeMap<Pane, Rectangle> {
        let mut regions = BTreeMap::new();
        let count = self.count();

        self.compute_regions_with_constraints(
            spacing,
            min_size,
            &Rectangle {
                x: 0.0,
                y: 0.0,
                width: bounds.width,
                height: bounds.height,
            },
            &count,
            constraints,
            &mut regions,
        );

        regions
    }

    /// Returns the axis, rectangular region, and ratio for each [`Split`] in
    /// the [`Node`] given the spacing between panes and the total available
    /// space.
    pub fn split_regions(
        &self,
        spacing: f32,
        min_size: f32,
        bounds: Size,
    ) -> BTreeMap<Split, (Axis, Rectangle, f32)> {
        let mut splits = BTreeMap::new();
        let count = self.count();

        self.compute_splits(
            spacing,
            min_size,
            &Rectangle {
                x: 0.0,
                y: 0.0,
                width: bounds.width,
                height: bounds.height,
            },
            &count,
            &mut splits,
        );

        splits
    }

    /// Returns the axis, rectangular region, and ratio for each [`Split`] in
    /// the [`Node`] given the spacing between panes, total available space,
    /// and per-pane constraints.
    pub fn split_regions_with_constraints(
        &self,
        spacing: f32,
        min_size: f32,
        bounds: Size,
        constraints: &BTreeMap<Pane, PaneConstraints>,
    ) -> BTreeMap<Split, (Axis, Rectangle, f32)> {
        let mut splits = BTreeMap::new();
        let count = self.count();

        self.compute_splits_with_constraints(
            spacing,
            min_size,
            &Rectangle {
                x: 0.0,
                y: 0.0,
                width: bounds.width,
                height: bounds.height,
            },
            &count,
            constraints,
            &mut splits,
        );

        splits
    }

    pub(crate) fn find(&mut self, pane: Pane) -> Option<&mut Node> {
        match self {
            Node::Split { a, b, .. } => a.find(pane).or_else(move || b.find(pane)),
            Node::Pane(p) => {
                if *p == pane {
                    Some(self)
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn split(&mut self, id: Split, axis: Axis, new_pane: Pane) {
        *self = Node::Split {
            id,
            axis,
            ratio: 0.5,
            a: Box::new(self.clone()),
            b: Box::new(Node::Pane(new_pane)),
        };
    }

    pub(crate) fn split_inverse(&mut self, id: Split, axis: Axis, pane: Pane) {
        *self = Node::Split {
            id,
            axis,
            ratio: 0.5,
            a: Box::new(Node::Pane(pane)),
            b: Box::new(self.clone()),
        };
    }

    pub(crate) fn update(&mut self, f: &impl Fn(&mut Node)) {
        if let Node::Split { a, b, .. } = self {
            a.update(f);
            b.update(f);
        }

        f(self);
    }

    pub(crate) fn resize(&mut self, split: Split, percentage: f32) -> bool {
        match self {
            Node::Split {
                id, ratio, a, b, ..
            } => {
                if *id == split {
                    *ratio = percentage;

                    true
                } else if a.resize(split, percentage) {
                    true
                } else {
                    b.resize(split, percentage)
                }
            }
            Node::Pane(_) => false,
        }
    }

    pub(crate) fn remove(&mut self, pane: Pane) -> Option<Pane> {
        match self {
            Node::Split { a, b, .. } => {
                if a.pane() == Some(pane) {
                    *self = *b.clone();
                    Some(self.first_pane())
                } else if b.pane() == Some(pane) {
                    *self = *a.clone();
                    Some(self.first_pane())
                } else {
                    a.remove(pane).or_else(|| b.remove(pane))
                }
            }
            Node::Pane(_) => None,
        }
    }

    fn pane(&self) -> Option<Pane> {
        match self {
            Node::Split { .. } => None,
            Node::Pane(pane) => Some(*pane),
        }
    }

    fn first_pane(&self) -> Pane {
        match self {
            Node::Split { a, .. } => a.first_pane(),
            Node::Pane(pane) => *pane,
        }
    }

    fn compute_regions(
        &self,
        spacing: f32,
        min_size: f32,
        current: &Rectangle,
        count: &Count,
        regions: &mut BTreeMap<Pane, Rectangle>,
    ) {
        match (self, count) {
            (
                Node::Split {
                    axis, ratio, a, b, ..
                },
                Count::Split {
                    a: count_a,
                    b: count_b,
                    ..
                },
            ) => {
                let (a_factor, b_factor) = match axis {
                    Axis::Horizontal => (count_a.horizontal(), count_b.horizontal()),
                    Axis::Vertical => (count_a.vertical(), count_b.vertical()),
                };

                let (region_a, region_b, _ratio) = axis.split(
                    current,
                    *ratio,
                    spacing,
                    min_size * (a_factor + 1) as f32 + spacing * a_factor as f32,
                    min_size * (b_factor + 1) as f32 + spacing * b_factor as f32,
                );

                a.compute_regions(spacing, min_size, &region_a, count_a, regions);
                b.compute_regions(spacing, min_size, &region_b, count_b, regions);
            }
            (Node::Pane(pane), Count::Pane) => {
                let _ = regions.insert(*pane, *current);
            }
            _ => {
                unreachable!("Node configuration and count do not match")
            }
        }
    }

    fn compute_splits(
        &self,
        spacing: f32,
        min_size: f32,
        current: &Rectangle,
        count: &Count,
        splits: &mut BTreeMap<Split, (Axis, Rectangle, f32)>,
    ) {
        match (self, count) {
            (
                Node::Split {
                    axis,
                    ratio,
                    a,
                    b,
                    id,
                },
                Count::Split {
                    a: count_a,
                    b: count_b,
                    ..
                },
            ) => {
                let (a_factor, b_factor) = match axis {
                    Axis::Horizontal => (count_a.horizontal(), count_b.horizontal()),
                    Axis::Vertical => (count_a.vertical(), count_b.vertical()),
                };

                let (region_a, region_b, ratio) = axis.split(
                    current,
                    *ratio,
                    spacing,
                    min_size * (a_factor + 1) as f32 + spacing * a_factor as f32,
                    min_size * (b_factor + 1) as f32 + spacing * b_factor as f32,
                );

                let _ = splits.insert(*id, (*axis, *current, ratio));

                a.compute_splits(spacing, min_size, &region_a, count_a, splits);
                b.compute_splits(spacing, min_size, &region_b, count_b, splits);
            }
            (Node::Pane(_), Count::Pane) => {}
            _ => {
                unreachable!("Node configuration and split count do not match")
            }
        }
    }

    /// Computes aggregated constraints for a subtree along a given axis.
    /// Returns (min_size, max_size) where max_size is None if unbounded.
    fn aggregate_constraints(
        &self,
        axis: Axis,
        spacing: f32,
        constraints: &BTreeMap<Pane, PaneConstraints>,
    ) -> (f32, Option<f32>) {
        match self {
            Node::Pane(pane) => {
                let c = constraints.get(pane).copied().unwrap_or_default();
                (c.min_size(axis), c.max_size(axis))
            }
            Node::Split {
                axis: split_axis,
                a,
                b,
                ..
            } => {
                let (min_a, max_a) = a.aggregate_constraints(axis, spacing, constraints);
                let (min_b, max_b) = b.aggregate_constraints(axis, spacing, constraints);

                if *split_axis == axis {
                    // Same axis as split: sizes add up
                    let min_total = min_a + min_b + spacing;
                    let max_total = match (max_a, max_b) {
                        (Some(a), Some(b)) => Some(a + b + spacing),
                        _ => None, // If either is unbounded, total is unbounded
                    };
                    (min_total, max_total)
                } else {
                    // Perpendicular axis: take the more restrictive constraints
                    let min_total = min_a.max(min_b);
                    let max_total = match (max_a, max_b) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    };
                    (min_total, max_total)
                }
            }
        }
    }

    fn compute_regions_with_constraints(
        &self,
        spacing: f32,
        min_size: f32,
        current: &Rectangle,
        count: &Count,
        constraints: &BTreeMap<Pane, PaneConstraints>,
        regions: &mut BTreeMap<Pane, Rectangle>,
    ) {
        match (self, count) {
            (
                Node::Split {
                    axis, ratio, a, b, ..
                },
                Count::Split {
                    a: count_a,
                    b: count_b,
                    ..
                },
            ) => {
                let (a_factor, b_factor) = match axis {
                    Axis::Horizontal => (count_a.horizontal(), count_b.horizontal()),
                    Axis::Vertical => (count_a.vertical(), count_b.vertical()),
                };

                // Get aggregated constraints for each subtree
                let (agg_min_a, agg_max_a) = a.aggregate_constraints(*axis, spacing, constraints);
                let (agg_min_b, agg_max_b) = b.aggregate_constraints(*axis, spacing, constraints);

                // Combine with count-based minimum
                let base_min_a = min_size * (a_factor + 1) as f32 + spacing * a_factor as f32;
                let base_min_b = min_size * (b_factor + 1) as f32 + spacing * b_factor as f32;

                let effective_min_a = agg_min_a.max(base_min_a);
                let effective_min_b = agg_min_b.max(base_min_b);

                let (region_a, region_b, _ratio) = axis.split_with_constraints(
                    current,
                    *ratio,
                    spacing,
                    effective_min_a,
                    agg_max_a,
                    effective_min_b,
                    agg_max_b,
                );

                a.compute_regions_with_constraints(
                    spacing,
                    min_size,
                    &region_a,
                    count_a,
                    constraints,
                    regions,
                );
                b.compute_regions_with_constraints(
                    spacing,
                    min_size,
                    &region_b,
                    count_b,
                    constraints,
                    regions,
                );
            }
            (Node::Pane(pane), Count::Pane) => {
                let _ = regions.insert(*pane, *current);
            }
            _ => {
                unreachable!("Node configuration and count do not match")
            }
        }
    }

    fn compute_splits_with_constraints(
        &self,
        spacing: f32,
        min_size: f32,
        current: &Rectangle,
        count: &Count,
        constraints: &BTreeMap<Pane, PaneConstraints>,
        splits: &mut BTreeMap<Split, (Axis, Rectangle, f32)>,
    ) {
        match (self, count) {
            (
                Node::Split {
                    axis,
                    ratio,
                    a,
                    b,
                    id,
                },
                Count::Split {
                    a: count_a,
                    b: count_b,
                    ..
                },
            ) => {
                let (a_factor, b_factor) = match axis {
                    Axis::Horizontal => (count_a.horizontal(), count_b.horizontal()),
                    Axis::Vertical => (count_a.vertical(), count_b.vertical()),
                };

                // Get aggregated constraints for each subtree
                let (agg_min_a, agg_max_a) = a.aggregate_constraints(*axis, spacing, constraints);
                let (agg_min_b, agg_max_b) = b.aggregate_constraints(*axis, spacing, constraints);

                // Combine with count-based minimum
                let base_min_a = min_size * (a_factor + 1) as f32 + spacing * a_factor as f32;
                let base_min_b = min_size * (b_factor + 1) as f32 + spacing * b_factor as f32;

                let effective_min_a = agg_min_a.max(base_min_a);
                let effective_min_b = agg_min_b.max(base_min_b);

                let (region_a, region_b, ratio) = axis.split_with_constraints(
                    current,
                    *ratio,
                    spacing,
                    effective_min_a,
                    agg_max_a,
                    effective_min_b,
                    agg_max_b,
                );

                let _ = splits.insert(*id, (*axis, *current, ratio));

                a.compute_splits_with_constraints(
                    spacing,
                    min_size,
                    &region_a,
                    count_a,
                    constraints,
                    splits,
                );
                b.compute_splits_with_constraints(
                    spacing,
                    min_size,
                    &region_b,
                    count_b,
                    constraints,
                    splits,
                );
            }
            (Node::Pane(_), Count::Pane) => {}
            _ => {
                unreachable!("Node configuration and split count do not match")
            }
        }
    }
}

impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Node::Split {
                id,
                axis,
                ratio,
                a,
                b,
            } => {
                id.hash(state);
                axis.hash(state);
                ((ratio * 100_000.0) as u32).hash(state);
                a.hash(state);
                b.hash(state);
            }
            Node::Pane(pane) => {
                pane.hash(state);
            }
        }
    }
}

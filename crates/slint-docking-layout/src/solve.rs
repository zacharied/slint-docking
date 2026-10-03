//! Solving: from a tree and a size to rectangles.

use crate::geometry::{Rect, Size};
use crate::tree::{Axis, GroupId, Layout, Node, SplitId};

/// The sizes of the parts that the docking system draws itself, in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// The height of the strip of tabs above a group.
    pub tab_bar_height: f32,
    /// The width of a tab when there is room. Tabs shrink equally when there is not.
    pub tab_width: f32,
    /// The thickness of the bar between two children of a split.
    pub splitter: f32,
    /// The least width a group can be given by dragging a splitter.
    pub min_panel_width: f32,
    /// The least height a group's panel can be given by dragging a splitter.
    pub min_panel_height: f32,
    /// How close to the edge of a host a drop counts as a drop on the edge.
    pub edge_band: f32,
    /// The share of a group, from each side, where a drop splits the group.
    pub zone_fraction: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Metrics {
            tab_bar_height: 28.0,
            tab_width: 140.0,
            splitter: 4.0,
            min_panel_width: 80.0,
            min_panel_height: 60.0,
            edge_band: 12.0,
            zone_fraction: 0.25,
        }
    }
}

/// A group with its rectangles.
#[derive(Clone, Debug, PartialEq)]
pub struct SolvedGroup {
    /// The group's ID.
    pub id: GroupId,
    /// The whole area of the group.
    pub rect: Rect,
    /// The strip of tabs, at the top of the area.
    pub tab_bar: Rect,
    /// The area of the panels, below the tabs.
    pub content: Rect,
    /// The width of each tab.
    pub tab_width: f32,
    /// The keys of the panels, in tab order.
    pub panels: Vec<String>,
    /// The index of the panel that is shown.
    pub active: usize,
}

impl SolvedGroup {
    /// The rectangle of the tab at `index`.
    pub fn tab_rect(&self, index: usize) -> Rect {
        Rect::new(
            self.tab_bar.x + index as f32 * self.tab_width,
            self.tab_bar.y,
            self.tab_width,
            self.tab_bar.height,
        )
    }
}

/// The bar between two children of a split.
#[derive(Clone, Debug, PartialEq)]
pub struct SolvedSplitter {
    /// The split.
    pub split: SplitId,
    /// Which bar of the split: bar `n` lies between child `n` and child `n + 1`.
    pub handle: usize,
    /// The axis of the split, which is the direction the bar is dragged in.
    pub axis: Axis,
    /// The bar's rectangle.
    pub rect: Rect,
}

/// A split with its rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct SolvedSplit {
    /// The split's ID.
    pub id: SplitId,
    /// The axis of the split.
    pub axis: Axis,
    /// The whole area of the split.
    pub rect: Rect,
    /// The start and extent of each child along the axis.
    pub spans: Vec<(f32, f32)>,
}

/// The rectangles of one host at one size.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solved {
    /// The size that was solved for.
    pub size: Size,
    /// The groups, depth first.
    pub groups: Vec<SolvedGroup>,
    /// The bars between the children of splits, depth first.
    pub splitters: Vec<SolvedSplitter>,
    /// The splits, depth first.
    pub splits: Vec<SolvedSplit>,
}

impl Solved {
    /// The group with the given ID.
    pub fn group(&self, id: GroupId) -> Option<&SolvedGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// The group that holds a panel, and the panel's index in it.
    pub fn group_of(&self, panel: &str) -> Option<(&SolvedGroup, usize)> {
        self.groups.iter().find_map(|g| g.panels.iter().position(|p| p == panel).map(|index| (g, index)))
    }

    /// The area a panel is given, and whether the panel is shown. A panel that is a tab
    /// behind another is given the same area as the panel in front.
    pub fn panel(&self, panel: &str) -> Option<(Rect, bool)> {
        self.group_of(panel).map(|(group, index)| (group.content, index == group.active))
    }

    /// The split with the given ID.
    pub fn split(&self, id: SplitId) -> Option<&SolvedSplit> {
        self.splits.iter().find(|s| s.id == id)
    }
}

impl Layout {
    /// Gives every group, split and splitter of a host its rectangle.
    ///
    /// Edges are placed on whole pixels. The weights of a split are followed exactly, so a
    /// host that is too small gives areas below the least sizes. The least sizes limit only
    /// how far a splitter can be dragged.
    pub fn solve(&self, host: usize, size: Size, metrics: &Metrics) -> Solved {
        let mut solved = Solved { size, ..Solved::default() };
        if let Some(root) = self.root(host) {
            let area = Rect::new(0.0, 0.0, size.width.max(0.0), size.height.max(0.0));
            solve(root, area, metrics, &mut solved);
        }
        solved
    }

    /// The weights that put a splitter at `position`, for [`Op::SetWeights`](crate::Op).
    ///
    /// `position` is where the middle of the bar should be, along the axis of the split,
    /// in the host. Only the two children beside the bar change size, and neither goes
    /// below its least size. Returns `None` if the split or the bar does not exist, or if
    /// there is no room to move.
    pub fn weights_for_drag(
        &self,
        solved: &Solved,
        split: SplitId,
        handle: usize,
        position: f32,
        metrics: &Metrics,
    ) -> Option<Vec<f32>> {
        let node = self.split(split)?;
        let spans = &solved.split(split)?.spans;
        let (before, after) = (spans.get(handle)?, spans.get(handle + 1)?);
        let axis = node.axis();
        let least_before = least(node.children().get(handle)?, axis, metrics);
        let least_after = least(node.children().get(handle + 1)?, axis, metrics);

        let both = before.1 + after.1;
        let most_before = both - least_after;
        if most_before < least_before {
            return None;
        }
        let wanted = position - metrics.splitter / 2.0 - before.0;
        let new_before = wanted.clamp(least_before, most_before).round();

        let mut extents: Vec<f32> = spans.iter().map(|s| s.1).collect();
        extents[handle] = new_before;
        extents[handle + 1] = both - new_before;
        let total: f32 = extents.iter().sum();
        if total <= 0.0 || extents.iter().any(|e| *e <= 0.0) {
            return None;
        }
        Some(extents.into_iter().map(|e| e / total).collect())
    }
}

fn solve(node: &Node, area: Rect, metrics: &Metrics, solved: &mut Solved) {
    match node {
        Node::Group(group) => {
            let bar_height = metrics.tab_bar_height.min(area.height);
            let tab_bar = Rect::new(area.x, area.y, area.width, bar_height);
            let content = Rect::new(area.x, area.y + bar_height, area.width, area.height - bar_height);
            let tabs = group.panels().len().max(1) as f32;
            solved.groups.push(SolvedGroup {
                id: group.id(),
                rect: area,
                tab_bar,
                content,
                tab_width: metrics.tab_width.min(area.width / tabs),
                panels: group.panels().to_vec(),
                active: group.active_index(),
            });
        }
        Node::Split(split) => {
            let axis = split.axis();
            let (start, extent) = area.span(axis);
            let bars = (split.children().len() - 1) as f32;
            let splitter = metrics.splitter.min(extent / bars.max(1.0));
            let room = (extent - splitter * bars).max(0.0);

            // Place the edges from running totals, so that rounding never adds up.
            let mut spans = Vec::with_capacity(split.children().len());
            let mut share = 0.0;
            for (i, weight) in split.weights().iter().enumerate() {
                let bars_before = splitter * i as f32;
                let from = (start + share * room + bars_before).round();
                share += weight;
                let to = if i + 1 == split.weights().len() {
                    start + extent
                } else {
                    (start + share * room + bars_before).round()
                };
                spans.push((from, (to - from).max(0.0)));
            }

            let at = solved.splits.len();
            solved.splits.push(SolvedSplit { id: split.id(), axis, rect: area, spans: spans.clone() });
            for (i, child) in split.children().iter().enumerate() {
                let (from, extent) = spans[i];
                solve(child, area.with_span(axis, from, extent), metrics, solved);
                if let Some((next, _)) = spans.get(i + 1) {
                    let end = from + extent;
                    solved.splitters.push(SolvedSplitter {
                        split: split.id(),
                        handle: i,
                        axis,
                        rect: area.with_span(axis, end, next - end),
                    });
                }
            }
            debug_assert_eq!(solved.splits[at].id, split.id());
        }
    }
}

/// The least extent of a node along an axis.
fn least(node: &Node, axis: Axis, metrics: &Metrics) -> f32 {
    match node {
        Node::Group(_) => match axis {
            Axis::Horizontal => metrics.min_panel_width,
            Axis::Vertical => metrics.tab_bar_height + metrics.min_panel_height,
        },
        Node::Split(split) => {
            let children = split.children().iter().map(|c| least(c, axis, metrics));
            if split.axis() == axis {
                children.sum::<f32>() + metrics.splitter * (split.children().len() - 1) as f32
            } else {
                children.fold(0.0, f32::max)
            }
        }
    }
}

//! Remembered places: where a closed panel was, and how it returns there.

use crate::ops::{OpError, Zone};
use crate::tree::{Axis, Group, GroupId, Layout, Node, Split};

/// Where a panel was when it was closed, so that it can return there.
#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    /// It shared a group with other panels.
    Tabs {
        /// The other panels of the group, in tab order.
        panels: Vec<String>,
        /// Its index among the tabs.
        index: usize,
    },
    /// It was alone in its group, a part of a split.
    Beside {
        /// The panels of the area beside it: the part of the split before it, where there
        /// is one, and otherwise the part after it.
        panels: Vec<String>,
        /// The side of that area it was on. Never the centre.
        zone: Zone,
        /// Its weight in the split that held it, between 0 and 1.
        share: f32,
    },
}

impl Place {
    /// The panels the place names.
    fn panels(&self) -> &[String] {
        match self {
            Place::Tabs { panels, .. } | Place::Beside { panels, .. } => panels,
        }
    }
}

impl Layout {
    /// The place that closing `panel` would remember. `None` if the panel is not placed,
    /// or is alone in its host.
    pub fn place_of(&self, panel: &str) -> Option<Place> {
        let at = self.find_panel(panel)?;
        let group = self.group(at.group)?;
        if group.panels.len() > 1 {
            let panels = group.panels.iter().filter(|p| *p != panel).cloned().collect();
            return Some(Place::Tabs { panels, index: at.index });
        }
        let (split, i) = parent_of(self.root(at.host)?, at.group)?;
        let (area, leading) = if i > 0 { (i - 1, false) } else { (i + 1, true) };
        let mut panels = Vec::new();
        split.children[area].for_each_group(&mut |g| panels.extend(g.panels.iter().cloned()));
        let zone = match (split.axis, leading) {
            (Axis::Horizontal, false) => Zone::Right,
            (Axis::Horizontal, true) => Zone::Left,
            (Axis::Vertical, false) => Zone::Bottom,
            (Axis::Vertical, true) => Zone::Top,
        };
        Some(Place::Beside { panels, zone, share: split.weights[i] })
    }

    /// Whether any panel that `place` names is placed, so that a panel can return to it.
    pub fn can_return(&self, place: &Place) -> bool {
        place.panels().iter().any(|p| self.contains(p))
    }

    /// Puts `panel` at `place`. See [`Op::Return`](crate::Op::Return).
    pub(crate) fn return_to(&mut self, panel: String, place: Place) -> Result<(), OpError> {
        // A panel that is placed moves. Taking it out first leaves the others where they
        // are, and the layout is thrown away if the place is refused.
        if self.contains(&panel) {
            self.retain_panels(|p| p != panel);
        }
        let named: Vec<&str> = place.panels().iter().map(String::as_str).filter(|p| *p != panel).collect();
        let Some(first) = named.iter().find_map(|p| self.find_panel(p)) else {
            return Err(OpError::NothingToReturnTo(panel));
        };
        let to = match &place {
            Place::Beside { zone, share, .. } => zone.axis().map(|axis| Beside {
                axis,
                leading: zone.is_leading(),
                share: if share.is_finite() { share.clamp(f32::EPSILON, 1.0 - f32::EPSILON) } else { 0.5 },
            }),
            Place::Tabs { .. } => None,
        };
        let Some(to) = to else {
            // Beside the centre is not a place that `place_of` gives. The panel joins the
            // group as its last tab.
            let index = match place {
                Place::Tabs { index, .. } => index,
                Place::Beside { .. } => usize::MAX,
            };
            if let Some(group) = self.group_mut(first.group) {
                let index = index.min(group.panels.len());
                group.panels.insert(index, panel);
                group.active = index;
            }
            return Ok(());
        };
        let host = first.host;
        let held: Vec<&str> =
            named.into_iter().filter(|p| self.find_panel(p).is_some_and(|at| at.host == host)).collect();
        let new = self.new_group(panel);
        if let Some(root) = self.hosts[host].take() {
            let root = self.put_beside(root, &held, to, new);
            self.hosts[host] = Some(root);
        }
        Ok(())
    }

    /// Puts `new` beside the smallest area under `node` that holds every panel of `held`,
    /// all of which `node` holds.
    fn put_beside(&mut self, node: Node, held: &[&str], to: Beside, new: Node) -> Node {
        let Node::Split(mut split) = node else { return self.wrap(node, to, new) };
        let holding: Vec<usize> =
            (0..split.children.len()).filter(|i| holds(&split.children[*i], held)).collect();
        let (Some(&first), Some(&last)) = (holding.first(), holding.last()) else {
            return self.wrap(Node::Split(split), to, new);
        };
        // The area is under one part. That part is the area itself only where it is a
        // group, or a split whose first and last parts both hold some of the panels.
        if first == last && !(split.axis == to.axis && is_whole(&split.children[first], held)) {
            let empty = Node::Group(Group { id: GroupId(0), panels: Vec::new(), active: 0 });
            let part = std::mem::replace(&mut split.children[first], empty);
            split.children[first] = self.put_beside(part, held, to, new);
            return Node::Split(split);
        }
        // The area is the parts from `first` to `last`. Along the axis, the panel joins
        // the split, and the other parts keep their shares among themselves.
        if split.axis == to.axis {
            split.weights.iter_mut().for_each(|w| *w *= 1.0 - to.share);
            let at = if to.leading { first } else { last + 1 };
            split.children.insert(at, new);
            split.weights.insert(at, to.share);
            return Node::Split(split);
        }
        if first == 0 && last == split.children.len() - 1 {
            return self.wrap(Node::Split(split), to, new);
        }
        // Across the axis, a run of parts is split along the axis between it and the panel.
        let run: Vec<Node> = split.children.drain(first..=last).collect();
        let weights: Vec<f32> = split.weights.drain(first..=last).collect();
        let weight = weights.iter().sum();
        let run = self.new_split(split.axis, run, weights);
        let wrapped = self.wrap(run, to, new);
        split.children.insert(first, wrapped);
        split.weights.insert(first, weight);
        Node::Split(split)
    }

    /// A split along the axis between `area` and `new`, which takes its share.
    fn wrap(&mut self, area: Node, to: Beside, new: Node) -> Node {
        let rest = 1.0 - to.share;
        if to.leading {
            self.new_split(to.axis, vec![new, area], vec![to.share, rest])
        } else {
            self.new_split(to.axis, vec![area, new], vec![rest, to.share])
        }
    }
}

/// How a panel returns beside an area: along which axis, on which side, with what share.
#[derive(Clone, Copy)]
struct Beside {
    axis: Axis,
    leading: bool,
    share: f32,
}

/// Whether any panel of `held` is under `node`.
fn holds(node: &Node, held: &[&str]) -> bool {
    let mut found = false;
    node.for_each_group(&mut |g| found |= g.panels.iter().any(|p| held.contains(&p.as_str())));
    found
}

/// Whether the smallest area under `node` that holds every panel of `held` is `node`
/// itself, given that `node` holds them all.
fn is_whole(node: &Node, held: &[&str]) -> bool {
    match node {
        Node::Group(_) => true,
        Node::Split(split) => {
            split.children.first().is_some_and(|c| holds(c, held))
                && split.children.last().is_some_and(|c| holds(c, held))
        }
    }
}

/// The split that holds the group as a part, and the group's index among its parts.
fn parent_of(node: &Node, group: GroupId) -> Option<(&Split, usize)> {
    let Node::Split(split) = node else { return None };
    match split.children.iter().position(|c| matches!(c, Node::Group(g) if g.id == group)) {
        Some(i) => Some((split, i)),
        None => split.children.iter().find_map(|c| parent_of(c, group)),
    }
}

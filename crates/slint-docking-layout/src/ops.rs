//! Operations: the only way a layout changes.

use std::fmt;

use crate::tree::{Axis, Group, GroupId, Layout, Node, SplitId};

/// Which part of an area a panel is dropped on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Zone {
    /// The middle. The panel joins what is there.
    Center,
    /// The left part. The panel takes the left of the area.
    Left,
    /// The right part.
    Right,
    /// The top part.
    Top,
    /// The bottom part.
    Bottom,
}

impl Zone {
    /// The axis along which a drop on this zone splits. `None` for the centre.
    pub fn axis(self) -> Option<Axis> {
        match self {
            Zone::Center => None,
            Zone::Left | Zone::Right => Some(Axis::Horizontal),
            Zone::Top | Zone::Bottom => Some(Axis::Vertical),
        }
    }

    /// Whether the dropped panel comes before what was there.
    fn is_leading(self) -> bool {
        matches!(self, Zone::Left | Zone::Top)
    }
}

/// Where a panel goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// A zone of a group. The centre adds the panel as the last tab. A side splits the
    /// group's area and gives the panel half.
    Group {
        /// The group.
        group: GroupId,
        /// The zone of the group.
        zone: Zone,
    },
    /// A place among the tabs of a group. Index 0 is before the first tab. An index past
    /// the end is after the last tab.
    Tab {
        /// The group.
        group: GroupId,
        /// The place among the tabs, counted before the panel is moved.
        index: usize,
    },
    /// An edge of a whole host. The panel takes that side of the host, beside everything
    /// else. The centre is allowed only for a host that is empty.
    Edge {
        /// The host.
        host: usize,
        /// The side of the host.
        zone: Zone,
    },
}

/// A change to a layout.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// Puts a panel at a target. If the panel is already placed it moves, otherwise it opens.
    /// The panel becomes the shown panel of its group.
    Place {
        /// The key of the panel.
        panel: String,
        /// Where it goes.
        target: Target,
    },
    /// Makes a panel the shown panel of its group.
    Activate {
        /// The key of the panel.
        panel: String,
    },
    /// Removes a panel from the layout.
    Close {
        /// The key of the panel.
        panel: String,
    },
    /// Sets the share each child of a split takes. The weights need not sum to 1.
    SetWeights {
        /// The split.
        split: SplitId,
        /// One weight per child, each positive.
        weights: Vec<f32>,
    },
}

/// Why an operation was refused. The layout is unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpError {
    /// The panel is not placed.
    UnknownPanel(String),
    /// No group has this ID.
    UnknownGroup(GroupId),
    /// No split has this ID.
    UnknownSplit(SplitId),
    /// No host has this index.
    UnknownHost(usize),
    /// The centre of a host was given as a target, and the host is not empty.
    HostNotEmpty(usize),
    /// The weights are not one positive number per child.
    BadWeights,
}

impl fmt::Display for OpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpError::UnknownPanel(panel) => write!(f, "the panel {panel} is not placed"),
            OpError::UnknownGroup(id) => write!(f, "there is no group {}", id.0),
            OpError::UnknownSplit(id) => write!(f, "there is no split {}", id.0),
            OpError::UnknownHost(host) => write!(f, "there is no host {host}"),
            OpError::HostNotEmpty(host) => {
                write!(f, "host {host} is not empty, so its centre is not a target")
            }
            OpError::BadWeights => write!(f, "the weights are not one positive number per child"),
        }
    }
}

impl std::error::Error for OpError {}

impl Layout {
    /// Applies an operation. Returns whether the layout changed.
    ///
    /// An operation that is refused leaves the layout exactly as it was.
    pub fn apply(&mut self, op: Op) -> Result<bool, OpError> {
        let mut next = self.clone();
        next.apply_unchecked(op)?;
        next.normalise();
        debug_assert_eq!(next.check(), Ok(()));
        if next == *self {
            return Ok(false);
        }
        *self = next;
        Ok(true)
    }

    fn apply_unchecked(&mut self, op: Op) -> Result<(), OpError> {
        match op {
            Op::Place { panel, target } => self.place(panel, target),
            Op::Activate { panel } => {
                let at = self.find_panel(&panel).ok_or(OpError::UnknownPanel(panel))?;
                if let Some(group) = self.group_mut(at.group) {
                    group.active = at.index;
                }
                Ok(())
            }
            Op::Close { panel } => {
                if !self.retain_panels(|p| p != panel) {
                    return Err(OpError::UnknownPanel(panel));
                }
                Ok(())
            }
            Op::SetWeights { split, weights } => {
                let split = self.split_mut(split).ok_or(OpError::UnknownSplit(split))?;
                if weights.len() != split.children.len()
                    || weights.iter().any(|w| !w.is_finite() || *w <= 0.0)
                {
                    return Err(OpError::BadWeights);
                }
                let total: f32 = weights.iter().sum();
                split.weights = weights.into_iter().map(|w| w / total).collect();
                Ok(())
            }
        }
    }

    fn place(&mut self, panel: String, target: Target) -> Result<(), OpError> {
        match target {
            Target::Group { group, .. } | Target::Tab { group, .. } => {
                self.group(group).ok_or(OpError::UnknownGroup(group))?;
            }
            Target::Edge { host, zone } => {
                let root = self.hosts.get(host).ok_or(OpError::UnknownHost(host))?;
                if zone == Zone::Center && root.is_some() {
                    return Err(OpError::HostNotEmpty(host));
                }
            }
        }

        let from = self.find_panel(&panel);

        // Within its own group a panel only changes its place among the tabs, unless it
        // is pulled out to one side.
        if let Some(from) = from {
            let own = self.group_mut(from.group).filter(|g| match target {
                Target::Tab { group, .. } => group == g.id,
                Target::Group { group, zone } => {
                    group == g.id && (zone == Zone::Center || g.panels.len() == 1)
                }
                Target::Edge { .. } => false,
            });
            if let Some(group) = own {
                if let Target::Tab { index, .. } = target {
                    let panel = group.panels.remove(from.index);
                    let index = if index > from.index { index - 1 } else { index };
                    let index = index.min(group.panels.len());
                    group.panels.insert(index, panel);
                    group.active = index;
                } else {
                    group.active = from.index;
                }
                return Ok(());
            }
        }

        // Take the panel out. The tree is not normalised yet, so the target group is
        // still there even if this leaves it, or another group, empty.
        if let Some(from) = from
            && let Some(group) = self.group_mut(from.group)
        {
            remove_tab(group, from.index);
        }

        match target {
            Target::Group { group, zone: Zone::Center } => {
                if let Some(group) = self.group_mut(group) {
                    group.panels.push(panel);
                    group.active = group.panels.len() - 1;
                }
            }
            Target::Tab { group, index } => {
                if let Some(group) = self.group_mut(group) {
                    let index = index.min(group.panels.len());
                    group.panels.insert(index, panel);
                    group.active = index;
                }
            }
            Target::Group { group, zone } => {
                let new = self.new_group(panel);
                let mut new = Some(new);
                let mut hosts = std::mem::take(&mut self.hosts);
                for root in hosts.iter_mut().flatten() {
                    self.insert_beside(root, group, zone, &mut new);
                }
                self.hosts = hosts;
            }
            Target::Edge { host, zone } => {
                let new = self.new_group(panel);
                let Some(axis) = zone.axis() else {
                    self.hosts[host] = Some(new);
                    return Ok(());
                };
                let root = match self.hosts[host].take() {
                    Some(root) => root,
                    None => {
                        self.hosts[host] = Some(new);
                        return Ok(());
                    }
                };
                self.hosts[host] = Some(match root {
                    // A root along the same axis takes one more child, which gets an equal
                    // share.
                    Node::Split(mut split) if split.axis == axis => {
                        let share = 1.0 / (split.children.len() + 1) as f32;
                        split.weights.iter_mut().for_each(|w| *w *= 1.0 - share);
                        if zone.is_leading() {
                            split.children.insert(0, new);
                            split.weights.insert(0, share);
                        } else {
                            split.children.push(new);
                            split.weights.push(share);
                        }
                        Node::Split(split)
                    }
                    // Any other root is put beside the panel, which gets a quarter.
                    root => {
                        if zone.is_leading() {
                            self.new_split(axis, vec![new, root], vec![0.25, 0.75])
                        } else {
                            self.new_split(axis, vec![root, new], vec![0.75, 0.25])
                        }
                    }
                });
            }
        }
        Ok(())
    }

    /// Puts `new` on the `zone` side of the group `target`, somewhere under `node`.
    /// Takes `new` when it is used.
    fn insert_beside(&mut self, node: &mut Node, target: GroupId, zone: Zone, new: &mut Option<Node>) {
        let Some(axis) = zone.axis() else { return };
        match node {
            Node::Group(group) if group.id == target => {
                let Some(new) = new.take() else { return };
                // Something must stand in the node's place while it is moved into the split.
                let empty = Node::Group(Group { id: GroupId(0), panels: Vec::new(), active: 0 });
                let old = std::mem::replace(node, empty);
                let children = if zone.is_leading() { vec![new, old] } else { vec![old, new] };
                *node = self.new_split(axis, children, vec![0.5, 0.5]);
            }
            Node::Group(_) => {}
            Node::Split(split) => {
                let at = split.children.iter().position(|c| matches!(c, Node::Group(g) if g.id == target));
                match at {
                    // The group's parent already runs along the axis, so the panel becomes
                    // a sibling and takes half of the group's share.
                    Some(at) if split.axis == axis => {
                        let Some(new) = new.take() else { return };
                        split.weights[at] /= 2.0;
                        let weight = split.weights[at];
                        let at = if zone.is_leading() { at } else { at + 1 };
                        split.children.insert(at, new);
                        split.weights.insert(at, weight);
                    }
                    _ => {
                        for child in &mut split.children {
                            self.insert_beside(child, target, zone, new);
                        }
                    }
                }
            }
        }
    }
}

/// Removes a tab, and shows the tab that takes its place.
fn remove_tab(group: &mut Group, index: usize) {
    group.panels.remove(index);
    if group.active > index || group.active >= group.panels.len() {
        group.active = group.active.saturating_sub(1);
    }
}

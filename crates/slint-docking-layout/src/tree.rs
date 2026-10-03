//! The layout tree: hosts, splits and tab groups.

use std::collections::HashSet;
use std::fmt;

/// The direction in which a split lays out its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Axis {
    /// Children sit side by side, left to right.
    Horizontal,
    /// Children are stacked, top to bottom.
    Vertical,
}

/// Identifies a tab group for as long as the group exists. Not saved with a layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroupId(pub u32);

/// Identifies a split for as long as the split exists. Not saved with a layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SplitId(pub u32);

/// A set of panels that share one area. One of them is shown, the rest are tabs.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Group {
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) id: GroupId,
    pub(crate) panels: Vec<String>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) active: usize,
}

impl Group {
    /// The group's ID.
    pub fn id(&self) -> GroupId {
        self.id
    }

    /// The keys of the panels, in tab order.
    pub fn panels(&self) -> &[String] {
        &self.panels
    }

    /// The index of the panel that is shown.
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// The key of the panel that is shown.
    pub fn active_panel(&self) -> Option<&str> {
        self.panels.get(self.active).map(String::as_str)
    }
}

/// An area divided among two or more children along one axis.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Split {
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) id: SplitId,
    pub(crate) axis: Axis,
    pub(crate) children: Vec<Node>,
    pub(crate) weights: Vec<f32>,
}

impl Split {
    /// The split's ID.
    pub fn id(&self) -> SplitId {
        self.id
    }

    /// The direction in which the children are laid out.
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// The children, in order.
    pub fn children(&self) -> &[Node] {
        &self.children
    }

    /// The share of the split each child takes. One per child, and they sum to 1.
    pub fn weights(&self) -> &[f32] {
        &self.weights
    }
}

/// A node of the layout tree.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Node {
    /// A tab group.
    Group(Group),
    /// A split.
    Split(Split),
}

impl Node {
    /// A tab group holding `panels`, the first of which is shown.
    pub fn tabs<I, S>(panels: I) -> Node
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Node::Group(Group { id: GroupId(0), panels: panels.into_iter().map(Into::into).collect(), active: 0 })
    }

    /// A split whose children sit side by side. Each child comes with its weight, which
    /// need not sum to anything in particular.
    pub fn row<I: IntoIterator<Item = (f32, Node)>>(children: I) -> Node {
        Node::split(Axis::Horizontal, children)
    }

    /// A split whose children are stacked. Each child comes with its weight.
    pub fn column<I: IntoIterator<Item = (f32, Node)>>(children: I) -> Node {
        Node::split(Axis::Vertical, children)
    }

    /// A split along `axis`. Each child comes with its weight.
    pub fn split<I: IntoIterator<Item = (f32, Node)>>(axis: Axis, children: I) -> Node {
        let (weights, children) = children.into_iter().unzip();
        Node::Split(Split { id: SplitId(0), axis, children, weights })
    }

    /// Calls `f` for every group under this node, depth first.
    pub(crate) fn for_each_group<'a>(&'a self, f: &mut impl FnMut(&'a Group)) {
        match self {
            Node::Group(group) => f(group),
            Node::Split(split) => split.children.iter().for_each(|c| c.for_each_group(f)),
        }
    }

    pub(crate) fn for_each_group_mut(&mut self, f: &mut impl FnMut(&mut Group)) {
        match self {
            Node::Group(group) => f(group),
            Node::Split(split) => split.children.iter_mut().for_each(|c| c.for_each_group_mut(f)),
        }
    }

    pub(crate) fn for_each_split<'a>(&'a self, f: &mut impl FnMut(&'a Split)) {
        if let Node::Split(split) = self {
            f(split);
            split.children.iter().for_each(|c| c.for_each_split(f));
        }
    }

    pub(crate) fn for_each_split_mut(&mut self, f: &mut impl FnMut(&mut Split)) {
        if let Node::Split(split) = self {
            f(split);
            split.children.iter_mut().for_each(|c| c.for_each_split_mut(f));
        }
    }
}

impl fmt::Display for Node {
    /// Writes the tree on one line, for logs and tests: `row(25: tabs(a*, b), 75: tabs(c*))`.
    /// The shown panel of a group carries a star, and weights are whole percentages.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Group(group) => {
                write!(f, "tabs(")?;
                for (i, panel) in group.panels.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{panel}")?;
                    if i == group.active {
                        write!(f, "*")?;
                    }
                }
                write!(f, ")")
            }
            Node::Split(split) => {
                match split.axis {
                    Axis::Horizontal => write!(f, "row(")?,
                    Axis::Vertical => write!(f, "col(")?,
                }
                for (i, (child, weight)) in split.children.iter().zip(&split.weights).enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {child}", (weight * 100.0).round())?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Where a panel is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    /// The host that holds the panel.
    pub host: usize,
    /// The group that holds the panel.
    pub group: GroupId,
    /// The panel's index among the group's tabs.
    pub index: usize,
}

/// Where every panel is. One tree per host.
///
/// A host is one area that panels are docked in. A window holds one host. A layout always
/// has at least one host, and a host may be empty.
///
/// The layout keeps itself in a normal form:
///
/// - every group holds at least one panel, and its shown panel exists
/// - every split has at least two children
/// - no split has a child that is a split along the same axis
/// - the weights of a split are positive and sum to 1
/// - a panel appears at most once
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(from = "SavedLayout", into = "SavedLayout"))]
pub struct Layout {
    pub(crate) hosts: Vec<Option<Node>>,
    pub(crate) next_id: u32,
}

#[cfg(feature = "serde")]
#[derive(serde::Serialize, serde::Deserialize)]
struct SavedLayout {
    hosts: Vec<Option<Node>>,
}

#[cfg(feature = "serde")]
impl From<SavedLayout> for Layout {
    fn from(saved: SavedLayout) -> Self {
        Layout::with_hosts(saved.hosts)
    }
}

#[cfg(feature = "serde")]
impl From<Layout> for SavedLayout {
    fn from(layout: Layout) -> Self {
        SavedLayout { hosts: layout.hosts }
    }
}

impl PartialEq for Layout {
    /// Two layouts are equal when they place the same panels in the same way. IDs are not
    /// compared, because they say nothing about where a panel is.
    fn eq(&self, other: &Self) -> bool {
        fn same(a: &Node, b: &Node) -> bool {
            match (a, b) {
                (Node::Group(a), Node::Group(b)) => a.panels == b.panels && a.active == b.active,
                (Node::Split(a), Node::Split(b)) => {
                    a.axis == b.axis
                        && a.weights == b.weights
                        && a.children.len() == b.children.len()
                        && a.children.iter().zip(&b.children).all(|(a, b)| same(a, b))
                }
                _ => false,
            }
        }
        self.hosts.len() == other.hosts.len()
            && self.hosts.iter().zip(&other.hosts).all(|(a, b)| match (a, b) {
                (None, None) => true,
                (Some(a), Some(b)) => same(a, b),
                _ => false,
            })
    }
}

impl Default for Layout {
    fn default() -> Self {
        Layout::empty()
    }
}

impl fmt::Display for Layout {
    /// Writes one host per line. An empty host is written as `empty`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, host) in self.hosts.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            match host {
                Some(root) => write!(f, "{root}")?,
                None => write!(f, "empty")?,
            }
        }
        Ok(())
    }
}

impl Layout {
    /// A layout of one host that holds nothing.
    pub fn empty() -> Self {
        Layout { hosts: vec![None], next_id: 1 }
    }

    /// A layout of one host that holds `root`.
    pub fn new(root: Node) -> Self {
        Layout::with_hosts(vec![Some(root)])
    }

    /// A layout of several hosts. `None` is an empty host.
    ///
    /// The trees are brought into normal form. A panel that appears more than once is kept
    /// where it appears first.
    pub fn with_hosts(hosts: Vec<Option<Node>>) -> Self {
        let mut layout = Layout { hosts, next_id: 1 };
        if layout.hosts.is_empty() {
            layout.hosts.push(None);
        }
        // IDs read from outside mean nothing here.
        for root in layout.hosts.iter_mut().flatten() {
            root.for_each_group_mut(&mut |g| g.id = GroupId(0));
            root.for_each_split_mut(&mut |s| s.id = SplitId(0));
        }
        layout.normalise();
        layout
    }

    /// The number of hosts.
    pub fn host_count(&self) -> usize {
        self.hosts.len()
    }

    /// Adds empty hosts until `host` exists.
    pub fn ensure_host(&mut self, host: usize) {
        while self.hosts.len() <= host {
            self.hosts.push(None);
        }
    }

    /// The tree of a host. `None` if the host is empty or does not exist.
    pub fn root(&self, host: usize) -> Option<&Node> {
        self.hosts.get(host).and_then(Option::as_ref)
    }

    /// The groups of a host, depth first, which is also left to right and top to bottom.
    pub fn groups(&self, host: usize) -> Vec<&Group> {
        let mut groups = Vec::new();
        if let Some(root) = self.root(host) {
            root.for_each_group(&mut |g| groups.push(g));
        }
        groups
    }

    /// The splits of a host, depth first.
    pub fn splits(&self, host: usize) -> Vec<&Split> {
        let mut splits = Vec::new();
        if let Some(root) = self.root(host) {
            root.for_each_split(&mut |s| splits.push(s));
        }
        splits
    }

    /// The group with the given ID.
    pub fn group(&self, id: GroupId) -> Option<&Group> {
        (0..self.hosts.len()).flat_map(|h| self.groups(h)).find(|g| g.id == id)
    }

    /// The split with the given ID.
    pub fn split(&self, id: SplitId) -> Option<&Split> {
        (0..self.hosts.len()).flat_map(|h| self.splits(h)).find(|s| s.id == id)
    }

    /// The host that holds the given group.
    pub fn host_of_group(&self, id: GroupId) -> Option<usize> {
        (0..self.hosts.len()).find(|h| self.groups(*h).iter().any(|g| g.id == id))
    }

    /// The keys of all panels that are placed, host by host, depth first.
    pub fn panels(&self) -> Vec<&str> {
        (0..self.hosts.len())
            .flat_map(|h| self.groups(h))
            .flat_map(|g| g.panels.iter().map(String::as_str))
            .collect()
    }

    /// Where a panel is. `None` if the panel is not placed.
    pub fn find_panel(&self, panel: &str) -> Option<Location> {
        for host in 0..self.hosts.len() {
            for group in self.groups(host) {
                if let Some(index) = group.panels.iter().position(|p| p == panel) {
                    return Some(Location { host, group: group.id, index });
                }
            }
        }
        None
    }

    /// Whether a panel is placed.
    pub fn contains(&self, panel: &str) -> bool {
        self.find_panel(panel).is_some()
    }

    /// Removes every panel for which `keep` returns false. Returns whether any was removed.
    pub fn retain_panels(&mut self, keep: impl Fn(&str) -> bool) -> bool {
        let mut removed = false;
        for root in self.hosts.iter_mut().flatten() {
            root.for_each_group_mut(&mut |group| {
                let shown = group.active_panel().map(str::to_owned);
                let before = group.panels.len();
                group.panels.retain(|p| keep(p));
                if group.panels.len() != before {
                    removed = true;
                    group.active = shown
                        .and_then(|shown| group.panels.iter().position(|p| *p == shown))
                        .unwrap_or_else(|| group.active.min(group.panels.len().saturating_sub(1)));
                }
            });
        }
        if removed {
            self.normalise();
        }
        removed
    }

    pub(crate) fn group_mut(&mut self, id: GroupId) -> Option<&mut Group> {
        self.hosts.iter_mut().flatten().find_map(|root| find_group_mut(root, id))
    }

    pub(crate) fn split_mut(&mut self, id: SplitId) -> Option<&mut Split> {
        self.hosts.iter_mut().flatten().find_map(|root| find_split_mut(root, id))
    }

    pub(crate) fn new_group(&mut self, panel: String) -> Node {
        let id = GroupId(self.next_id);
        self.next_id += 1;
        Node::Group(Group { id, panels: vec![panel], active: 0 })
    }

    pub(crate) fn new_split(&mut self, axis: Axis, children: Vec<Node>, weights: Vec<f32>) -> Node {
        let id = SplitId(self.next_id);
        self.next_id += 1;
        Node::Split(Split { id, axis, children, weights })
    }

    /// Brings every tree into normal form, and gives an ID to every node that has none.
    pub(crate) fn normalise(&mut self) {
        let mut seen = HashSet::new();
        for host in &mut self.hosts {
            *host = host.take().and_then(|root| normalise(root, &mut seen));
        }
        let mut next = self.next_id.max(1);
        for root in self.hosts.iter_mut().flatten() {
            assign_ids(root, &mut next);
        }
        self.next_id = next;
    }

    /// Checks that the layout is in normal form. For tests. Every operation leaves the
    /// layout in normal form, so an error here is a bug in this crate.
    pub fn check(&self) -> Result<(), String> {
        let mut panels = HashSet::new();
        let mut ids = HashSet::new();
        if self.hosts.is_empty() {
            return Err("no hosts".into());
        }
        for root in self.hosts.iter().flatten() {
            check(root, None, &mut panels, &mut ids)?;
        }
        Ok(())
    }
}

fn find_group_mut(node: &mut Node, id: GroupId) -> Option<&mut Group> {
    match node {
        Node::Group(group) => (group.id == id).then_some(group),
        Node::Split(split) => split.children.iter_mut().find_map(|c| find_group_mut(c, id)),
    }
}

fn find_split_mut(node: &mut Node, id: SplitId) -> Option<&mut Split> {
    match node {
        Node::Group(_) => None,
        Node::Split(split) => {
            if split.id == id {
                Some(split)
            } else {
                split.children.iter_mut().find_map(|c| find_split_mut(c, id))
            }
        }
    }
}

fn normalise(node: Node, seen: &mut HashSet<String>) -> Option<Node> {
    match node {
        Node::Group(mut group) => {
            let shown = group.active_panel().map(str::to_owned);
            group.panels.retain(|p| seen.insert(p.clone()));
            if group.panels.is_empty() {
                return None;
            }
            group.active = shown
                .and_then(|shown| group.panels.iter().position(|p| *p == shown))
                .unwrap_or_else(|| group.active.min(group.panels.len() - 1));
            Some(Node::Group(group))
        }
        Node::Split(split) => {
            let Split { id, axis, children, weights } = split;
            let mut kept = Vec::new();
            let mut kept_weights = Vec::new();
            for (i, child) in children.into_iter().enumerate() {
                let weight = match weights.get(i) {
                    Some(w) if w.is_finite() && *w > 0.0 => *w,
                    _ => 1.0,
                };
                match normalise(child, seen) {
                    // A split along the same axis adds nothing. Its children move up.
                    Some(Node::Split(inner)) if inner.axis == axis => {
                        for (child, inner_weight) in inner.children.into_iter().zip(inner.weights) {
                            kept.push(child);
                            kept_weights.push(weight * inner_weight);
                        }
                    }
                    Some(child) => {
                        kept.push(child);
                        kept_weights.push(weight);
                    }
                    None => {}
                }
            }
            match kept.len() {
                0 => None,
                1 => kept.pop(),
                _ => {
                    let total: f32 = kept_weights.iter().sum();
                    kept_weights.iter_mut().for_each(|w| *w /= total);
                    Some(Node::Split(Split { id, axis, children: kept, weights: kept_weights }))
                }
            }
        }
    }
}

fn assign_ids(node: &mut Node, next: &mut u32) {
    match node {
        Node::Group(group) => {
            if group.id.0 == 0 {
                group.id = GroupId(*next);
                *next += 1;
            }
        }
        Node::Split(split) => {
            if split.id.0 == 0 {
                split.id = SplitId(*next);
                *next += 1;
            }
            split.children.iter_mut().for_each(|c| assign_ids(c, next));
        }
    }
}

fn check(
    node: &Node,
    parent_axis: Option<Axis>,
    panels: &mut HashSet<String>,
    ids: &mut HashSet<u32>,
) -> Result<(), String> {
    match node {
        Node::Group(group) => {
            if group.id.0 == 0 || !ids.insert(group.id.0) {
                return Err(format!("group ID {} is missing or used twice", group.id.0));
            }
            if group.panels.is_empty() {
                return Err(format!("group {} is empty", group.id.0));
            }
            if group.active >= group.panels.len() {
                return Err(format!("group {} shows a panel it does not hold", group.id.0));
            }
            for panel in &group.panels {
                if !panels.insert(panel.clone()) {
                    return Err(format!("panel {panel} appears twice"));
                }
            }
            Ok(())
        }
        Node::Split(split) => {
            if split.id.0 == 0 || !ids.insert(split.id.0) {
                return Err(format!("split ID {} is missing or used twice", split.id.0));
            }
            if split.children.len() < 2 {
                return Err(format!("split {} has fewer than two children", split.id.0));
            }
            if split.weights.len() != split.children.len() {
                return Err(format!("split {} has the wrong number of weights", split.id.0));
            }
            if split.weights.iter().any(|w| !w.is_finite() || *w <= 0.0) {
                return Err(format!("split {} has a weight that is not positive", split.id.0));
            }
            let total: f32 = split.weights.iter().sum();
            if (total - 1.0).abs() > 1e-3 {
                return Err(format!("the weights of split {} sum to {total}", split.id.0));
            }
            if parent_axis == Some(split.axis) {
                return Err(format!("split {} has the axis of its parent", split.id.0));
            }
            for child in &split.children {
                check(child, Some(split.axis), panels, ids)?;
            }
            Ok(())
        }
    }
}

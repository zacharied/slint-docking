//! The [`Dock`]: the layout, the panels, and the windows that show them.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use slint_docking_layout::{Axis, Layout, Metrics, Op, OpError, Point, Rect, Size, Solved, Target, Zone};

use crate::bridge::{GroupView, HostLink, HostView, SplitterView, TabView};

/// What is known about a panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelInfo {
    /// The key of the panel.
    pub key: String,
    /// The text of the panel's tab.
    pub title: String,
    /// Whether the panel's tab has a button to close it.
    pub closable: bool,
    /// Whether the panel is placed in the layout.
    pub open: bool,
}

/// A tab that is dragged outside the dock area it came from.
///
/// This is what lets an application take a panel to another window. See
/// [`Dock::on_drag_outside`].
#[derive(Clone, Debug, PartialEq)]
pub struct DragOutside {
    /// The key of the panel.
    pub panel: String,
    /// The host the tab is dragged from.
    pub host: usize,
    /// Where the pointer is, in the window of that host, in logical pixels. Outside the
    /// window this is negative, or more than the size of the window.
    pub point: Point,
    /// Whether the button was let go. If not, the pointer only moved.
    pub dropped: bool,
}

struct Panel {
    key: String,
    title: String,
    closable: bool,
}

struct Drag {
    panel: String,
}

struct State {
    layout: Layout,
    metrics: Metrics,
    generation: i32,
    /// Every panel that is known, in the order it became known.
    panels: Vec<Panel>,
    /// For each host, the key of the panel that each slot of its window belongs to.
    slots: Vec<Vec<String>>,
    /// For each host, the rectangles at the size they were last asked for.
    solved: Vec<Option<Rc<Solved>>>,
    /// For each host, where its dock area is in its window, once that is known.
    areas: Vec<Option<Rect>>,
    drag: Option<Drag>,
    /// For each panel that was closed, the panels it shared a group with.
    beside: HashMap<String, Vec<String>>,
}

type LayoutListener = Rc<dyn Fn(&Layout)>;
type OutsideListener = Rc<dyn Fn(&DragOutside)>;

struct Inner {
    state: RefCell<State>,
    links: RefCell<Vec<Option<Rc<HostLink>>>>,
    listeners: RefCell<Vec<LayoutListener>>,
    outside: RefCell<Vec<OutsideListener>>,
}

/// A docking system: one layout, and the windows that show it.
///
/// A `Dock` is a handle. Cloning it gives another handle to the same docking system.
///
/// ```ignore
/// let window = MainWindow::new()?;
/// let dock = Dock::new(Layout::new(Node::row([
///     (1.0, Node::tabs(["browser"])),
///     (3.0, Node::tabs(["chart", "preview"])),
/// ])));
/// slint_docking::attach!(dock, window);
/// ```
///
/// The panels are the `DockPanel` elements of the window. A panel that the layout does not
/// place is closed, and [`Dock::open`] places it. A panel that the layout places and no
/// window declares is removed from the layout when the window is attached, so a layout
/// that was saved by an older version of an application is safe to read.
#[derive(Clone)]
pub struct Dock {
    inner: Rc<Inner>,
}

impl Dock {
    /// A docking system that starts with the given layout.
    pub fn new(layout: Layout) -> Self {
        Dock::with_metrics(layout, Metrics::default())
    }

    /// A docking system with its own sizes for tabs, splitters and drop zones.
    pub fn with_metrics(layout: Layout, metrics: Metrics) -> Self {
        Dock {
            inner: Rc::new(Inner {
                state: RefCell::new(State {
                    layout,
                    metrics,
                    generation: 0,
                    panels: Vec::new(),
                    slots: Vec::new(),
                    solved: Vec::new(),
                    areas: Vec::new(),
                    drag: None,
                    beside: HashMap::new(),
                }),
                links: RefCell::new(Vec::new()),
                listeners: RefCell::new(Vec::new()),
                outside: RefCell::new(Vec::new()),
            }),
        }
    }

    /// The sizes of tabs, splitters and drop zones.
    pub fn metrics(&self) -> Metrics {
        self.inner.state.borrow().metrics
    }

    /// The layout as it is now.
    pub fn layout(&self) -> Layout {
        self.inner.state.borrow().layout.clone()
    }

    /// Replaces the layout. Panels that are not known are removed from it first.
    pub fn set_layout(&self, layout: Layout) {
        {
            let mut state = self.inner.state.borrow_mut();
            let hosts = state.layout.host_count();
            state.layout = layout;
            state.layout.ensure_host(hosts.saturating_sub(1));
            state.forget_unknown_panels();
            state.beside.clear();
        }
        self.refresh();
        self.notify();
    }

    /// Calls `listener` after every change to the layout, with the layout as it then is.
    ///
    /// Dragging a splitter changes the layout with every move of the pointer.
    pub fn on_layout_changed(&self, listener: impl Fn(&Layout) + 'static) {
        self.inner.listeners.borrow_mut().push(Rc::new(listener));
    }

    /// Makes a panel known before any window declares it.
    ///
    /// Needed only for a panel that is declared under `if`, which no window declares
    /// until the layout places the panel in it.
    pub fn register(&self, key: impl Into<String>, title: impl Into<String>) {
        self.inner.state.borrow_mut().learn(key.into(), title.into(), true);
        self.refresh();
    }

    /// Every panel that is known, in the order it became known.
    pub fn panels(&self) -> Vec<PanelInfo> {
        let state = self.inner.state.borrow();
        state
            .panels
            .iter()
            .map(|p| PanelInfo {
                key: p.key.clone(),
                title: p.title.clone(),
                closable: p.closable,
                open: state.layout.contains(&p.key),
            })
            .collect()
    }

    /// Whether a panel is placed in the layout.
    pub fn is_open(&self, panel: &str) -> bool {
        self.inner.state.borrow().layout.contains(panel)
    }

    /// Applies an operation to the layout. Returns whether the layout changed.
    ///
    /// Everything that changes the layout goes through here, whether it comes from the
    /// pointer or from the application.
    pub fn apply(&self, op: Op) -> Result<bool, OpError> {
        let changed = {
            let mut state = self.inner.state.borrow_mut();
            match &op {
                Op::Place { panel, .. } if state.panel(panel).is_none() => {
                    return Err(OpError::UnknownPanel(panel.clone()));
                }
                Op::Close { panel } => {
                    let beside = state.layout.find_panel(panel).and_then(|at| {
                        let group = state.layout.group(at.group)?;
                        Some(group.panels().iter().filter(|p| *p != panel).cloned().collect())
                    });
                    if let Some(beside) = beside {
                        state.beside.insert(panel.clone(), beside);
                    }
                }
                _ => {}
            }
            state.layout.apply(op)?
        };
        if changed {
            self.refresh();
            self.notify();
        }
        Ok(changed)
    }

    /// Brings a panel to the front, placing it first if it is closed.
    ///
    /// A closed panel goes back to the group it was closed in, if any panel of that group
    /// is still placed. Otherwise it joins the first group of the first host.
    pub fn open(&self, panel: &str) -> Result<bool, OpError> {
        let target = {
            let state = self.inner.state.borrow();
            if state.layout.contains(panel) {
                None
            } else {
                let beside =
                    state.beside.get(panel).into_iter().flatten().find_map(|p| state.layout.find_panel(p));
                Some(match beside {
                    Some(at) => Target::Group { group: at.group, zone: Zone::Center },
                    None => match state.layout.groups(0).first() {
                        Some(group) => Target::Group { group: group.id(), zone: Zone::Center },
                        None => Target::Edge { host: 0, zone: Zone::Center },
                    },
                })
            }
        };
        match target {
            Some(target) => self.apply(Op::Place { panel: panel.into(), target }),
            None => self.apply(Op::Activate { panel: panel.into() }),
        }
    }

    /// Closes a panel. The panel stays known, and [`Dock::open`] brings it back.
    pub fn close(&self, panel: &str) -> Result<bool, OpError> {
        self.apply(Op::Close { panel: panel.into() })
    }

    /// Moves a panel to another host, where it takes the right side, or all of an empty
    /// host. A panel that is already in the host stays where it is.
    pub fn move_to_host(&self, panel: &str, host: usize) -> Result<bool, OpError> {
        let zone = {
            let mut state = self.inner.state.borrow_mut();
            state.layout.ensure_host(host);
            if state.layout.find_panel(panel).is_some_and(|at| at.host == host) {
                return Ok(false);
            }
            match state.layout.root(host) {
                Some(_) => Zone::Right,
                None => Zone::Center,
            }
        };
        self.apply(Op::Place { panel: panel.into(), target: Target::Edge { host, zone } })
    }

    /// Calls `listener` while a tab is dragged outside the dock area it came from, and
    /// when it is let go there.
    ///
    /// The docking system does nothing with such a drag itself. An application with more
    /// than one window uses this to take a panel from one window to another:
    ///
    /// - While the pointer moves, find the window under it, and call [`Dock::hover_at`]
    ///   to mark where the panel would go.
    /// - When the button is let go over a window, call [`Dock::place_at`].
    /// - When it is let go over no window, call [`Dock::move_to_host`] to give the panel
    ///   a window of its own.
    ///
    /// Points are in the window of a host. Taking a point from one window to another
    /// through the position of each on the screen is up to the application, because only
    /// it knows its windows.
    pub fn on_drag_outside(&self, listener: impl Fn(&DragOutside) + 'static) {
        self.inner.outside.borrow_mut().push(Rc::new(listener));
    }

    /// Where the dock area of a host is in its window. `None` until the host has shown
    /// a group.
    pub fn area(&self, host: usize) -> Option<Rect> {
        self.inner.state.borrow().areas.get(host).copied().flatten()
    }

    /// Marks where the dragged panel would go if it were let go at a point of the window
    /// of a host. Returns whether it would go anywhere. Does nothing unless a tab is being
    /// dragged.
    pub fn hover_at(&self, host: usize, point: Point) -> bool {
        let panel = self.inner.state.borrow().drag.as_ref().map(|d| d.panel.clone());
        let preview = panel.and_then(|panel| self.op_at(&panel, host, point)).map(|(_, r)| r);
        if let Some(link) = self.link(host) {
            (link.preview)(preview);
        }
        preview.is_some()
    }

    /// Puts a panel where a drop at a point of the window of a host would put it. Returns
    /// whether the layout changed. Nothing changes if the point is outside the dock area
    /// of the host, or the area is not yet known.
    pub fn place_at(&self, panel: &str, host: usize, point: Point) -> Result<bool, OpError> {
        if self.inner.state.borrow().panel(panel).is_none() {
            return Err(OpError::UnknownPanel(panel.to_owned()));
        }
        match self.op_at(panel, host, point) {
            Some((op, _)) => self.apply(op),
            None => Ok(false),
        }
    }

    /// The operation that a drop of a panel at a point of the window of a host leads to,
    /// and the area to mark. `None` if the drop would change nothing.
    fn op_at(&self, panel: &str, host: usize, point: Point) -> Option<(Op, Rect)> {
        let area = self.area(host)?;
        let solved = self.solved(host, Size::new(area.width, area.height));
        let state = self.inner.state.borrow();
        let point = Point::new(point.x - area.x, point.y - area.y);
        let hit = state.layout.target_at(host, &solved, point, &state.metrics)?;
        let op = Op::Place { panel: panel.to_owned(), target: hit.target };
        let changes = state.layout.clone().apply(op.clone()).unwrap_or(false);
        changes.then_some((op, hit.preview))
    }

    /// Tells the application of a drag that is outside its dock area. Returns whether
    /// the point is outside.
    fn tell_outside(&self, host: usize, at: (f32, f32), size: (f32, f32), dropped: bool) -> bool {
        if Rect::new(0.0, 0.0, size.0, size.1).contains(Point::new(at.0, at.1)) {
            return false;
        }
        let event = {
            let state = self.inner.state.borrow();
            let Some(drag) = &state.drag else { return true };
            let origin = state.areas.get(host).copied().flatten().unwrap_or_default();
            DragOutside {
                panel: drag.panel.clone(),
                host,
                point: Point::new(at.0 + origin.x, at.1 + origin.y),
                dropped,
            }
        };
        if dropped {
            self.drag_cancelled();
        }
        let listeners: Vec<_> = self.inner.outside.borrow().clone();
        for listener in listeners {
            listener(&event);
        }
        true
    }

    fn refresh(&self) {
        let (views, generation) = {
            let mut state = self.inner.state.borrow_mut();
            state.generation = state.generation.wrapping_add(1);
            state.solved.clear();
            let hosts = state.layout.host_count();
            ((0..hosts).map(|h| state.view(h)).collect::<Vec<_>>(), state.generation)
        };
        let links: Vec<_> = self.inner.links.borrow().clone();
        for (host, link) in links.iter().enumerate() {
            if let Some(link) = link {
                let empty = HostView::default();
                (link.show)(views.get(host).unwrap_or(&empty), generation);
            }
        }
    }

    fn notify(&self) {
        let listeners: Vec<_> = self.inner.listeners.borrow().clone();
        if listeners.is_empty() {
            return;
        }
        let layout = self.layout();
        for listener in listeners {
            listener(&layout);
        }
    }

    fn link(&self, host: usize) -> Option<Rc<HostLink>> {
        self.inner.links.borrow().get(host).cloned().flatten()
    }

    fn solved(&self, host: usize, size: Size) -> Rc<Solved> {
        let mut state = self.inner.state.borrow_mut();
        if state.solved.len() <= host {
            state.solved.resize(host + 1, None);
        }
        match &state.solved[host] {
            Some(solved) if solved.size == size => solved.clone(),
            _ => {
                let solved = Rc::new(state.layout.solve(host, size, &state.metrics));
                state.solved[host] = Some(solved.clone());
                solved
            }
        }
    }
}

/// What [`attach!`](crate::attach) calls. Not for applications.
#[doc(hidden)]
impl Dock {
    pub fn attach_host(&self, host: usize, link: HostLink) {
        {
            let mut links = self.inner.links.borrow_mut();
            if links.len() <= host {
                links.resize(host + 1, None);
            }
            links[host] = Some(Rc::new(link));
        }
        self.inner.state.borrow_mut().layout.ensure_host(host);
        self.panels_declared(host);
        let forgot = self.inner.state.borrow_mut().forget_unknown_panels();
        self.refresh();
        if forgot {
            self.notify();
        }
    }

    pub fn detach_host(&self, host: usize) {
        if let Some(link) = self.inner.links.borrow_mut().get_mut(host) {
            *link = None;
        }
    }

    pub fn panels_declared(&self, host: usize) {
        let Some(link) = self.link(host) else { return };
        let declared = (link.declared)();
        {
            let mut state = self.inner.state.borrow_mut();
            if state.slots.len() <= host {
                state.slots.resize(host + 1, Vec::new());
            }
            let mut slots = Vec::new();
            for record in declared.split('\u{1e}').filter(|r| !r.is_empty()) {
                let mut fields = record.split('\u{1f}');
                let key = fields.next().unwrap_or_default().to_owned();
                let title = fields.next().unwrap_or_default().to_owned();
                let closable = fields.next() != Some("0");
                state.learn(key.clone(), title, closable);
                slots.push(key);
            }
            if state.slots[host] == slots {
                return;
            }
            state.slots[host] = slots;
        }
        self.refresh();
    }

    pub fn title_changed(&self, host: usize, slot: i32, title: &str) {
        let key = {
            let state = self.inner.state.borrow();
            state.key(host, slot).map(str::to_owned)
        };
        let Some(key) = key else { return };
        if let Some(panel) = self.inner.state.borrow_mut().panel_mut(&key) {
            panel.title = title.to_owned();
        }
        self.refresh();
    }

    pub fn panel_rect(&self, host: usize, slot: i32, width: f32, height: f32) -> (Rect, bool) {
        let key = {
            let state = self.inner.state.borrow();
            state.key(host, slot).map(str::to_owned)
        };
        key.and_then(|key| self.solved(host, Size::new(width, height)).panel(&key)).unwrap_or_default()
    }

    pub fn group_rect(
        &self,
        host: usize,
        index: i32,
        width: f32,
        height: f32,
        area_x: f32,
        area_y: f32,
    ) -> (Rect, f32) {
        {
            let mut state = self.inner.state.borrow_mut();
            if state.areas.len() <= host {
                state.areas.resize(host + 1, None);
            }
            state.areas[host] = Some(Rect::new(area_x, area_y, width, height));
        }
        let solved = self.solved(host, Size::new(width, height));
        usize::try_from(index)
            .ok()
            .and_then(|i| solved.groups.get(i))
            .map(|g| (g.tab_bar, g.tab_width))
            .unwrap_or_default()
    }

    pub fn splitter_rect(&self, host: usize, index: i32, width: f32, height: f32) -> Rect {
        let solved = self.solved(host, Size::new(width, height));
        usize::try_from(index).ok().and_then(|i| solved.splitters.get(i)).map(|s| s.rect).unwrap_or_default()
    }

    pub fn holds(&self, host: usize, panel: &str) -> bool {
        self.inner.state.borrow().layout.find_panel(panel).is_some_and(|at| at.host == host)
    }

    pub fn tab_pressed(&self, panel: &str) {
        let _ = self.apply(Op::Activate { panel: panel.into() });
    }

    pub fn tab_closed(&self, panel: &str) {
        let _ = self.close(panel);
    }

    pub fn drag_started(&self, host: usize, panel: &str) {
        let title = {
            let mut state = self.inner.state.borrow_mut();
            state.drag = Some(Drag { panel: panel.to_owned() });
            state.panel(panel).map(|p| p.title.clone()).unwrap_or_else(|| panel.to_owned())
        };
        if let Some(link) = self.link(host) {
            (link.dragging)(Some(&title));
        }
    }

    pub fn drag_moved(&self, host: usize, x: f32, y: f32, width: f32, height: f32) {
        let preview = self.drop_at(host, x, y, width, height).map(|(_, preview)| preview);
        let links: Vec<_> = self.inner.links.borrow().clone();
        for (other, link) in links.iter().enumerate() {
            if let Some(link) = link {
                (link.preview)(preview.filter(|_| other == host));
            }
        }
        self.tell_outside(host, (x, y), (width, height), false);
    }

    pub fn drag_dropped(&self, host: usize, x: f32, y: f32, width: f32, height: f32) {
        if self.tell_outside(host, (x, y), (width, height), true) {
            return;
        }
        let op = self.drop_at(host, x, y, width, height).map(|(op, _)| op);
        self.drag_cancelled();
        if let Some(op) = op {
            let _ = self.apply(op);
        }
    }

    pub fn drag_cancelled(&self) {
        self.inner.state.borrow_mut().drag = None;
        let links: Vec<_> = self.inner.links.borrow().clone();
        for link in links.iter().flatten() {
            (link.preview)(None);
            (link.dragging)(None);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn splitter_dragged(
        &self,
        host: usize,
        split: i32,
        handle: i32,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        let (Ok(split), Ok(handle)) = (u32::try_from(split), usize::try_from(handle)) else {
            return;
        };
        let split = slint_docking_layout::SplitId(split);
        let solved = self.solved(host, Size::new(width, height));
        let weights = {
            let state = self.inner.state.borrow();
            let Some(axis) = state.layout.split(split).map(|s| s.axis()) else { return };
            let position = match axis {
                Axis::Horizontal => x,
                Axis::Vertical => y,
            };
            state.layout.weights_for_drag(&solved, split, handle, position, &state.metrics)
        };
        if let Some(weights) = weights {
            let _ = self.apply(Op::SetWeights { split, weights });
        }
    }

    /// The operation that dropping the dragged panel at a point leads to, and the area to
    /// mark. `None` if nothing is dragged, or the drop would change nothing.
    fn drop_at(&self, host: usize, x: f32, y: f32, width: f32, height: f32) -> Option<(Op, Rect)> {
        let solved = self.solved(host, Size::new(width, height));
        let state = self.inner.state.borrow();
        let panel = state.drag.as_ref()?.panel.clone();
        let hit = state.layout.target_at(host, &solved, Point::new(x, y), &state.metrics)?;
        let op = Op::Place { panel, target: hit.target };
        let changes = state.layout.clone().apply(op.clone()).unwrap_or(false);
        changes.then_some((op, hit.preview))
    }
}

impl State {
    fn panel(&self, key: &str) -> Option<&Panel> {
        self.panels.iter().find(|p| p.key == key)
    }

    fn panel_mut(&mut self, key: &str) -> Option<&mut Panel> {
        self.panels.iter_mut().find(|p| p.key == key)
    }

    fn learn(&mut self, key: String, title: String, closable: bool) {
        match self.panel_mut(&key) {
            Some(panel) => {
                panel.title = title;
                panel.closable = closable;
            }
            None => self.panels.push(Panel { key, title, closable }),
        }
    }

    fn key(&self, host: usize, slot: i32) -> Option<&str> {
        let slot = usize::try_from(slot).ok()?;
        self.slots.get(host)?.get(slot).map(String::as_str)
    }

    fn forget_unknown_panels(&mut self) -> bool {
        let known: Vec<&str> = self.panels.iter().map(|p| p.key.as_str()).collect();
        self.layout.retain_panels(|p| known.contains(&p))
    }

    fn view(&self, host: usize) -> HostView {
        let groups = self
            .layout
            .groups(host)
            .into_iter()
            .map(|group| GroupView {
                id: group.id().0,
                tabs: group
                    .panels()
                    .iter()
                    .enumerate()
                    .map(|(i, key)| {
                        let panel = self.panel(key);
                        TabView {
                            key: key.clone(),
                            title: panel.map_or_else(|| key.clone(), |p| p.title.clone()),
                            active: i == group.active_index(),
                            closable: panel.is_none_or(|p| p.closable),
                        }
                    })
                    .collect(),
            })
            .collect();
        // The order of the bars depends on the tree alone, so any size gives it.
        let splitters = self
            .layout
            .solve(host, Size::new(1000.0, 1000.0), &self.metrics)
            .splitters
            .iter()
            .map(|bar| SplitterView {
                split: bar.split.0,
                handle: bar.handle as u32,
                horizontal: bar.axis == Axis::Horizontal,
            })
            .collect();
        HostView { groups, splitters }
    }
}

//! What the interface tests share: the windows they drive, and how they find and move
//! things in them.
//!
//! The tests run without a display, on the testing backend that Slint's own tests use.
//! Time is mocked, so a drag takes no real time and gives the same result every run.

use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementRoot};
use slint::platform::PointerEventButton;
use slint::{ComponentHandle, LogicalPosition, LogicalSize};
use slint_docking::{Dock, Layout, Metrics, Node, Rect};

slint::include_modules!();

/// The sizes the tests are written against.
pub fn metrics() -> Metrics {
    Metrics {
        tab_bar_height: 30.0,
        tab_width: 100.0,
        splitter: 4.0,
        min_panel_width: 80.0,
        min_panel_height: 60.0,
        edge_band: 12.0,
        zone_fraction: 0.25,
    }
}

/// The layout most tests start from: Alpha and Beta on the left, Gamma over Delta on the
/// right.
pub fn start() -> Layout {
    Layout::new(Node::row([
        (1.0, Node::tabs(["a", "b"])),
        (3.0, Node::column([(1.0, Node::tabs(["c"])), (1.0, Node::tabs(["d"]))])),
    ]))
}

/// A window of 1000 by 600 with a dock attached.
pub struct Rig {
    pub window: Fixture,
    pub dock: Dock,
}

/// Sets up the testing backend for this thread. The test runner gives each test a thread
/// of its own, and a test may make more than one window.
pub fn init() {
    thread_local!(static DONE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
    if !DONE.replace(true) {
        i_slint_backend_testing::init_no_event_loop();
    }
}

/// Makes the window and attaches a dock with the given layout.
pub fn rig(layout: Layout) -> Rig {
    init();
    let window = Fixture::new().expect("the window is made");
    let dock = Dock::with_metrics(layout, metrics());
    slint_docking::attach!(dock, window);
    window.show().expect("the window is shown");
    window.window().set_size(LogicalSize::new(1000.0, 600.0));
    Rig { window, dock }
}

impl Rig {
    /// The layout, written on one line.
    pub fn layout(&self) -> String {
        self.dock.layout().to_string()
    }
}

fn find(window: &impl ElementRoot, label: &str, role: AccessibleRole) -> Option<ElementHandle> {
    ElementHandle::find_by_accessible_label(window, label).find(|e| e.accessible_role() == Some(role))
}

/// The tab with the given title.
pub fn tab(window: &impl ElementRoot, title: &str) -> ElementHandle {
    find(window, title, AccessibleRole::Tab).unwrap_or_else(|| panic!("no tab {title}"))
}

/// Whether there is a tab with the given title.
pub fn has_tab(window: &impl ElementRoot, title: &str) -> bool {
    find(window, title, AccessibleRole::Tab).is_some()
}

/// The titles of all tabs, in the order they are drawn: group by group, left to right.
pub fn tabs(window: &impl ElementRoot) -> Vec<String> {
    ElementHandle::find_by_element_type_name(window, "DockTab")
        .filter_map(|e| e.accessible_label())
        .map(|l| l.to_string())
        .collect()
}

/// The button that closes the panel with the given title.
pub fn close_button(window: &impl ElementRoot, title: &str) -> ElementHandle {
    find(window, &format!("Close {title}"), AccessibleRole::Button)
        .unwrap_or_else(|| panic!("no close button for {title}"))
}

/// The rectangle of an element, in the window.
pub fn rect(element: &ElementHandle) -> Rect {
    let at = element.absolute_position();
    let size = element.size();
    Rect::new(at.x, at.y, size.width, size.height)
}

/// The rectangle of the panel with the given title, or `None` if the panel is not shown.
///
/// Slint leaves an element that is not visible out of what the testing backend finds, so
/// a panel that is closed, or is a tab behind another, is not found.
pub fn shown(window: &impl ElementRoot, title: &str) -> Option<Rect> {
    find(window, title, AccessibleRole::TabPanel).map(|panel| rect(&panel))
}

/// The bars between groups, in the order of the layout.
pub fn splitters(window: &impl ElementRoot) -> Vec<ElementHandle> {
    ElementHandle::find_by_element_id(window, "DockArea::splitter").collect()
}

/// The mark of where a dragged panel would go, or `None` if it is not shown.
pub fn preview(window: &impl ElementRoot) -> Option<Rect> {
    ElementHandle::find_by_element_id(window, "DockArea::preview").next().map(|e| rect(&e))
}

/// The number the counter in a panel shows, or `None` if the panel is not shown.
pub fn count(window: &impl ElementRoot, name: &str) -> Option<String> {
    find(window, &format!("count {name}"), AccessibleRole::Text)
        .and_then(|e| e.accessible_value())
        .map(|v| v.to_string())
}

/// The middle of the counter in a panel, which is the middle of the panel.
pub fn counter(window: &impl ElementRoot, name: &str) -> ElementHandle {
    find(window, &format!("count {name}"), AccessibleRole::Text)
        .unwrap_or_else(|| panic!("no counter {name}"))
}

/// Presses the left button on the middle of an element and lets go.
pub fn click(element: &ElementHandle) {
    element.mock_single_click(PointerEventButton::Left);
}

/// Clicks a point of the window.
pub fn click_at<W: ComponentHandle>(window: &W, x: f32, y: f32) {
    use slint::platform::WindowEvent;
    let position = LogicalPosition::new(x, y);
    let button = PointerEventButton::Left;
    window.window().dispatch_event(WindowEvent::PointerMoved { position });
    window.window().dispatch_event(WindowEvent::PointerPressed { position, button });
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(50));
    window.window().dispatch_event(WindowEvent::PointerReleased { position, button });
}

/// Drags an element by its middle to a point of the window, and lets go there.
pub fn drag(element: &ElementHandle, x: f32, y: f32) {
    element.mock_drag(LogicalPosition::new(x, y), PointerEventButton::Left);
}

/// Presses the left button on the middle of an element and moves to a point of the
/// window, in steps, without letting go.
pub fn drag_and_hold<W: ComponentHandle>(window: &W, element: &ElementHandle, x: f32, y: f32) {
    use slint::platform::WindowEvent;
    let from = rect(element).center();
    let button = PointerEventButton::Left;
    let position = LogicalPosition::new(from.x, from.y);
    window.window().dispatch_event(WindowEvent::PointerMoved { position });
    window.window().dispatch_event(WindowEvent::PointerPressed { position, button });
    for step in 1..=10 {
        let t = step as f32 / 10.0;
        let position = LogicalPosition::new(from.x + (x - from.x) * t, from.y + (y - from.y) * t);
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
        window.window().dispatch_event(WindowEvent::PointerMoved { position });
    }
}

/// Lets go of the left button at a point of the window.
pub fn release<W: ComponentHandle>(window: &W, x: f32, y: f32) {
    use slint::platform::WindowEvent;
    let position = LogicalPosition::new(x, y);
    window
        .window()
        .dispatch_event(WindowEvent::PointerReleased { position, button: PointerEventButton::Left });
}

/// Checks that a window shows what the layout says: the tabs in their order and place,
/// the panels that are in front where the solver puts them and no others, and the bars.
///
/// `area` is the rectangle of the dock area in the window.
pub fn check_host(window: &impl ElementRoot, dock: &Dock, host: usize, area: Rect) {
    let layout = dock.layout();
    let size = slint_docking::Size::new(area.width, area.height);
    let solved = layout.solve(host, size, &dock.metrics());
    let panels = dock.panels();
    let title = |key: &str| {
        panels
            .iter()
            .find(|p| p.key == key)
            .map(|p| p.title.clone())
            .unwrap_or_else(|| panic!("the layout places {key}, which is not known"))
    };
    let moved = |r: Rect| Rect::new(r.x + area.x, r.y + area.y, r.width, r.height);

    let expected: Vec<String> =
        solved.groups.iter().flat_map(|g| g.panels.iter().map(|k| title(k))).collect();
    assert_eq!(tabs(window), expected, "tabs of {layout}");

    for group in &solved.groups {
        for (i, key) in group.panels.iter().enumerate() {
            let tab = tab(window, &title(key));
            assert_eq!(rect(&tab), moved(group.tab_rect(i)), "tab {key} of {layout}");
        }
    }
    for panel in &panels {
        let expected = solved.panel(&panel.key).filter(|(_, shown)| *shown).map(|(r, _)| moved(r));
        assert_eq!(shown(window, &panel.title), expected, "panel {} of {layout}", panel.key);
    }
    let bars: Vec<Rect> = splitters(window).iter().map(rect).collect();
    let expected: Vec<Rect> = solved.splitters.iter().map(|s| moved(s.rect)).collect();
    assert_eq!(bars, expected, "bars of {layout}");
}

impl Rig {
    /// Checks that the window shows what the layout says.
    pub fn check(&self) {
        let size = self.window.window().size().to_logical(self.window.window().scale_factor());
        check_host(&self.window, &self.dock, 0, Rect::new(0.0, 0.0, size.width, size.height));
    }
}

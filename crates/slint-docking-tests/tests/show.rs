//! A window shows what the layout says.

use slint::{ComponentHandle, LogicalSize};
use slint_docking::{Layout, Node, Rect};
use slint_docking_tests::*;

#[test]
fn panels_are_placed_where_the_layout_says() {
    let rig = rig(start());
    assert_eq!(tabs(&rig.window), ["Alpha", "Beta", "Gamma", "Delta"]);
    assert_eq!(shown(&rig.window, "Alpha"), Some(Rect::new(0.0, 30.0, 249.0, 570.0)));
    assert_eq!(shown(&rig.window, "Beta"), None);
    assert_eq!(shown(&rig.window, "Gamma"), Some(Rect::new(253.0, 30.0, 747.0, 268.0)));
    assert_eq!(shown(&rig.window, "Delta"), Some(Rect::new(253.0, 332.0, 747.0, 268.0)));
    assert_eq!(rect(&tab(&rig.window, "Beta")), Rect::new(100.0, 0.0, 100.0, 30.0));
    rig.check();
}

#[test]
fn rectangles_follow_the_size_of_the_window() {
    let rig = rig(start());
    for (width, height) in [(800.0, 500.0), (1600.0, 1200.0), (300.0, 200.0), (1000.0, 600.0)] {
        rig.window.window().set_size(LogicalSize::new(width, height));
        rig.check();
    }
    assert_eq!(shown(&rig.window, "Alpha"), Some(Rect::new(0.0, 30.0, 249.0, 570.0)));
}

#[test]
fn a_panel_the_layout_does_not_place_is_closed() {
    let rig = rig(Layout::new(Node::tabs(["a", "c"])));
    assert_eq!(tabs(&rig.window), ["Alpha", "Gamma"]);
    assert_eq!(shown(&rig.window, "Beta"), None);
    assert!(!rig.dock.is_open("b"));
    let known: Vec<_> = rig.dock.panels().into_iter().map(|p| (p.key, p.open)).collect();
    assert_eq!(known, [("a".into(), true), ("b".into(), false), ("c".into(), true), ("d".into(), false)]);
    rig.check();
}

#[test]
fn a_panel_no_window_declares_is_removed_from_the_layout() {
    let rig =
        rig(Layout::new(Node::row([(1.0, Node::tabs(["a", "gone"])), (1.0, Node::tabs(["also gone"]))])));
    assert_eq!(rig.layout(), "tabs(a*)");
    rig.check();
}

#[test]
fn an_empty_layout_shows_nothing() {
    let rig = rig(Layout::empty());
    assert!(tabs(&rig.window).is_empty());
    assert_eq!(shown(&rig.window, "Alpha"), None);
    rig.check();
}

#[test]
fn the_title_of_a_panel_is_the_text_of_its_tab() {
    let rig = rig(start());
    rig.window.set_delta_title("Delta, changed".into());
    // Slint tells of a changed property when the event loop next runs.
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
    assert_eq!(tabs(&rig.window), ["Alpha", "Beta", "Gamma", "Delta, changed"]);
    assert_eq!(rig.dock.panels()[3].title, "Delta, changed");
    rig.check();
}

#[test]
fn a_panel_that_is_declared_late_becomes_known() {
    let rig = rig(start());
    assert_eq!(rig.dock.panels().len(), 4);
    rig.window.set_has_late(true);
    // The panel is made when the window is next laid out, which looking for it causes.
    assert_eq!(shown(&rig.window, "Late"), None);
    assert_eq!(rig.dock.panels().len(), 5);
    assert!(!rig.dock.is_open("late"));

    assert_eq!(rig.dock.open("late"), Ok(true));
    assert_eq!(rig.layout(), "row(25: tabs(a, b, late*), 75: col(50: tabs(c*), 50: tabs(d*)))");
    assert_eq!(shown(&rig.window, "Late"), Some(Rect::new(0.0, 30.0, 249.0, 570.0)));
    rig.check();
}

#[test]
fn a_dock_area_need_not_fill_the_window() {
    init();
    let window = Inset::new().expect("the window is made");
    let dock = slint_docking::Dock::with_metrics(
        Layout::new(Node::row([(1.0, Node::tabs(["a"])), (1.0, Node::tabs(["b"]))])),
        metrics(),
    );
    slint_docking::attach!(dock, window);
    window.show().expect("the window is shown");
    window.window().set_size(LogicalSize::new(1000.0, 600.0));

    let area = Rect::new(100.0, 40.0, 900.0, 560.0);
    check_host(&window, &dock, 0, area);

    // Points are taken in the dock area, not in the window. This point is in the right
    // zone of Beta's group in the area. Taken in the window it would be outside the area.
    drag(&tab(&window, "Alpha"), 960.0, 300.0);
    assert_eq!(dock.layout().to_string(), "row(50: tabs(b*), 50: tabs(a*))");
    check_host(&window, &dock, 0, area);

    drag(&splitters(&window)[0], 500.0, 300.0);
    assert_eq!(shown(&window, "Beta"), Some(Rect::new(100.0, 70.0, 398.0, 530.0)));
    check_host(&window, &dock, 0, area);
}

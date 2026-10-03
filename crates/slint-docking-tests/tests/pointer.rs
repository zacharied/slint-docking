//! What the pointer does.

use slint::platform::PointerEventButton;
use slint_docking::{Layout, Node, Rect};
use slint_docking_tests::*;

const START: &str = "row(25: tabs(a*, b), 75: col(50: tabs(c*), 50: tabs(d*)))";
const BETA_IN_FRONT: &str = "row(25: tabs(a, b*), 75: col(50: tabs(c*), 50: tabs(d*)))";

#[test]
fn pressing_a_tab_brings_its_panel_to_the_front() {
    let rig = rig(start());
    assert_eq!(rig.layout(), START);
    click(&tab(&rig.window, "Beta"));
    assert_eq!(rig.layout(), BETA_IN_FRONT);
    assert_eq!(shown(&rig.window, "Alpha"), None);
    assert_eq!(shown(&rig.window, "Beta"), Some(Rect::new(0.0, 30.0, 249.0, 570.0)));
    rig.check();
}

#[test]
fn a_tab_dropped_on_the_middle_of_a_group_joins_it() {
    let rig = rig(start());
    drag(&tab(&rig.window, "Beta"), 600.0, 160.0);
    assert_eq!(rig.layout(), "row(25: tabs(a*), 75: col(50: tabs(c, b*), 50: tabs(d*)))");
    rig.check();
}

#[test]
fn a_tab_dropped_on_a_side_of_a_group_splits_it() {
    // Gamma's group is a child of the column on the right. A drop on its left or right
    // puts a row in its place. A drop on its top or bottom adds a child to the column.
    let cases = [
        (300.0, 160.0, "col(50: row(50: tabs(b*), 50: tabs(c*)), 50: tabs(d*))"),
        (950.0, 160.0, "col(50: row(50: tabs(c*), 50: tabs(b*)), 50: tabs(d*))"),
        (600.0, 50.0, "col(25: tabs(b*), 25: tabs(c*), 50: tabs(d*))"),
        (600.0, 280.0, "col(25: tabs(c*), 25: tabs(b*), 50: tabs(d*))"),
    ];
    for (x, y, right) in cases {
        let rig = rig(start());
        drag(&tab(&rig.window, "Beta"), x, y);
        assert_eq!(rig.layout(), format!("row(25: tabs(a*), 75: {right})"), "at {x}, {y}");
        rig.check();
    }
}

#[test]
fn a_tab_dropped_among_tabs_takes_that_place() {
    let rig = rig(Layout::new(Node::row([(1.0, Node::tabs(["a", "b", "c"])), (1.0, Node::tabs(["d"]))])));
    // Within its own group.
    drag(&tab(&rig.window, "Alpha"), 290.0, 15.0);
    assert_eq!(rig.layout(), "row(50: tabs(b, c, a*), 50: tabs(d*))");
    drag(&tab(&rig.window, "Alpha"), 110.0, 15.0);
    assert_eq!(rig.layout(), "row(50: tabs(b, a*, c), 50: tabs(d*))");
    rig.check();

    // Into another group, before its only tab. Delta's tab starts at 502.
    drag(&tab(&rig.window, "Alpha"), 520.0, 15.0);
    assert_eq!(rig.layout(), "row(50: tabs(b, c*), 50: tabs(a*, d))");
    rig.check();
}

#[test]
fn a_tab_dropped_on_an_edge_of_the_window_takes_that_side() {
    let rig = rig(start());
    drag(&tab(&rig.window, "Beta"), 500.0, 595.0);
    assert_eq!(rig.layout(), "col(75: row(25: tabs(a*), 75: col(50: tabs(c*), 50: tabs(d*))), 25: tabs(b*))");
    assert_eq!(shown(&rig.window, "Beta"), Some(Rect::new(0.0, 481.0, 1000.0, 119.0)));
    rig.check();
}

#[test]
fn the_last_tab_of_a_group_takes_the_group_with_it() {
    let rig = rig(start());
    drag(&tab(&rig.window, "Gamma"), 100.0, 300.0);
    assert_eq!(rig.layout(), "row(25: tabs(a, b, c*), 75: tabs(d*))");
    assert_eq!(splitters(&rig.window).len(), 1);
    rig.check();
}

#[test]
fn a_press_that_hardly_moves_is_not_a_drag() {
    let rig = rig(start());
    let beta = tab(&rig.window, "Beta");
    let middle = rect(&beta).center();
    drag_and_hold(&rig.window, &beta, middle.x + 4.0, middle.y + 4.0);
    assert_eq!(preview(&rig.window), None);
    release(&rig.window, middle.x + 4.0, middle.y + 4.0);
    assert_eq!(rig.layout(), BETA_IN_FRONT);
    rig.check();
}

#[test]
fn the_place_a_tab_would_take_is_marked_while_it_is_dragged() {
    let rig = rig(start());
    let beta = tab(&rig.window, "Beta");

    drag_and_hold(&rig.window, &beta, 950.0, 160.0);
    assert_eq!(preview(&rig.window), Some(Rect::new(626.5, 0.0, 373.5, 298.0)));
    // Nothing moves until the button is let go.
    assert_eq!(rig.layout(), BETA_IN_FRONT);

    release(&rig.window, 950.0, 160.0);
    assert_eq!(preview(&rig.window), None);
    assert_eq!(rig.layout(), "row(25: tabs(a*), 75: col(50: row(50: tabs(c*), 50: tabs(b*)), 50: tabs(d*)))");
    rig.check();
}

#[test]
fn a_drop_that_would_change_nothing_is_not_marked() {
    let rig = rig(start());
    // Gamma is alone in its group, so there is nowhere in the group for it to go.
    for (x, y) in [(600.0, 160.0), (950.0, 160.0), (600.0, 280.0)] {
        let gamma = tab(&rig.window, "Gamma");
        drag_and_hold(&rig.window, &gamma, x, y);
        assert_eq!(preview(&rig.window), None, "at {x}, {y}");
        release(&rig.window, x, y);
        assert_eq!(rig.dock.layout(), start());
    }
    rig.check();
}

#[test]
fn a_tab_dropped_outside_the_window_stays() {
    let rig = rig(start());
    let beta = tab(&rig.window, "Beta");
    drag_and_hold(&rig.window, &beta, 1100.0, -40.0);
    assert_eq!(preview(&rig.window), None);
    release(&rig.window, 1100.0, -40.0);
    assert_eq!(rig.layout(), BETA_IN_FRONT);
    rig.check();
}

#[test]
fn one_drag_can_follow_another() {
    let rig = rig(start());
    drag(&tab(&rig.window, "Beta"), 950.0, 160.0);
    drag(&tab(&rig.window, "Beta"), 600.0, 450.0);
    drag(&tab(&rig.window, "Alpha"), 600.0, 450.0);
    drag(&tab(&rig.window, "Gamma"), 5.0, 300.0);
    assert_eq!(rig.layout(), "row(25: tabs(c*), 75: tabs(d, b, a*))");
    rig.check();
}

#[test]
fn a_bar_is_dragged_to_resize() {
    let rig = rig(start());
    drag(&splitters(&rig.window)[0], 502.0, 300.0);
    assert_eq!(shown(&rig.window, "Alpha"), Some(Rect::new(0.0, 30.0, 500.0, 570.0)));
    assert_eq!(shown(&rig.window, "Gamma"), Some(Rect::new(504.0, 30.0, 496.0, 268.0)));
    rig.check();

    drag(&splitters(&rig.window)[1], 700.0, 102.0);
    assert_eq!(shown(&rig.window, "Gamma"), Some(Rect::new(504.0, 30.0, 496.0, 70.0)));
    rig.check();
}

#[test]
fn a_bar_stops_at_the_least_size_of_a_group() {
    let rig = rig(start());
    drag(&splitters(&rig.window)[0], -200.0, 300.0);
    assert_eq!(shown(&rig.window, "Alpha"), Some(Rect::new(0.0, 30.0, 80.0, 570.0)));
    drag(&splitters(&rig.window)[0], 1500.0, 300.0);
    assert_eq!(shown(&rig.window, "Gamma"), Some(Rect::new(920.0, 30.0, 80.0, 268.0)));
    rig.check();
}

#[test]
fn the_button_on_a_tab_closes_its_panel() {
    let rig = rig(start());
    click(&close_button(&rig.window, "Alpha"));
    assert_eq!(rig.layout(), "row(25: tabs(b*), 75: col(50: tabs(c*), 50: tabs(d*)))");
    assert!(!has_tab(&rig.window, "Alpha"));
    assert_eq!(shown(&rig.window, "Alpha"), None);
    assert!(!rig.dock.is_open("a"));
    rig.check();
}

#[test]
fn the_middle_button_closes_a_panel() {
    let rig = rig(start());
    tab(&rig.window, "Delta").mock_single_click(PointerEventButton::Middle);
    assert_eq!(rig.layout(), "row(25: tabs(a*, b), 75: tabs(c*))");
    rig.check();
}

#[test]
fn a_panel_that_cannot_be_closed_has_no_button_and_ignores_the_middle_button() {
    let rig = rig(start());
    let buttons =
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(&rig.window, "Close Gamma");
    assert_eq!(buttons.count(), 0);
    tab(&rig.window, "Gamma").mock_single_click(PointerEventButton::Middle);
    assert_eq!(rig.dock.layout(), start());
    // The application can still close it.
    assert_eq!(rig.dock.close("c"), Ok(true));
    rig.check();
}

#[test]
fn a_panel_keeps_its_content_wherever_it_goes() {
    let rig = rig(start());
    click(&counter(&rig.window, "a"));
    click(&counter(&rig.window, "a"));
    click(&counter(&rig.window, "c"));
    assert_eq!(count(&rig.window, "a").as_deref(), Some("2"));

    // Moved to another group.
    drag(&tab(&rig.window, "Alpha"), 950.0, 450.0);
    assert_eq!(count(&rig.window, "a").as_deref(), Some("2"));

    // Behind another tab, and back.
    drag(&tab(&rig.window, "Gamma"), 950.0, 315.0);
    click(&tab(&rig.window, "Alpha"));
    assert_eq!(count(&rig.window, "c"), None);
    click(&tab(&rig.window, "Gamma"));
    assert_eq!(count(&rig.window, "c").as_deref(), Some("1"));

    // Closed, and opened again.
    click(&tab(&rig.window, "Alpha"));
    click(&close_button(&rig.window, "Alpha"));
    assert_eq!(count(&rig.window, "a"), None);
    rig.dock.open("a").expect("the panel is known");
    assert_eq!(count(&rig.window, "a").as_deref(), Some("2"));

    // And it still takes input where it now is.
    click(&counter(&rig.window, "a"));
    assert_eq!(count(&rig.window, "a").as_deref(), Some("3"));
    rig.check();
}

#[test]
fn a_panel_behind_another_takes_no_input() {
    let rig = rig(start());
    // Alpha and Beta share an area, and Alpha is in front.
    click_at(&rig.window, 120.0, 300.0);
    click(&tab(&rig.window, "Beta"));
    assert_eq!(count(&rig.window, "b").as_deref(), Some("0"));
    click_at(&rig.window, 120.0, 300.0);
    assert_eq!(count(&rig.window, "b").as_deref(), Some("1"));
    click(&tab(&rig.window, "Alpha"));
    assert_eq!(count(&rig.window, "a").as_deref(), Some("1"));
}

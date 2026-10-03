//! What the application does through the `Dock`.

use std::cell::RefCell;
use std::rc::Rc;

use slint_docking::{Layout, Node, Op, OpError, Rect, Target, Zone};
use slint_docking_tests::*;

#[test]
fn an_operation_from_the_application_is_shown() {
    let rig = rig(start());
    let group = rig.dock.layout().find_panel("d").expect("placed").group;
    let op = Op::Place { panel: "a".into(), target: Target::Group { group, zone: Zone::Left } };
    assert_eq!(rig.dock.apply(op), Ok(true));
    assert_eq!(rig.layout(), "row(25: tabs(b*), 75: col(50: tabs(c*), 50: row(50: tabs(a*), 50: tabs(d*))))");
    rig.check();
}

#[test]
fn a_refused_operation_changes_nothing() {
    let rig = rig(start());
    let group = rig.dock.layout().find_panel("d").expect("placed").group;
    let op = Op::Place { panel: "zzz".into(), target: Target::Group { group, zone: Zone::Left } };
    assert_eq!(rig.dock.apply(op), Err(OpError::UnknownPanel("zzz".into())));
    assert_eq!(rig.dock.open("zzz"), Err(OpError::UnknownPanel("zzz".into())));
    assert_eq!(rig.dock.close("zzz"), Err(OpError::UnknownPanel("zzz".into())));
    assert_eq!(rig.dock.layout(), start());
    rig.check();
}

#[test]
fn a_closed_panel_opens_where_it_was() {
    let rig = rig(start());
    rig.dock.close("b").expect("placed");
    assert_eq!(rig.dock.open("b"), Ok(true));
    assert_eq!(rig.layout(), "row(25: tabs(a, b*), 75: col(50: tabs(c*), 50: tabs(d*)))");

    // With nothing left of the group it was in, it joins the first group.
    rig.dock.close("d").expect("placed");
    assert_eq!(rig.dock.open("d"), Ok(true));
    assert_eq!(rig.layout(), "row(25: tabs(a, b, d*), 75: tabs(c*))");
    rig.check();
}

#[test]
fn opening_a_panel_that_is_open_brings_it_to_the_front() {
    let rig = rig(start());
    assert_eq!(rig.dock.open("b"), Ok(true));
    assert_eq!(rig.dock.open("b"), Ok(false));
    assert_eq!(shown(&rig.window, "Beta"), Some(Rect::new(0.0, 30.0, 249.0, 570.0)));
}

#[test]
fn every_panel_can_be_closed_and_opened_again() {
    let rig = rig(start());
    for key in ["a", "b", "c", "d"] {
        rig.dock.close(key).expect("placed");
        rig.check();
    }
    assert_eq!(rig.layout(), "empty");
    for key in ["d", "c", "b", "a"] {
        rig.dock.open(key).expect("known");
        rig.check();
    }
    assert_eq!(rig.layout(), "tabs(d, c, b, a*)");
}

#[test]
fn the_application_hears_of_every_change() {
    let rig = rig(start());
    let heard = Rc::new(RefCell::new(Vec::new()));
    rig.dock.on_layout_changed({
        let heard = heard.clone();
        move |layout| heard.borrow_mut().push(layout.to_string())
    });

    click(&tab(&rig.window, "Beta"));
    drag(&tab(&rig.window, "Beta"), 600.0, 160.0);
    rig.dock.close("d").expect("placed");
    // Nothing changes, so nothing is heard.
    click(&tab(&rig.window, "Beta"));
    let _ = rig.dock.close("zzz");

    assert_eq!(
        *heard.borrow(),
        [
            "row(25: tabs(a, b*), 75: col(50: tabs(c*), 50: tabs(d*)))",
            "row(25: tabs(a*), 75: col(50: tabs(c, b*), 50: tabs(d*)))",
            "row(25: tabs(a*), 75: tabs(c, b*))",
        ]
    );
}

#[test]
fn a_layout_is_saved_and_read_back() {
    let rig = rig(start());
    drag(&tab(&rig.window, "Beta"), 500.0, 595.0);
    drag(&splitters(&rig.window)[0], 500.0, 302.0);
    let saved = serde_json::to_string(&rig.dock.layout()).expect("saved");
    let before = rig.dock.layout();

    rig.dock.set_layout(start());
    assert_eq!(rig.dock.layout(), start());
    rig.check();

    rig.dock.set_layout(serde_json::from_str(&saved).expect("read"));
    assert_eq!(rig.dock.layout(), before);
    rig.check();
}

#[test]
fn a_layout_that_is_set_loses_the_panels_that_are_not_known() {
    let rig = rig(start());
    rig.dock.set_layout(Layout::new(Node::row([(1.0, Node::tabs(["gone", "d"])), (1.0, Node::tabs(["b"]))])));
    assert_eq!(rig.layout(), "row(50: tabs(d*), 50: tabs(b*))");
    rig.check();
}

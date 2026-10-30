//! Where a closed panel is remembered to have been, and how it returns there.

use slint_docking_layout::{GroupId, Layout, Node, Op, OpError, Place, Target, Zone};

fn group_of(layout: &Layout, panel: &str) -> GroupId {
    layout.find_panel(panel).expect("the panel is placed").group
}

fn apply(layout: &mut Layout, op: Op) {
    layout.apply(op).expect("accepted");
    layout.check().expect("normal form");
}

fn close(layout: &mut Layout, panel: &str) {
    apply(layout, Op::Close { panel: panel.into() });
}

fn back(layout: &mut Layout, panel: &str, place: Place) {
    apply(layout, Op::Return { panel: panel.into(), place });
}

fn names(panels: &[&str]) -> Vec<String> {
    panels.iter().map(|p| p.to_string()).collect()
}

/// Closes `panel` and returns it to the place it was closed from. Gives the place.
fn close_and_return(layout: &mut Layout, panel: &str) -> Place {
    let place = layout.place_of(panel).expect("a place");
    close(layout, panel);
    back(layout, panel, place.clone());
    place
}

#[test]
fn a_tab_returns_to_its_index() {
    let mut layout = Layout::new(Node::tabs(["a", "p", "b"]));
    apply(&mut layout, Op::Activate { panel: "p".into() });
    assert_eq!(layout.place_of("p"), Some(Place::Tabs { panels: names(&["a", "b"]), index: 1 }));
    close_and_return(&mut layout, "p");
    assert_eq!(layout.to_string(), "tabs(a, p*, b)");
}

#[test]
fn a_tab_returns_after_the_last_where_its_index_is_gone() {
    let mut layout = Layout::new(Node::tabs(["a", "b", "p"]));
    let place = layout.place_of("p").expect("a place");
    close(&mut layout, "p");
    close(&mut layout, "b");
    back(&mut layout, "p", place);
    assert_eq!(layout.to_string(), "tabs(a, p*)");
}

#[test]
fn a_quarter_returns_as_a_quarter() {
    let mut layout = Layout::new(Node::row([(75.0, Node::tabs(["chart"])), (25.0, Node::tabs(["p"]))]));
    let place = layout.place_of("p").expect("a place");
    assert_eq!(place, Place::Beside { panels: names(&["chart"]), zone: Zone::Right, share: 0.25 });
    close(&mut layout, "p");
    assert_eq!(layout.to_string(), "tabs(chart*)");
    back(&mut layout, "p", place);
    assert_eq!(layout.to_string(), "row(75: tabs(chart*), 25: tabs(p*))");
}

#[test]
fn a_resized_share_returns() {
    let mut layout = Layout::new(Node::row([(60.0, Node::tabs(["chart"])), (40.0, Node::tabs(["p"]))]));
    let place = close_and_return(&mut layout, "p");
    assert_eq!(place, Place::Beside { panels: names(&["chart"]), zone: Zone::Right, share: 0.4 });
    assert_eq!(layout.to_string(), "row(60: tabs(chart*), 40: tabs(p*))");
}

#[test]
fn a_middle_part_returns_between() {
    let mut layout = Layout::new(Node::row([
        (30.0, Node::tabs(["a"])),
        (20.0, Node::tabs(["p"])),
        (50.0, Node::tabs(["b"])),
    ]));
    close_and_return(&mut layout, "p");
    assert_eq!(layout.to_string(), "row(30: tabs(a*), 20: tabs(p*), 50: tabs(b*))");
}

#[test]
fn a_leading_part_returns_first() {
    let mut layout = Layout::new(Node::row([(25.0, Node::tabs(["p"])), (75.0, Node::tabs(["a"]))]));
    let place = close_and_return(&mut layout, "p");
    assert_eq!(place, Place::Beside { panels: names(&["a"]), zone: Zone::Left, share: 0.25 });
    assert_eq!(layout.to_string(), "row(25: tabs(p*), 75: tabs(a*))");
}

#[test]
fn a_part_of_a_column_returns_below() {
    let mut layout = Layout::new(Node::row([
        (50.0, Node::tabs(["a"])),
        (50.0, Node::column([(50.0, Node::tabs(["b"])), (50.0, Node::tabs(["p"]))])),
    ]));
    let place = layout.place_of("p").expect("a place");
    assert_eq!(place, Place::Beside { panels: names(&["b"]), zone: Zone::Bottom, share: 0.5 });
    close(&mut layout, "p");
    assert_eq!(layout.to_string(), "row(50: tabs(a*), 50: tabs(b*))");
    back(&mut layout, "p", place);
    assert_eq!(layout.to_string(), "row(50: tabs(a*), 50: col(50: tabs(b*), 50: tabs(p*)))");
}

#[test]
fn it_returns_beside_the_area_that_still_holds_its_panels() {
    let mut layout = Layout::new(Node::row([
        (20.0, Node::tabs(["a"])),
        (50.0, Node::column([(50.0, Node::tabs(["b"])), (50.0, Node::tabs(["c"]))])),
        (30.0, Node::tabs(["p"])),
    ]));
    let place = layout.place_of("p").expect("a place");
    assert_eq!(place, Place::Beside { panels: names(&["b", "c"]), zone: Zone::Right, share: 0.3 });
    close(&mut layout, "p");
    close(&mut layout, "c");
    back(&mut layout, "p", place);
    assert_eq!(layout.to_string(), "row(20: tabs(a*), 50: tabs(b*), 30: tabs(p*))");
}

#[test]
fn it_returns_beside_a_run_of_a_split() {
    // Closing p leaves the row of b and c inside the row of a, so they become a run of
    // three parts. The area beside which p returns is that run, not the whole row.
    let start = || {
        Layout::new(Node::row([
            (50.0, Node::tabs(["a"])),
            (
                50.0,
                Node::column([
                    (50.0, Node::row([(50.0, Node::tabs(["b"])), (50.0, Node::tabs(["c"]))])),
                    (50.0, Node::tabs(["p"])),
                ]),
            ),
        ]))
    };
    let mut layout = start();
    close(&mut layout, "p");
    assert_eq!(layout.to_string(), "row(50: tabs(a*), 25: tabs(b*), 25: tabs(c*))");
    let mut layout = start();
    close_and_return(&mut layout, "p");
    assert_eq!(layout.to_string(), start().to_string());
}

#[test]
fn it_returns_into_the_split_along_its_axis() {
    let mut layout = Layout::new(Node::column([
        (50.0, Node::row([(80.0, Node::tabs(["a"])), (20.0, Node::tabs(["p"]))])),
        (50.0, Node::row([(50.0, Node::tabs(["b"])), (50.0, Node::tabs(["c"]))])),
    ]));
    let place = layout.place_of("p").expect("a place");
    close(&mut layout, "p");
    let c = group_of(&layout, "c");
    apply(
        &mut layout,
        Op::Place { panel: "a".into(), target: Target::Group { group: c, zone: Zone::Right } },
    );
    assert_eq!(layout.to_string(), "row(50: tabs(b*), 25: tabs(c*), 25: tabs(a*))");
    back(&mut layout, "p", place);
    assert_eq!(layout.to_string(), "row(40: tabs(b*), 20: tabs(c*), 20: tabs(a*), 20: tabs(p*))");
}

#[test]
fn a_panel_alone_in_its_host_has_no_place() {
    let layout = Layout::with_hosts(vec![Some(Node::tabs(["p"])), Some(Node::tabs(["a"]))]);
    assert_eq!(layout.place_of("p"), None);
    assert_eq!(layout.place_of("zzz"), None);
}

#[test]
fn nothing_to_return_to_is_refused() {
    for start in [
        Layout::new(Node::row([(50.0, Node::tabs(["a"])), (50.0, Node::tabs(["p"]))])),
        Layout::new(Node::tabs(["a", "p"])),
    ] {
        let mut layout = start;
        let place = layout.place_of("p").expect("a place");
        assert!(layout.can_return(&place));
        close(&mut layout, "p");
        close(&mut layout, "a");
        assert!(!layout.can_return(&place));
        let before = layout.clone();
        assert_eq!(
            layout.apply(Op::Return { panel: "p".into(), place }),
            Err(OpError::NothingToReturnTo("p".into()))
        );
        assert_eq!(layout, before);
    }
}

#[test]
fn a_placed_panel_moves_to_its_place() {
    let mut layout = Layout::new(Node::row([(75.0, Node::tabs(["chart"])), (25.0, Node::tabs(["p"]))]));
    let place = layout.place_of("p").expect("a place");
    let chart = group_of(&layout, "chart");
    apply(
        &mut layout,
        Op::Place { panel: "p".into(), target: Target::Group { group: chart, zone: Zone::Top } },
    );
    assert_eq!(layout.to_string(), "col(50: tabs(p*), 50: tabs(chart*))");
    back(&mut layout, "p", place);
    assert_eq!(layout.to_string(), "row(75: tabs(chart*), 25: tabs(p*))");
}

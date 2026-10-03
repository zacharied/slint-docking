//! What each operation does to a layout.

use slint_docking_layout::{Layout, Node, Op, OpError, SplitId, Target, Zone};

fn two_groups() -> Layout {
    Layout::new(Node::row([(1.0, Node::tabs(["a", "b"])), (1.0, Node::tabs(["c"]))]))
}

fn group_of(layout: &Layout, panel: &str) -> slint_docking_layout::GroupId {
    layout.find_panel(panel).expect("the panel is placed").group
}

fn place(layout: &mut Layout, panel: &str, target: Target) -> bool {
    let changed = layout.apply(Op::Place { panel: panel.into(), target }).expect("accepted");
    layout.check().expect("normal form");
    changed
}

#[test]
fn a_new_layout_is_in_normal_form() {
    let layout = Layout::new(Node::row([
        (1.0, Node::tabs(["a"])),
        (1.0, Node::row([(1.0, Node::tabs(["b"])), (3.0, Node::tabs(Vec::<String>::new()))])),
        (2.0, Node::column([(1.0, Node::tabs(["c", "a"]))])),
    ]));
    layout.check().expect("normal form");
    // The empty group is gone, the row in a row has moved up, the column of one child is
    // that child, and the second "a" is dropped.
    assert_eq!(layout.to_string(), "row(25: tabs(a*), 25: tabs(b*), 50: tabs(c*))");
}

#[test]
fn the_centre_of_a_group_adds_a_tab() {
    let mut layout = two_groups();
    let target = Target::Group { group: group_of(&layout, "c"), zone: Zone::Center };
    assert!(place(&mut layout, "a", target));
    assert_eq!(layout.to_string(), "row(50: tabs(b*), 50: tabs(c, a*))");
}

#[test]
fn a_side_of_a_group_splits_it() {
    let mut layout = two_groups();
    let target = Target::Group { group: group_of(&layout, "c"), zone: Zone::Top };
    assert!(place(&mut layout, "a", target));
    assert_eq!(layout.to_string(), "row(50: tabs(b*), 50: col(50: tabs(a*), 50: tabs(c*)))");
}

#[test]
fn a_side_along_the_parent_adds_a_sibling() {
    let mut layout = two_groups();
    let target = Target::Group { group: group_of(&layout, "c"), zone: Zone::Left };
    assert!(place(&mut layout, "a", target));
    assert_eq!(layout.to_string(), "row(50: tabs(b*), 25: tabs(a*), 25: tabs(c*))");
}

#[test]
fn a_panel_splits_off_from_its_own_group() {
    let mut layout = two_groups();
    let target = Target::Group { group: group_of(&layout, "a"), zone: Zone::Right };
    assert!(place(&mut layout, "a", target));
    assert_eq!(layout.to_string(), "row(25: tabs(b*), 25: tabs(a*), 50: tabs(c*))");
}

#[test]
fn a_panel_alone_in_its_group_stays_when_dropped_on_it() {
    let mut layout = two_groups();
    for zone in [Zone::Center, Zone::Left, Zone::Right, Zone::Top, Zone::Bottom] {
        let target = Target::Group { group: group_of(&layout, "c"), zone };
        assert!(!place(&mut layout, "c", target), "{zone:?}");
    }
    assert_eq!(layout, two_groups());
}

#[test]
fn the_last_panel_leaving_removes_the_group() {
    let mut layout = two_groups();
    let target = Target::Group { group: group_of(&layout, "a"), zone: Zone::Center };
    assert!(place(&mut layout, "c", target));
    assert_eq!(layout.to_string(), "tabs(a, b, c*)");
}

#[test]
fn tabs_are_reordered_within_a_group() {
    let mut layout = Layout::new(Node::tabs(["a", "b", "c"]));
    let group = group_of(&layout, "a");

    assert!(place(&mut layout, "a", Target::Tab { group, index: 3 }));
    assert_eq!(layout.to_string(), "tabs(b, c, a*)");

    assert!(place(&mut layout, "a", Target::Tab { group, index: 0 }));
    assert_eq!(layout.to_string(), "tabs(a*, b, c)");

    assert!(place(&mut layout, "c", Target::Tab { group, index: 1 }));
    assert_eq!(layout.to_string(), "tabs(a, c*, b)");

    // Either side of its own tab is where it already is.
    assert!(!place(&mut layout, "c", Target::Tab { group, index: 1 }));
    assert!(!place(&mut layout, "c", Target::Tab { group, index: 2 }));
}

#[test]
fn a_tab_is_inserted_into_another_group() {
    let mut layout = two_groups();
    let group = group_of(&layout, "a");
    assert!(place(&mut layout, "c", Target::Tab { group, index: 1 }));
    assert_eq!(layout.to_string(), "tabs(a, c*, b)");
}

#[test]
fn an_edge_of_the_host_goes_beside_everything() {
    let mut layout = two_groups();
    assert!(place(&mut layout, "a", Target::Edge { host: 0, zone: Zone::Bottom }));
    assert_eq!(layout.to_string(), "col(75: row(50: tabs(b*), 50: tabs(c*)), 25: tabs(a*))");

    assert!(place(&mut layout, "b", Target::Edge { host: 0, zone: Zone::Top }));
    assert_eq!(layout.to_string(), "col(33: tabs(b*), 50: tabs(c*), 17: tabs(a*))");
}

#[test]
fn a_panel_that_is_not_placed_opens() {
    let mut layout = two_groups();
    let target = Target::Group { group: group_of(&layout, "c"), zone: Zone::Center };
    assert!(place(&mut layout, "new", target));
    assert_eq!(layout.to_string(), "row(50: tabs(a*, b), 50: tabs(c, new*))");
}

#[test]
fn an_empty_host_takes_a_panel_whole() {
    let mut layout = Layout::empty();
    assert!(place(&mut layout, "a", Target::Edge { host: 0, zone: Zone::Center }));
    assert_eq!(layout.to_string(), "tabs(a*)");
    assert_eq!(
        layout.apply(Op::Place { panel: "b".into(), target: Target::Edge { host: 0, zone: Zone::Center } }),
        Err(OpError::HostNotEmpty(0))
    );
}

#[test]
fn a_panel_moves_between_hosts() {
    let mut layout = two_groups();
    layout.ensure_host(1);
    assert!(place(&mut layout, "c", Target::Edge { host: 1, zone: Zone::Center }));
    assert_eq!(layout.to_string(), "tabs(a*, b)\ntabs(c*)");
    assert_eq!(layout.find_panel("c").map(|l| l.host), Some(1));

    let target = Target::Group { group: group_of(&layout, "a"), zone: Zone::Right };
    assert!(place(&mut layout, "c", target));
    assert_eq!(layout.to_string(), "row(50: tabs(a*, b), 50: tabs(c*))\nempty");
}

#[test]
fn closing_shows_the_tab_that_takes_its_place() {
    let mut layout = Layout::new(Node::tabs(["a", "b", "c"]));
    layout.apply(Op::Activate { panel: "b".into() }).expect("accepted");
    layout.apply(Op::Close { panel: "b".into() }).expect("accepted");
    assert_eq!(layout.to_string(), "tabs(a, c*)");
    layout.apply(Op::Close { panel: "c".into() }).expect("accepted");
    assert_eq!(layout.to_string(), "tabs(a*)");
    layout.apply(Op::Close { panel: "a".into() }).expect("accepted");
    assert_eq!(layout.to_string(), "empty");
}

#[test]
fn closing_a_tab_behind_keeps_the_shown_panel() {
    let mut layout = Layout::new(Node::tabs(["a", "b", "c"]));
    layout.apply(Op::Activate { panel: "c".into() }).expect("accepted");
    layout.apply(Op::Close { panel: "a".into() }).expect("accepted");
    assert_eq!(layout.to_string(), "tabs(b, c*)");
}

#[test]
fn weights_are_set_and_brought_to_a_sum_of_one() {
    let mut layout = two_groups();
    let split = layout.splits(0)[0].id();
    assert_eq!(layout.apply(Op::SetWeights { split, weights: vec![1.0, 3.0] }), Ok(true));
    assert_eq!(layout.to_string(), "row(25: tabs(a*, b), 75: tabs(c*))");
}

#[test]
fn a_refused_operation_changes_nothing() {
    let mut layout = two_groups();
    let split = layout.splits(0)[0].id();
    let group = group_of(&layout, "a");
    let refused = [
        (Op::Activate { panel: "x".into() }, OpError::UnknownPanel("x".into())),
        (Op::Close { panel: "x".into() }, OpError::UnknownPanel("x".into())),
        (Op::SetWeights { split, weights: vec![1.0] }, OpError::BadWeights),
        (Op::SetWeights { split, weights: vec![1.0, 0.0] }, OpError::BadWeights),
        (Op::SetWeights { split, weights: vec![1.0, f32::NAN] }, OpError::BadWeights),
        (
            Op::SetWeights { split: SplitId(999), weights: vec![1.0, 1.0] },
            OpError::UnknownSplit(SplitId(999)),
        ),
        (
            Op::Place { panel: "a".into(), target: Target::Edge { host: 7, zone: Zone::Left } },
            OpError::UnknownHost(7),
        ),
        (
            Op::Place {
                panel: "a".into(),
                target: Target::Group { group: slint_docking_layout::GroupId(999), zone: Zone::Left },
            },
            OpError::UnknownGroup(slint_docking_layout::GroupId(999)),
        ),
    ];
    for (op, error) in refused {
        assert_eq!(layout.apply(op.clone()), Err(error), "{op:?}");
        assert_eq!(layout, two_groups());
        assert_eq!(group_of(&layout, "a"), group);
    }
}

#[test]
fn groups_keep_their_ids_when_the_tree_around_them_changes() {
    let mut layout = two_groups();
    let ab = group_of(&layout, "a");
    let c = group_of(&layout, "c");
    place(&mut layout, "b", Target::Edge { host: 0, zone: Zone::Bottom });
    place(&mut layout, "b", Target::Group { group: c, zone: Zone::Center });
    assert_eq!(layout.to_string(), "row(50: tabs(a*), 50: tabs(c, b*))");
    assert_eq!(group_of(&layout, "a"), ab);
    assert_eq!(group_of(&layout, "c"), c);
}

#[cfg(feature = "serde")]
#[test]
fn a_layout_is_saved_and_read_back() {
    let layout = Layout::with_hosts(vec![
        Some(Node::row([
            (1.0, Node::tabs(["a", "b"])),
            (3.0, Node::column([(1.0, Node::tabs(["c"])), (1.0, Node::tabs(["d"]))])),
        ])),
        None,
    ]);
    let saved = serde_json::to_string(&layout).expect("saved");
    assert!(!saved.contains("id"), "IDs are not saved: {saved}");
    let read: Layout = serde_json::from_str(&saved).expect("read");
    read.check().expect("normal form");
    assert_eq!(read, layout);
}

#[cfg(feature = "serde")]
#[test]
fn a_saved_layout_that_is_not_in_normal_form_is_normalised() {
    let saved = r#"{"hosts":[{"split":{"axis":"horizontal","weights":[0,-1],"children":[
        {"group":{"panels":["a","a"],"active":9}},
        {"split":{"axis":"horizontal","weights":[1,1],"children":[
            {"group":{"panels":[]}},
            {"group":{"panels":["b"]}}]}}]}}]}"#;
    let read: Layout = serde_json::from_str(saved).expect("read");
    read.check().expect("normal form");
    assert_eq!(read.to_string(), "row(50: tabs(a*), 50: tabs(b*))");
}

//! Rectangles, splitter drags and drop targets.

use slint_docking_layout::{Layout, Metrics, Node, Op, Point, Rect, Size, Target, Zone};

const SIZE: Size = Size { width: 1000.0, height: 600.0 };

fn metrics() -> Metrics {
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

fn three() -> Layout {
    Layout::new(Node::row([
        (1.0, Node::tabs(["a", "b"])),
        (3.0, Node::column([(1.0, Node::tabs(["c"])), (1.0, Node::tabs(["d"]))])),
    ]))
}

#[test]
fn a_single_group_fills_the_host() {
    let layout = Layout::new(Node::tabs(["a", "b"]));
    let solved = layout.solve(0, SIZE, &metrics());
    assert_eq!(solved.groups.len(), 1);
    assert_eq!(solved.groups[0].tab_bar, Rect::new(0.0, 0.0, 1000.0, 30.0));
    assert_eq!(solved.groups[0].content, Rect::new(0.0, 30.0, 1000.0, 570.0));
    assert_eq!(solved.panel("a"), Some((Rect::new(0.0, 30.0, 1000.0, 570.0), true)));
    assert_eq!(solved.panel("b"), Some((Rect::new(0.0, 30.0, 1000.0, 570.0), false)));
    assert_eq!(solved.panel("x"), None);
    assert!(solved.splitters.is_empty());
}

#[test]
fn splits_share_the_room_left_by_the_splitters() {
    let solved = three().solve(0, SIZE, &metrics());
    // 996 is left across: 249 and 747. 596 is left down: 298 and 298.
    assert_eq!(solved.group_of("a").map(|g| g.0.rect), Some(Rect::new(0.0, 0.0, 249.0, 600.0)));
    assert_eq!(solved.group_of("c").map(|g| g.0.rect), Some(Rect::new(253.0, 0.0, 747.0, 298.0)));
    assert_eq!(solved.group_of("d").map(|g| g.0.rect), Some(Rect::new(253.0, 302.0, 747.0, 298.0)));
    let bars: Vec<Rect> = solved.splitters.iter().map(|s| s.rect).collect();
    assert_eq!(bars, vec![Rect::new(249.0, 0.0, 4.0, 600.0), Rect::new(253.0, 298.0, 747.0, 4.0)]);
}

#[test]
fn tabs_shrink_when_there_is_no_room() {
    let layout = Layout::new(Node::tabs(["a", "b", "c", "d"]));
    let solved = layout.solve(0, Size::new(200.0, 100.0), &metrics());
    assert_eq!(solved.groups[0].tab_width, 50.0);
    assert_eq!(solved.groups[0].tab_rect(2), Rect::new(100.0, 0.0, 50.0, 30.0));
}

#[test]
fn an_empty_host_has_no_rectangles() {
    let solved = Layout::empty().solve(0, SIZE, &metrics());
    assert!(solved.groups.is_empty());
    let solved = three().solve(5, SIZE, &metrics());
    assert!(solved.groups.is_empty());
}

#[test]
fn a_host_of_no_size_is_solved_without_trouble() {
    let solved = three().solve(0, Size::new(0.0, 0.0), &metrics());
    assert_eq!(solved.groups.len(), 3);
    for group in &solved.groups {
        assert!(group.rect.width >= 0.0 && group.rect.height >= 0.0, "{group:?}");
        assert!(group.content.height >= 0.0, "{group:?}");
    }
}

#[test]
fn dragging_a_splitter_moves_the_edge_to_the_pointer() {
    let mut layout = three();
    let m = metrics();
    let solved = layout.solve(0, SIZE, &m);
    let split = solved.splitters[0].split;
    let weights = layout.weights_for_drag(&solved, split, 0, 502.0, &m).expect("room to move");
    layout.apply(Op::SetWeights { split, weights }).expect("accepted");
    let solved = layout.solve(0, SIZE, &m);
    assert_eq!(solved.splitters[0].rect, Rect::new(500.0, 0.0, 4.0, 600.0));
}

#[test]
fn a_splitter_stops_at_the_least_sizes() {
    let mut layout = three();
    let m = metrics();
    let solved = layout.solve(0, SIZE, &m);
    let across = solved.splitters[0].split;
    let down = solved.splitters[1].split;

    let weights = layout.weights_for_drag(&solved, across, 0, -50.0, &m).expect("room to move");
    layout.apply(Op::SetWeights { split: across, weights }).expect("accepted");
    assert_eq!(layout.solve(0, SIZE, &m).group_of("a").map(|g| g.0.rect.width), Some(80.0));

    let solved = layout.solve(0, SIZE, &m);
    let weights = layout.weights_for_drag(&solved, across, 0, 5000.0, &m).expect("room to move");
    layout.apply(Op::SetWeights { split: across, weights }).expect("accepted");
    assert_eq!(layout.solve(0, SIZE, &m).group_of("c").map(|g| g.0.rect.width), Some(80.0));

    // Down, the least is the tabs and the least height of a panel.
    let solved = layout.solve(0, SIZE, &m);
    let weights = layout.weights_for_drag(&solved, down, 0, 0.0, &m).expect("room to move");
    layout.apply(Op::SetWeights { split: down, weights }).expect("accepted");
    assert_eq!(layout.solve(0, SIZE, &m).group_of("c").map(|g| g.0.rect.height), Some(90.0));
}

#[test]
fn a_splitter_with_no_room_does_not_move() {
    let layout = three();
    let m = metrics();
    let solved = layout.solve(0, Size::new(100.0, 100.0), &m);
    let split = solved.splitters[0].split;
    assert_eq!(layout.weights_for_drag(&solved, split, 0, 50.0, &m), None);
    assert_eq!(layout.weights_for_drag(&solved, split, 9, 50.0, &m), None);
}

fn target_at(layout: &Layout, x: f32, y: f32) -> Option<(Target, Rect)> {
    let m = metrics();
    let solved = layout.solve(0, SIZE, &m);
    layout.target_at(0, &solved, Point::new(x, y), &m).map(|hit| (hit.target, hit.preview))
}

#[test]
fn a_point_outside_the_host_has_no_target() {
    let layout = three();
    assert_eq!(target_at(&layout, -1.0, 10.0), None);
    assert_eq!(target_at(&layout, 1000.0, 10.0), None);
    assert_eq!(target_at(&layout, 10.0, 600.0), None);
}

#[test]
fn an_empty_host_is_one_target() {
    let layout = Layout::empty();
    assert_eq!(
        target_at(&layout, 500.0, 300.0),
        Some((Target::Edge { host: 0, zone: Zone::Center }, Rect::new(0.0, 0.0, 1000.0, 600.0)))
    );
}

#[test]
fn the_zones_of_a_group() {
    let layout = three();
    let c = layout.find_panel("c").expect("placed").group;
    // The content of c is x 253 to 1000, y 30 to 298.
    let whole = Rect::new(253.0, 0.0, 747.0, 298.0);
    let cases = [
        (600.0, 160.0, Zone::Center, whole),
        (300.0, 160.0, Zone::Left, Rect::new(253.0, 0.0, 373.5, 298.0)),
        (950.0, 160.0, Zone::Right, Rect::new(626.5, 0.0, 373.5, 298.0)),
        (600.0, 50.0, Zone::Top, Rect::new(253.0, 0.0, 747.0, 149.0)),
        (600.0, 280.0, Zone::Bottom, Rect::new(253.0, 149.0, 747.0, 149.0)),
    ];
    for (x, y, zone, preview) in cases {
        assert_eq!(target_at(&layout, x, y), Some((Target::Group { group: c, zone }, preview)), "{zone:?}");
    }
}

#[test]
fn the_tabs_of_a_group_are_places_among_the_tabs() {
    let layout = three();
    let ab = layout.find_panel("a").expect("placed").group;
    // The tabs of the group are 100 wide: a is 0 to 100, b is 100 to 200. The strip near
    // the edge of the host comes first, so stay clear of the top 12.
    let cases = [(20.0, 0), (49.0, 0), (51.0, 1), (149.0, 1), (151.0, 2), (240.0, 2)];
    for (x, index) in cases {
        let (target, _) = target_at(&layout, x, 20.0).expect("inside");
        assert_eq!(target, Target::Tab { group: ab, index }, "x = {x}");
    }
}

#[test]
fn the_edges_of_a_host_come_before_the_groups() {
    let layout = three();
    let cases = [
        (5.0, 300.0, Zone::Left, Rect::new(0.0, 0.0, 250.0, 600.0)),
        (995.0, 300.0, Zone::Right, Rect::new(750.0, 0.0, 250.0, 600.0)),
        (500.0, 5.0, Zone::Top, Rect::new(0.0, 0.0, 1000.0, 150.0)),
        (500.0, 595.0, Zone::Bottom, Rect::new(0.0, 450.0, 1000.0, 150.0)),
    ];
    for (x, y, zone, preview) in cases {
        assert_eq!(target_at(&layout, x, y), Some((Target::Edge { host: 0, zone }, preview)), "{zone:?}");
    }
}

#[test]
fn a_host_of_one_group_has_no_edge_targets() {
    let layout = Layout::new(Node::tabs(["a", "b"]));
    let group = layout.find_panel("a").expect("placed").group;
    let (target, _) = target_at(&layout, 5.0, 300.0).expect("inside");
    assert_eq!(target, Target::Group { group, zone: Zone::Left });
}

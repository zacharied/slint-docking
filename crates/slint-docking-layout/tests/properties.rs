//! Properties that hold for any sequence of operations.

use proptest::prelude::*;
use slint_docking_layout::{Layout, Metrics, Node, Op, Point, Rect, Size, Target, Zone};

const PANELS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

/// One step, in terms that make sense whatever the layout is when it is taken.
#[derive(Clone, Debug)]
enum Step {
    /// Drop a panel at a point, as a drag would.
    Drop {
        panel: usize,
        x: f32,
        y: f32,
    },
    /// Place a panel in the nth group.
    Group {
        panel: usize,
        group: usize,
        zone: Zone,
    },
    /// Place a panel among the tabs of the nth group.
    Tab {
        panel: usize,
        group: usize,
        index: usize,
    },
    /// Place a panel on an edge of a host.
    Edge {
        panel: usize,
        host: usize,
        zone: Zone,
    },
    Activate {
        panel: usize,
    },
    Close {
        panel: usize,
    },
    /// Drag the nth splitter to a position.
    Drag {
        splitter: usize,
        position: f32,
    },
}

fn zone() -> impl Strategy<Value = Zone> {
    prop_oneof![Just(Zone::Center), Just(Zone::Left), Just(Zone::Right), Just(Zone::Top), Just(Zone::Bottom),]
}

fn step() -> impl Strategy<Value = Step> {
    let panel = 0..PANELS.len();
    prop_oneof![
        3 => (panel.clone(), 0.0..1.0f32, 0.0..1.0f32).prop_map(|(panel, x, y)| Step::Drop { panel, x, y }),
        2 => (panel.clone(), 0..8usize, zone()).prop_map(|(panel, group, zone)| Step::Group { panel, group, zone }),
        2 => (panel.clone(), 0..8usize, 0..8usize).prop_map(|(panel, group, index)| Step::Tab { panel, group, index }),
        1 => (panel.clone(), 0..2usize, zone()).prop_map(|(panel, host, zone)| Step::Edge { panel, host, zone }),
        1 => panel.clone().prop_map(|panel| Step::Activate { panel }),
        1 => panel.prop_map(|panel| Step::Close { panel }),
        2 => (0..8usize, -0.2..1.2f32).prop_map(|(splitter, position)| Step::Drag { splitter, position }),
    ]
}

fn size() -> impl Strategy<Value = Size> {
    (0.0..2000.0f32, 0.0..1500.0f32).prop_map(|(w, h)| Size::new(w.round(), h.round()))
}

/// Whether two trees are the same: the same splits and groups, the same panels shown,
/// and weights within 1e-4, since closing and returning divide and multiply them.
fn same(a: &Node, b: &Node) -> bool {
    match (a, b) {
        (Node::Group(a), Node::Group(b)) => a.panels() == b.panels() && a.active_index() == b.active_index(),
        (Node::Split(a), Node::Split(b)) => {
            a.axis() == b.axis()
                && a.children().len() == b.children().len()
                && a.weights().iter().zip(b.weights()).all(|(a, b)| (a - b).abs() <= 1e-4)
                && a.children().iter().zip(b.children()).all(|(a, b)| same(a, b))
        }
        _ => false,
    }
}

fn same_layout(a: &Layout, b: &Layout) -> bool {
    a.host_count() == b.host_count()
        && (0..a.host_count()).all(|h| match (a.root(h), b.root(h)) {
            (None, None) => true,
            (Some(a), Some(b)) => same(a, b),
            _ => false,
        })
}

fn start() -> Layout {
    Layout::with_hosts(vec![
        Some(Node::row([
            (1.0, Node::tabs(["a", "b"])),
            (2.0, Node::column([(1.0, Node::tabs(["c"])), (1.0, Node::tabs(["d", "e"]))])),
        ])),
        None,
    ])
}

/// Takes a step. Returns the operation that was applied, if the step led to one.
fn take(layout: &mut Layout, step: &Step, size: Size, metrics: &Metrics) -> Option<Op> {
    let groups: Vec<_> = (0..layout.host_count()).flat_map(|h| layout.groups(h)).map(|g| g.id()).collect();
    let op = match step {
        Step::Drop { panel, x, y } => {
            let solved = layout.solve(0, size, metrics);
            let point = Point::new(x * size.width, y * size.height);
            let hit = layout.target_at(0, &solved, point, metrics)?;
            Op::Place { panel: PANELS[*panel].into(), target: hit.target }
        }
        Step::Group { panel, group, zone } => Op::Place {
            panel: PANELS[*panel].into(),
            target: Target::Group { group: *groups.get(group % groups.len().max(1))?, zone: *zone },
        },
        Step::Tab { panel, group, index } => Op::Place {
            panel: PANELS[*panel].into(),
            target: Target::Tab { group: *groups.get(group % groups.len().max(1))?, index: *index },
        },
        Step::Edge { panel, host, zone } => {
            Op::Place { panel: PANELS[*panel].into(), target: Target::Edge { host: *host, zone: *zone } }
        }
        Step::Activate { panel } => Op::Activate { panel: PANELS[*panel].into() },
        Step::Close { panel } => Op::Close { panel: PANELS[*panel].into() },
        Step::Drag { splitter, position } => {
            let solved = layout.solve(0, size, metrics);
            let bar = solved.splitters.get(splitter % solved.splitters.len().max(1))?;
            let (start, extent) = solved.split(bar.split)?.rect.span(bar.axis);
            let position = start + position * extent;
            let weights = layout.weights_for_drag(&solved, bar.split, bar.handle, position, metrics)?;
            Op::SetWeights { split: bar.split, weights }
        }
    };
    Some(op)
}

proptest! {
    /// Whatever is done, the layout stays in normal form, a refused operation changes
    /// nothing, and no panel is lost or made except by opening and closing.
    #[test]
    fn operations_keep_the_layout_in_normal_form(
        steps in prop::collection::vec(step(), 1..60),
        size in size(),
    ) {
        let metrics = Metrics::default();
        let mut layout = start();
        for step in &steps {
            let before = layout.clone();
            let mut panels: Vec<String> = layout.panels().into_iter().map(Into::into).collect();
            let Some(op) = take(&mut layout, step, size, &metrics) else { continue };
            match layout.apply(op.clone()) {
                Ok(changed) => {
                    prop_assert_eq!(changed, layout != before, "{:?}", op);
                    match &op {
                        Op::Place { panel, .. } if !panels.contains(panel) => panels.push(panel.clone()),
                        Op::Close { panel } => panels.retain(|p| p != panel),
                        _ => {}
                    }
                }
                Err(_) => prop_assert_eq!(&layout, &before, "{:?}", op),
            }
            prop_assert_eq!(layout.check(), Ok(()), "{:?} gave {}", op, layout);
            let mut now: Vec<String> = layout.panels().into_iter().map(Into::into).collect();
            now.sort();
            panels.sort();
            prop_assert_eq!(now, panels, "{:?}", op);
        }
    }

    /// The groups and splitters of a host cover it exactly once.
    #[test]
    fn rectangles_tile_the_host(
        steps in prop::collection::vec(step(), 0..40),
        size in size(),
    ) {
        let metrics = Metrics::default();
        let mut layout = start();
        for step in &steps {
            if let Some(op) = take(&mut layout, step, size, &metrics) {
                let _ = layout.apply(op);
            }
        }
        let solved = layout.solve(0, size, &metrics);
        let mut rects: Vec<Rect> = solved.groups.iter().map(|g| g.rect).collect();
        rects.extend(solved.splitters.iter().map(|s| s.rect));

        for rect in &rects {
            prop_assert!(rect.width >= 0.0 && rect.height >= 0.0, "{:?}", rect);
            prop_assert!(rect.x >= 0.0 && rect.y >= 0.0, "{:?}", rect);
            prop_assert!(rect.right() <= size.width && rect.bottom() <= size.height, "{:?}", rect);
            prop_assert_eq!(rect.x.fract(), 0.0);
            prop_assert_eq!(rect.y.fract(), 0.0);
        }
        for (i, a) in rects.iter().enumerate() {
            for b in &rects[i + 1..] {
                let overlap_w = a.right().min(b.right()) - a.x.max(b.x);
                let overlap_h = a.bottom().min(b.bottom()) - a.y.max(b.y);
                prop_assert!(overlap_w <= 0.0 || overlap_h <= 0.0, "{:?} and {:?} overlap", a, b);
            }
        }
        if layout.root(0).is_some() {
            let covered: f32 = rects.iter().map(Rect::area).sum();
            prop_assert_eq!(covered, size.width * size.height);
        }
        for group in &solved.groups {
            prop_assert_eq!(group.tab_bar.height + group.content.height, group.rect.height);
        }
    }

    /// Every point of a host has a target, and the operation it leads to is accepted.
    #[test]
    fn every_point_in_a_host_is_a_target(
        steps in prop::collection::vec(step(), 0..30),
        x in 0.0..1.0f32,
        y in 0.0..1.0f32,
        panel in 0..PANELS.len(),
    ) {
        let metrics = Metrics::default();
        let size = Size::new(1200.0, 800.0);
        let mut layout = start();
        for step in &steps {
            if let Some(op) = take(&mut layout, step, size, &metrics) {
                let _ = layout.apply(op);
            }
        }
        let solved = layout.solve(0, size, &metrics);
        let point = Point::new((x * size.width).floor(), (y * size.height).floor());
        let hit = layout.target_at(0, &solved, point, &metrics);
        // A point on a splitter belongs to no group, and is no target unless it is near
        // an edge of the host.
        let on_splitter = solved.splitters.iter().any(|s| s.rect.contains(point));
        if !on_splitter {
            prop_assert!(hit.is_some(), "no target at {:?} in {}", point, layout);
        }
        if let Some(hit) = hit {
            let op = Op::Place { panel: PANELS[panel].into(), target: hit.target };
            prop_assert!(layout.apply(op).is_ok());
            prop_assert_eq!(layout.check(), Ok(()));
        }
    }

    /// A panel closed and returned to the place it was closed from gives back the layout.
    #[test]
    fn closing_and_returning_gives_back_the_layout(
        steps in prop::collection::vec(step(), 0..40),
        size in size(),
    ) {
        let metrics = Metrics::default();
        let mut layout = start();
        for step in &steps {
            if let Some(op) = take(&mut layout, step, size, &metrics) {
                let _ = layout.apply(op);
            }
        }
        let panels: Vec<String> = layout.panels().into_iter().map(Into::into).collect();
        for panel in panels {
            let mut shown = layout.clone();
            shown.apply(Op::Activate { panel: panel.clone() }).expect("placed");
            let Some(place) = shown.place_of(&panel) else { continue };
            let mut back = shown.clone();
            back.apply(Op::Close { panel: panel.clone() }).expect("placed");
            prop_assert!(back.can_return(&place), "{:?} from {}", place, shown);
            back.apply(Op::Return { panel: panel.clone(), place: place.clone() }).expect("accepted");
            prop_assert_eq!(back.check(), Ok(()));
            prop_assert!(
                same_layout(&back, &shown),
                "{} closed and returned to {:?} from {} gave {}",
                panel,
                place,
                shown,
                back
            );
        }
    }
}

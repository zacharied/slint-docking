//! Taking a panel from one window to another.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{ComponentHandle, LogicalSize};
use slint_docking::{Dock, DragOutside, OpError, Point, Rect};
use slint_docking_tests::*;

/// Two windows on one dock: the main window, and one whose panels exist only while it
/// holds them.
fn two_windows() -> (Fixture, Floating, Dock) {
    let rig = rig(start());
    let floating = Floating::new().expect("the window is made");
    slint_docking::attach!(rig.dock, floating, host = 1);
    floating.show().expect("the window is shown");
    floating.window().set_size(LogicalSize::new(600.0, 400.0));
    (rig.window, floating, rig.dock)
}

fn check_both(main: &Fixture, floating: &Floating, dock: &Dock) {
    check_host(main, dock, 0, Rect::new(0.0, 0.0, 1000.0, 600.0));
    check_host(floating, dock, 1, Rect::new(0.0, 0.0, 600.0, 400.0));
}

#[test]
fn a_second_window_starts_empty() {
    let (main, floating, dock) = two_windows();
    assert_eq!(dock.layout().to_string(), format!("{}\nempty", start()));
    assert!(tabs(&floating).is_empty());
    check_both(&main, &floating, &dock);
}

#[test]
fn a_panel_moves_to_another_window_and_back() {
    let (main, floating, dock) = two_windows();

    assert_eq!(dock.move_to_host("a", 1), Ok(true));
    assert_eq!(dock.layout().to_string(), "row(25: tabs(b*), 75: col(50: tabs(c*), 50: tabs(d*)))\ntabs(a*)");
    assert_eq!(tabs(&floating), ["Alpha"]);
    assert_eq!(shown(&floating, "Alpha"), Some(Rect::new(0.0, 30.0, 600.0, 370.0)));
    assert_eq!(shown(&main, "Alpha"), None);
    check_both(&main, &floating, &dock);

    assert_eq!(dock.move_to_host("b", 1), Ok(true));
    assert_eq!(dock.move_to_host("b", 1), Ok(false));
    assert_eq!(dock.layout().to_string(), "col(50: tabs(c*), 50: tabs(d*))\nrow(75: tabs(a*), 25: tabs(b*))");
    check_both(&main, &floating, &dock);

    assert_eq!(dock.move_to_host("a", 0), Ok(true));
    assert_eq!(dock.move_to_host("b", 0), Ok(true));
    assert_eq!(
        dock.layout().to_string(),
        "row(50: col(50: tabs(c*), 50: tabs(d*)), 17: tabs(a*), 33: tabs(b*))\nempty"
    );
    assert!(tabs(&floating).is_empty());
    check_both(&main, &floating, &dock);
}

#[test]
fn tabs_are_dragged_within_a_second_window() {
    let (main, floating, dock) = two_windows();
    dock.move_to_host("a", 1).expect("known");
    dock.move_to_host("b", 1).expect("known");

    drag(&tab(&floating, "Beta"), 200.0, 200.0);
    assert_eq!(dock.layout().to_string(), "col(50: tabs(c*), 50: tabs(d*))\ntabs(a, b*)");
    drag(&tab(&floating, "Alpha"), 300.0, 380.0);
    assert_eq!(dock.layout().to_string(), "col(50: tabs(c*), 50: tabs(d*))\ncol(50: tabs(b*), 50: tabs(a*))");
    check_both(&main, &floating, &dock);
}

#[test]
fn content_made_on_demand_starts_again_in_a_new_window_and_is_kept_in_the_old() {
    let (main, floating, dock) = two_windows();
    click(&counter(&main, "a"));
    click(&counter(&main, "a"));

    dock.move_to_host("a", 1).expect("known");
    // The floating window makes its own content, which knows nothing of the other.
    assert_eq!(count(&floating, "a").as_deref(), Some("0"));
    click(&counter(&floating, "a"));
    assert_eq!(count(&floating, "a").as_deref(), Some("1"));

    // The main window declares its panels outright, so its content was only hidden.
    dock.move_to_host("a", 0).expect("known");
    assert_eq!(count(&main, "a").as_deref(), Some("2"));
    assert_eq!(shown(&floating, "Alpha"), None);
}

#[test]
fn the_application_hears_of_a_tab_that_is_dragged_outside_its_area() {
    let rig = rig(start());
    let heard = Rc::new(RefCell::new(Vec::<DragOutside>::new()));
    rig.dock.on_drag_outside({
        let heard = heard.clone();
        move |event| heard.borrow_mut().push(event.clone())
    });

    // Inside, nothing is heard.
    drag(&tab(&rig.window, "Beta"), 600.0, 160.0);
    assert!(heard.borrow().is_empty());

    let beta = tab(&rig.window, "Beta");
    drag_and_hold(&rig.window, &beta, 1100.0, 300.0);
    let moves = heard.borrow().clone();
    assert!(!moves.is_empty());
    assert!(moves.iter().all(|e| e.panel == "b" && e.host == 0 && !e.dropped), "{moves:?}");
    assert_eq!(moves.last().map(|e| e.point), Some(Point::new(1100.0, 300.0)));

    let before = rig.layout();
    release(&rig.window, 1100.0, 300.0);
    assert_eq!(
        heard.borrow().last(),
        Some(&DragOutside { panel: "b".into(), host: 0, point: Point::new(1100.0, 300.0), dropped: true })
    );
    // The docking system does nothing with the drop itself.
    assert_eq!(rig.layout(), before);
    assert_eq!(preview(&rig.window), None);
    rig.check();
}

#[test]
fn points_outside_are_in_the_window_not_in_the_dock_area() {
    init();
    let window = Inset::new().expect("the window is made");
    let dock = Dock::with_metrics(start(), metrics());
    slint_docking::attach!(dock, window);
    window.show().expect("the window is shown");
    window.window().set_size(LogicalSize::new(1000.0, 600.0));
    assert_eq!(dock.area(0), None);
    check_host(&window, &dock, 0, Rect::new(100.0, 40.0, 900.0, 560.0));
    assert_eq!(dock.area(0), Some(Rect::new(100.0, 40.0, 900.0, 560.0)));

    let heard = Rc::new(RefCell::new(Vec::<DragOutside>::new()));
    dock.on_drag_outside({
        let heard = heard.clone();
        move |event| heard.borrow_mut().push(event.clone())
    });
    // This point is in the window, and outside the dock area, which starts at 100.
    drag(&tab(&window, "Beta"), 50.0, 300.0);
    assert_eq!(heard.borrow().last().map(|e| (e.point, e.dropped)), Some((Point::new(50.0, 300.0), true)));
}

#[test]
fn a_tab_let_go_outside_is_given_a_window_of_its_own() {
    let (main, floating, dock) = two_windows();
    dock.on_drag_outside({
        let dock = dock.clone();
        move |event| {
            if event.dropped {
                dock.move_to_host(&event.panel, 1).expect("known");
            }
        }
    });

    drag(&tab(&main, "Beta"), 1300.0, 200.0);
    assert_eq!(dock.layout().to_string(), "row(25: tabs(a*), 75: col(50: tabs(c*), 50: tabs(d*)))\ntabs(b*)");
    assert_eq!(shown(&floating, "Beta"), Some(Rect::new(0.0, 30.0, 600.0, 370.0)));
    check_both(&main, &floating, &dock);
}

#[test]
fn a_tab_is_dragged_from_one_window_and_let_go_in_another() {
    let (main, floating, dock) = two_windows();
    dock.move_to_host("a", 1).expect("known");
    check_both(&main, &floating, &dock);

    // Say that on the screen the floating window is 2000 to the right of the main one.
    // A real application works this out from where its windows are.
    dock.on_drag_outside({
        let dock = dock.clone();
        move |event| {
            let point = Point::new(event.point.x - 2000.0, event.point.y);
            if event.dropped {
                dock.place_at(&event.panel, 1, point).expect("known");
            } else {
                dock.hover_at(1, point);
            }
        }
    });

    let beta = tab(&main, "Beta");
    drag_and_hold(&main, &beta, 2500.0, 300.0);
    assert_eq!(preview(&main), None);
    assert_eq!(preview(&floating), Some(Rect::new(300.0, 0.0, 300.0, 400.0)));

    release(&main, 2500.0, 300.0);
    assert_eq!(preview(&floating), None);
    assert_eq!(dock.layout().to_string(), "col(50: tabs(c*), 50: tabs(d*))\nrow(50: tabs(a*), 50: tabs(b*))");
    check_both(&main, &floating, &dock);
}

#[test]
fn the_mark_in_another_window_goes_when_the_pointer_comes_back() {
    let (main, floating, dock) = two_windows();
    dock.move_to_host("a", 1).expect("known");
    check_both(&main, &floating, &dock);
    dock.on_drag_outside({
        let dock = dock.clone();
        move |event| {
            dock.hover_at(1, Point::new(event.point.x - 2000.0, event.point.y));
        }
    });

    let beta = tab(&main, "Beta");
    drag_and_hold(&main, &beta, 2500.0, 300.0);
    assert!(preview(&floating).is_some());

    use slint::platform::WindowEvent;
    let position = slint::LogicalPosition::new(600.0, 160.0);
    main.window().dispatch_event(WindowEvent::PointerMoved { position });
    assert_eq!(preview(&floating), None);
    assert_eq!(preview(&main), Some(Rect::new(253.0, 0.0, 747.0, 298.0)));
    release(&main, 600.0, 160.0);
    assert_eq!(dock.layout().to_string(), "col(50: tabs(c, b*), 50: tabs(d*))\ntabs(a*)");
}

#[test]
fn placing_at_a_point_outside_a_dock_area_changes_nothing() {
    let (main, floating, dock) = two_windows();
    check_both(&main, &floating, &dock);
    assert_eq!(dock.place_at("a", 0, Point::new(-5.0, 10.0)), Ok(false));
    // The floating window has shown no group, so where its area is is not known.
    assert_eq!(dock.place_at("a", 1, Point::new(10.0, 10.0)), Ok(false));
    assert_eq!(dock.place_at("zzz", 0, Point::new(10.0, 10.0)), Err(OpError::UnknownPanel("zzz".into())));
    assert_eq!(dock.layout().to_string(), format!("{}\nempty", start()));

    assert_eq!(dock.place_at("a", 0, Point::new(600.0, 450.0)), Ok(true));
    assert_eq!(dock.layout().to_string(), "row(25: tabs(b*), 75: col(50: tabs(c*), 50: tabs(d, a*)))\nempty");
}

//! What the theme sets: the sizes the layout is solved with, and what the tab bar draws.

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use slint::{Brush, Color, ComponentHandle, LogicalSize};
use slint_docking::{Dock, Rect};
use slint_docking_tests::*;

/// The strips of tabs, in the order of the layout.
fn tab_bars(window: &impl ElementRoot) -> Vec<Rect> {
    ElementHandle::find_by_element_id(window, "DockArea::tab-bar").map(|e| rect(&e)).collect()
}

/// The lines that mark the tabs in front, in the order of the layout.
fn indicators(window: &impl ElementRoot) -> Vec<Rect> {
    ElementHandle::find_by_element_id(window, "DockTab::indicator").map(|e| rect(&e)).collect()
}

/// The parts of the hairlines under the strips of tabs: those under tabs, then those under
/// no tab.
fn lines(window: &impl ElementRoot) -> Vec<Rect> {
    ElementHandle::find_by_element_id(window, "DockTab::line")
        .chain(ElementHandle::find_by_element_id(window, "DockArea::line"))
        .map(|e| rect(&e))
        .collect()
}

fn transparent() -> Brush {
    Brush::SolidColor(Color::from_argb_u8(0, 0, 0, 0))
}

fn red() -> Brush {
    Brush::SolidColor(Color::from_rgb_u8(255, 0, 0))
}

#[test]
fn theme_sizes_reach_the_layout() {
    init();
    let window = Fixture::new().expect("the window is made");
    let theme = window.global::<DockTheme>();
    theme.set_tab_bar_height(26.0);
    theme.set_splitter_thickness(1.0);
    let dock = Dock::with_metrics(start(), metrics());
    slint_docking::attach!(dock, window);
    window.show().expect("the window is shown");
    window.window().set_size(LogicalSize::new(1000.0, 600.0));
    let area = Rect::new(0.0, 0.0, 1000.0, 600.0);

    assert_eq!(dock.metrics().tab_bar_height, 26.0);
    assert_eq!(dock.metrics().splitter, 1.0);
    let bars = tab_bars(&window);
    assert_eq!(bars.len(), 3);
    assert!(bars.iter().all(|r| r.height == 26.0), "tab bars {bars:?}");
    assert_eq!(rect(&tab(&window, "Alpha")).height, 26.0);
    assert_eq!(shown(&window, "Alpha").map(|r| r.y), Some(26.0));
    // The bar between the two columns, then the bar between Gamma and Delta.
    let bars: Vec<Rect> = splitters(&window).iter().map(rect).collect();
    assert_eq!(bars.len(), 2);
    assert_eq!((bars[0].width, bars[1].height), (1.0, 1.0), "splitters {bars:?}");
    check_host(&window, &dock, 0, area);

    // A change after the window is attached is followed too, once the event loop runs.
    theme.set_tab_bar_height(32.0);
    theme.set_splitter_thickness(6.0);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
    assert_eq!(dock.metrics().tab_bar_height, 32.0);
    assert_eq!(dock.metrics().splitter, 6.0);
    assert!(tab_bars(&window).iter().all(|r| r.height == 32.0));
    let bars: Vec<Rect> = splitters(&window).iter().map(rect).collect();
    assert_eq!((bars[0].width, bars[1].height), (6.0, 6.0), "splitters {bars:?}");
    check_host(&window, &dock, 0, area);
}

#[test]
fn the_active_indicator_can_be_hidden() {
    let rig = rig(start());
    let theme = rig.window.global::<DockTheme>();
    // Alpha, Gamma and Delta are in front of their groups.
    let expected: Vec<Rect> = ["Alpha", "Gamma", "Delta"]
        .into_iter()
        .map(|title| {
            let tab = rect(&tab(&rig.window, title));
            Rect::new(tab.x, tab.y, tab.width, 2.0)
        })
        .collect();
    assert_eq!(theme.get_tab_active_indicator(), theme.get_accent());
    assert_eq!(indicators(&rig.window), expected);

    theme.set_tab_active_indicator(transparent());
    assert_eq!(indicators(&rig.window), []);

    theme.set_tab_active_indicator(red());
    assert_eq!(indicators(&rig.window), expected);
    rig.check();
}

#[test]
fn the_hairline_is_drawn_under_the_tab_bar() {
    let rig = rig(start());
    let theme = rig.window.global::<DockTheme>();
    assert_eq!(lines(&rig.window), []);

    theme.set_line(red());
    check_lines(&rig.window, &[]);
    assert!(!lines(&rig.window).is_empty());

    // A tab behind under the pointer keeps the line under it, whatever its background.
    theme.set_tab_hover_background(red());
    let beta = rect(&tab(&rig.window, "Beta"));
    move_to(&rig.window, beta.center().x, beta.center().y);
    check_lines(&rig.window, &["Beta"]);
    let under_beta = Rect::new(beta.x, beta.y + beta.height - 1.0, beta.width, 1.0);
    assert!(lines(&rig.window).contains(&under_beta), "{:?}", lines(&rig.window));

    theme.set_line(transparent());
    assert_eq!(lines(&rig.window), []);
    rig.check();
}

#[test]
fn the_dragged_tab_and_the_drop_mark_take_their_own_colours() {
    let rig = rig(start());
    let theme = rig.window.global::<DockTheme>();
    // The defaults are what the two were drawn with before they had properties of their own.
    assert_eq!(theme.get_drag_ghost_background(), theme.get_tab_active_background());
    assert_eq!(theme.get_drag_ghost_border(), theme.get_accent());
    assert_eq!(theme.get_drag_ghost_opacity(), 0.9);
    assert_eq!(theme.get_preview_border(), theme.get_accent());

    theme.set_drag_ghost_background(red());
    theme.set_drag_ghost_border(red());
    theme.set_drag_ghost_opacity(1.0);
    theme.set_preview_border(red());
    assert_eq!(theme.get_drag_ghost_background(), red());
    assert_eq!(theme.get_drag_ghost_border(), red());
    assert_eq!(theme.get_preview_border(), red());

    // The testing backend reads no brush of an element, so of what is drawn only the
    // dragged tab's opacity is checked. The brushes are bound in `dock.slint`.
    assert_eq!(ElementHandle::find_by_element_id(&rig.window, "DockArea::ghost").count(), 0);
    drag_and_hold(&rig.window, &tab(&rig.window, "Beta"), 600.0, 160.0);
    let ghosts: Vec<ElementHandle> =
        ElementHandle::find_by_element_id(&rig.window, "DockArea::ghost").collect();
    assert_eq!(ghosts.len(), 1);
    assert_eq!(ghosts[0].computed_opacity(), 1.0);
    assert!(preview(&rig.window).is_some(), "the drop mark is shown");

    theme.set_drag_ghost_opacity(0.5);
    assert_eq!(ghosts[0].computed_opacity(), 0.5);
    release(&rig.window, 600.0, 160.0);
    assert_eq!(ElementHandle::find_by_element_id(&rig.window, "DockArea::ghost").count(), 0);
    rig.check();
}

#[test]
fn the_splitter_can_be_taken_from_beside_it() {
    for (grab, resized) in [(8.0, true), (0.0, false)] {
        let rig = rig(start());
        let theme = rig.window.global::<DockTheme>();
        theme.set_splitter_thickness(1.0);
        theme.set_splitter_grab(grab);
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
        let before = rig.layout();
        let bar = rect(&splitters(&rig.window)[0]);
        assert_eq!(bar.width, 1.0, "grab {grab}");

        // 3 pixels left of the drawn bar, over Alpha's panel.
        drag_from(&rig.window, (bar.x - 3.0, 300.0), (500.0, 300.0));
        let bar = rect(&splitters(&rig.window)[0]);
        if resized {
            assert_ne!(rig.layout(), before, "grab {grab}");
            assert!((bar.center().x - 500.0).abs() <= 4.0, "grab {grab}: {bar:?}");
        } else {
            assert_eq!(rig.layout(), before, "grab {grab}");
        }
        rig.check();
    }
}

/// Checks the line along the bottom of every strip of tabs: it is under all of the strip
/// but the tab in front, and is drawn over every tab it runs under, of whose title `hovered`
/// is one.
fn check_lines(window: &impl ElementRoot, hovered: &[&str]) {
    let bars: Vec<ElementHandle> = ElementHandle::find_by_element_id(window, "DockArea::tab-bar").collect();
    assert_eq!(bars.len(), 3);
    for (bar, front) in bars.iter().zip(["Alpha", "Gamma", "Delta"]) {
        let b = rect(bar);
        let front = rect(&tab(window, front));
        let mut parts: Vec<Rect> = bar
            .query_descendants()
            .match_predicate(|e| e.id().is_some_and(|id| id == "DockArea::line" || id == "DockTab::line"))
            .find_all()
            .iter()
            .map(rect)
            .collect();
        parts.sort_by(|p, q| p.x.total_cmp(&q.x));
        // The parts, joined where they meet.
        let mut runs: Vec<(f32, f32)> = Vec::new();
        for part in &parts {
            assert_eq!((part.y, part.height), (b.y + b.height - 1.0, 1.0), "{part:?} in {b:?}");
            match runs.last_mut() {
                Some(run) if run.1 == part.x => run.1 = part.x + part.width,
                _ => runs.push((part.x, part.x + part.width)),
            }
        }
        let expected: Vec<(f32, f32)> = [(b.x, front.x), (front.x + front.width, b.x + b.width)]
            .into_iter()
            .filter(|(from, to)| to > from)
            .collect();
        assert_eq!(runs, expected, "the line of {b:?}, with the tab in front at {front:?}");
    }
    for title in hovered {
        let t = tab(window, title);
        let own = t.query_descendants().match_id("DockTab::line").find_all();
        assert_eq!(own.len(), 1, "{title} draws the line under itself");
    }
}

/// Moves the pointer to a point of the window, with no button down.
fn move_to<W: ComponentHandle>(window: &W, x: f32, y: f32) {
    use slint::platform::WindowEvent;
    let position = slint::LogicalPosition::new(x, y);
    window.window().dispatch_event(WindowEvent::PointerMoved { position });
}

/// Presses the left button at one point of the window, moves to another in steps, and lets
/// go there.
fn drag_from<W: ComponentHandle>(window: &W, from: (f32, f32), to: (f32, f32)) {
    use slint::platform::{PointerEventButton, WindowEvent};
    let button = PointerEventButton::Left;
    let position = slint::LogicalPosition::new(from.0, from.1);
    window.window().dispatch_event(WindowEvent::PointerMoved { position });
    window.window().dispatch_event(WindowEvent::PointerPressed { position, button });
    for step in 1..=10 {
        let t = step as f32 / 10.0;
        let position =
            slint::LogicalPosition::new(from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t);
        i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(16));
        window.window().dispatch_event(WindowEvent::PointerMoved { position });
    }
    let position = slint::LogicalPosition::new(to.0, to.1);
    window.window().dispatch_event(WindowEvent::PointerReleased { position, button });
}

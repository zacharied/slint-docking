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

/// The hairlines under the strips of tabs, in the order of the layout.
fn lines(window: &impl ElementRoot) -> Vec<Rect> {
    ElementHandle::find_by_element_id(window, "DockArea::line").map(|e| rect(&e)).collect()
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
    let expected: Vec<Rect> =
        tab_bars(&rig.window).iter().map(|b| Rect::new(b.x, b.y + b.height - 1.0, b.width, 1.0)).collect();
    assert_eq!(expected.len(), 3);
    assert_eq!(lines(&rig.window), expected);

    // The tab in front covers the line under it: it reaches the bottom of the bar, and it
    // is drawn after the line.
    let bars = ElementHandle::find_by_element_id(&rig.window, "DockArea::tab-bar");
    for ((bar, line), title) in bars.zip(lines(&rig.window)).zip(["Alpha", "Gamma", "Delta"]) {
        let tab = rect(&tab(&rig.window, title));
        assert_eq!(tab.y + tab.height, line.y + line.height, "{title}");
        assert!(tab.x >= line.x && tab.x + tab.width <= line.x + line.width, "{title}");
        let drawn: Vec<String> = bar
            .query_descendants()
            .match_predicate(|e| {
                e.id().is_some_and(|id| id == "DockArea::line")
                    || e.type_name().is_some_and(|t| t == "DockTab")
            })
            .find_all()
            .iter()
            .map(|e| e.accessible_label().map(|l| l.to_string()).unwrap_or_else(|| "line".into()))
            .collect();
        assert_eq!(drawn.first().map(String::as_str), Some("line"), "{title}: {drawn:?}");
        assert!(drawn.iter().any(|d| d == title), "{title}: {drawn:?}");
    }

    theme.set_line(transparent());
    assert_eq!(lines(&rig.window), []);
    rig.check();
}

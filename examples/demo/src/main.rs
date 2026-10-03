//! A window to try `slint-docking` in.
//!
//! ```text
//! cargo run -p demo
//! ```
//!
//! The layout is saved when it changes and read when the demo starts. "Reset layout" goes
//! back to the layout the demo is built with.
//!
//! There are two windows. A tab that is let go outside the main window gets the floating
//! window, and a tab dragged from either window onto the other is docked there.
//!
//! With `--screenshot <file.png>` the demo draws itself to a file and ends. With
//! `--screenshot-drag <file.png>` it does so in the middle of dragging a tab. With
//! `--screenshot-windows <folder>` it takes a tab out of the main window and back, and
//! draws both windows on the way. These are for looking at the demo where there is nobody
//! to look at the screen.

mod shots;

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use slint::winit_030::WinitWindowAccessor;
use slint::{CloseRequestResponse, ComponentHandle, ModelRc, Timer, VecModel};
use slint_docking::{Dock, DragOutside, Layout, Node, Point};

slint::include_modules!();

/// The host that the floating window shows.
const FLOATING: usize = 1;

fn built_in_layout() -> Layout {
    Layout::with_hosts(vec![
        Some(Node::row([
            (1.0, Node::tabs(["browser"])),
            (
                3.0,
                Node::column([
                    (3.0, Node::tabs(["chart", "preview"])),
                    (1.0, Node::tabs(["timeline", "log"])),
                ]),
            ),
            (1.0, Node::tabs(["inspector"])),
        ])),
        None,
    ])
}

fn layout_file() -> PathBuf {
    std::env::temp_dir().join("slint-docking-demo-layout.json")
}

fn saved_layout() -> Option<Layout> {
    let saved = std::fs::read_to_string(layout_file()).ok()?;
    serde_json::from_str(&saved).ok()
}

fn save_layout(layout: &Layout) {
    if let Ok(saved) = serde_json::to_string_pretty(layout) {
        // A layout that cannot be saved is not worth stopping for.
        let _ = std::fs::write(layout_file(), saved);
    }
}

/// Where the content of a window is on the screen, in physical pixels.
///
/// Slint tells where a window is with its frame, and not how thick the frame is. Winit
/// tells where the content is. Without winit the frame is taken to be of no size, which
/// puts a point too high by the height of the title bar.
fn content_origin(window: &slint::Window) -> (f32, f32) {
    let inner =
        window.with_winit_window(|w| w.inner_position().ok()).flatten().map(|at| (at.x as f32, at.y as f32));
    inner.unwrap_or_else(|| {
        let at = window.position();
        (at.x as f32, at.y as f32)
    })
}

/// Takes a point of one window to the screen, in physical pixels.
pub(crate) fn to_screen(window: &slint::Window, point: Point) -> (f32, f32) {
    let origin = content_origin(window);
    let scale = window.scale_factor();
    (origin.0 + point.x * scale, origin.1 + point.y * scale)
}

/// Takes a point of the screen to a window.
pub(crate) fn from_screen(window: &slint::Window, screen: (f32, f32)) -> Point {
    let origin = content_origin(window);
    let scale = window.scale_factor();
    Point::new((screen.0 - origin.0) / scale, (screen.1 - origin.1) / scale)
}

/// What to do with a tab that is dragged outside the window it came from.
fn drag_outside(dock: &Dock, main: &MainWindow, floating: &FloatingWindow, event: &DragOutside) {
    let (from, to, other) = match event.host {
        0 => (main.window(), floating.window(), FLOATING),
        _ => (floating.window(), main.window(), 0),
    };
    let screen = to_screen(from, event.point);
    // A window that holds nothing is hidden, so nothing can be let go over it.
    let over_other = dock.layout().root(other).is_some()
        && dock.area(other).is_some_and(|area| {
            let point = from_screen(to, screen);
            area.contains(point)
        });

    if !event.dropped {
        if over_other {
            dock.hover_at(other, from_screen(to, screen));
        }
        return;
    }
    if over_other {
        let _ = dock.place_at(&event.panel, other, from_screen(to, screen));
    } else if event.host == 0 {
        // Let go over nothing: the panel gets the floating window, under the pointer.
        let _ = dock.move_to_host(&event.panel, FLOATING);
        let at = slint::PhysicalPosition::new(screen.0 as i32 - 60, screen.1 as i32 - 20);
        floating.window().set_position(at);
    }
}

fn main() -> Result<(), slint::PlatformError> {
    // The winit backend is the one that can tell where a window is.
    slint::BackendSelector::new().backend_name("winit".into()).select()?;
    let shot = shots::Shot::from_args();
    let layout = match shot.is_none() {
        true => saved_layout().unwrap_or_else(built_in_layout),
        false => built_in_layout(),
    };

    let main = MainWindow::new()?;
    let floating = FloatingWindow::new()?;
    let dock = Dock::new(layout);
    slint_docking::attach!(dock, main);
    slint_docking::attach!(dock, floating, host = FLOATING);

    // The buttons above the dock area.
    let toggles = Rc::new(VecModel::<PanelToggle>::default());
    main.global::<App>().set_panels(ModelRc::from(toggles.clone()));
    let show_toggles = {
        let dock = dock.clone();
        move |layout: &Layout| {
            let panels: Vec<PanelToggle> = dock
                .panels()
                .into_iter()
                .map(|panel| PanelToggle {
                    floating: layout.find_panel(&panel.key).is_some_and(|at| at.host == FLOATING),
                    key: panel.key.into(),
                    title: panel.title.into(),
                    open: panel.open,
                })
                .collect();
            toggles.set_vec(panels);
        }
    };

    // The floating window is shown while it holds anything.
    let show_floating = {
        let floating = floating.as_weak();
        move |layout: &Layout| {
            let Some(floating) = floating.upgrade() else { return };
            let _ = match layout.root(FLOATING) {
                Some(_) => floating.show(),
                None => floating.hide(),
            };
        }
    };

    let saves = shot.is_none();
    let changed = move |layout: &Layout| {
        show_toggles(layout);
        show_floating(layout);
        if saves {
            save_layout(layout);
        }
    };
    changed(&dock.layout());
    dock.on_layout_changed(changed);

    main.global::<App>().on_toggle({
        let dock = dock.clone();
        move |key| {
            let _ = match dock.is_open(&key) {
                true => dock.close(&key),
                false => dock.open(&key),
            };
        }
    });
    main.global::<App>().on_float({
        let dock = dock.clone();
        move |key| {
            let floats = dock.layout().find_panel(&key).is_some_and(|at| at.host == FLOATING);
            let _ = dock.move_to_host(&key, if floats { 0 } else { FLOATING });
        }
    });
    main.global::<App>().on_reset_layout({
        let dock = dock.clone();
        move || dock.set_layout(built_in_layout())
    });

    dock.on_drag_outside({
        let (dock, main, floating) = (dock.clone(), main.as_weak(), floating.as_weak());
        move |event| {
            if let (Some(main), Some(floating)) = (main.upgrade(), floating.upgrade()) {
                drag_outside(&dock, &main, &floating, event);
            }
        }
    });

    // Closing the floating window sends its panels home.
    floating.window().on_close_requested({
        let dock = dock.clone();
        move || {
            let held: Vec<String> =
                dock.layout().groups(FLOATING).iter().flat_map(|group| group.panels().to_vec()).collect();
            for panel in held {
                let _ = dock.move_to_host(&panel, 0);
            }
            CloseRequestResponse::HideWindow
        }
    });
    main.window().on_close_requested(|| {
        let _ = slint::quit_event_loop();
        CloseRequestResponse::HideWindow
    });

    // State that lives in Rust is the same in every window.
    let beat = Rc::new(std::cell::Cell::new((0, true)));
    let show_beat = {
        let (main, floating, beat) = (main.as_weak(), floating.as_weak(), beat.clone());
        move || {
            let (count, playing) = beat.get();
            if let Some(main) = main.upgrade() {
                main.global::<Transport>().set_beat(count);
                main.global::<Transport>().set_playing(playing);
            }
            if let Some(floating) = floating.upgrade() {
                floating.global::<Transport>().set_beat(count);
                floating.global::<Transport>().set_playing(playing);
            }
        }
    };
    let toggle_playing = {
        let (beat, show_beat) = (beat.clone(), show_beat.clone());
        move || {
            let (count, playing) = beat.get();
            beat.set((count, !playing));
            show_beat();
        }
    };
    main.global::<Transport>().on_toggle_playing(toggle_playing.clone());
    floating.global::<Transport>().on_toggle_playing(toggle_playing);
    let ticker = Timer::default();
    if saves {
        ticker.start(slint::TimerMode::Repeated, Duration::from_millis(500), move || {
            let (count, playing) = beat.get();
            if playing {
                beat.set((count + 1, playing));
                show_beat();
            }
        });
    }

    main.show()?;

    shots::start(shot, &main, &floating, &dock);
    slint::run_event_loop()
}

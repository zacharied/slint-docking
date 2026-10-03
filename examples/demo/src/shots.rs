//! Drawing the demo to files, for looking at it where there is nobody to look at the screen.
//!
//! The pointer is played by sending the windows the events that the pointer would cause.
//! Everything after that is the real thing: real windows, on the real backend.

use std::path::{Path, PathBuf};
use std::time::Duration;

use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, Timer};
use slint_docking::{Dock, Point};

use crate::{FloatingWindow, MainWindow, from_screen, to_screen};

/// What to draw.
pub enum Shot {
    /// Nothing. The demo runs as usual.
    None,
    /// The main window as it starts.
    Still(PathBuf),
    /// The main window while a tab is dragged.
    Drag(PathBuf),
    /// Both windows, as a tab is taken out of the main window and back. The files go into
    /// this folder.
    Windows(PathBuf),
}

impl Shot {
    /// Reads what to draw from the arguments of the program.
    pub fn from_args() -> Shot {
        let mut args = std::env::args().skip(1);
        match (args.next().as_deref(), args.next()) {
            (Some("--screenshot"), Some(file)) => Shot::Still(file.into()),
            (Some("--screenshot-drag"), Some(file)) => Shot::Drag(file.into()),
            (Some("--screenshot-windows"), Some(folder)) => Shot::Windows(folder.into()),
            _ => Shot::None,
        }
    }

    /// Whether the demo runs as usual.
    pub fn is_none(&self) -> bool {
        matches!(self, Shot::None)
    }
}

/// How long to wait for a window to draw what has changed.
const SETTLE: Duration = Duration::from_millis(600);

/// Draws what was asked for, then ends the program.
pub fn start(shot: Shot, main: &MainWindow, floating: &FloatingWindow, dock: &Dock) {
    let (main, floating, dock) = (main.clone_strong(), floating.clone_strong(), dock.clone());
    match shot {
        Shot::None => {}
        Shot::Still(file) => Timer::single_shot(SETTLE, move || {
            save(main.window(), &file);
            end();
        }),
        Shot::Drag(file) => Timer::single_shot(SETTLE, move || {
            let from = tab(&dock, 0, "preview");
            let size = main.window().size().to_logical(main.window().scale_factor());
            press_and_move(main.window(), from, Point::new(size.width * 0.5, size.height - 60.0));
            Timer::single_shot(SETTLE, move || {
                save(main.window(), &file);
                end();
            });
        }),
        Shot::Windows(folder) => Timer::single_shot(SETTLE, move || {
            // Out of the main window, and let go over nothing.
            let from = tab(&dock, 0, "preview");
            let size = main.window().size().to_logical(main.window().scale_factor());
            let outside = Point::new(size.width + 150.0, 200.0);
            press_and_move(main.window(), from, outside);
            release(main.window(), outside);
            println!("after the tab is let go outside:\n{}", dock.layout());

            Timer::single_shot(SETTLE, move || {
                save(floating.window(), &folder.join("1-floating.png"));
                save(main.window(), &folder.join("1-main.png"));

                // Out of the floating window, and onto the chart in the main one.
                let from = tab(&dock, 1, "preview");
                let onto = to_screen(main.window(), Point::new(size.width * 0.5, 300.0));
                let onto = from_screen(floating.window(), onto);
                press_and_move(floating.window(), from, onto);
                Timer::single_shot(SETTLE, move || {
                    save(main.window(), &folder.join("2-main-while-dragged-over.png"));
                    release(floating.window(), onto);
                    println!("after the tab is let go over the main window:\n{}", dock.layout());
                    Timer::single_shot(SETTLE, move || {
                        save(main.window(), &folder.join("3-main.png"));
                        end();
                    });
                });
            });
        }),
    }
}

/// The middle of the tab of a panel, in the window of its host.
fn tab(dock: &Dock, host: usize, panel: &str) -> Point {
    let area = dock.area(host).unwrap_or_default();
    let size = slint_docking::Size::new(area.width, area.height);
    let solved = dock.layout().solve(host, size, &dock.metrics());
    let (group, index) = solved.group_of(panel).expect("the panel is in the host");
    let middle = group.tab_rect(index).center();
    Point::new(middle.x + area.x, middle.y + area.y)
}

fn press_and_move(window: &slint::Window, from: Point, to: Point) {
    let position = LogicalPosition::new(from.x, from.y);
    window.dispatch_event(WindowEvent::PointerMoved { position });
    window.dispatch_event(WindowEvent::PointerPressed { position, button: PointerEventButton::Left });
    for step in 1..=8 {
        let t = step as f32 / 8.0;
        let position = LogicalPosition::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        window.dispatch_event(WindowEvent::PointerMoved { position });
    }
}

fn release(window: &slint::Window, at: Point) {
    let position = LogicalPosition::new(at.x, at.y);
    window.dispatch_event(WindowEvent::PointerReleased { position, button: PointerEventButton::Left });
}

fn save(window: &slint::Window, file: &Path) {
    match window.take_snapshot() {
        Ok(pixels) => {
            let saved = image::save_buffer(
                file,
                pixels.as_bytes(),
                pixels.width(),
                pixels.height(),
                image::ColorType::Rgba8,
            );
            if let Err(error) = saved {
                eprintln!("{} was not saved: {error}", file.display());
            }
        }
        Err(error) => eprintln!("{} was not drawn: {error}", file.display()),
    }
}

fn end() {
    let _ = slint::quit_event_loop();
}

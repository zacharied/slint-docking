//! Docking for Slint.
//!
//! Panels sit in tab groups, and tab groups sit in splits. The user rearranges them by
//! dragging a tab: onto the side of a group to split it, onto its middle or its tabs to
//! join it, or onto the edge of the window to take that whole side. Bars between the
//! groups are dragged to resize them. The behaviour follows Visual Studio Code and
//! Dockview.
//!
//! A panel holds anything. Its content is ordinary Slint, is created once, and is never
//! created again however the panel is moved, so it keeps its state.
//!
//! # Use
//!
//! `build.rs`, with `slint-docking-build` as a build dependency:
//!
//! ```ignore
//! fn main() {
//!     slint_docking_build::compile("ui/app.slint").unwrap();
//! }
//! ```
//!
//! `ui/app.slint`:
//!
//! ```slint,ignore
//! import { DockArea, DockPanel } from "@docking";
//! export { DockBackend, DockTheme } from "@docking";
//!
//! export component MainWindow inherits Window {
//!     DockArea {
//!         DockPanel { key: "browser"; title: "Browser"; Browser { } }
//!         DockPanel { key: "chart"; title: "Chart"; ChartView { } }
//!         DockPanel { key: "inspector"; title: "Inspector"; Inspector { } }
//!     }
//! }
//! ```
//!
//! `src/main.rs`:
//!
//! ```ignore
//! use slint_docking::{Dock, Layout, Node};
//!
//! slint::include_modules!();
//!
//! fn main() -> Result<(), slint::PlatformError> {
//!     let window = MainWindow::new()?;
//!     let dock = Dock::new(Layout::new(Node::row([
//!         (1.0, Node::tabs(["browser"])),
//!         (3.0, Node::tabs(["chart"])),
//!         (1.0, Node::tabs(["inspector"])),
//!     ])));
//!     slint_docking::attach!(dock, window);
//!     window.run()
//! }
//! ```
//!
//! # Testing
//!
//! Everything the pointer does goes through [`Dock::apply`] as an [`Op`], so most tests of
//! an application need no pointer: apply the operation and look at [`Dock::layout`].
//!
//! For tests that drive the interface itself, the tabs, panels and close buttons carry
//! accessible roles and labels, which `i-slint-backend-testing` finds them by. See the
//! tests of this workspace for how.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bridge;
mod dock;

pub use dock::{Dock, DragOutside, PanelInfo};
pub use slint_docking_layout::{
    Axis, Group, GroupId, Hit, Layout, Location, Metrics, Node, Op, OpError, Point, Rect, Size, Solved,
    SolvedGroup, SolvedSplit, SolvedSplitter, Split, SplitId, Target, Zone,
};

#[doc(hidden)]
pub use slint;

/// Connects a window to a [`Dock`].
///
/// ```ignore
/// slint_docking::attach!(dock, window);
/// slint_docking::attach!(dock, second_window, host = 1);
/// slint_docking::attach!(dock, window, types = crate::ui);
/// ```
///
/// The window shows one host of the layout, host 0 unless another is given.
///
/// The macro uses the Rust types that Slint generates for `dock.slint`. They exist only if
/// the `.slint` file of the window has the line `export { DockBackend, DockTheme } from "@docking";`. They must be
/// in scope where the macro is called, or in the module given as `types`.
///
/// This is a macro because the generated types belong to the crate that compiles the
/// `.slint` file, so no function in this crate could name them.
#[macro_export]
macro_rules! attach {
    ($dock:expr, $window:expr $(,)?) => {
        $crate::attach!(@attach $dock, $window, 0usize, {})
    };
    ($dock:expr, $window:expr, host = $host:expr $(,)?) => {
        $crate::attach!(@attach $dock, $window, $host, {})
    };
    ($dock:expr, $window:expr, types = $($types:ident)::+ $(,)?) => {
        $crate::attach!(@attach $dock, $window, 0usize, { $($types)::+ })
    };
    ($dock:expr, $window:expr, host = $host:expr, types = $($types:ident)::+ $(,)?) => {
        $crate::attach!(@attach $dock, $window, $host, { $($types)::+ })
    };
    (@attach $dock:expr, $window:expr, $host:expr, { $($($types:ident)::+)? }) => {{
        $(
            #[allow(unused_imports)]
            use $($types)::+::{
                DockBackend, DockGroupRect, DockGroupView, DockPanelRect, DockPreview, DockRect,
                DockSplitterView, DockTabView,
            };
        )?
        use $crate::slint::ComponentHandle as _;

        struct Bridge;
        impl $crate::bridge::Bridge for Bridge {
            type Tab = DockTabView;
            type Group = DockGroupView;
            type Splitter = DockSplitterView;

            fn tab(view: &$crate::bridge::TabView) -> DockTabView {
                DockTabView {
                    key: view.key.as_str().into(),
                    title: view.title.as_str().into(),
                    active: view.active,
                    closable: view.closable,
                }
            }

            fn group(
                view: &$crate::bridge::GroupView,
                tabs: $crate::slint::ModelRc<DockTabView>,
            ) -> DockGroupView {
                DockGroupView { id: view.id as i32, tabs }
            }

            fn splitter(view: &$crate::bridge::SplitterView) -> DockSplitterView {
                DockSplitterView {
                    split: view.split as i32,
                    handle: view.handle as i32,
                    horizontal: view.horizontal,
                }
            }
        }

        let dock: $crate::Dock = ::core::clone::Clone::clone(&$dock);
        let host: usize = $host;
        let window = &$window;
        let backend = window.global::<DockBackend>();
        let models = ::std::rc::Rc::new($crate::bridge::Models::<Bridge>::new());
        backend.set_groups(models.groups());
        backend.set_splitters(models.splitters());

        backend.on_panel_rect({
            let dock = dock.clone();
            move |slot, width, height, _| {
                let (rect, shown) = dock.panel_rect(host, slot, width, height);
                DockPanelRect {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                    shown,
                }
            }
        });
        backend.on_group_rect({
            let dock = dock.clone();
            move |index, width, height, x, y, _| {
                let (rect, tab_width) = dock.group_rect(host, index, width, height, x, y);
                DockGroupRect {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                    tab_width,
                }
            }
        });
        backend.on_splitter_rect({
            let dock = dock.clone();
            move |index, width, height, _| {
                let rect = dock.splitter_rect(host, index, width, height);
                DockRect { x: rect.x, y: rect.y, width: rect.width, height: rect.height }
            }
        });
        backend.on_holds({
            let dock = dock.clone();
            move |key, _| dock.holds(host, key.as_str())
        });
        backend.on_panel_declared({
            let dock = dock.clone();
            move || dock.panels_declared(host)
        });
        backend.on_title_changed({
            let dock = dock.clone();
            move |slot, title| dock.title_changed(host, slot, title.as_str())
        });
        backend.on_tab_pressed({
            let dock = dock.clone();
            move |key| dock.tab_pressed(key.as_str())
        });
        backend.on_tab_closed({
            let dock = dock.clone();
            move |key| dock.tab_closed(key.as_str())
        });
        backend.on_drag_started({
            let dock = dock.clone();
            move |key| dock.drag_started(host, key.as_str())
        });
        backend.on_drag_moved({
            let dock = dock.clone();
            move |x, y, width, height| dock.drag_moved(host, x, y, width, height)
        });
        backend.on_drag_dropped({
            let dock = dock.clone();
            move |x, y, width, height| dock.drag_dropped(host, x, y, width, height)
        });
        backend.on_drag_cancelled({
            let dock = dock.clone();
            move || dock.drag_cancelled()
        });
        backend.on_splitter_dragged({
            let dock = dock.clone();
            move |split, handle, x, y, width, height| {
                dock.splitter_dragged(host, split, handle, x, y, width, height)
            }
        });

        dock.attach_host(
            host,
            $crate::bridge::HostLink {
                declared: ::std::boxed::Box::new({
                    let window = window.as_weak();
                    move || {
                        window
                            .upgrade()
                            .map(|w| w.global::<DockBackend>().get_declared().to_string())
                            .unwrap_or_default()
                    }
                }),
                show: ::std::boxed::Box::new({
                    let window = window.as_weak();
                    move |view, generation| {
                        models.show(view);
                        if let Some(window) = window.upgrade() {
                            window.global::<DockBackend>().set_generation(generation);
                        }
                    }
                }),
                preview: ::std::boxed::Box::new({
                    let window = window.as_weak();
                    move |rect| {
                        if let Some(window) = window.upgrade() {
                            let rect: ::core::option::Option<$crate::Rect> = rect;
                            window.global::<DockBackend>().set_preview(match rect {
                                Some(rect) => DockPreview {
                                    shown: true,
                                    x: rect.x,
                                    y: rect.y,
                                    width: rect.width,
                                    height: rect.height,
                                },
                                None => DockPreview::default(),
                            });
                        }
                    }
                }),
                dragging: ::std::boxed::Box::new({
                    let window = window.as_weak();
                    move |title| {
                        if let Some(window) = window.upgrade() {
                            let title: ::core::option::Option<&str> = title;
                            let backend = window.global::<DockBackend>();
                            backend.set_dragging(title.is_some());
                            backend.set_drag_title(title.unwrap_or_default().into());
                        }
                    }
                }),
            },
        );
    }};
}

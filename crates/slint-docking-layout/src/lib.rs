//! The layout model behind `slint-docking`.
//!
//! A [`Layout`] says where every panel is. It is a tree per host: a [`Node`] is either a
//! tab group or a split, and a host is one area that panels are docked in. This crate
//! knows nothing about any interface toolkit, so everything in it is tested without one.
//!
//! - A layout changes only through [`Layout::apply`], which takes an [`Op`].
//! - [`Layout::solve`] turns a host and a size into rectangles.
//! - [`Layout::target_at`] turns a point into the [`Target`] a drop there would have.
//! - [`Layout::place_of`] says where a panel is, as a [`Place`] that [`Op::Return`]
//!   puts it back at once it is closed.
//!
//! Panels are named by keys, which are strings the application chooses.
//!
//! ```
//! use slint_docking_layout::{Layout, Node, Op, Target, Zone};
//!
//! let mut layout = Layout::new(Node::row([
//!     (1.0, Node::tabs(["browser"])),
//!     (3.0, Node::tabs(["chart", "preview"])),
//! ]));
//! assert_eq!(layout.to_string(), "row(25: tabs(browser*), 75: tabs(chart*, preview))");
//!
//! let chart = layout.find_panel("chart").unwrap().group;
//! layout
//!     .apply(Op::Place {
//!         panel: "preview".into(),
//!         target: Target::Group { group: chart, zone: Zone::Bottom },
//!     })
//!     .unwrap();
//! assert_eq!(
//!     layout.to_string(),
//!     "row(25: tabs(browser*), 75: col(50: tabs(chart*), 50: tabs(preview*)))"
//! );
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod geometry;
mod hit;
mod ops;
mod place;
mod solve;
mod tree;

pub use geometry::{Point, Rect, Size};
pub use hit::Hit;
pub use ops::{Op, OpError, Target, Zone};
pub use place::Place;
pub use solve::{Metrics, Solved, SolvedGroup, SolvedSplit, SolvedSplitter};
pub use tree::{Axis, Group, GroupId, Layout, Location, Node, Split, SplitId};

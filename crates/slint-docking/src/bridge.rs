//! What passes between a [`Dock`](crate::Dock) and the Slint side of one window.
//!
//! Slint generates the Rust types of a `.slint` file inside the crate that compiles the
//! file, so this crate cannot name them. [`attach!`](crate::attach) can, because a macro
//! is expanded in the crate that calls it. Everything here exists to keep that macro
//! small: the macro converts between these types and the generated ones, and no more.
//!
//! An application has no reason to use this module directly.

use std::cell::RefCell;
use std::rc::Rc;

use slint::{Model, ModelRc, VecModel};
use slint_docking_layout::Rect;

/// A tab, as the Slint side shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TabView {
    /// The key of the panel.
    pub key: String,
    /// The text of the tab.
    pub title: String,
    /// Whether the panel is in front.
    pub active: bool,
    /// Whether the tab has a button to close the panel.
    pub closable: bool,
}

/// The tabs of a group, as the Slint side shows them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupView {
    /// The ID of the group.
    pub id: u32,
    /// The tabs, in order.
    pub tabs: Vec<TabView>,
}

/// A bar between two children of a split, as the Slint side shows it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SplitterView {
    /// The ID of the split.
    pub split: u32,
    /// Which bar of the split.
    pub handle: u32,
    /// Whether the bar is dragged left and right.
    pub horizontal: bool,
}

/// Everything the Slint side of one host draws that is not a rectangle. Rectangles are
/// asked for through callbacks, so that they follow the size of the window.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HostView {
    /// The groups, in the order of [`Solved::groups`](slint_docking_layout::Solved).
    pub groups: Vec<GroupView>,
    /// The bars, in the order of [`Solved::splitters`](slint_docking_layout::Solved).
    pub splitters: Vec<SplitterView>,
}

/// Shows a view, with the generation that makes every rectangle be asked for again.
pub type Show = Box<dyn Fn(&HostView, i32)>;

/// Shows the dragged tab beside the pointer, given its title, or hides it.
pub type ShowDragging = Box<dyn Fn(Option<&str>)>;

/// What a [`Dock`](crate::Dock) can do to the Slint side of one host.
pub struct HostLink {
    /// Reads what the panels of the window have declared.
    pub declared: Box<dyn Fn() -> String>,
    /// Shows a view, and makes every rectangle be asked for again.
    pub show: Show,
    /// Shows or hides the mark of where a dragged panel would go.
    pub preview: Box<dyn Fn(Option<Rect>)>,
    /// Shows the dragged tab beside the pointer, or hides it.
    pub dragging: ShowDragging,
}

/// Converts the views to the types that Slint generated.
pub trait Bridge: 'static {
    /// The generated `DockTabView`.
    type Tab: Clone + 'static;
    /// The generated `DockGroupView`.
    type Group: Clone + 'static;
    /// The generated `DockSplitterView`.
    type Splitter: Clone + 'static;

    /// Converts a tab.
    fn tab(view: &TabView) -> Self::Tab;
    /// Converts a group. `tabs` holds the group's tabs, already converted.
    fn group(view: &GroupView, tabs: ModelRc<Self::Tab>) -> Self::Group;
    /// Converts a bar.
    fn splitter(view: &SplitterView) -> Self::Splitter;
}

/// The models behind the tabs and bars of one host.
///
/// A view is shown by changing the rows that differ, never by replacing a model. Slint
/// keeps the element of a row that changes, and makes a new one for a model that is
/// replaced. The tab or bar under the pointer must outlive a press, or the press would
/// end when its first effect was shown.
pub struct Models<B: Bridge> {
    groups: Rc<VecModel<B::Group>>,
    tabs: RefCell<Vec<Rc<VecModel<B::Tab>>>>,
    splitters: Rc<VecModel<B::Splitter>>,
    shown: RefCell<HostView>,
}

impl<B: Bridge> Default for Models<B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B: Bridge> Models<B> {
    /// Models that show nothing.
    pub fn new() -> Self {
        Models {
            groups: Rc::new(VecModel::default()),
            tabs: RefCell::new(Vec::new()),
            splitters: Rc::new(VecModel::default()),
            shown: RefCell::new(HostView::default()),
        }
    }

    /// The model of the groups, for the `groups` property.
    pub fn groups(&self) -> ModelRc<B::Group> {
        self.groups.clone().into()
    }

    /// The model of the bars, for the `splitters` property.
    pub fn splitters(&self) -> ModelRc<B::Splitter> {
        self.splitters.clone().into()
    }

    /// Shows a view.
    pub fn show(&self, view: &HostView) {
        let mut shown = self.shown.borrow_mut();
        let mut tabs = self.tabs.borrow_mut();

        for (i, group) in view.groups.iter().enumerate() {
            match shown.groups.get(i) {
                Some(old) => {
                    sync(&tabs[i], &old.tabs, &group.tabs, B::tab);
                    if old.id != group.id {
                        self.groups.set_row_data(i, B::group(group, tabs[i].clone().into()));
                    }
                }
                None => {
                    let model = Rc::new(VecModel::from(group.tabs.iter().map(B::tab).collect::<Vec<_>>()));
                    self.groups.push(B::group(group, model.clone().into()));
                    tabs.push(model);
                }
            }
        }
        while tabs.len() > view.groups.len() {
            tabs.pop();
            self.groups.remove(tabs.len());
        }

        sync(&self.splitters, &shown.splitters, &view.splitters, B::splitter);
        *shown = view.clone();
    }
}

/// Makes `model` show `new`, given that it shows `old`, by changing only what differs.
fn sync<V: PartialEq, T: Clone + 'static>(
    model: &VecModel<T>,
    old: &[V],
    new: &[V],
    convert: impl Fn(&V) -> T,
) {
    for (i, item) in new.iter().enumerate() {
        match old.get(i) {
            Some(old) if old == item => {}
            Some(_) => model.set_row_data(i, convert(item)),
            None => model.push(convert(item)),
        }
    }
    while model.row_count() > new.len() {
        model.remove(model.row_count() - 1);
    }
}

# slint-docking

Docking for [Slint](https://slint.dev). An experiment.

Panels sit in tab groups, and tab groups sit in splits. The user rearranges them by dragging tabs. A panel holds any Slint content, and the content knows nothing about docking.

## Status

| Part | State |
|---|---|
| Docking inside one window | Works. Tested without a display |
| Saving and reading a layout | Works. Tested |
| A second window, moved to from code | Works. Tested without a display |
| A second window, by dragging a tab out and back | Works in the demo when the pointer is played by sent events. **Not yet tried with a real mouse** |
| Floating panels inside a window, auto-hide bars, maximise | Not done |
| Panels made at run time, such as one tab per open file | Not done |

Requires Slint 1.18.1 exactly, and Rust 1.92 or later.

## Try it

```bash
cargo run -p demo
```

```bash
cargo test --workspace --all-features
```

## What the pointer does

The behaviour follows Visual Studio Code and Dockview.

| Action | Result |
|---|---|
| Press a tab | Its panel comes to the front |
| Drag a tab onto the middle of a group | The panel joins the group as its last tab |
| Drag a tab onto the left, right, top or bottom quarter of a group | The group's area is split, and the panel takes that half |
| Drag a tab among the tabs of a group | The panel takes that place among the tabs |
| Drag a tab to within 12 pixels of an edge of the dock area, when the area holds more than one group | The panel takes that whole side, beside everything else |
| Drag the bar between two groups | The two are resized. Neither goes below its least size |
| Press the cross on a tab, or press a tab with the middle button | The panel closes |
| Let go of a tab outside the dock area | Nothing, unless the application listens. See [More than one window](#more-than-one-window) |

While a tab is dragged, the area it would take is marked. Nothing moves until the button is let go. A drop that would change nothing is not marked.

## Use

Add `slint-docking` as a dependency and `slint-docking-build` as a build dependency.

`build.rs`:

```rust
fn main() {
    slint_docking_build::compile("ui/app.slint").unwrap();
}
```

`ui/app.slint`:

```slint
import { DockArea, DockPanel } from "@docking";
export { DockBackend, DockTheme } from "@docking";

export component MainWindow inherits Window {
    DockArea {
        DockPanel { key: "browser"; title: "Browser"; Browser { } }
        DockPanel { key: "chart"; title: "Chart"; closable: false; ChartView { } }
        DockPanel { key: "inspector"; title: "Inspector"; Inspector { } }
    }
}
```

The `export` line is required. Without it Slint does not generate the Rust types that `attach!` uses.

`src/main.rs`:

```rust
use slint_docking::{Dock, Layout, Node};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;
    let dock = Dock::new(Layout::new(Node::row([
        (1.0, Node::tabs(["browser"])),
        (3.0, Node::tabs(["chart"])),
        (1.0, Node::tabs(["inspector"])),
    ])));
    slint_docking::attach!(dock, window);
    window.run()
}
```

### The `Dock`

| Method | Does |
|---|---|
| `layout()`, `set_layout(layout)` | Reads and replaces the layout |
| `apply(op)` | Applies an operation. Everything the pointer does goes through here too |
| `open(key)`, `close(key)`, `is_open(key)` | Opens, closes and asks about a panel. A closed panel opens in the group it was closed in, if any panel of that group is still placed |
| `panels()` | Every panel that is known, with its title and whether it is open |
| `on_layout_changed(f)` | Calls `f` after every change, for saving |
| `move_to_host(key, host)` | Moves a panel to another window |
| `on_drag_outside(f)`, `hover_at`, `place_at`, `area` | For taking a dragged tab from one window to another |

### Saving a layout

With the `serde` feature, `Layout` can be saved and read with any serde format. IDs are not saved. Reading is tolerant: a layout that is not in normal form is normalised, and a panel that no window declares is removed when the window is attached, so a layout saved by an older version of an application is safe to read.

### Looks

Set the properties of the `DockTheme` global, from Slint or from Rust. They default to the colours of the Slint style in use. The sizes of tabs, bars and drop zones are in `Metrics`, given to `Dock::with_metrics`.

## How it works

Slint cannot move an element from one parent to another, and cannot make an element from a description at run time. So a docking system cannot hold panels in a tree of containers, as it would in most toolkits.

Instead, every panel is declared once, as a child of the `DockArea`, and stays there. The layout is a tree of plain data in Rust. Each panel asks Rust for its rectangle and places itself. The tabs, bars and the drop mark are drawn by the `DockArea` from models that Rust fills in.

| Crate | Contains | Depends on |
|---|---|---|
| `slint-docking-layout` | The tree, the operations, the solver that gives rectangles, and hit-testing for drops | Nothing |
| `slint-docking-build` | The `.slint` components, and the function a build script calls | `slint-build` |
| `slint-docking` | The `Dock`, and the `attach!` macro that connects a window to it | The layout crate, Slint |
| `slint-docking-tests` | Tests that drive a window | All of the above |
| `examples/demo` | A window to try it in | All of the above |

Some decisions that are not obvious:

- **A layout changes only through an operation.** `Op` is plain data. An operation that is refused leaves the layout exactly as it was. After every operation the layout is in normal form: no empty group, no split of one child, no split inside a split of the same axis.
- **Rectangles are asked for, not pushed.** A panel's rectangle is bound to a pure callback that takes the size of the dock area. So when the window is resized, the rectangles follow in the same frame. Pushing rectangles from Rust would lag one frame behind.
- **Models are changed row by row, never replaced.** Slint keeps the element of a row that changes and makes a new one for a model that is replaced. The tab under the pointer must outlive the press.
- **Dragging uses `TouchArea`, not Slint's `DragArea`.** A `TouchArea` that is pressed keeps getting the pointer's moves outside the window, which taking a tab to another window needs. It also keeps all of the logic in Rust, where it is tested.
- **`attach!` is a macro.** Slint generates the Rust types of a `.slint` file in the crate that compiles it, so no function in this crate can name them. The macro only converts types. The logic is in `Dock`.
- **Panels tell of themselves by adding to a string.** A panel cannot call into Rust as it is made, because the window is made before Rust can set a callback, and Slint cannot add to an array that Rust has not given it. So each panel appends its key and title to a string property, which `attach!` reads.

## Testing

Slint's own tests run on `i-slint-backend-testing`, which needs no display and runs on mocked time. The tests here use the same backend.

| Layer | Where | Checks |
|---|---|---|
| Layout | `crates/slint-docking-layout/tests` | What each operation does. Rectangles. Drop targets. With `proptest`: any sequence of operations keeps the layout in normal form, the rectangles tile the area exactly, and every point is a target |
| Interface | `crates/slint-docking-tests/tests` | Dragging a real tab to a real place gives the expected layout, and the window shows what the layout says. A panel keeps its state. Two windows share a dock |
| Demo | `cargo run -p demo -- --screenshot-windows <folder>` | The real backend, with the pointer played by sent events |

### Testing an application

Most tests of an application need no pointer. Apply the operation and look at the layout:

```rust
dock.apply(Op::Place { panel: "log".into(), target })?;
assert_eq!(dock.layout().to_string(), "row(25: tabs(browser*), 75: tabs(chart, log*))");
```

To drive the interface, build the `.slint` file with `slint_docking_build::compile_for_tests`, which adds the debug information the testing backend needs. Tabs, panels and close buttons carry accessible roles and labels to find them by:

| Element | Role | Label |
|---|---|---|
| Tab | `Tab` | The title |
| Panel | `TabPanel` | The title |
| Close button | `Button` | `Close` and the title |

A panel that is closed, or is a tab behind another, is not visible, and the testing backend does not find it. `crates/slint-docking-tests/src/lib.rs` has helpers to copy.

The testing backend is an internal crate of Slint. Its version must match Slint's exactly, and its interface may change with any release.

## More than one window

A layout holds a list of hosts, and a window shows one host. `attach!(dock, window, host = 1)` connects a second window to the same dock.

A panel cannot leave its window, so a panel that can be in two windows is declared in both. Declare it under `if`, so that its content exists only in the window that holds it:

```slint
if DockBackend.holds("inspector", DockBackend.generation): DockPanel {
    key: "inspector";
    title: "Inspector";
    Inspector { }
}
```

| | Declared outright | Declared under `if` |
|---|---|---|
| Content exists | Always, hidden while the panel is elsewhere | Only while the window holds the panel |
| State in Slint when the panel comes back | Kept | Lost |
| Cost | Every window builds every panel | Content is built again on each move between windows |

Either way, state that must be the same in every window has to live in Rust. A global in Slint belongs to one window.

To take a tab from one window to another by dragging, the application listens with `on_drag_outside`. It is told where the pointer is in the window the tab came from. It works out which of its windows is under that point on the screen, then calls `hover_at` while the pointer moves and `place_at` when the button is let go. The demo does this in `examples/demo/src/main.rs`.

What is known, and how:

| Fact | How it is known |
|---|---|
| A panel under `if` is made and declared when its window comes to hold it | Tested |
| A tab dragged from one window is marked and placed in another | Tested without a display. Also run on the real backend with sent events |
| A pressed `TouchArea` gets moves outside its window on the desktop | Read in the source of Slint's winit backend, which sends no exit while a button is down. **Not tried with a real mouse** |
| The same on the web | It does not, by the same source. There Slint cancels the press when the pointer leaves. The tab handles a cancelled press by ending the drag, which is **not tested** |
| Where a window's content is on the screen | Slint tells where the frame is, not the content. The demo asks winit, through Slint's `unstable-winit-030` feature |
| Placing a window | Not possible on Wayland |

Not done: a mark that shows a panel would get a window of its own, the floating window following the pointer during the drag, and more than one floating window in the demo.

## Limits

- The title of a panel is set in Slint. It cannot be set from Rust, except with `Dock::register` for a panel that no window has declared yet.
- A panel that is declared under `if`, and is then removed from the window, stays known to the dock. Close it first.
- A host that is too small gives areas below the least sizes. The least sizes limit only how far a bar can be dragged.
- Tabs have one width. They shrink equally when there is no room, and do not scroll.
- A drag cannot be cancelled with the Escape key.
- Nothing can be reached with the keyboard alone.

## Licence

MIT. See [LICENSE](LICENSE).

This code depends on Slint, which is under the GPL 3.0, a royalty-free licence, or a commercial licence. You pick one of those for Slint. See [Slint's licences](https://slint.dev/pricing). The MIT licence here works with all of them.

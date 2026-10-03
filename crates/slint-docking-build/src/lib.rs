//! Build script support for `slint-docking`.
//!
//! This crate holds the `.slint` components of the docking system, and tells the Slint
//! compiler where they are, under the library name `docking`. It is a crate of its own so
//! that a build script does not have to build the Slint runtime.
//!
//! In `build.rs`:
//!
//! ```no_run
//! slint_docking_build::compile("ui/app.slint").unwrap();
//! ```
//!
//! In `ui/app.slint`:
//!
//! ```slint,ignore
//! import { DockArea, DockPanel } from "@docking";
//! export { DockBackend, DockTheme } from "@docking";
//! ```
//!
//! The second line is required. It makes Slint generate the Rust types that
//! `slint_docking::attach!` uses.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use slint_build::{CompileError, CompilerConfiguration};

/// The library name that the components are imported under, without the `@`.
pub const LIBRARY: &str = "docking";

/// The file that holds the components.
pub fn library_file() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("ui").join("dock.slint")
}

/// The library paths to give to the Slint compiler. Add your own libraries to the map
/// before passing it to [`CompilerConfiguration::with_library_paths`].
pub fn library_paths() -> HashMap<String, PathBuf> {
    HashMap::from([(LIBRARY.to_owned(), library_file())])
}

/// A compiler configuration that knows the `docking` library. Change it further as you
/// would any other, then pass it to [`slint_build::compile_with_config`].
pub fn config() -> CompilerConfiguration {
    CompilerConfiguration::new().with_library_paths(library_paths())
}

/// Compiles a `.slint` file that imports from `@docking`.
pub fn compile(path: impl AsRef<Path>) -> Result<(), CompileError> {
    println!("cargo:rerun-if-changed={}", library_file().display());
    slint_build::compile_with_config(path, config())
}

/// Compiles a `.slint` file that imports from `@docking`, with the debug information that
/// the `ElementHandle` API of `i-slint-backend-testing` needs. For crates that hold
/// interface tests.
pub fn compile_for_tests(path: impl AsRef<Path>) -> Result<(), CompileError> {
    println!("cargo:rerun-if-changed={}", library_file().display());
    slint_build::compile_with_config(path, config().with_debug_info(true))
}

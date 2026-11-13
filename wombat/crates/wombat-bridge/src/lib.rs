//! The bridge between the Wombat core and an interface written in Flutter (decision A121).
//!
//! It is a thin layer: the core does the work, and this crate carries requests and answers
//! across the boundary between the two languages with `flutter_rust_bridge`.
//!
//! - [`api`]: what Dart can call and the data that crosses. [`api::document`] has the open
//!   document (open, create, read one grid, apply a command, undo, redo, save, overwrite,
//!   close);
//!   [`api::wire`] has the serialised form of commands, change reports and errors.
//! - [`convert`]: between that form and the types of the core.
//! - [`record`]: the same form as JSON text, to record commands and play them back.
//! - `open_files`: the files that are open in this process, so that none is open twice.
//!
//! Errors cross as values ([`api::wire::BridgeError`]). Nothing here panics on what comes
//! over the bridge. Should the core ever panic in a call, `flutter_rust_bridge` catches it
//! at the boundary and Dart gets an exception, not a crash; a document that the panic may
//! have left half changed refuses every later call. A panic while Dart drops a handle would
//! not be caught by anyone, so what happens there is guarded here (decisions A129, A142
//! and A143).
//!
//! `frb_generated.rs` is written by `flutter_rust_bridge_codegen generate`, run in `app/`,
//! and is never edited by hand. It is the one place in Wombat with `unsafe` code, which
//! calling into another language needs (decision A122).

// See the end of the workspace's `Cargo.toml`.
#[cfg(not(panic = "unwind"))]
compile_error!(
    "wombat_bridge must be built with unwinding panics: with `panic = \"abort\"` a panic \
     ends the app instead of reaching Dart as an exception"
);

pub mod api;
pub mod convert;
mod open_files;
pub mod record;

#[allow(unsafe_code, clippy::unwrap_used)]
mod frb_generated;

#[cfg(test)]
mod panic_tests;

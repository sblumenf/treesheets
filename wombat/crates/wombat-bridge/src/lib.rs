//! The bridge between the Wombat core and an interface written in Flutter (decision A121).
//!
//! It is a thin layer: the core does the work, and this crate carries requests and answers
//! across the boundary between the two languages with `flutter_rust_bridge`.
//!
//! - [`api`]: what Dart can call and the data that crosses. [`api::document`] has the open
//!   document (open, create, read one grid, apply a command, undo, redo, save);
//!   [`api::wire`] has the serialised form of commands, change reports and errors.
//! - [`convert`]: between that form and the types of the core.
//! - [`record`]: the same form as JSON text, to record commands and play them back.
//!
//! Errors cross as values ([`api::wire::BridgeError`]). Nothing here panics on what comes
//! over the bridge; should the core ever panic, `flutter_rust_bridge` catches it at the
//! boundary and Dart gets an exception, not a crash.
//!
//! `frb_generated.rs` is written by `flutter_rust_bridge_codegen generate`, run in `app/`,
//! and is never edited by hand. It is the one place in Wombat with `unsafe` code, which
//! calling into another language needs (decision A122).

pub mod api;
pub mod convert;
pub mod record;

#[allow(unsafe_code, clippy::unwrap_used)]
mod frb_generated;

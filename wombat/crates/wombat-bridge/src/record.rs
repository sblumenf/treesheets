//! Commands and change reports as JSON text, for recording what was done to a document
//! and playing it back (in tests, say). It is the serialised form of `api/wire.rs` as
//! `serde` writes it: a command is `{"SetText": {"cell": "...", "text": "..."}}`, and one
//! without fields is its name, `"Run"`.
//!
//! Reading is strict: every field must be there and no other may be (decision A136).
//! One thing is read that is never written: `{"Run": null}` for `"Run"`, and likewise
//! for any other name without fields. `serde` reads a variant without fields from either
//! form, both say the same, and refusing one would take a reader written by hand for
//! every such type.

use serde::{Deserialize, Deserializer};
use wombat_core as core;

use crate::api::wire::{BridgeError, Command, Report};
use crate::convert::{command_in, command_out};

/// Reads a field that may be none, and must be there all the same: `serde` takes a
/// missing `Option` for `None` unless the field is read by a function like this one.
/// Leaving out the colour of `SetRangeColor` would otherwise remove the colour.
pub(crate) fn present<'de, D, T>(from: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(from)
}

fn bad(error: serde_json::Error) -> BridgeError {
    BridgeError::bad_request(format!("not a command or report as JSON: {error}"))
}

/// A command as JSON text.
pub fn command_to_json(command: &core::Command) -> Result<String, BridgeError> {
    serde_json::to_string(&command_out(command)?).map_err(bad)
}

/// The command that JSON text stands for, checked as one that comes over the bridge.
pub fn command_from_json(text: &str) -> Result<core::Command, BridgeError> {
    command_in(serde_json::from_str::<Command>(text).map_err(bad)?)
}

/// A change report as JSON text.
pub fn report_to_json(report: &Report) -> Result<String, BridgeError> {
    serde_json::to_string(report).map_err(bad)
}

/// The change report that JSON text stands for.
pub fn report_from_json(text: &str) -> Result<Report, BridgeError> {
    serde_json::from_str(text).map_err(bad)
}

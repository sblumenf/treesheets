//! Commands and change reports as JSON text, for recording what was done to a document
//! and playing it back (in tests, say). It is the serialised form of `api/wire.rs` as
//! `serde` writes it: a command is `{"SetText": {"cell": "...", "text": "..."}}`, and one
//! without fields is its name, `"Run"`.

use wombat_core as core;

use crate::api::wire::{BridgeError, Command, Report};
use crate::convert::{command_in, command_out};

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

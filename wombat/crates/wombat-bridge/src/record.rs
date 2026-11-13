//! Commands and change reports as JSON text, for recording what was done to a document
//! and playing it back (in tests, say). It is the serialised form of `api/wire.rs` as
//! `serde` writes it: a command is `{"SetText": {"cell": "...", "text": "..."}}`, and one
//! without fields is its name, `"Run"`.
//!
//! Reading is strict (decision A136): only what is written is read. Every field must be
//! there and no other may be, and the text must have the form that the value it stands
//! for is written in. What may differ is what JSON itself takes for the same: spaces and
//! line ends, the order of the fields of an object, and the way a number is spelt (`1`
//! for `1.0` where a fraction may stand).
//!
//! The second rule is there because `serde` reads more forms than it writes: an object
//! also from a list of its values in order (`{"InsertText": ["<id>", 1, "x"]}`, a colour
//! as `[1, 2, 3]`), and a name without fields also as `{"Run": null}`. A list has no
//! names to check, so a value in the wrong place would be read as another field.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
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

/// Whether the JSON that was `given` has the form of what is `written` for the value
/// that was read from it. Objects are compared field by field whatever their order, and
/// a whole number stands for the same number where a fraction is written.
///
/// This calls itself for what is inside a list or an object. That is as deep as the
/// types of `api/wire.rs` go, a handful of levels: they do not contain themselves, and
/// what is nested deeper than they are was refused when it was read.
fn same_form(given: &Value, written: &Value) -> bool {
    match (given, written) {
        (Value::Number(given), Value::Number(written)) => {
            given == written || (written.is_f64() && given.as_f64() == written.as_f64())
        }
        (Value::Array(given), Value::Array(written)) => {
            given.len() == written.len() && given.iter().zip(written).all(|(a, b)| same_form(a, b))
        }
        (Value::Object(given), Value::Object(written)) => {
            given.len() == written.len()
                && given
                    .iter()
                    .all(|(name, a)| written.get(name).is_some_and(|b| same_form(a, b)))
        }
        _ => given == written,
    }
}

/// Reads JSON text as a `T`, and refuses text that `serde` reads but that is not what a
/// `T` is written as. The text is read twice for that, once as a `T` and once as JSON of
/// any kind, and the `T` is written out once: reading costs about three times what it
/// would, which a record played back can afford, and no reader has to be written by hand
/// for every type.
fn read<T: DeserializeOwned + Serialize>(text: &str) -> Result<T, BridgeError> {
    let value: T = serde_json::from_str(text).map_err(bad)?;
    let given: Value = serde_json::from_str(text).map_err(bad)?;
    let written = serde_json::to_value(&value).map_err(bad)?;
    if same_form(&given, &written) {
        Ok(value)
    } else {
        Err(BridgeError::bad_request(
            "not a command or report as JSON is written: a list where an object with \
             named fields belongs, or `null` after a name that has no fields",
        ))
    }
}

/// A command as JSON text.
pub fn command_to_json(command: &core::Command) -> Result<String, BridgeError> {
    serde_json::to_string(&command_out(command)?).map_err(bad)
}

/// The command that JSON text stands for, checked as one that comes over the bridge.
pub fn command_from_json(text: &str) -> Result<core::Command, BridgeError> {
    command_in(read::<Command>(text)?)
}

/// A change report as JSON text.
pub fn report_to_json(report: &Report) -> Result<String, BridgeError> {
    serde_json::to_string(report).map_err(bad)
}

/// The change report that JSON text stands for.
pub fn report_from_json(text: &str) -> Result<Report, BridgeError> {
    read(text)
}

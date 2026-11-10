//! Helpers shared by the document and view-state formats: bounded parsing, typed access to
//! fields, and deterministic writing.

use std::fmt::Write as _;

use serde_json::Value;

use crate::error::{invalid, Error};
use crate::id::Id;
use crate::limits::MAX_FILE_BYTES;
use crate::model::{Color, Extra};

pub(crate) type Map = serde_json::Map<String, Value>;

/// Parses a whole file. serde_json limits nesting to 128 levels, so hostile input cannot
/// overflow the stack; the size limit bounds memory.
pub(crate) fn parse(bytes: &[u8]) -> Result<Value, Error> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(Error::Limit(format!(
            "file larger than {MAX_FILE_BYTES} bytes"
        )));
    }
    serde_json::from_slice(bytes).map_err(|e| Error::Syntax(e.to_string()))
}

/// Checks the `format`, `version` and `minor` fields and returns the minor version.
pub(crate) fn take_header(map: &mut Map, format: &str, supported: u32) -> Result<u32, Error> {
    match map.remove("format") {
        Some(Value::String(found)) if found == format => {}
        Some(Value::String(found)) => {
            return Err(Error::WrongFormat(format!(
                "\"format\" is {found:?}, not {format:?}"
            )))
        }
        _ => {
            return Err(Error::WrongFormat(format!(
                "no \"format\": {format:?} field"
            )))
        }
    }
    // Checked before anything else is looked at, so that a newer file is refused as a whole.
    let Some(major) = map.remove("version").as_ref().and_then(Value::as_u64) else {
        return Err(Error::WrongFormat(
            "no whole-number \"version\" field".into(),
        ));
    };
    if major > u64::from(supported) {
        return Err(Error::NewerVersion {
            found: major,
            supported,
        });
    }
    if major == 0 {
        return invalid("format version 0 does not exist");
    }
    Ok(take_u32(map, "minor", "the file")?.unwrap_or(0))
}

/// The minor version to write: the file's own if it came from a newer core, since its unknown
/// fields are written back, and otherwise ours.
pub(crate) fn minor_to_write(loaded: u32, ours: u32) -> u32 {
    loaded.max(ours)
}

pub(crate) fn into_object(value: Value, place: &str) -> Result<Map, Error> {
    match value {
        Value::Object(map) => Ok(map),
        _ => invalid(format!("{place} is not an object")),
    }
}

pub(crate) fn into_extra(map: Map) -> Extra {
    map.into_iter().collect()
}

pub(crate) fn take_string(map: &mut Map, key: &str, place: &str) -> Result<Option<String>, Error> {
    match map.remove(key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => invalid(format!("{place}: {key:?} is not a string")),
    }
}

pub(crate) fn take_bool(map: &mut Map, key: &str, place: &str) -> Result<bool, Error> {
    match map.remove(key) {
        None => Ok(false),
        Some(Value::Bool(b)) => Ok(b),
        Some(_) => invalid(format!("{place}: {key:?} is not true or false")),
    }
}

pub(crate) fn take_i64(map: &mut Map, key: &str, place: &str) -> Result<Option<i64>, Error> {
    match map.remove(key) {
        None => Ok(None),
        Some(value) => match value.as_i64() {
            Some(n) => Ok(Some(n)),
            None => invalid(format!("{place}: {key:?} is not a whole number in range")),
        },
    }
}

pub(crate) fn take_u32(map: &mut Map, key: &str, place: &str) -> Result<Option<u32>, Error> {
    match take_i64(map, key, place)?.map(u32::try_from) {
        None => Ok(None),
        Some(Ok(n)) => Ok(Some(n)),
        Some(Err(_)) => invalid(format!("{place}: {key:?} is not a whole number in range")),
    }
}

pub(crate) fn take_i32(map: &mut Map, key: &str, place: &str) -> Result<Option<i32>, Error> {
    match take_i64(map, key, place)?.map(i32::try_from) {
        None => Ok(None),
        Some(Ok(n)) => Ok(Some(n)),
        Some(Err(_)) => invalid(format!("{place}: {key:?} is not a whole number in range")),
    }
}

pub(crate) fn take_array(
    map: &mut Map,
    key: &str,
    place: &str,
) -> Result<Option<Vec<Value>>, Error> {
    match map.remove(key) {
        None => Ok(None),
        Some(Value::Array(a)) => Ok(Some(a)),
        Some(_) => invalid(format!("{place}: {key:?} is not an array")),
    }
}

pub(crate) fn take_id(map: &mut Map, key: &str, place: &str) -> Result<Option<Id>, Error> {
    match take_string(map, key, place)? {
        None => Ok(None),
        Some(text) => match Id::parse(&text) {
            Some(id) => Ok(Some(id)),
            None => invalid(format!("{place}: {key:?} is not an ID in canonical form")),
        },
    }
}

pub(crate) fn take_color(map: &mut Map, key: &str, place: &str) -> Result<Option<Color>, Error> {
    match take_string(map, key, place)? {
        None => Ok(None),
        Some(text) => match Color::parse(&text) {
            Some(color) => Ok(Some(color)),
            None => invalid(format!("{place}: {key:?} is not a colour like \"#rrggbb\"")),
        },
    }
}

/// Writes a JSON string. Only what JSON requires is escaped, always the same way.
pub(crate) fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                // Writing to a String cannot fail.
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Writes an unrecognised value in serde_json's compact form (object keys sorted).
pub(crate) fn write_value(out: &mut String, value: &Value) -> Result<(), Error> {
    let text = serde_json::to_string(value).map_err(|e| Error::Invalid(e.to_string()))?;
    out.push_str(&text);
    Ok(())
}

/// Writes the fields of one object on one line: `"a": 1, "b": 2`.
pub(crate) struct Fields<'a> {
    pub(crate) out: &'a mut String,
    first: bool,
}

impl<'a> Fields<'a> {
    pub(crate) fn new(out: &'a mut String) -> Fields<'a> {
        Fields { out, first: true }
    }

    /// Writes the separator and the key; the caller then writes the value to `out`.
    pub(crate) fn key(&mut self, key: &str) {
        if !self.first {
            self.out.push_str(", ");
        }
        self.first = false;
        write_string(self.out, key);
        self.out.push_str(": ");
    }

    pub(crate) fn string(&mut self, key: &str, value: &str) {
        self.key(key);
        write_string(self.out, value);
    }

    /// A value that needs no escaping: a number, `true`, or text known to be plain.
    pub(crate) fn display(&mut self, key: &str, value: impl std::fmt::Display) {
        self.key(key);
        let _ = write!(self.out, "{value}");
    }

    /// An ID or a colour, as a string.
    pub(crate) fn quoted(&mut self, key: &str, value: impl std::fmt::Display) {
        self.key(key);
        let _ = write!(self.out, "\"{value}\"");
    }

    pub(crate) fn extra(&mut self, extra: &Extra) -> Result<(), Error> {
        for (key, value) in extra {
            self.key(key);
            write_value(self.out, value)?;
        }
        Ok(())
    }
}

//! Helpers shared by the document and view-state formats: bounded parsing, typed access to
//! fields, and deterministic writing.

use std::fmt::{self, Write as _};
use std::path::PathBuf;

use serde_core::de::{DeserializeSeed, Error as _, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::map::Entry;
use serde_json::Value;

use crate::error::{invalid, Error};
use crate::id::Id;
use crate::limits::MAX_FILE_BYTES;
use crate::model::{Color, Extra};

pub(crate) type Map = serde_json::Map<String, Value>;

/// What kind of file is expected: the value of its `format` field, the newest major version
/// that is read, and the name of the field that holds the document's ID.
#[derive(Clone, Copy)]
pub(crate) struct Kind {
    pub(crate) format: &'static str,
    pub(crate) version: u32,
    pub(crate) id_key: &'static str,
}

fn json_error(error: serde_json::Error) -> Error {
    use serde_json::error::Category;
    match error.classify() {
        // Well-formed JSON that breaks a rule of ours, such as a repeated key.
        Category::Data => Error::Invalid(error.to_string()),
        // The caller, who knows which file was being read, fills in the path.
        Category::Io => Error::Io {
            path: PathBuf::new(),
            source: error.into(),
        },
        Category::Syntax | Category::Eof => Error::Syntax(error.to_string()),
    }
}

/// Reads any JSON value into a tree, refusing an object that repeats a key, at every level
/// (serde_json's own tree would silently keep the last).
struct Strict;

impl<'de> DeserializeSeed<'de> for Strict {
    type Value = Value;

    fn deserialize<D: serde_core::Deserializer<'de>>(self, de: D) -> Result<Value, D::Error> {
        de.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Strict {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.to_owned()))
    }

    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element_seed(Strict)? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
        let mut map = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            match map.entry(key) {
                Entry::Occupied(taken) => return Err(repeated(taken.key())),
                Entry::Vacant(free) => free.insert(access.next_value_seed(Strict)?),
            };
        }
        Ok(Value::Object(map))
    }
}

fn repeated<E: serde_core::de::Error>(key: &str) -> E {
    E::custom(format_args!("the key {key:?} is repeated in one object"))
}

/// The value of a header field: all that matters is a string or a whole number.
enum Scalar {
    Text(String),
    Whole(u64),
    Other,
}

impl<'de> Visitor<'de> for Scalar {
    type Value = Scalar;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<Scalar, E> {
        Ok(Scalar::Other)
    }

    fn visit_i64<E>(self, _: i64) -> Result<Scalar, E> {
        Ok(Scalar::Other)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Scalar, E> {
        Ok(Scalar::Whole(v))
    }

    fn visit_f64<E>(self, _: f64) -> Result<Scalar, E> {
        Ok(Scalar::Other)
    }

    fn visit_str<E>(self, v: &str) -> Result<Scalar, E> {
        Ok(Scalar::Text(v.to_owned()))
    }

    fn visit_unit<E>(self) -> Result<Scalar, E> {
        Ok(Scalar::Other)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Scalar, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Scalar::Other)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Scalar, A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(Scalar::Other)
    }
}

impl<'de> DeserializeSeed<'de> for Scalar {
    type Value = Scalar;

    fn deserialize<D: serde_core::Deserializer<'de>>(self, de: D) -> Result<Scalar, D::Error> {
        de.deserialize_any(self)
    }
}

/// The header fields of a file, as far as [`Header::scan`] read them.
struct Header {
    kind: Kind,
    /// Whether the ID is wanted, or only the verdict on `format` and `version`.
    want_id: bool,
    format: Option<Scalar>,
    version: Option<Scalar>,
    id: Option<Scalar>,
    /// Reading stopped on purpose, before the end of the file.
    stopped: bool,
}

impl Header {
    /// Reads the top-level object only as far as the header fields, building nothing else.
    /// It stops as soon as the file is known to be refused or everything wanted is known.
    /// With `want_id` unset it also stops at the first other field, leaving a header that
    /// is not at the start to the full parse.
    fn scan<'de, R: serde_json::de::Read<'de>>(
        de: &mut serde_json::Deserializer<R>,
        kind: Kind,
        want_id: bool,
    ) -> (Header, Result<(), serde_json::Error>) {
        let mut header = Header {
            kind,
            want_id,
            format: None,
            version: None,
            id: None,
            stopped: false,
        };
        let result = (&mut header).deserialize(de);
        (header, result)
    }

    fn format(&self) -> Option<&str> {
        match &self.format {
            Some(Scalar::Text(format)) => Some(format),
            _ => None,
        }
    }

    fn version(&self) -> Option<u64> {
        match self.version {
            Some(Scalar::Whole(version)) => Some(version),
            _ => None,
        }
    }

    /// Whether nothing further on in the file can change the outcome.
    fn decided(&self) -> bool {
        match (&self.format, &self.version) {
            (Some(_), _) if self.format() != Some(self.kind.format) => true,
            (Some(_), Some(_)) => {
                check_header(self.format(), self.version(), self.kind).is_err()
                    || !self.want_id
                    || self.id.is_some()
            }
            _ => false,
        }
    }
}

impl<'de> DeserializeSeed<'de> for &mut Header {
    type Value = ();

    fn deserialize<D: serde_core::Deserializer<'de>>(self, de: D) -> Result<(), D::Error> {
        de.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for &mut Header {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            let slot = match key.as_str() {
                "format" => &mut self.format,
                "version" => &mut self.version,
                key if key == self.kind.id_key => &mut self.id,
                _ if self.want_id => {
                    map.next_value::<IgnoredAny>()?;
                    continue;
                }
                _ => {
                    self.stopped = true;
                    return Err(A::Error::custom("stopped"));
                }
            };
            if slot.is_some() {
                return Err(repeated(&key));
            }
            *slot = Some(map.next_value_seed(Scalar::Other)?);
            if self.decided() {
                self.stopped = true;
                return Err(A::Error::custom("stopped"));
            }
        }
        Ok(())
    }
}

/// The rules for `format` and `version`: `format` absent or not a string and `version`
/// absent or not a whole number are passed as `None`.
fn check_header(format: Option<&str>, version: Option<u64>, kind: Kind) -> Result<(), Error> {
    match format {
        Some(found) if found == kind.format => {}
        Some(found) => {
            return Err(Error::WrongFormat(format!(
                "\"format\" is {found:?}, not {:?}",
                kind.format
            )))
        }
        None => {
            return Err(Error::WrongFormat(format!(
                "no \"format\": {:?} field",
                kind.format
            )))
        }
    }
    // Checked before anything else is looked at, so that a newer file is refused as a whole.
    let Some(major) = version else {
        return Err(Error::WrongFormat(
            "no whole-number \"version\" field".into(),
        ));
    };
    if major > u64::from(kind.version) {
        return Err(Error::NewerVersion {
            found: major,
            supported: kind.version,
        });
    }
    if major == 0 {
        return invalid("format version 0 does not exist");
    }
    Ok(())
}

/// Parses a whole file into a tree. A file of another format or of a newer major version is
/// refused from its first fields, before the rest is read. serde_json limits nesting to
/// [`crate::limits::MAX_JSON_DEPTH`] levels, so hostile input cannot overflow the stack; the
/// size limit bounds memory.
pub(crate) fn parse(bytes: &[u8], kind: Kind) -> Result<Value, Error> {
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(Error::Limit(format!(
            "file larger than {MAX_FILE_BYTES} bytes"
        )));
    }
    let (header, _) = Header::scan(
        &mut serde_json::Deserializer::from_slice(bytes),
        kind,
        false,
    );
    if header.stopped && header.decided() {
        check_header(header.format(), header.version(), kind)?;
    }
    let mut de = serde_json::Deserializer::from_slice(bytes);
    let value = Strict.deserialize(&mut de).map_err(json_error)?;
    de.end().map_err(json_error)?;
    Ok(value)
}

/// Reads only the header of a file and returns the ID in it, without building the document:
/// reading stops at the last header field, so what follows is not checked at all. A header
/// that is not at the start is still found, by skipping over what comes before it. The
/// caller bounds the length of what `reader` gives.
pub(crate) fn read_id(reader: impl std::io::Read, kind: Kind) -> Result<Id, Error> {
    let mut de = serde_json::Deserializer::from_reader(reader);
    let (header, result) = Header::scan(&mut de, kind, true);
    if !header.stopped {
        result.map_err(json_error)?;
        de.end().map_err(json_error)?;
    }
    check_header(header.format(), header.version(), kind)?;
    match &header.id {
        Some(Scalar::Text(text)) => Id::parse(text).ok_or_else(|| {
            Error::Invalid(format!("{:?} is not an ID in canonical form", kind.id_key))
        }),
        Some(_) => invalid(format!("{:?} is not a string", kind.id_key)),
        None => invalid(format!("the file has no {:?}", kind.id_key)),
    }
}

/// Checks the `format`, `version` and `minor` fields and returns the minor version.
pub(crate) fn take_header(map: &mut Map, kind: Kind) -> Result<u32, Error> {
    let format = map.remove("format");
    let version = map.remove("version");
    check_header(
        format.as_ref().and_then(Value::as_str),
        version.as_ref().and_then(Value::as_u64),
        kind,
    )?;
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

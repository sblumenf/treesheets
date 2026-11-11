//! What an index over a workspace is made of: one summary per document (decision A53).
//!
//! An index (of links here, of words for search in milestone 6) is a disposable copy of
//! what the documents say (decision A13). It is kept as one [`Summary`] per document, which
//!
//! - is made by reading the document once ([`Summary::of`]),
//! - is brought up to date after an edit from the engine's change report, at a cost that
//!   depends on the change and not on the document ([`Summary::update`]),
//! - can be written to a cache and read back ([`Summary::encode`], [`Summary::decode`]).
//!
//! [`crate::scan`] turns a workspace folder into summaries, reading each document once and
//! keeping them in a cache on disk that is checked against the files. This module has no
//! file access, so an index can also be built where there are no files.

use serde_json::Value;

use crate::edit::Changes;
use crate::id::Id;
use crate::model::Document;

/// What an index keeps about one document. See the module documentation.
pub trait Summary: Sized {
    /// Names the kind of index; part of the name of the cache file, so lower-case letters
    /// and digits only.
    const KIND: &'static str;
    /// Changes whenever [`Summary::encode`] writes something that an older
    /// [`Summary::decode`] would misread; a cache written with another number is not used.
    const VERSION: u32;

    /// Reads a whole document.
    fn of(doc: &Document) -> Self;

    /// Brings the summary up to date after a command, undo, redo or group changed `doc`:
    /// `changes` is what the editor returned, `doc` the document afterwards. The result
    /// must equal [`Summary::of`] of the document; the cost must not depend on its size.
    fn update(&mut self, doc: &Document, changes: &Changes);

    /// The summary as JSON, the same for equal summaries.
    fn encode(&self) -> Value;

    /// Reads what [`Summary::encode`] wrote; `None` for anything else.
    fn decode(value: &Value) -> Option<Self>;
}

/// An ID as 32 hexadecimal digits, the short form used in caches.
pub fn id_to_hex(id: Id, out: &mut String) {
    use std::fmt::Write;
    // Writing to a String cannot fail.
    let _ = write!(out, "{:032x}", id.as_u128());
}

/// Reads 32 lower-case hexadecimal digits.
pub fn id_from_hex(text: &str) -> Option<Id> {
    if text.len() != 32
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    u128::from_str_radix(text, 16).ok().map(Id::from_u128)
}

/// 128-bit FNV-1a: a plain checksum to tell that bytes are the ones seen before. It is not
/// proof against someone constructing a collision on purpose.
pub fn checksum(bytes: &[u8]) -> u128 {
    const BASIS: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;
    bytes.iter().fold(BASIS, |hash, byte| {
        (hash ^ u128::from(*byte)).wrapping_mul(PRIME)
    })
}

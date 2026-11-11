//! What an index over a workspace is made of: one summary per document (decision A53).
//!
//! An index (of links here, of words for search in milestone 6) is a disposable copy of
//! what the documents say (decision A13). It is kept as one [`Summary`] per document, which
//!
//! - is made by reading the document once ([`Summary::of`]),
//! - is brought up to date after an edit from the engine's change report, at a cost that
//!   depends on the change and not on the document ([`Summary::update`]),
//! - can be written to a cache and read back ([`Summary::encode`], [`Summary::decode`]), as
//!   compact bytes ([`Writer`], [`Reader`]) whose reading takes memory in proportion to
//!   their length and no more.
//!
//! [`crate::scan`] turns a workspace folder into summaries, reading each document once and
//! keeping them in a cache on disk that is checked against the files. This module has no
//! file access, so an index can also be built where there are no files.

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

    /// A number that changes when the rules by which a summary is made change (how text
    /// is folded or cut up, say), computed by applying those rules to fixed samples, so
    /// that forgetting to raise [`Summary::VERSION`] cannot make an old cache believed.
    /// It is part of every cache entry's first line.
    fn fingerprint() -> u64 {
        0
    }

    /// The summary as bytes, the same for equal summaries.
    fn encode(&self) -> Vec<u8>;

    /// Reads what [`Summary::encode`] wrote; `None` for anything else. Must not take
    /// memory out of proportion to `bytes`, whatever they hold (they come from a file).
    fn decode(bytes: &[u8]) -> Option<Self>;
}

/// Writes the bytes of a cache entry: numbers as variable-length integers, texts and byte
/// strings with their length first, IDs as 16 bytes.
#[derive(Default, Debug)]
pub struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    /// An empty writer.
    pub fn new() -> Writer {
        Writer::default()
    }

    /// One byte.
    pub fn byte(&mut self, byte: u8) {
        self.bytes.push(byte);
    }

    /// A number, in 1 to 10 bytes (7 bits each, low bits first).
    pub fn num(&mut self, mut number: u64) {
        while number >= 0x80 {
            self.bytes.push((number & 0x7f) as u8 | 0x80);
            number >>= 7;
        }
        self.bytes.push(number as u8);
    }

    /// A signed number.
    pub fn signed(&mut self, number: i64) {
        // Zigzag: small negative numbers stay short.
        self.num(((number << 1) ^ (number >> 63)) as u64);
    }

    /// Bytes, with their count first.
    pub fn bytes(&mut self, bytes: &[u8]) {
        self.num(bytes.len() as u64);
        self.bytes.extend_from_slice(bytes);
    }

    /// Text, with its length in bytes first.
    pub fn text(&mut self, text: &str) {
        self.bytes(text.as_bytes());
    }

    /// An ID, as 16 bytes.
    pub fn id(&mut self, id: Id) {
        self.bytes.extend_from_slice(&id.as_u128().to_be_bytes());
    }

    /// What was written.
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

/// Reads what a [`Writer`] wrote. Every method returns `None` when the bytes do not hold
/// what is asked for; nothing is allocated here.
#[derive(Clone, Debug)]
pub struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// Reads from the start of `bytes`.
    pub fn new(bytes: &'a [u8]) -> Reader<'a> {
        Reader { rest: bytes }
    }

    /// How many bytes are left.
    pub fn remaining(&self) -> usize {
        self.rest.len()
    }

    /// Whether everything was read.
    pub fn is_empty(&self) -> bool {
        self.rest.is_empty()
    }

    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let (taken, rest) = self.rest.split_at_checked(count)?;
        self.rest = rest;
        Some(taken)
    }

    /// One byte.
    pub fn byte(&mut self) -> Option<u8> {
        self.take(1)?.first().copied()
    }

    /// A number, which must be written in its shortest form.
    pub fn num(&mut self) -> Option<u64> {
        let mut number = 0u64;
        for shift in (0..64).step_by(7) {
            let byte = self.byte()?;
            let bits = u64::from(byte & 0x7f);
            if (bits << shift) >> shift != bits {
                return None;
            }
            number |= bits << shift;
            if byte & 0x80 == 0 {
                // One form per number, so that equal summaries have equal bytes.
                return (byte != 0 || shift == 0).then_some(number);
            }
        }
        None
    }

    /// A number that fits 32 bits.
    pub fn num32(&mut self) -> Option<u32> {
        u32::try_from(self.num()?).ok()
    }

    /// A count of things that take at least `each` bytes (not 0) each: refused if the
    /// bytes left could not hold that many, so that a count can be trusted to allocate by.
    pub fn count(&mut self, each: usize) -> Option<usize> {
        let count = usize::try_from(self.num()?).ok()?;
        (count.checked_mul(each.max(1))? <= self.rest.len()).then_some(count)
    }

    /// A signed number.
    pub fn signed(&mut self) -> Option<i64> {
        let number = self.num()?;
        Some((number >> 1) as i64 ^ -((number & 1) as i64))
    }

    /// Bytes that were written with their count.
    pub fn bytes(&mut self) -> Option<&'a [u8]> {
        let count = usize::try_from(self.num()?).ok()?;
        self.take(count)
    }

    /// Text.
    pub fn text(&mut self) -> Option<&'a str> {
        std::str::from_utf8(self.bytes()?).ok()
    }

    /// An ID.
    pub fn id(&mut self) -> Option<Id> {
        let bytes: [u8; 16] = self.take(16)?.try_into().ok()?;
        Some(Id::from_u128(u128::from_be_bytes(bytes)))
    }
}

/// A fast 128-bit checksum of bytes, eight at a time, to tell that they are the ones seen
/// before: any change within one stretch of eight bytes always changes it, other damage
/// all but certainly. Like [`checksum`] it is no proof against a collision made on purpose.
pub fn digest(bytes: &[u8]) -> u128 {
    const A: u64 = 0x9e37_79b9_7f4a_7c15;
    const B: u64 = 0xc2b2_ae3d_27d4_eb4f;
    let (mut one, mut two) = (0x6c62_272e_07bb_0142u64, 0x62b8_2175_6295_c58du64);
    let mut chunks = bytes.chunks_exact(8);
    for chunk in &mut chunks {
        let word = u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8]));
        // Each line is a one-to-one function of its state for a given word.
        one = (one ^ word).wrapping_mul(A).rotate_left(29);
        two = (two.rotate_left(31) ^ word).wrapping_mul(B);
    }
    let mut last = [0u8; 8];
    for (to, from) in last.iter_mut().zip(chunks.remainder()) {
        *to = *from;
    }
    let word = u64::from_le_bytes(last);
    one = (one ^ word).wrapping_mul(A).rotate_left(29) ^ bytes.len() as u64;
    two = (two.rotate_left(31) ^ word).wrapping_mul(B) ^ (bytes.len() as u64).rotate_left(32);
    one = (one ^ (one >> 32)).wrapping_mul(B);
    two = (two ^ (two >> 29)).wrapping_mul(A);
    (u128::from(one) << 64) | u128::from(two ^ one.rotate_left(17))
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

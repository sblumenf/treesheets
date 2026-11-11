//! Permanent IDs of cells and documents (decision A5).
//!
//! An ID is 128 bits, written as a UUID (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, lower case).
//! The core never reads randomness itself: new IDs come from an [`IdSource`] the caller passes in.

use std::fmt;

/// A 128-bit identifier, unique across all documents when it comes from a well-seeded source.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(u128);

impl Id {
    /// Makes an ID from its 128 bits.
    pub const fn from_u128(bits: u128) -> Id {
        Id(bits)
    }

    /// The 128 bits of the ID.
    pub const fn as_u128(self) -> u128 {
        self.0
    }

    /// Reads the text form. Only the canonical form is accepted (36 characters, lower-case
    /// hexadecimal digits, hyphens after digits 8, 12, 16 and 20), so that one ID has one text.
    pub fn parse(text: &str) -> Option<Id> {
        let bytes = text.as_bytes();
        if bytes.len() != 36 {
            return None;
        }
        let mut bits: u128 = 0;
        for (i, &b) in bytes.iter().enumerate() {
            if matches!(i, 8 | 13 | 18 | 23) {
                if b != b'-' {
                    return None;
                }
                continue;
            }
            let digit = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                _ => return None,
            };
            bits = (bits << 4) | u128::from(digit);
        }
        Some(Id(bits))
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let b = self.0;
        write!(
            f,
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            b >> 96,
            (b >> 80) & 0xffff,
            (b >> 64) & 0xffff,
            (b >> 48) & 0xffff,
            b & 0xffff_ffff_ffff
        )
    }
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub Id);

        impl $name {
            /// Reads the canonical text form, see [`Id::parse`].
            pub fn parse(text: &str) -> Option<$name> {
                Id::parse(text).map($name)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

id_type!(
    /// The permanent ID of a cell.
    CellId
);
id_type!(
    /// The permanent ID of a document.
    DocId
);

/// Where new IDs come from. An interface seeds one from the operating system's randomness;
/// tests use a fixed seed so that they are repeatable.
pub trait IdSource {
    /// Returns an ID that has not been returned before and will not be returned again.
    fn next_id(&mut self) -> Id;
}

/// An [`IdSource`] that derives random (version 4) UUIDs from a 256-bit seed with the
/// xoshiro256** generator. IDs are only unique across documents if every seed is fresh
/// randomness; a fixed seed repeats the same IDs and is for tests only.
#[derive(Clone, Debug)]
pub struct SeededIds {
    state: [u64; 4],
}

impl SeededIds {
    /// Starts from 32 bytes of seed, which should come from the operating system.
    pub fn from_seed(seed: [u8; 32]) -> SeededIds {
        let mut state = [0u64; 4];
        for (word, chunk) in state.iter_mut().zip(seed.chunks(8)) {
            for &b in chunk {
                *word = (*word << 8) | u64::from(b);
            }
        }
        // The generator cannot leave the all-zero state.
        if state == [0; 4] {
            state = [0x9e37_79b9_7f4a_7c15, 1, 2, 3];
        }
        SeededIds { state }
    }

    /// Starts from a small number. For tests: the same number gives the same IDs.
    pub fn from_u64(seed: u64) -> SeededIds {
        // splitmix64 spreads the small seed over the whole state.
        let mut x = seed;
        let mut state = [0u64; 4];
        for word in &mut state {
            x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = x;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            *word = z ^ (z >> 31);
        }
        if state == [0; 4] {
            state = [0x9e37_79b9_7f4a_7c15, 1, 2, 3];
        }
        SeededIds { state }
    }

    fn next_u64(&mut self) -> u64 {
        let [s0, s1, s2, s3] = self.state;
        let result = s1.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s1 << 17;
        let s2 = s2 ^ s0;
        let s3 = s3 ^ s1;
        let s1 = s1 ^ s2;
        let s0 = s0 ^ s3;
        let s2 = s2 ^ t;
        let s3 = s3.rotate_left(45);
        self.state = [s0, s1, s2, s3];
        result
    }
}

impl IdSource for SeededIds {
    fn next_id(&mut self) -> Id {
        let bits = (u128::from(self.next_u64()) << 64) | u128::from(self.next_u64());
        // Mark as a version 4, variant 1 UUID, as other tools expect of a random UUID.
        let bits = (bits & !(0xf << 76)) | (0x4 << 76);
        let bits = (bits & !(0x3 << 62)) | (0x2 << 62);
        Id(bits)
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn text_form_round_trips() {
        let id = Id::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
        assert_eq!(id.to_string(), "01234567-89ab-cdef-0123-456789abcdef");
        assert_eq!(Id::parse(&id.to_string()), Some(id));
        assert_eq!(
            Id::from_u128(0).to_string(),
            "00000000-0000-0000-0000-000000000000"
        );
        assert_eq!(
            Id::parse(&Id::from_u128(u128::MAX).to_string())
                .unwrap()
                .as_u128(),
            u128::MAX
        );
    }

    #[test]
    fn only_the_canonical_text_is_accepted() {
        for bad in [
            "",
            "01234567-89ab-cdef-0123-456789abcde",
            "01234567-89ab-cdef-0123-456789abcdef0",
            "01234567-89AB-cdef-0123-456789abcdef",
            "0123456789abcdef0123456789abcdef----",
            "01234567 89ab-cdef-0123-456789abcdef",
            "01234567-89ab-cdef-0123-456789abcdeg",
            "é1234567-89ab-cdef-0123-456789abcde",
        ] {
            assert_eq!(Id::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn seeded_ids_are_repeatable_distinct_and_version_4() {
        let mut a = SeededIds::from_u64(7);
        let mut b = SeededIds::from_u64(7);
        let mut seen = HashSet::new();
        for _ in 0..10_000 {
            let id = a.next_id();
            assert_eq!(id, b.next_id());
            assert!(seen.insert(id));
            let text = id.to_string();
            assert_eq!(text.as_bytes()[14], b'4');
            assert!(matches!(text.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
        }
        assert_ne!(
            SeededIds::from_u64(8).next_id(),
            SeededIds::from_u64(7).next_id()
        );
    }

    #[test]
    fn zero_seed_still_produces_ids() {
        let mut ids = SeededIds::from_seed([0; 32]);
        assert_ne!(ids.next_id(), ids.next_id());
    }
}

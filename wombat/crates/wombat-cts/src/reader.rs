//! Reading the primitive values of a `.cts` file from a byte slice, without ever reading past
//! its end: numbers as `wxDataInputStream` reads them, and strings as `ReadString` does.

use crate::CtsError;

/// A cursor over bytes. Every read is bounds-checked and returns an error at the end.
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    /// TreeSheets' own loader switches its stream to big-endian while skipping the chunks of
    /// an image stored without a length (format versions before 22) and never switches back,
    /// so the numbers that follow in the header are read that way. This mirrors it.
    pub(crate) big_endian: bool,
    /// How many strings were not valid UTF-8 and were therefore read as empty.
    pub(crate) undecodable: usize,
}

fn truncated<T>() -> Result<T, CtsError> {
    Err(CtsError::Corrupt("the file ends too early".into()))
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Reader<'a> {
        Reader {
            data,
            pos: 0,
            big_endian: false,
            undecodable: 0,
        }
    }

    /// The number of bytes not yet read.
    pub(crate) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// The bytes not yet read.
    pub(crate) fn rest(&self) -> &'a [u8] {
        self.data.get(self.pos..).unwrap_or_default()
    }

    /// The next `n` bytes.
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], CtsError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|end| *end <= self.data.len());
        match end.and_then(|end| self.data.get(self.pos..end).map(|bytes| (bytes, end))) {
            Some((bytes, end)) => {
                self.pos = end;
                Ok(bytes)
            }
            None => truncated(),
        }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], CtsError> {
        match <[u8; N]>::try_from(self.take(N)?) {
            Ok(array) => Ok(array),
            Err(_) => truncated(),
        }
    }

    pub(crate) fn u8(&mut self) -> Result<u8, CtsError> {
        Ok(u8::from_le_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, CtsError> {
        let bytes = self.array()?;
        Ok(if self.big_endian {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        })
    }

    /// A 32-bit number that TreeSheets reads into a signed `int`.
    pub(crate) fn i32(&mut self) -> Result<i32, CtsError> {
        Ok(self.u32()? as i32)
    }

    pub(crate) fn i64(&mut self) -> Result<i64, CtsError> {
        let bytes = self.array()?;
        Ok(if self.big_endian {
            i64::from_be_bytes(bytes)
        } else {
            i64::from_le_bytes(bytes)
        })
    }

    /// A string as `wxDataOutputStream::WriteString` writes it: the number of bytes as a
    /// 32-bit number, then that many bytes of UTF-8, with no terminator. As in wxWidgets,
    /// bytes that are not valid UTF-8 give an empty string (counted in `undecodable`).
    pub(crate) fn string(&mut self) -> Result<String, CtsError> {
        let len = self.u32()?;
        let bytes = match usize::try_from(len) {
            Ok(len) => self.take(len)?,
            Err(_) => return truncated(),
        };
        match std::str::from_utf8(bytes) {
            Ok(text) => Ok(text.to_string()),
            Err(_) => {
                self.undecodable = self.undecodable.saturating_add(1);
                Ok(String::new())
            }
        }
    }

    /// A floating-point number as `wxDataOutputStream::WriteDouble` writes it by default: not
    /// 8 bytes, but the 10 bytes of an 80-bit IEEE extended number, most significant first
    /// whatever the byte order of the stream. Infinity and NaN both read as infinity.
    pub(crate) fn extended(&mut self) -> Result<f64, CtsError> {
        let [b0, b1, m0, m1, m2, m3, m4, m5, m6, m7] = self.array::<10>()?;
        let exponent = i32::from(u16::from_be_bytes([b0 & 0x7f, b1]));
        let high = u32::from_be_bytes([m0, m1, m2, m3]);
        let low = u32::from_be_bytes([m4, m5, m6, m7]);
        let part = |mantissa: u32, shift: i32| {
            if mantissa == 0 {
                0.0
            } else {
                f64::from(mantissa) * 2f64.powi(exponent.saturating_sub(shift))
            }
        };
        let value = if exponent == 0x7fff {
            f64::INFINITY
        } else {
            part(high, 16383 + 31) + part(low, 16383 + 63)
        };
        Ok(if b0 & 0x80 != 0 { -value } else { value })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_little_endian_unless_switched() {
        let mut r = Reader::new(&[1, 2, 3, 4, 1, 2, 3, 4, 0xff, 0xff, 0xff, 0xff]);
        assert_eq!(r.u32().unwrap(), 0x0403_0201);
        r.big_endian = true;
        assert_eq!(r.u32().unwrap(), 0x0102_0304);
        assert_eq!(r.i32().unwrap(), -1);
        assert_eq!(r.remaining(), 0);
        assert!(r.u8().is_err());
        let mut r = Reader::new(&[0x15, 0xcd, 0x5b, 0x07, 0, 0, 0, 0]);
        assert_eq!(r.i64().unwrap(), 123_456_789);
    }

    #[test]
    fn strings_are_a_byte_count_then_utf8() {
        // "a", e-acute (2 bytes), a non-BMP emoji (4 bytes): 7 bytes, 3 code points.
        let text = "a\u{e9}\u{1F600}";
        let mut bytes = vec![7, 0, 0, 0];
        bytes.extend_from_slice(text.as_bytes());
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        let mut r = Reader::new(&bytes);
        assert_eq!(r.string().unwrap(), text);
        assert_eq!(r.string().unwrap(), "");
        assert_eq!((r.remaining(), r.undecodable), (0, 0));
    }

    #[test]
    fn a_string_may_hold_a_zero_byte() {
        let mut r = Reader::new(&[3, 0, 0, 0, b'a', 0, b'b']);
        assert_eq!(r.string().unwrap(), "a\0b");
    }

    #[test]
    fn invalid_utf8_reads_as_empty_and_is_counted() {
        // Latin-1 "e-acute", an overlong "/", a surrogate, and a cut-off sequence.
        for bad in [
            &[0xe9u8][..],
            &[0xc0, 0xaf],
            &[0xed, 0xa0, 0x80],
            &[0xe2, 0x82],
        ] {
            let mut bytes = (bad.len() as u32).to_le_bytes().to_vec();
            bytes.extend_from_slice(bad);
            bytes.push(7);
            let mut r = Reader::new(&bytes);
            assert_eq!(r.string().unwrap(), "");
            assert_eq!(r.undecodable, 1);
            // The bytes were still consumed: the stream stays in step.
            assert_eq!(r.u8().unwrap(), 7);
        }
    }

    #[test]
    fn a_string_longer_than_the_data_is_an_error_not_an_allocation() {
        let mut r = Reader::new(&[0xff, 0xff, 0xff, 0xff, b'a']);
        assert!(matches!(r.string(), Err(CtsError::Corrupt(_))));
        let mut r = Reader::new(&[2, 0, 0]);
        assert!(r.string().is_err());
    }

    #[test]
    fn extended_numbers_match_the_ones_in_the_example_files() {
        let read = |hex: [u8; 10]| Reader::new(&hex).extended().unwrap();
        assert_eq!(read([0x3f, 0xff, 0x80, 0, 0, 0, 0, 0, 0, 0]), 1.0);
        assert_eq!(read([0x40, 0x00, 0xc0, 0, 0, 0, 0, 0, 0, 0]), 3.0);
        assert_eq!(read([0x3f, 0xfe, 0x80, 0, 0, 0, 0, 0, 0, 0]), 0.5);
        assert_eq!(read([0xbf, 0xff, 0xc0, 0, 0, 0, 0, 0, 0, 0]), -1.5);
        assert_eq!(read([0; 10]), 0.0);
        assert_eq!(read([0x7f, 0xff, 0x80, 0, 0, 0, 0, 0, 0, 0]), f64::INFINITY);
        assert_eq!(
            read([0x7f, 0xfe, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0]),
            f64::INFINITY
        );
        assert_eq!(read([0x00, 0x01, 0x80, 0, 0, 0, 0, 0, 0, 0]), 0.0);
        // 1 + 2^-32: the low half of the mantissa is used.
        assert_eq!(
            read([0x3f, 0xff, 0x80, 0, 0, 0, 0x80, 0, 0, 0]),
            1.0 + 2f64.powi(-32)
        );
        assert!(Reader::new(&[0; 9]).extended().is_err());
    }
}

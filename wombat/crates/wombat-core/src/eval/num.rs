//! Numbers and characters as TreeSheets' formulas and operations read and write them.
//!
//! TreeSheets takes these from the C and C++ libraries. They are written out here so that the
//! core gives the same answer on every machine and in every locale (decision A60):
//! [`strict_num`] is `Text::StrictNum` (wxWidgets' `ToCDouble`, which is `std::from_chars`
//! after an optional `+` and `0x`), [`stream_num`] is `Text::GetNum` (reading a `double` from
//! a C++ stream), [`num_text`] is `Text::NumText`, and the character classes are those of
//! the C library in a UTF-8 locale.

/// The spaces that `wxString::Trim` removes and that a number may start with.
fn is_ascii_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r')
}

/// The text without the ASCII spaces around it (`wxString::Trim` on both sides).
pub(crate) fn trim(text: &str) -> &str {
    text.trim_matches(is_ascii_space)
}

/// The text without the ASCII spaces at its end (`wxString::Trim`).
pub(crate) fn trim_end(text: &str) -> &str {
    text.trim_end_matches(is_ascii_space)
}

/// The number that a whole text is, if it is one and finite: `Text::StrictNum`. Spaces
/// around it are ignored. Besides decimal numbers (`12`, `-1.5`, `.5`, `5.`, `1e3`) it takes
/// what the libraries behind TreeSheets take: one `+` in front, and hexadecimal after `0x`
/// (`0x10`, `0x1p3`). A number that starts with a zero is read without that zero, which
/// makes `0e5` and `0.` no numbers and `0-5` minus five. Too large for a double, or so
/// small that it would become zero, is no number either.
pub fn strict_num(text: &str) -> Option<f64> {
    let s = trim(text);
    let s = s.strip_prefix('+').unwrap_or(s);
    let bytes = s.as_bytes();
    let value = if bytes.len() > 1 && bytes.first() == Some(&b'0') {
        match bytes.get(1) {
            Some(b'x' | b'X') => hex(s.get(2..)?),
            _ => decimal(s.get(1..)?),
        }
    } else {
        decimal(s)
    }?;
    value.is_finite().then_some(value)
}

/// `std::from_chars` for a decimal double, which must be the whole of `s`.
fn decimal(s: &str) -> Option<f64> {
    let digits = s.strip_prefix('-').unwrap_or(s);
    let (mantissa, exponent) = match digits.find(['e', 'E']) {
        Some(at) => (digits.get(..at)?, Some(digits.get(at.checked_add(1)?..)?)),
        None => (digits, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all_digits = |part: &str| part.bytes().all(|b| b.is_ascii_digit());
    if (whole.is_empty() && fraction.is_empty()) || !all_digits(whole) || !all_digits(fraction) {
        return None;
    }
    if let Some(exponent) = exponent {
        let power = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if power.is_empty() || !all_digits(power) {
            return None;
        }
    }
    let value: f64 = s.parse().ok()?;
    let zero = mantissa.bytes().all(|b| b == b'0' || b == b'.');
    // Out of range: too large, or not zero as written but zero as a double.
    (value.is_finite() && (value != 0.0 || zero)).then_some(value)
}

/// Two to the power `exp`, for `exp` in the range of normal doubles.
fn pow2(exp: i64) -> f64 {
    let biased = exp.clamp(-1022, 1023).saturating_add(1023);
    f64::from_bits(u64::try_from(biased).unwrap_or(0) << 52)
}

/// `std::from_chars` for a hexadecimal double without its `0x`, which must be the whole of
/// `s`: hexadecimal digits with an optional point, then optionally `p` and a power of two.
fn hex(s: &str) -> Option<f64> {
    let (negative, s) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (mantissa, power) = match s.find(['p', 'P']) {
        Some(at) => (s.get(..at)?, Some(s.get(at.checked_add(1)?..)?)),
        None => (s, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    // The digits as a 64-bit number with the highest bit set, the lost bits remembered in
    // `sticky`, times two to the power `exp`.
    let (mut bits, mut exp, mut sticky) = (0u64, 0i64, false);
    for (digit, in_fraction) in whole
        .chars()
        .map(|c| (c, false))
        .chain(fraction.chars().map(|c| (c, true)))
    {
        let digit = u64::from(digit.to_digit(16)?);
        if bits >> 60 == 0 {
            bits = (bits << 4) | digit;
            if in_fraction {
                exp = exp.saturating_sub(4);
            }
        } else {
            sticky |= digit != 0;
            if !in_fraction {
                exp = exp.saturating_add(4);
            }
        }
    }
    if let Some(power) = power {
        let (minus, digits) = match power.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, power.strip_prefix('+').unwrap_or(power)),
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        // Anything beyond this is out of range whatever the digits are.
        let power = digits.parse::<i64>().unwrap_or(1 << 40).min(1 << 40);
        exp = exp.saturating_add(if minus { -power } else { power });
    }
    let sign = if negative { -1.0 } else { 1.0 };
    if bits == 0 {
        return Some(0.0 * sign);
    }
    let shift = bits.leading_zeros();
    bits <<= shift;
    // The value is in [2^top, 2^(top+1)).
    let top = exp.saturating_sub(i64::from(shift)).saturating_add(63);
    if top > 1023 {
        return None;
    }
    // How many bits a double has for a number of this size (fewer for the smallest ones).
    let keep = if top >= -1022 {
        53
    } else {
        53i64.saturating_sub(-1022i64.saturating_sub(top))
    };
    if keep < 0 {
        return None;
    }
    let keep = u32::try_from(keep).ok()?;
    let (mut kept, rest) = if keep == 0 {
        (0u64, bits)
    } else {
        (bits >> (64 - keep), bits << keep)
    };
    let half = 1u64 << 63;
    if rest > half || (rest == half && (sticky || kept & 1 == 1)) {
        kept = kept.saturating_add(1);
    }
    if kept == 0 {
        return None;
    }
    // kept * 2^(top - keep + 1), in two exact steps so that nothing is rounded twice.
    let scale = top.saturating_sub(i64::from(keep)).saturating_add(1);
    #[allow(clippy::cast_precision_loss)] // at most 2^53
    let mut value = kept as f64;
    let first = scale.clamp(-900, 900);
    value *= pow2(first);
    value *= pow2(scale.saturating_sub(first));
    value.is_finite().then_some(value * sign)
}

/// The number a text starts with, as a C++ stream reads a `double`: `Text::GetNum`, which
/// the operations use. Text that does not start with a number is 0, a text that is empty
/// or only spaces is not a number (NaN), and a number too large for a double is the
/// largest double.
pub fn stream_num(text: &str) -> f64 {
    let s = text.trim_start_matches(is_ascii_space);
    if s.is_empty() {
        return f64::NAN;
    }
    // What the stream takes: a sign, digits with one point, and an exponent with a sign
    // once there is a digit. It then fails if that is not a whole number ("1e", ".").
    let mut taken = String::new();
    let (mut digit, mut point, mut exponent) = (false, false, false);
    let mut chars = s.chars().peekable();
    if let Some(sign) = chars.next_if(|c| matches!(c, '+' | '-')) {
        taken.push(sign);
    }
    while let Some(c) = chars.peek().copied() {
        if c.is_ascii_digit() {
            digit = true;
        } else if c == '.' && !point && !exponent {
            point = true;
        } else if matches!(c, 'e' | 'E') && !exponent && digit {
            exponent = true;
            taken.push('e');
            chars.next();
            if let Some(sign) = chars.next_if(|c| matches!(c, '+' | '-')) {
                taken.push(sign);
            }
            continue;
        } else {
            break;
        }
        taken.push(c);
        chars.next();
    }
    match taken.parse::<f64>() {
        Ok(v) if v == f64::INFINITY => f64::MAX,
        Ok(v) if v == f64::NEG_INFINITY => f64::MIN,
        Ok(v) => v,
        Err(_) => 0.0,
    }
}

/// A number as text: `Text::NumText`. Never an exponent, at most ten decimals, no zeroes at
/// the end. So 0.1 + 0.2 is `0.3`, a third is `0.3333333333`, and a tiny negative number is
/// `-0`.
pub fn num_text(d: f64) -> String {
    if d.is_nan() {
        return if d.is_sign_negative() { "-nan" } else { "nan" }.to_string();
    }
    if d.is_infinite() {
        return if d < 0.0 { "-inf" } else { "inf" }.to_string();
    }
    let mut s = format!("{:.10}", d.abs());
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if d.is_sign_negative() {
        s.insert(0, '-');
    }
    s
}

/// The result of an operation on doubles with the sign of "not a number" that an x86
/// processor gives it, which is what TreeSheets shows (`nan` or `-nan`): a NaN that went in
/// comes out, and one that is made new is negative. Other machines differ, so the core
/// does not leave it to them.
pub(crate) fn x86(a: f64, b: f64, result: f64) -> f64 {
    if !result.is_nan() {
        result
    } else if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else {
        -f64::NAN.abs()
    }
}

/// Ranges of the characters that are numbers but neither digits nor letters (Unicode
/// category No, such as superscripts and fractions): the C library does not count them as
/// alphanumeric, and the standard library cannot tell them from digits.
const OTHER_NUMBERS: &[(u32, u32)] = &[
    (0xB2, 0xB3),
    (0xB9, 0xB9),
    (0xBC, 0xBE),
    (0x9F4, 0x9F9),
    (0xB72, 0xB77),
    (0xBF0, 0xBF2),
    (0xC78, 0xC7E),
    (0xD58, 0xD5E),
    (0xD70, 0xD78),
    (0xF2A, 0xF33),
    (0x1369, 0x137C),
    (0x17F0, 0x17F9),
    (0x19DA, 0x19DA),
    (0x2070, 0x2070),
    (0x2074, 0x2079),
    (0x2080, 0x2089),
    (0x2150, 0x215F),
    (0x2189, 0x2189),
    (0x2460, 0x249B),
    (0x24EA, 0x24FF),
    (0x2776, 0x2793),
    (0x2CFD, 0x2CFD),
    (0x3192, 0x3195),
    (0x3220, 0x3229),
    (0x3248, 0x324F),
    (0x3251, 0x325F),
    (0x3280, 0x3289),
    (0x32B1, 0x32BF),
    (0xA830, 0xA835),
    (0x10107, 0x10133),
    (0x10175, 0x10178),
    (0x1018A, 0x1018B),
    (0x102E1, 0x102FB),
    (0x10320, 0x10323),
    (0x10858, 0x1085F),
    (0x10879, 0x1087F),
    (0x108A7, 0x108AF),
    (0x108FB, 0x108FF),
    (0x10916, 0x1091B),
    (0x109BC, 0x109BD),
    (0x109C0, 0x109CF),
    (0x109D2, 0x109FF),
    (0x10A40, 0x10A48),
    (0x10A7D, 0x10A7E),
    (0x10A9D, 0x10A9F),
    (0x10AEB, 0x10AEF),
    (0x10B58, 0x10B5F),
    (0x10B78, 0x10B7F),
    (0x10BA9, 0x10BAF),
    (0x10CFA, 0x10CFF),
    (0x10E60, 0x10E7E),
    (0x10F1D, 0x10F26),
    (0x10F51, 0x10F54),
    (0x10FC5, 0x10FCB),
    (0x11052, 0x11065),
    (0x111E1, 0x111F4),
    (0x1173A, 0x1173B),
    (0x118EA, 0x118F2),
    (0x11C5A, 0x11C6C),
    (0x11FC0, 0x11FD4),
    (0x16B5B, 0x16B61),
    (0x16E80, 0x16E96),
    (0x1D2C0, 0x1D2D3),
    (0x1D2E0, 0x1D2F3),
    (0x1D360, 0x1D378),
    (0x1E8C7, 0x1E8CF),
    (0x1EC71, 0x1ECAB),
    (0x1ECAD, 0x1ECAF),
    (0x1ECB1, 0x1ECB4),
    (0x1ED01, 0x1ED2D),
    (0x1ED2F, 0x1ED3D),
    (0x1F100, 0x1F10C),
];

/// A character of a name that needs no quotes in a formula: a letter, a digit or `_`
/// (`FormulaEvaluator::IsNameChar`, with the C library's `iswalnum` in a UTF-8 locale).
pub fn is_name_char(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_alphanumeric() || c == '_';
    }
    let code = u32::from(c);
    c.is_alphanumeric() && !OTHER_NUMBERS.iter().any(|(a, b)| (*a..=*b).contains(&code))
}

/// A space between the parts of a formula (the C library's `iswspace` in a UTF-8 locale:
/// Unicode's spaces without the no-break ones).
pub fn is_space(c: char) -> bool {
    c.is_whitespace() && !matches!(c, '\u{85}' | '\u{a0}' | '\u{2007}' | '\u{202f}')
}

/// The single upper-case character of `c` (the C library's `towupper`): where Unicode's
/// upper case is several characters (`ß`), the character stays as it is.
pub fn upper(c: char) -> char {
    let code = u32::from(c);
    // Greek letters with a iota below: upper case is two letters, but there is a single
    // character too, which `towupper` gives.
    let single = match code {
        0x1F80..=0x1F87 | 0x1F90..=0x1F97 | 0x1FA0..=0x1FA7 => code.checked_add(8),
        0x1FB3 | 0x1FC3 | 0x1FF3 => code.checked_add(9),
        _ => None,
    };
    if let Some(single) = single.and_then(char::from_u32) {
        return single;
    }
    let mut all = c.to_uppercase();
    match (all.next(), all.next()) {
        (Some(one), None) => one,
        _ => c,
    }
}

/// The single lower-case character of `c` (the C library's `towlower`).
pub fn lower(c: char) -> char {
    if c == '\u{130}' {
        return 'i';
    }
    let mut all = c.to_lowercase();
    match (all.next(), all.next()) {
        (Some(one), None) => one,
        _ => c,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn strict_numbers_are_what_treesheets_takes() {
        // Checked against the real app (see referee/formulas).
        for (text, value) in [
            ("5", 5.0),
            (" 7 ", 7.0),
            ("1e5", 1e5),
            ("-.5", -0.5),
            ("5.", 5.0),
            ("+3", 3.0),
            ("00", 0.0),
            ("0-5", -5.0),
            ("+-5", -5.0),
            ("0x10", 16.0),
            ("0X1F", 31.0),
            ("+0x10", 16.0),
            ("0x-10", -16.0),
            ("0x1p3", 8.0),
            ("0x1P+2", 4.0),
            ("0x.8", 0.5),
            ("0x1e2", 482.0),
            ("0x1.fffffffffffff8p0", 2.0),
            ("1.e2", 100.0),
            ("1e-310", 1e-310),
            ("4e-324", 5e-324),
            ("0x1p-1074", 5e-324),
            ("0x1.8p-1074", 1e-323),
        ] {
            assert_eq!(strict_num(text), Some(value), "{text:?}");
        }
        for text in [
            "",
            " ",
            "abc",
            "12abc",
            "1,5",
            "0e5",
            "0.",
            "0x",
            "0x1p",
            "-0x10",
            "+ 5",
            "--5",
            "5\u{a0}",
            "1e-400",
            "2e-324",
            "1e400",
            "0x1p1024",
            "0x1p-1075",
            "inf",
            "nan",
            "infinity",
            "0nan",
            "0inf",
            "1e5.",
            ".",
            "-",
            "+",
            "e2",
            ".e2",
            "0x1p 2",
            "0x 1",
            "0xg",
            "000x10",
            "0.5e",
            "1d5",
            "1f",
            "\u{663}",
            "1_0",
            "++5",
        ] {
            assert_eq!(strict_num(text), None, "{text:?}");
        }
    }

    #[test]
    fn stream_numbers_take_the_start_of_the_text() {
        for (text, value) in [
            ("12", 12.0),
            ("  12abc", 12.0),
            ("abc", 0.0),
            ("-.5x", -0.5),
            ("1e", 0.0),
            ("1e+", 0.0),
            ("1e3", 1000.0),
            ("1.2.3", 1.2),
            (".", 0.0),
            ("-", 0.0),
            ("0x10", 0.0),
            ("1e999", f64::MAX),
            ("-1e999", f64::MIN),
            ("inf", 0.0),
        ] {
            assert_eq!(stream_num(text), value, "{text:?}");
        }
        assert!(stream_num("").is_nan());
        assert!(stream_num("  ").is_nan());
    }

    #[test]
    fn numbers_are_written_without_exponent_or_trailing_zeroes() {
        for (value, text) in [
            (0.0, "0"),
            (-0.0, "-0"),
            (1.0, "1"),
            (2.5, "2.5"),
            (0.1 + 0.2, "0.3"),
            (1.0 / 3.0, "0.3333333333"),
            (-1e-11, "-0"),
            (1e22, "10000000000000000000000"),
            (100.0, "100"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
            (f64::NAN, "nan"),
        ] {
            assert_eq!(num_text(value), text);
        }
        assert_eq!(
            num_text(x86(f64::INFINITY, f64::INFINITY, f64::NAN)),
            "-nan"
        );
    }

    #[test]
    fn characters_follow_the_c_library() {
        assert!(is_name_char('a') && is_name_char('_') && is_name_char('ö') && is_name_char('٣'));
        assert!(is_name_char('Ⅳ') && is_name_char('Ⓐ'));
        assert!(!is_name_char('²') && !is_name_char('½') && !is_name_char('①'));
        assert!(!is_name_char(' ') && !is_name_char('-') && !is_name_char('\u{e47}'));
        assert!(is_space(' ') && is_space('\u{3000}') && is_space('\u{2003}'));
        assert!(!is_space('\u{a0}') && !is_space('\u{2007}'));
        let up: String = "ᾀ ŉ ﬁ ǅ i ß é".chars().map(upper).collect();
        assert_eq!(up, "ᾈ ŉ ﬁ Ǆ I ß É");
        let low: String = "ǅ Σ ẞ İ É".chars().map(lower).collect();
        assert_eq!(low, "ǆ σ ß i é");
    }
}

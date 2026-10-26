//! Conversion of the numeric ECO code (`ecoT`) stored in the `IndexEntry` to
//! its standard textual representation (`"B01"`, `"C10"`, ...).
//!
//! Ported directly from `eco_ToString()` (`src/misc.cpp` of the provided
//! SCID source tree). Numbering: no code = 0, `A00` = 1, `A01` = 132
//! (each base code = previous + 131; the remaining 130 sub-codes are
//! the SCID extension `a`, `a1`..`a4`, `b`, `b1`..`b4`, ..., `z`, `z1`..`z4`).
//! Only the base code (3 characters) is produced here: the
//! SCID-specific extensions have no standard PGN equivalent and are not
//! needed for a valid ECO tag.

/// Converts a raw ECO code into a `"A00"`..`"E99"` string, or `None` if the
/// code is 0 (`ECO_None`, no classification).
#[must_use]
pub fn eco_to_string(eco_code: u16) -> Option<String> {
    if eco_code == 0 {
        return None;
    }
    let code = u32::from(eco_code) - 1;
    let basic_code = code / 131; // 131 = 26 * 5 + 1 sub-codes

    let letter = u8::try_from(basic_code / 100).ok()? + b'A';
    let tens   = (basic_code % 100) / 10;
    let units  = basic_code % 10;
    if letter > b'E' {
        // Out-of-bounds code ("A".."E" expected): corrupted data, ignored
        // rather than producing an invalid ECO tag.
        return None;
    }

    Some(format!("{}{tens}{units}", letter as char))
}

/// The exact inverse of [`eco_to_string`]'s arithmetic: parses a strict
/// `"A00"`..`"E99"` base code (uppercase letter, exactly two ASCII
/// digits, nothing else) back into its raw numeric form.
///
/// Only ever produces the FIRST raw code of the base code's group (SCID
/// sub-code `0`, i.e. no `a`/`a1`../`z4` suffix) — the same information
/// loss already present on the write side, since [`eco_to_string`] never
/// emits a sub-code either. Round-tripping a game through PGN text can
/// therefore change (but never invalidate) its exact raw `eco_code`
/// while always preserving the base code a human or another tool sees in
/// the `[ECO]` tag.
///
/// Returns `None` for anything that isn't exactly this 3-character shape
/// (wrong length, lowercase, out-of-range letter/digits, ...) — never a
/// panic on malformed external input.
#[must_use]
pub fn eco_from_string(s: &str) -> Option<u16> {
    let bytes = s.as_bytes();
    let [letter, tens, units] = *bytes else { return None };
    if !(b'A'..=b'E').contains(&letter) || !tens.is_ascii_digit() || !units.is_ascii_digit() {
        return None;
    }

    let basic_code = u32::from(letter - b'A') * 100 + u32::from(tens - b'0') * 10 + u32::from(units - b'0');
    u16::try_from(basic_code * 131 + 1).ok()
}

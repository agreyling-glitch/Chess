//! Reading the `.sn5` name file: an APPEND-ONLY journal, WITHOUT a header,
//! varint encoding (LEB128-style) — see `docs/scid-si5-specification.md` §3,
//! reverse-engineered from `codec_scid5.h` (`NameBase`/`.sn5` handling).
//!
//! A format VERY different from `.sn4` (no alphabetical sorting, no
//! prefix compression, no header, IDs assigned implicitly by
//! insertion order rather than read from disk) — but producing the
//! SAME neutral structure [`crate::names::NameTables`] as `si4::namebase`.
//!
//! Unlike `.si4`/`.sn4`, this format has NO verifiable magic byte
//! at opening time (see §2.1/§3.1 of the spec): any file of the
//! correct structural shape would be accepted with no way to tell it
//! apart — this is a limitation of the format itself, not of this
//! implementation.

use crate::error::ScidError;
use crate::names::NameTables;

/// Reads and reconstructs the 4 name tables of the `.sn5` file — walks all
/// records until EOF (no explicit end marker).
///
/// # Errors
/// [`ScidError::Truncated`] if a varint or a string is truncated at the
/// end of the file.
pub fn read_namebase(data: &[u8]) -> Result<NameTables, ScidError> {
    let mut tables = NameTables::default();
    let mut pos: usize = 0;

    while pos < data.len() {
        let (header, consumed) =
            read_varint(&data[pos..]).ok_or(ScidError::Truncated(".sn5 : varint corrompu"))?;
        pos += consumed;

        let name_type = header & 0b111;
        // Length in bytes of the string that follows (`header >> 3`) —
        // implicitly bounded by the remaining file size via `data.get`
        // below, no need to check a theoretical maximum here.
        let len = usize::try_from(header >> 3).map_err(|_| ScidError::Truncated(".sn5 : longueur de nom absurde"))?;

        // `checked_add` (robustness audit 12/07/2026, finding 3) rather
        // than a plain `pos + len`: `len` comes from an almost entirely
        // file-controlled varint, so on a hypothetical 32-bit target
        // with a multi-gigabyte `.sn5` file this addition could in
        // theory overflow `usize` before ever reaching the `data.get`
        // bounds check below — harmless in practice on the 64-bit
        // desktop targets this crate is built for (`len` tops out
        // around 2^61, nowhere near overflowing a 64-bit `usize`), but
        // free to close outright regardless of target width.
        let end = pos.checked_add(len).ok_or(ScidError::Truncated(".sn5 : chaîne de nom tronquée"))?;
        let bytes = data.get(pos..end).ok_or(ScidError::Truncated(".sn5 : chaîne de nom tronquée"))?;
        pos = end;

        let name = String::from_utf8_lossy(bytes).into_owned();
        match name_type {
            0 => tables.players.push(name),
            1 => tables.events.push(name),
            2 => tables.sites.push(name),
            3 => tables.rounds.push(name),
            // Type 4 ("NAME_INFO" — database metadata: description,
            // autoload, custom flag labels...) and any other unknown type
            // (future format evolution) are both silently ignored here:
            // not a browsable name, no sequential ID, no current use of
            // this metadata by this crate (V1) — but its string was
            // already read/consumed above, so reading of the following
            // records stays correctly synchronized either way.
            _ => {}
        }
    }

    Ok(tables)
}

/// Serializes the 4 name tables back into `.sn5` journal bytes — the
/// write-side mirror of [`read_namebase`].
///
/// Each table is written as a contiguous run of records (all players,
/// then all events, then all sites, then all rounds); this differs from
/// how a real SCID session would interleave them (in the order names were
/// first encountered across games) but decodes back to the exact same 4
/// tables either way: [`read_namebase`] assigns each ID from the position
/// within its OWN type's stream, independently of how the 4 types are
/// interleaved in the file (see its `match name_type` above) — the
/// simplest correct encoding, not a bug-for-bug reproduction of the
/// reference writer's interleaving.
#[must_use]
pub fn write_namebase(names: &NameTables) -> Vec<u8> {
    let mut out = Vec::new();
    write_group(&mut out, 0, &names.players);
    write_group(&mut out, 1, &names.events);
    write_group(&mut out, 2, &names.sites);
    write_group(&mut out, 3, &names.rounds);
    out
}

/// Writes one `(header varint, raw UTF-8 bytes)` record per name of
/// `list`, tagged with `name_type` (see [`read_namebase`]'s `match`).
fn write_group(out: &mut Vec<u8>, name_type: u8, list: &[String]) {
    for name in list {
        let bytes = name.as_bytes();
        let header = (bytes.len() as u64) << 3 | u64::from(name_type);
        write_varint(out, header);
        out.extend_from_slice(bytes);
    }
}

/// Encodes `value` as a LEB128-style varint (least-significant 7-bit
/// groups first, bit 0x80 = continuation flag) — the write-side mirror of
/// [`read_varint`].
fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        #[allow(clippy::cast_possible_truncation)]
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

/// Decodes a varint integer (LEB128-style, least-significant 7-bit groups
/// first, bit 0x80 = continuation flag) at the start of `data`.
///
/// Returns `(value, bytes_consumed)`, or `None` if `data` runs out
/// before an end-of-varint byte (bit 0x80 clear) is encountered,
/// or if the varint exceeds 64 bits (defense against a corrupted file).
fn read_varint(data: &[u8]) -> Option<(u64, usize)> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    let mut consumed: usize = 0;

    loop {
        let byte = *data.get(consumed)?;
        consumed += 1;
        result |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Some((result, consumed));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_then_read_roundtrip() {
        let mut names = NameTables::default();
        names.players.push("Carlsen, Magnus".into());
        names.players.push("Nepomniachtchi, Ian".into());
        names.events.push("World Championship".into());
        names.sites.push("Astana".into());
        names.rounds.push("1".into());

        let bytes = write_namebase(&names);
        let decoded = read_namebase(&bytes).unwrap();

        assert_eq!(decoded.players, names.players);
        assert_eq!(decoded.events, names.events);
        assert_eq!(decoded.sites, names.sites);
        assert_eq!(decoded.rounds, names.rounds);
    }

    #[test]
    fn test_write_empty_tables_roundtrips_to_empty_file() {
        let names = NameTables::default();
        let bytes = write_namebase(&names);
        assert!(bytes.is_empty());
        let decoded = read_namebase(&bytes).unwrap();
        assert!(decoded.players.is_empty());
    }
}

//! Reading the `.sn4` name file: a header (36 bytes) then, for each
//! name type (PLAYER, EVENT, SITE, ROUND, in that order), the
//! front-coded records sorted alphabetically.
//!
//! Ported from `codec_scid4.cpp::namefileRead`, cross-checked against
//! `docs/scid-si4-specification.md` §3.

use crate::bytes::{BeReader, BeWriter, Eof};
use crate::error::ScidError;
use crate::names::NameTables;

pub const NAMEBASE_MAGIC: &[u8; 8] = b"Scid.sn\0";
pub const HEADER_SIZE: usize = 36;

/// Longest name (in UTF-8 bytes) the `.sn4` format can represent: the
/// on-disk `length` field is a single byte (see [`write_group`]). A name
/// longer than this is truncated (at a valid UTF-8 character boundary) when
/// written — this crate has no realistic case of a player/event/site/round
/// name anywhere near 255 bytes, but truncating defensively is safer than
/// panicking or silently corrupting the following record.
const MAX_NAME_LEN: usize = 255;

/// Index of the 4 name types, in the order they appear in the file.
pub const NAME_PLAYER: usize = 0;
pub const NAME_EVENT: usize = 1;
pub const NAME_SITE: usize = 2;
pub const NAME_ROUND: usize = 3;
const NUM_NAME_TYPES: usize = 4;

/// Reads and reconstructs the 4 name tables of the `.sn4` file.
///
/// # Errors
/// [`ScidError::BadMagic`] if the magic number does not match;
/// [`ScidError::Truncated`] if the file is incomplete or inconsistent
/// (front-coded prefix longer than the name, out-of-bounds ID...).
pub fn read_namebase(data: &[u8]) -> Result<NameTables, ScidError> {
    let mut r = BeReader::new(data);
    let trunc = |e: Eof| e.truncated("en-tête .sn4");

    let magic = r.read_bytes(8).map_err(trunc)?;
    if magic != NAMEBASE_MAGIC {
        return Err(ScidError::BadMagic);
    }
    r.skip(4).map_err(trunc)?; // timeStamp, obsolete, unused

    let mut num_names = [0u32; NUM_NAME_TYPES];
    for n in &mut num_names {
        *n = r.read_u24().map_err(trunc)?;
    }
    let mut max_freq = [0u32; NUM_NAME_TYPES];
    for f in &mut max_freq {
        *f = r.read_u24().map_err(trunc)?;
    }
    debug_assert_eq!(r.position(), HEADER_SIZE);

    // Defensive bound (11/07/2026, robustness audit finding 2.1):
    // `num_names[nt]` is fully file-controlled (raw `u24`, up to
    // 16,777,215) and was used directly below to size a `Vec<String>`
    // allocation, unlike `si4::index::read_all_entries` which already
    // clamps its own entry count by the file's real size. A corrupted or
    // malicious `.sn4` file of a few dozen bytes could claim ~16M names
    // per type and trigger ~1.6 GB of allocation before a single further
    // byte is validated — worse than an ordinary panic, since a failed
    // allocation aborts the whole process in Rust rather than unwinding.
    // Each record needs at least 1 on-disk byte (id/frequency/length are
    // never omitted), so no type can legitimately claim more names than
    // there are bytes left in the file after the header: reject early
    // with the usual `Truncated` error instead of allocating.
    let remaining_after_header = data.len().saturating_sub(r.position());
    if num_names.iter().any(|&n| n as usize > remaining_after_header) {
        return Err(ScidError::Truncated(".sn4 : nombre de noms incohérent avec la taille du fichier"));
    }

    let mut tables = NameTables {
        players: vec![String::new(); num_names[NAME_PLAYER] as usize],
        events:  vec![String::new(); num_names[NAME_EVENT] as usize],
        sites:   vec![String::new(); num_names[NAME_SITE] as usize],
        rounds:  vec![String::new(); num_names[NAME_ROUND] as usize],
    };

    for nt in [NAME_PLAYER, NAME_EVENT, NAME_SITE, NAME_ROUND] {
        let dest = match nt {
            NAME_PLAYER => &mut tables.players,
            NAME_EVENT  => &mut tables.events,
            NAME_SITE   => &mut tables.sites,
            _           => &mut tables.rounds,
        };
        read_one_type(&mut r, num_names[nt], max_freq[nt], dest)?;
    }

    Ok(tables)
}

/// Reads the `num_names` front-coded records of a name type, and
/// places them into `dest` at the index given by their `id` field (NOT the
/// reading order, which is alphabetical — see spec §3.3).
fn read_one_type(
    r: &mut BeReader<'_>,
    num_names: u32,
    max_freq: u32,
    dest: &mut [String],
) -> Result<(), ScidError> {
    let trunc = |e: Eof| e.truncated(".sn4 : enregistrement de nom");
    // Front-coding operates on raw BYTES (not necessarily valid UTF-8
    // characters taken in isolation): we therefore accumulate the previous
    // name as `Vec<u8>`, not as `String`, to avoid any risk of a panic
    // from slicing in the middle of a multi-byte character (accented
    // names...). The conversion to UTF-8 (with replacement, "lossy")
    // only happens once the full name has been reconstructed.
    let mut prev_name: Vec<u8> = Vec::new();

    for i in 0..num_names {
        let id = if num_names >= 65536 {
            r.read_u24().map_err(trunc)?
        } else {
            u32::from(r.read_u16().map_err(trunc)?)
        };

        // Usage frequency: obsolete, but must be read to correctly
        // advance the cursor (its on-disk size depends on the
        // overall maximum frequency of the type, not on the individual value).
        if max_freq >= 65536 {
            r.skip(3).map_err(trunc)?;
        } else if max_freq >= 256 {
            r.skip(2).map_err(trunc)?;
        } else {
            r.skip(1).map_err(trunc)?;
        }

        let length = usize::from(r.read_u8().map_err(trunc)?);
        let prefix = if i > 0 { usize::from(r.read_u8().map_err(trunc)?) } else { 0 };
        if prefix > length || prefix > prev_name.len() {
            return Err(ScidError::Truncated(".sn4 : préfixe front-codé invalide"));
        }

        let new_chars = length - prefix;
        let suffix = r.read_bytes(new_chars).map_err(trunc)?;

        let mut name_bytes = Vec::with_capacity(length);
        name_bytes.extend_from_slice(&prev_name[..prefix]);
        name_bytes.extend_from_slice(suffix);

        let slot = dest
            .get_mut(id as usize)
            .ok_or(ScidError::Truncated(".sn4 : ID de nom hors bornes"))?;
        *slot = String::from_utf8_lossy(&name_bytes).into_owned();
        prev_name = name_bytes;
    }
    Ok(())
}

// -----------------------------------------------------------------------
// Writing
// -----------------------------------------------------------------------

/// Serializes the 4 name tables back into `.sn4` bytes (header + one
/// front-coded, alphabetically sorted group per name type) — the
/// write-side mirror of [`read_namebase`].
///
/// Usage frequency is not tracked by [`NameTables`] (it is read-and-discarded
/// on the read side too, see [`read_one_type`]): every record is written
/// with frequency 0 and each type's header `max_freq` is 0, which makes
/// [`read_one_type`] use the narrowest, 1-byte frequency field throughout
/// — a real SCID installation recomputes actual frequencies itself the
/// next time it touches the database, exactly as it already recomputes
/// the derived per-entry fields [`crate::si4::index::write_index_entry`]
/// never populates.
#[must_use]
pub fn write_namebase(names: &NameTables) -> Vec<u8> {
    let groups = [&names.players, &names.events, &names.sites, &names.rounds];

    let mut w = BeWriter::with_capacity(HEADER_SIZE);
    w.write_bytes(NAMEBASE_MAGIC);
    w.write_u32(0); // timeStamp, obsolete, unused
    for list in groups {
        w.write_u24(u32::try_from(list.len()).unwrap_or(0x00FF_FFFF));
    }
    for _ in groups {
        w.write_u24(0); // max_freq: frequency is never tracked, see doc above
    }
    debug_assert_eq!(w.position(), HEADER_SIZE);

    for list in groups {
        write_group(&mut w, list);
    }
    w.into_bytes()
}

/// Writes one front-coded, alphabetically sorted group of names (all
/// records of a single name type) — the write-side mirror of
/// [`read_one_type`]. Each name keeps the ID given by its position in
/// `list` (its insertion order, i.e. the ID referenced by an `IndexEntry`),
/// which is independent of the alphabetical order records are written in
/// — exactly as [`read_one_type`] places each decoded record at `dest[id]`
/// rather than at its position in the file.
fn write_group(w: &mut BeWriter, list: &[String]) {
    let wide_id = list.len() >= 65536;

    let mut order: Vec<(u32, Vec<u8>)> = list
        .iter()
        .enumerate()
        .map(|(i, name)| (u32::try_from(i).unwrap_or(u32::MAX), truncate_name_bytes(name)))
        .collect();
    order.sort_by(|a, b| a.1.cmp(&b.1));

    let mut prev_name: &[u8] = &[];
    for (i, (id, name_bytes)) in order.iter().enumerate() {
        if wide_id {
            w.write_u24(*id);
        } else {
            #[allow(clippy::cast_possible_truncation)]
            w.write_u16(*id as u16);
        }
        w.write_u8(0); // frequency, always 0, see `write_namebase`'s doc

        #[allow(clippy::cast_possible_truncation)]
        let length = name_bytes.len() as u8; // <= MAX_NAME_LEN, enforced by truncate_name_bytes
        w.write_u8(length);

        let prefix_len = if i > 0 { common_prefix_len(prev_name, name_bytes) } else { 0 };
        if i > 0 {
            #[allow(clippy::cast_possible_truncation)]
            w.write_u8(prefix_len as u8); // <= MAX_NAME_LEN, both operands are
        }
        w.write_bytes(&name_bytes[prefix_len..]);

        prev_name = name_bytes;
    }
}

/// Truncates `name` to at most [`MAX_NAME_LEN`] UTF-8 bytes, at a valid
/// character boundary — see [`MAX_NAME_LEN`]'s doc.
fn truncate_name_bytes(name: &str) -> Vec<u8> {
    if name.len() <= MAX_NAME_LEN {
        return name.as_bytes().to_vec();
    }
    let mut end = MAX_NAME_LEN;
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    name.as_bytes()[..end].to_vec()
}

/// Length, in bytes, of the common prefix of `a` and `b` — always
/// `<= min(a.len(), b.len())`, itself always `<= MAX_NAME_LEN` since both
/// come from [`truncate_name_bytes`], so the result always fits the
/// format's single-byte `prefix` field.
fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_then_read_namebase_roundtrip() {
        let mut names = NameTables::default();
        names.players.push("Carlsen, Magnus".into());
        names.players.push("Nepomniachtchi, Ian".into());
        names.players.push("Caruana, Fabiano".into()); // shares a "Car" prefix with Carlsen alphabetically
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
    fn test_write_preserves_ids_independent_of_alphabetical_order() {
        // Insertion order ("Zebra" = id 0) differs from alphabetical
        // order ("Anna" comes first on disk) — the IDs used by
        // `IndexEntry` must survive the round trip regardless.
        let mut names = NameTables::default();
        names.players.push("Zebra".into());
        names.players.push("Anna".into());

        let bytes = write_namebase(&names);
        let decoded = read_namebase(&bytes).unwrap();

        assert_eq!(decoded.players[0], "Zebra");
        assert_eq!(decoded.players[1], "Anna");
    }

    #[test]
    fn test_write_empty_tables_roundtrips_to_header_only() {
        let names = NameTables::default();
        let bytes = write_namebase(&names);
        assert_eq!(bytes.len(), HEADER_SIZE);
        let decoded = read_namebase(&bytes).unwrap();
        assert!(decoded.players.is_empty());
    }

    #[test]
    fn test_write_truncates_a_name_longer_than_255_bytes() {
        let mut names = NameTables::default();
        let long_name: String = "a".repeat(300);
        names.players.push(long_name);

        let bytes = write_namebase(&names);
        let decoded = read_namebase(&bytes).unwrap();
        assert_eq!(decoded.players[0].len(), MAX_NAME_LEN);
    }

    #[test]
    fn test_common_prefix_len() {
        assert_eq!(common_prefix_len(b"Carlsen", b"Caruana"), 3); // "Car" commun, puis 'l' != 'u'
        assert_eq!(common_prefix_len(b"abc", b"abc"), 3);
        assert_eq!(common_prefix_len(b"abc", b"xyz"), 0);
        assert_eq!(common_prefix_len(b"ab", b"abcdef"), 2);
    }
}

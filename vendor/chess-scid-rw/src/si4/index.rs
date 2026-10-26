//! Reading the `.si4` index file: a header (182 bytes) then the
//! `IndexEntry` records (47 bytes each).
//!
//! Ported from `codec_scid4.cpp` (`readIndexHeader`, `decodeIndexEntry`),
//! cross-checked against `docs/scid-si4-specification.md` §2.

use crate::bytes::{BeReader, BeWriter, Eof};
use crate::entry::IndexEntry;
use crate::error::ScidError;

pub const INDEX_MAGIC: &[u8; 8] = b"Scid.si\0";
pub const INDEX_HEADER_SIZE: usize = 182;
pub const INDEX_ENTRY_SIZE: usize = 47;
const SCID_DESC_LEN: usize = 107;
const CUSTOM_FLAG_MAX: usize = 6;
const CUSTOM_FLAG_DESC_LEN: usize = 8;

/// Width, in bits, of the `offset` field of a si4 `IndexEntry` record (a
/// plain 32-bit big-endian word — see [`write_index_entry`]) — used by
/// [`crate::scid_common::LoadedDatabase::add_game`] to reject a games file
/// that has grown beyond what this format can address (~4 GB), unlike
/// si5's much wider 47-bit offset (see `si5::index::OFFSET_BITS`).
pub(crate) const OFFSET_BITS: u32 = 32;

/// Header of the `.si4` file (first 182 bytes).
#[derive(Debug, Clone)]
pub struct Si4Header {
    pub version:     u16,
    pub base_type:   u32,
    pub num_games:   u32,
    pub auto_load:   u32,
    pub description: String,
}

/// Reads the header (182 bytes) at the start of the `.si4` file.
///
/// # Errors
/// [`ScidError::BadMagic`] if the magic number does not match;
/// [`ScidError::BadVersion`] if the version is not 400 (the only si4 version
/// supported by this implementation); [`ScidError::Truncated`] if the
/// file is shorter than 182 bytes.
pub fn read_header(data: &[u8]) -> Result<Si4Header, ScidError> {
    let mut r = BeReader::new(data);

    let magic = r.read_bytes(8).map_err(|e| e.truncated("en-tête .si4"))?;
    if magic != INDEX_MAGIC {
        return Err(ScidError::BadMagic);
    }

    let trunc = |e: Eof| e.truncated("en-tête .si4");
    let version   = r.read_u16().map_err(trunc)?;
    let base_type = r.read_u32().map_err(trunc)?;
    let num_games = r.read_u24().map_err(trunc)?;
    let auto_load = r.read_u24().map_err(trunc)?;
    let description = r.read_fixed_cstr(SCID_DESC_LEN + 1).map_err(trunc)?;

    // Only version 400 (current si4) is supported for reading by this
    // V1 implementation; si3 databases (version < 400) do not have this
    // 54-byte section and have a header of a different size.
    if version != 400 {
        return Err(ScidError::BadVersion(version));
    }
    for _ in 0..CUSTOM_FLAG_MAX {
        r.skip(CUSTOM_FLAG_DESC_LEN + 1).map_err(trunc)?;
    }
    debug_assert_eq!(r.position(), INDEX_HEADER_SIZE);

    Ok(Si4Header { version, base_type, num_games, auto_load, description })
}

/// Decodes an `IndexEntry` (exactly [`INDEX_ENTRY_SIZE`] bytes).
///
/// # Errors
/// [`ScidError::Truncated`] if `data` is fewer than 47 bytes.
pub fn read_index_entry(data: &[u8]) -> Result<IndexEntry, ScidError> {
    let mut r = BeReader::new(data);
    let trunc = |e: Eof| e.truncated("IndexEntry .si4");

    let offset = u64::from(r.read_u32().map_err(trunc)?);

    let len_low   = u32::from(r.read_u16().map_err(trunc)?);
    let len_flags = r.read_u8().map_err(trunc)?;
    let length = ((u32::from(len_flags) & 0x80) << 9) | len_low;

    let flags16 = u32::from(r.read_u16().map_err(trunc)?);
    let full_flags = ((u32::from(len_flags) & 0x3F) << 16) | flags16;
    let non_standard_start = (full_flags & 1) != 0; // bit 0 = IDX_FLAG_START
    let deleted = (full_flags & 0x08) != 0; // bit 3 = IDX_FLAG_DELETE

    let wb_high    = u32::from(r.read_u8().map_err(trunc)?);
    let white_low  = u32::from(r.read_u16().map_err(trunc)?);
    let black_low  = u32::from(r.read_u16().map_err(trunc)?);
    let white_id = ((wb_high & 0xF0) << 12) | white_low;
    let black_id = ((wb_high & 0x0F) << 16) | black_low;

    let esr_high   = u32::from(r.read_u8().map_err(trunc)?);
    let event_low  = u32::from(r.read_u16().map_err(trunc)?);
    let site_low   = u32::from(r.read_u16().map_err(trunc)?);
    let round_low  = u32::from(r.read_u16().map_err(trunc)?);
    let event_id = ((esr_high & 0xE0) << 11) | event_low;
    let site_id  = ((esr_high & 0x1C) << 14) | site_low;
    let round_id = ((esr_high & 0x03) << 16) | round_low;

    let var_counts = r.read_u16().map_err(trunc)?;
    #[allow(clippy::cast_possible_truncation)]
    let result = ((var_counts >> 12) & 0x0F) as u8;

    let eco_code = r.read_u16().map_err(trunc)?;

    // Combined Dates field (§2.8): Date (low 20 bits) + relative EventDate
    // (high 12 bits). Only the Date is kept in V1 (EventDate is
    // not a standard PGN tag and is not used by `import_one`).
    let date_edate = r.read_u32().map_err(trunc)?;
    let raw_date = date_edate & 0xFFFFF;

    let white_elo_raw = r.read_u16().map_err(trunc)?;
    let black_elo_raw = r.read_u16().map_err(trunc)?;
    let white_elo = white_elo_raw & 0x0FFF;
    let black_elo = black_elo_raw & 0x0FFF;

    // FinalMatSig (4 bytes): derived field, not needed for a valid PGN.
    r.skip(4).map_err(trunc)?;
    // Low NumHalfMoves (1 byte) + combined byte (HomePawnData[0]/high
    // NumHalfMoves): neither is needed to reconstruct the PGN
    // (the actual number of half-moves is derived from actually decoding the moves).
    r.skip(2).map_err(trunc)?;
    // HomePawnData[1..8]: derived heuristic field, ignored.
    r.skip(8).map_err(trunc)?;

    debug_assert_eq!(r.position(), INDEX_ENTRY_SIZE);

    Ok(IndexEntry {
        offset,
        length,
        white_id,
        black_id,
        event_id,
        site_id,
        round_id,
        result,
        eco_code,
        date: raw_date,
        white_elo,
        black_elo,
        non_standard_start,
        deleted,
    })
}

/// Reads all `IndexEntry` records of the `.si4` file (after the header).
///
/// The number of games announced by the header (`num_games`) is compared to
/// the one derivable from the actual file size; the smaller of the two
/// is used, out of caution against a truncated file (defense in depth,
/// not a guarantee of integrity — a corrupted file can still produce
/// individually invalid entries, detected while decoding each
/// game).
///
/// # Errors
/// [`ScidError::Truncated`] if an entry is incomplete.
pub fn read_all_entries(data: &[u8], header: &Si4Header) -> Result<Vec<IndexEntry>, ScidError> {
    let body = &data[INDEX_HEADER_SIZE.min(data.len())..];
    let size_derived = body.len() / INDEX_ENTRY_SIZE;
    let n = (header.num_games as usize).min(size_derived);

    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        let start = i * INDEX_ENTRY_SIZE;
        let entry = read_index_entry(&body[start..start + INDEX_ENTRY_SIZE])?;
        entries.push(entry);
    }
    Ok(entries)
}

// -----------------------------------------------------------------------
// Writing
// -----------------------------------------------------------------------

/// Encodes the 182-byte `.si4` header — the write-side mirror of
/// [`read_header`]. Always writes version 400 (the only version this
/// crate reads or writes), `base_type = 0`, `auto_load = 0`, an empty
/// description, and zeroed custom-flag descriptions: none of these are
/// tracked by [`crate::names::NameTables`]/[`IndexEntry`] (the neutral
/// structures this crate builds a si4 database from), so a real SCID
/// installation will recompute or ignore them exactly as it already
/// ignores the per-entry derived fields (see [`write_index_entry`]).
///
/// `num_games` is masked down to the 24 bits the field actually stores
/// (see [`write_index_entry`]'s doc for this crate's general masking
/// philosophy) — `crate::scid_common::LoadedDatabase` never realistically
/// grows anywhere near the 16,777,215 cap this implies.
#[must_use]
pub fn write_header(num_games: u32) -> Vec<u8> {
    let mut w = BeWriter::with_capacity(INDEX_HEADER_SIZE);
    w.write_bytes(INDEX_MAGIC);
    w.write_u16(400); // version
    w.write_u32(0); // base_type
    w.write_u24(num_games);
    w.write_u24(0); // auto_load
    w.write_fixed_cstr("", SCID_DESC_LEN + 1);
    for _ in 0..CUSTOM_FLAG_MAX {
        w.write_fixed_cstr("", CUSTOM_FLAG_DESC_LEN + 1);
    }
    let bytes = w.into_bytes();
    debug_assert_eq!(bytes.len(), INDEX_HEADER_SIZE);
    bytes
}

/// Encodes an `IndexEntry` back into exactly [`INDEX_ENTRY_SIZE`] bytes —
/// the write-side mirror of [`read_index_entry`] (see its doc for the full
/// bit layout this inverts field by field).
///
/// Fields this crate never populates when reading (`FinalMatSig`,
/// `StoredLineCode`, most of `NumHalfMoves`/the variation-comment-NAG
/// counts, `EventDate`, `HomePawnData`, and any flag bit other than
/// `START`/`DELETE`) are written as zero — exactly as
/// [`crate::si5::index::write_index_entry`] already does for the
/// equivalent si5 fields.
///
/// `entry`'s numeric fields are masked down to the bit width si4 actually
/// stores for each of them (narrower than si5's for most fields — e.g. 20
/// bits for `white_id`/`black_id` vs si5's 28, and a plain 32-bit `offset`
/// vs si5's 47) rather than rejected outright: a caller that needs a hard
/// guarantee should validate them before reaching this function — see
/// [`crate::scid_common::LoadedDatabase::add_game`], which is the only
/// writer of `offset`/`length` in this crate and validates them against
/// the format's limits (via [`OFFSET_BITS`]) before ever constructing the
/// entry.
#[must_use]
pub fn write_index_entry(entry: &IndexEntry) -> [u8; INDEX_ENTRY_SIZE] {
    let mut w = BeWriter::with_capacity(INDEX_ENTRY_SIZE);

    #[allow(clippy::cast_possible_truncation)]
    w.write_u32((entry.offset & 0xFFFF_FFFF) as u32);

    let length17 = entry.length & 0x1_FFFF;
    #[allow(clippy::cast_possible_truncation)]
    w.write_u16((length17 & 0xFFFF) as u16);
    #[allow(clippy::cast_possible_truncation)]
    let length_bit16 = ((length17 >> 16) & 1) as u8;

    // Only START (bit 0) and DELETE (bit 3) are ever represented in the
    // neutral `IndexEntry` — every other bit of the 22-bit raw Flags field
    // (promoted/annotation markers `import_one` never needed) is written
    // as zero.
    let full_flags = u32::from(entry.non_standard_start) | (u32::from(entry.deleted) << 3);
    #[allow(clippy::cast_possible_truncation)]
    let len_flags = (length_bit16 << 7) | (((full_flags >> 16) & 0x3F) as u8);
    w.write_u8(len_flags);
    #[allow(clippy::cast_possible_truncation)]
    w.write_u16((full_flags & 0xFFFF) as u16);

    let white_id = entry.white_id & 0xF_FFFF; // 20 bits
    let black_id = entry.black_id & 0xF_FFFF; // 20 bits
    #[allow(clippy::cast_possible_truncation)]
    let wb_high = ((((white_id >> 16) & 0xF) << 4) | ((black_id >> 16) & 0xF)) as u8;
    w.write_u8(wb_high);
    #[allow(clippy::cast_possible_truncation)]
    w.write_u16((white_id & 0xFFFF) as u16);
    #[allow(clippy::cast_possible_truncation)]
    w.write_u16((black_id & 0xFFFF) as u16);

    let event_id = entry.event_id & 0x7_FFFF; // 19 bits
    let site_id = entry.site_id & 0x7_FFFF; // 19 bits
    let round_id = entry.round_id & 0x3_FFFF; // 18 bits
    #[allow(clippy::cast_possible_truncation)]
    let esr_high = ((((event_id >> 16) & 0x7) << 5)
        | (((site_id >> 16) & 0x7) << 2)
        | ((round_id >> 16) & 0x3)) as u8;
    w.write_u8(esr_high);
    #[allow(clippy::cast_possible_truncation)]
    w.write_u16((event_id & 0xFFFF) as u16);
    #[allow(clippy::cast_possible_truncation)]
    w.write_u16((site_id & 0xFFFF) as u16);
    #[allow(clippy::cast_possible_truncation)]
    w.write_u16((round_id & 0xFFFF) as u16);

    // var_counts: top nibble = result, low 12 bits (variation/comment/NAG
    // counts) always zero, exactly like the derived counts skipped below.
    w.write_u16(u16::from(entry.result & 0x0F) << 12);

    w.write_u16(entry.eco_code);

    // date_edate: low 20 bits = Date, high 12 bits (EventDate) always zero.
    w.write_u32(entry.date & 0x000F_FFFF);

    w.write_u16(entry.white_elo & 0x0FFF);
    w.write_u16(entry.black_elo & 0x0FFF);

    w.write_zeros(4); // FinalMatSig: derived field, not reproduced
    w.write_zeros(2); // low NumHalfMoves + combined byte: derived
    w.write_zeros(8); // HomePawnData[1..8]: derived heuristic field

    let bytes = w.into_bytes();
    debug_assert_eq!(bytes.len(), INDEX_ENTRY_SIZE);
    bytes
        .try_into()
        .expect("BeWriter produit toujours exactement INDEX_ENTRY_SIZE octets pour un IndexEntry")
}

/// Serializes a full `.si4` index file: the 182-byte header followed by
/// one 47-byte record per entry — the write-side mirror of [`read_header`]
/// and [`read_all_entries`] combined, since (unlike si5, whose index file
/// is just a raw entry array with no header) the two are always written
/// together here.
#[must_use]
pub fn write_all_entries(entries: &[IndexEntry]) -> Vec<u8> {
    let num_games = u32::try_from(entries.len()).unwrap_or(u32::MAX);
    let mut out = write_header(num_games);
    out.reserve(entries.len() * INDEX_ENTRY_SIZE);
    for entry in entries {
        out.extend_from_slice(&write_index_entry(entry));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> IndexEntry {
        IndexEntry {
            offset: 123_456,
            length: 789,
            white_id: 3,
            black_id: 7,
            event_id: 1,
            site_id: 2,
            round_id: 0,
            result: 1,
            eco_code: 42,
            date: crate::dates::make(2024, 7, 12),
            white_elo: 2750,
            black_elo: 2760,
            non_standard_start: false,
            deleted: false,
        }
    }

    #[test]
    fn test_write_then_read_index_entry_roundtrip() {
        let entry = sample_entry();
        let bytes = write_index_entry(&entry);
        let decoded = read_index_entry(&bytes).unwrap();

        assert_eq!(decoded.offset, entry.offset);
        assert_eq!(decoded.length, entry.length);
        assert_eq!(decoded.white_id, entry.white_id);
        assert_eq!(decoded.black_id, entry.black_id);
        assert_eq!(decoded.event_id, entry.event_id);
        assert_eq!(decoded.site_id, entry.site_id);
        assert_eq!(decoded.round_id, entry.round_id);
        assert_eq!(decoded.result, entry.result);
        assert_eq!(decoded.eco_code, entry.eco_code);
        assert_eq!(decoded.date, entry.date);
        assert_eq!(decoded.white_elo, entry.white_elo);
        assert_eq!(decoded.black_elo, entry.black_elo);
        assert_eq!(decoded.non_standard_start, entry.non_standard_start);
    }

    #[test]
    fn test_write_non_standard_start_flag_roundtrips() {
        let mut entry = sample_entry();
        entry.non_standard_start = true;
        let bytes = write_index_entry(&entry);
        assert!(read_index_entry(&bytes).unwrap().non_standard_start);
    }

    #[test]
    fn test_write_deleted_flag_roundtrips() {
        let mut entry = sample_entry();
        entry.deleted = true;
        let bytes = write_index_entry(&entry);
        assert!(read_index_entry(&bytes).unwrap().deleted);
    }

    #[test]
    fn test_write_start_and_deleted_flags_are_independent() {
        let mut entry = sample_entry();
        entry.non_standard_start = true;
        entry.deleted = false;
        let bytes = write_index_entry(&entry);
        let decoded = read_index_entry(&bytes).unwrap();
        assert!(decoded.non_standard_start);
        assert!(!decoded.deleted);
    }

    #[test]
    fn test_write_index_entry_masks_ids_wider_than_their_field() {
        // white_id/black_id are 20-bit fields in si4 (unlike si5's 28
        // bits) — a value that does not fit must be masked down rather
        // than corrupt an adjacent field.
        let mut entry = sample_entry();
        entry.white_id = 0xFFF_FFFF; // 28 bits set
        let bytes = write_index_entry(&entry);
        let decoded = read_index_entry(&bytes).unwrap();
        assert_eq!(decoded.white_id, 0xFFF_FFFF & 0xF_FFFF);
    }

    #[test]
    fn test_write_header_then_read_header_roundtrip() {
        let bytes = write_header(3);
        let header = read_header(&bytes).unwrap();
        assert_eq!(header.version, 400);
        assert_eq!(header.num_games, 3);
    }

    #[test]
    fn test_write_all_entries_produces_header_plus_records_and_round_trips() {
        let entries = vec![sample_entry(), sample_entry(), sample_entry()];
        let bytes = write_all_entries(&entries);
        assert_eq!(bytes.len(), INDEX_HEADER_SIZE + 3 * INDEX_ENTRY_SIZE);

        let header = read_header(&bytes).unwrap();
        assert_eq!(header.num_games, 3);
        let decoded = read_all_entries(&bytes, &header).unwrap();
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0].white_id, entries[0].white_id);
    }

    #[test]
    fn test_write_all_entries_of_empty_database() {
        let bytes = write_all_entries(&[]);
        assert_eq!(bytes.len(), INDEX_HEADER_SIZE);
        let header = read_header(&bytes).unwrap();
        assert_eq!(header.num_games, 0);
        assert!(read_all_entries(&bytes, &header).unwrap().is_empty());
    }
}

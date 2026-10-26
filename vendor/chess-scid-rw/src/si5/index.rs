//! Reading the `.si5` index file: NO header, a raw array of
//! 56-byte records each (12 LITTLE-ENDIAN 32-bit words +
//! 8 raw bytes of `HomePawn` data) — see `docs/scid-si5-specification.md`
//! §2, reverse-engineered from `codec_scid5.h` (`encode_IndexEntry`/
//! `decode_IndexEntry`) and `indexentry.h` (corresponding C++ bitfields).
//!
//! A format VERY different from `.si4` (no header, words packed
//! differently, LITTLE-ENDIAN instead of BIG-ENDIAN) — but producing the SAME
//! neutral structure [`crate::entry::IndexEntry`] as `si4::index`, which
//! allows `pgn_build`/`si5::database` to remain identical to their
//! si4 equivalents.

use crate::entry::IndexEntry;
use crate::error::ScidError;

/// Fixed size of a `.si5` record (12 32-bit words + 8 bytes of
/// `HomePawn`), see `docs/scid-si5-specification.md` §2.2.
pub const INDEX_ENTRY_SIZE: usize = 56;

/// Width, in bits, of the `offset` field of a si5 `IndexEntry` record
/// (split 15 high bits in word 8 + 32 low bits in word 9, see
/// [`write_index_entry`]) — used by
/// [`crate::scid_common::LoadedDatabase::add_game`] to reject a games file
/// that has grown beyond what this format can address (~140 TB), far
/// wider than si4's plain 32-bit offset (see `si4::index::OFFSET_BITS`) —
/// the very reason si5 exists alongside si4.
pub(crate) const OFFSET_BITS: u32 = 47;

/// Reads a LITTLE-ENDIAN 32-bit word at offset `word_index` (0-11) of a
/// 56-byte record.
fn word_le(record: &[u8], word_index: usize) -> u32 {
    let start = word_index * 4;
    let bytes: [u8; 4] = record[start..start + 4]
        .try_into()
        .expect("découpe de 4 octets toujours valide dans un enregistrement de 56 octets");
    u32::from_le_bytes(bytes)
}

/// Decodes an `IndexEntry` (exactly [`INDEX_ENTRY_SIZE`] bytes).
///
/// Layout of the 12 words — see `docs/scid-si5-specification.md` §2.2/§2.3 for
/// the full detail (only the fields useful for reconstructing a PGN
/// are kept, the heuristic/derived fields are read for the record
/// but discarded, exactly as for si4):
///   word 0 (bytes 0-3)   : 4 bits comment count (ignored) | 28 bits White ID
///   word 1 (bytes 4-7)   : 4 bits variation count (ignored) | 28 bits Black ID
///   word 2 (bytes 8-11)  : 4 bits NAG count (ignored)      | 28 bits Event ID
///   word 3 (bytes 12-15) : 32 bits Site ID (full word)
///   word 4 (bytes 16-19) : 1 bit Chess960 (ignored)        | 31 bits Round ID
///   word 5 (bytes 20-23) : 12 bits White Elo               | 20 bits Date
///   word 6 (bytes 24-27) : 12 bits Black Elo                | 20 bits `EventDate` (ignored)
///   word 7 (bytes 28-31) : 10 bits `NumHalfMoves` (ignored)   | 22 bits raw Flags
///   word 8 (bytes 32-35) : 17 bits Length                   | 15 bits Offset (high bits 32-46)
///   word 9 (bytes 36-39) : 32 bits Offset (low bits 0-31)
///   word 10 (bytes 40-43): 8 bits `StoredLineCode` (ignored)  | 24 bits `FinalMatSig` (ignored)
///   word 11 (bytes 44-47): 8 bits `HomePawn` count (ignored) | 3+3 bits Elo types (ignored) | 2 bits Result || 16 bits ECO
///   bytes 48-55          : raw `HomePawn` data (ignored)
///
/// # Errors
/// [`ScidError::Truncated`] if `data` is fewer than 56 bytes.
pub fn read_index_entry(data: &[u8]) -> Result<IndexEntry, ScidError> {
    let record = data.get(..INDEX_ENTRY_SIZE).ok_or(ScidError::Truncated("IndexEntry .si5"))?;

    let word0 = word_le(record, 0);
    let white_id = word0 & 0x0FFF_FFFF;

    let word1 = word_le(record, 1);
    let black_id = word1 & 0x0FFF_FFFF;

    let word2 = word_le(record, 2);
    let event_id = word2 & 0x0FFF_FFFF;

    let site_id = word_le(record, 3);

    let word4 = word_le(record, 4);
    let round_id = word4 & 0x7FFF_FFFF;

    let word5 = word_le(record, 5);
    let raw_date = word5 & 0xFFFFF;
    #[allow(clippy::cast_possible_truncation)]
    let white_elo = ((word5 >> 20) & 0xFFF) as u16;

    let word6 = word_le(record, 6);
    #[allow(clippy::cast_possible_truncation)]
    let black_elo = ((word6 >> 20) & 0xFFF) as u16;

    let word7 = word_le(record, 7);
    let flags = word7 & 0x3F_FFFF; // 22 raw bits (bitmask)
    let non_standard_start = (flags & 1) != 0; // bit 0 = START
    let deleted = (flags & 0x08) != 0; // bit 3 = DELETE

    let word8 = word_le(record, 8);
    let offset_high15 = u64::from(word8 & 0x7FFF);
    let length = word8 >> 15;

    let offset_low32 = u64::from(word_le(record, 9));
    let offset = (offset_high15 << 32) | offset_low32;

    let word11 = word_le(record, 11);
    #[allow(clippy::cast_possible_truncation)]
    let eco_code = (word11 & 0xFFFF) as u16;
    #[allow(clippy::cast_possible_truncation)]
    let result = ((word11 >> 16) & 0x3) as u8;

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

/// Reads all `IndexEntry` records of the `.si5` file — no header, the
/// number of games is derived directly from the file size (see
/// `docs/scid-si5-specification.md` §2.1: `n_games = file_size / 56`, must
/// be an exact multiple, otherwise the database is truncated/corrupted — the
/// possible last partial entry is silently ignored rather
/// than failing to open the entire database, the same defense in
/// depth as `si4::index::read_all_entries`).
///
/// # Errors
/// Should never fail in practice (each 56-byte chunk is
/// already validated in size by construction); the `Result` type is kept
/// for symmetry with `si4::index::read_all_entries` and possible future
/// validation.
pub fn read_all_entries(data: &[u8]) -> Result<Vec<IndexEntry>, ScidError> {
    let n = data.len() / INDEX_ENTRY_SIZE;
    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        let start = i * INDEX_ENTRY_SIZE;
        let entry = read_index_entry(&data[start..start + INDEX_ENTRY_SIZE])?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Writes a LITTLE-ENDIAN 32-bit word at offset `word_index` (0-11) of a
/// 56-byte record — the write-side mirror of [`word_le`].
fn put_word_le(record: &mut [u8; INDEX_ENTRY_SIZE], word_index: usize, value: u32) {
    let start = word_index * 4;
    record[start..start + 4].copy_from_slice(&value.to_le_bytes());
}

/// Encodes an `IndexEntry` back into exactly [`INDEX_ENTRY_SIZE`] bytes —
/// the write-side mirror of [`read_index_entry`] (see its doc for the
/// full word layout).
///
/// The fields this crate never populates when reading (comment/variation/
/// NAG counts, Chess960, `EventDate`, `NumHalfMoves`, `StoredLineCode`,
/// `FinalMatSig`, Elo types, `HomePawn` data) are written as zero — a
/// real SCID installation recomputes/ignores these heuristic fields
/// itself when it next touches the game, exactly as this crate already
/// ignores them on read.
///
/// `entry.length`/`entry.offset` are masked down to the 17/47 bits the
/// format actually stores rather than rejected outright: callers that
/// need a hard guarantee should validate them before reaching this
/// function (see [`crate::scid_common::LoadedDatabase::add_game`], which
/// is the only writer of `length`/`offset` in this crate and validates
/// them before ever constructing the entry).
#[must_use]
pub fn write_index_entry(entry: &IndexEntry) -> [u8; INDEX_ENTRY_SIZE] {
    let mut record = [0u8; INDEX_ENTRY_SIZE];

    put_word_le(&mut record, 0, entry.white_id & 0x0FFF_FFFF);
    put_word_le(&mut record, 1, entry.black_id & 0x0FFF_FFFF);
    put_word_le(&mut record, 2, entry.event_id & 0x0FFF_FFFF);
    put_word_le(&mut record, 3, entry.site_id);
    put_word_le(&mut record, 4, entry.round_id & 0x7FFF_FFFF);

    let white_elo = u32::from(entry.white_elo) & 0xFFF;
    put_word_le(&mut record, 5, (white_elo << 20) | (entry.date & 0xFFFFF));

    let black_elo = u32::from(entry.black_elo) & 0xFFF;
    put_word_le(&mut record, 6, black_elo << 20);

    let flags7 = u32::from(entry.non_standard_start) | (u32::from(entry.deleted) << 3);
    put_word_le(&mut record, 7, flags7); // bit 0 = START, bit 3 = DELETE, rest unused

    let length17 = entry.length & 0x1FFFF;
    let offset_high15 = u32::try_from((entry.offset >> 32) & 0x7FFF).unwrap_or(0);
    put_word_le(&mut record, 8, (length17 << 15) | offset_high15);
    #[allow(clippy::cast_possible_truncation)]
    let offset_low32 = (entry.offset & 0xFFFF_FFFF) as u32;
    put_word_le(&mut record, 9, offset_low32);

    put_word_le(&mut record, 10, 0);
    put_word_le(&mut record, 11, (u32::from(entry.result & 0x3) << 16) | u32::from(entry.eco_code));

    record
}

/// Serializes every entry of `entries`, concatenated in order — the
/// write-side mirror of [`read_all_entries`].
#[must_use]
pub fn write_all_entries(entries: &[IndexEntry]) -> Vec<u8> {
    let mut out = Vec::with_capacity(entries.len() * INDEX_ENTRY_SIZE);
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
    fn test_write_then_read_roundtrip() {
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
    fn test_write_all_entries_concatenates_records() {
        let entries = vec![sample_entry(), sample_entry()];
        let bytes = write_all_entries(&entries);
        assert_eq!(bytes.len(), 2 * INDEX_ENTRY_SIZE);
        let decoded = read_all_entries(&bytes).unwrap();
        assert_eq!(decoded.len(), 2);
    }
}

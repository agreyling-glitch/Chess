//! Read-only, in-memory si4 adapter shared by native and browser imports.
use chess_scid_rw::{entry::IndexEntry, names::NameTables, pgn_build, si4::{index, namebase}};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const MAX_BYTES: usize = 64 * 1024 * 1024;

/// si5 keeps names only; index records and game blobs are supplied as bounded
/// chunks from File.slice(), preserving 47-bit offsets on wasm32.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct Scid5Reader { names: NameTables }

impl Scid5Reader {
    pub fn read(names: &[u8]) -> Result<Self, String> {
        if names.len() > 128 * 1024 * 1024 { return Err("The Scid name file exceeds 128 MiB.".into()); }
        Ok(Self { names: chess_scid_rw::si5::namebase::read_namebase(names).map_err(|e| format!("Invalid .sn5 names: {e}"))? })
    }
    pub fn entries(&self, bytes: &[u8]) -> Result<String, String> {
        if bytes.len() % 56 != 0 || bytes.len() > 56 * 1024 { return Err("Invalid Scid 5 index chunk.".into()); }
        let rows = bytes.chunks_exact(56).map(|record| {
            let e = chess_scid_rw::si5::index::read_index_entry(record).map_err(|e| e.to_string())?;
            self.validate(&e)?;
            Ok(serde_json::json!({"offset": e.offset, "length": e.length, "deleted": e.deleted,
                "white": self.names.player(e.white_id), "black": self.names.player(e.black_id),
                "date": chess_scid_rw::dates::date_to_pgn(e.date), "result": e.result}))
        }).collect::<Result<Vec<_>, String>>()?;
        serde_json::to_string(&rows).map_err(|e| e.to_string())
    }
    fn validate(&self, e: &IndexEntry) -> Result<(), String> {
        if !e.deleted && (e.white_id as usize >= self.names.players.len() || e.black_id as usize >= self.names.players.len()
            || e.event_id as usize >= self.names.events.len() || e.site_id as usize >= self.names.sites.len()
            || e.round_id as usize >= self.names.rounds.len() || e.length == 0) {
            return Err("Scid index refers to missing names or an empty game; select files from the same database.".into());
        }
        Ok(())
    }
    pub fn decode(&self, record: &[u8], blob: &[u8]) -> Result<String, String> {
        let e = chess_scid_rw::si5::index::read_index_entry(record).map_err(|e| e.to_string())?;
        self.validate(&e)?;
        if e.deleted { return Ok(String::new()); }
        if blob.len() != e.length as usize { return Err("The .sg5 game is truncated.".into()); }
        pgn_build::build_pgn(&e, &self.names, blob).map_err(|e| e.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl Scid5Reader {
    #[wasm_bindgen(constructor)]
    pub fn new(names: &[u8]) -> Result<Scid5Reader, JsValue> { Self::read(names).map_err(|e| JsValue::from_str(&e)) }
    pub fn index_entries(&self, bytes: &[u8]) -> Result<String, JsValue> { self.entries(bytes).map_err(|e| JsValue::from_str(&e)) }
    pub fn decode_game(&self, record: &[u8], blob: &[u8]) -> Result<String, JsValue> { self.decode(record, blob).map_err(|e| JsValue::from_str(&e)) }
}

pub fn is_scid_path(path: &std::path::Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| matches!(s.to_ascii_lowercase().as_str(), "si3" | "sn3" | "sg3" | "si4" | "sn4" | "sg4" | "si5" | "sn5" | "sg5"))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_files(files: &[(std::path::PathBuf, Vec<u8>)]) -> Result<String, String> {
    let mut parts = std::collections::HashMap::new();
    let mut base = None;
    let mut total = 0usize;
    for (path, bytes) in files {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or_default().to_ascii_lowercase();
        if !matches!(extension.as_str(), "si4" | "sn4" | "sg4") {
            return Err("Select only the matching .si4, .sn4 and .sg4 files. Other Scid versions must be exported to PGN first.".into());
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_ascii_lowercase();
        if base.as_ref().is_some_and(|base| base != &stem) || parts.insert(extension, bytes).is_some() {
            return Err("Select one set of three Scid files with the same database name.".into());
        }
        base = Some(stem);
        total = total.saturating_add(bytes.len());
        if total > MAX_BYTES { return Err("The three Scid files together exceed the 64 MiB import limit.".into()); }
    }
    if parts.len() != 3 { return Err("Select the .si4 index together with its matching .sn4 names and .sg4 games files.".into()); }
    let database = ScidImport::read(parts["si4"], parts["sn4"], parts["sg4"])?;
    let mut text = String::new();
    for n in 0..database.count() {
        let pgn = database.pgn(n)?;
        if pgn.is_empty() { continue; }
        if text.len() + pgn.len() + 2 > MAX_BYTES { return Err("Converted Scid games exceed the 64 MiB PGN import limit.".into()); }
        if !text.is_empty() { text.push_str("\n\n"); }
        text.push_str(&pgn);
    }
    if text.is_empty() { return Err("This Scid database contains no games available to import.".into()); }
    Ok(text)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct ScidImport {
    entries: Vec<IndexEntry>,
    names: NameTables,
    games: Vec<u8>,
}

impl ScidImport {
    pub fn read(si4: &[u8], sn4: &[u8], sg4: &[u8]) -> Result<Self, String> {
        if si4.len().saturating_add(sn4.len()).saturating_add(sg4.len()) > MAX_BYTES {
            return Err("The three Scid files together exceed the 64 MiB import limit.".into());
        }
        let header = index::read_header(si4).map_err(|e| format!("Invalid .si4 index: {e}"))?;
        // Upstream tolerates a truncated index; imports must never silently lose games.
        if si4.len() != index::INDEX_HEADER_SIZE + header.num_games as usize * index::INDEX_ENTRY_SIZE {
            return Err("The .si4 index is truncated or has an inconsistent game count.".into());
        }
        // Bound name-table allocations by actual minimum record sizes before upstream parsing.
        if sn4.len() < namebase::HEADER_SIZE { return Err("The .sn4 name file is truncated.".into()); }
        let mut required = namebase::HEADER_SIZE;
        for offset in [12, 15, 18, 21] {
            let count = (usize::from(sn4[offset]) << 16) | (usize::from(sn4[offset+1]) << 8) | usize::from(sn4[offset+2]);
            required = required.saturating_add(count.saturating_mul(4));
        }
        if required > sn4.len() { return Err("The .sn4 name counts exceed the file's size.".into()); }
        let names = namebase::read_namebase(sn4).map_err(|e| format!("Invalid .sn4 names: {e}"))?;
        let entries = index::read_all_entries(si4, &header).map_err(|e| format!("Invalid .si4 index: {e}"))?;
        for (n, entry) in entries.iter().enumerate().filter(|(_, e)| !e.deleted) {
            let end = entry.offset.checked_add(u64::from(entry.length)).unwrap_or(u64::MAX);
            if end > sg4.len() as u64 || entry.length == 0 {
                return Err(format!("Scid game {} points outside the .sg4 game file.", n + 1));
            }
            if entry.white_id as usize >= names.players.len() || entry.black_id as usize >= names.players.len()
                || entry.event_id as usize >= names.events.len() || entry.site_id as usize >= names.sites.len()
                || entry.round_id as usize >= names.rounds.len() {
                return Err(format!("Scid game {} refers to missing names; select files from the same database.", n + 1));
            }
        }
        Ok(Self { entries, names, games: sg4.to_vec() })
    }

    pub fn count(&self) -> usize { self.entries.len() }

    pub fn pgn(&self, n: usize) -> Result<String, String> {
        let entry = self.entries.get(n).ok_or("Scid game number is out of range.")?;
        if entry.deleted { return Ok(String::new()); }
        let start = entry.offset as usize;
        let blob = self.games.get(start..start + entry.length as usize).ok_or("Invalid Scid game offset.")?;
        pgn_build::build_pgn(entry, &self.names, blob).map_err(|e| format!("Could not decode Scid game {}: {e}", n+1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const INDEX: &[u8] = include_bytes!("../tests/fixtures/scid/sample.si4");
    const NAMES: &[u8] = include_bytes!("../tests/fixtures/scid/sample.sn4");
    const GAMES: &[u8] = include_bytes!("../tests/fixtures/scid/sample.sg4");

    #[test]
    fn scid5_reads_chunked_index_and_keeps_unsigned_47_bit_offsets() {
        let names=include_bytes!("../tests/fixtures/scid/sample.sn5");
        let mut index=include_bytes!("../tests/fixtures/scid/sample.si5").to_vec();
        let reader=Scid5Reader::read(names).unwrap();
        let rows: serde_json::Value=serde_json::from_str(&reader.entries(&index).unwrap()).unwrap();
        assert_eq!(rows[0]["white"],"Ada");assert_eq!(rows[2]["deleted"],true);
        let pgn=reader.decode(&index[..56],GAMES).unwrap();assert!(pgn.contains("Nf3"));
        let length=GAMES.len() as u32;
        index[32..36].copy_from_slice(&((length<<15)|1).to_le_bytes());
        index[36..40].copy_from_slice(&12u32.to_le_bytes());
        let rows: serde_json::Value=serde_json::from_str(&reader.entries(&index[..56]).unwrap()).unwrap();
        assert_eq!(rows[0]["offset"],(1u64<<32)+12);
        assert_eq!(reader.decode(&index[..56],GAMES).unwrap(),pgn);
        assert!(reader.entries(&index[..55]).is_err());assert!(reader.decode(&index[..56],&GAMES[..5]).is_err());
    }

    #[test]
    fn scid_decodes_metadata_annotations_variations_and_skips_deleted_games() {
        let database = ScidImport::read(INDEX, NAMES, GAMES).unwrap();
        assert_eq!(database.count(), 3);
        let pgn = database.pgn(0).unwrap();
        assert!(pgn.contains("[White \"Ada\"]"), "{pgn}");
        assert!(pgn.contains("[Black \"Ben\"]"));
        assert!(pgn.contains("[Date \"2026.10.24\"]"));
        assert!(pgn.contains("[WhiteElo \"2100\"]"));
        assert!(pgn.contains("$1"));
        assert!(pgn.contains("{Develop a knight.}"));
        assert!(pgn.contains("Nc3"));
        assert_eq!(database.pgn(1).unwrap(), pgn);
        assert!(database.pgn(2).unwrap().is_empty());
        let (_, moves) = crate::app::ChessApp::parse_pgn_mainline(&pgn).unwrap();
        assert_eq!(moves, vec!["e4", "e5", "Nf3", "Nc6"]);
        let prepared: serde_json::Value = serde_json::from_str(&crate::app::ChessApp::prepare_archived_pgn(&pgn).unwrap()).unwrap();
        assert!(prepared.to_string().contains("Develop a knight."));
    }

    #[test]
    fn scid_preserves_custom_start_and_promotions() {
        use chess_scid_rw::{chess::{game::GameState, types::{chess_move::Move, piece::PieceKind, square::Square}}, game_blob};
        let fen = "8/P7/4k3/8/8/8/7p/4K3 w - - 0 1";
        let mut game = GameState::from_fen(fen).unwrap();
        game.play(Move::promotion(Square::from_algebraic("a7").unwrap(), Square::from_algebraic("a8").unwrap(), PieceKind::Queen)).unwrap();
        game.play(Move::promotion(Square::from_algebraic("h2").unwrap(), Square::from_algebraic("h1").unwrap(), PieceKind::Queen)).unwrap();
        let blob = game_blob::encode_mainline(&game).unwrap();
        let header = index::read_header(INDEX).unwrap();
        let mut entry = index::read_all_entries(INDEX, &header).unwrap().remove(0);
        entry.offset = 0; entry.length = blob.len() as u32; entry.non_standard_start = true;
        let mut si4 = index::write_header(1);
        si4.extend(index::write_index_entry(&entry));
        let database = ScidImport::read(&si4, NAMES, &blob).unwrap();
        let pgn = database.pgn(0).unwrap();
        assert!(pgn.contains(&format!("[FEN \"{fen}\"]")), "{pgn}");
        assert!(pgn.contains("a8=Q")); assert!(pgn.contains("h1=Q"));
        assert!(crate::app::ChessApp::parse_pgn_mainline(&pgn).is_ok());
    }

    #[test]
    fn scid_rejects_corrupt_headers_names_offsets_and_moves() {
        assert!(ScidImport::read(&INDEX[..INDEX.len()-1], NAMES, GAMES).is_err());
        assert!(ScidImport::read(b"not a database", NAMES, GAMES).is_err());
        assert!(ScidImport::read(INDEX, &NAMES[..36], GAMES).is_err());
        assert!(ScidImport::read(INDEX, NAMES, &GAMES[..4]).is_err());
        let mut broken = INDEX.to_vec(); broken[8..10].copy_from_slice(&500u16.to_be_bytes());
        assert!(ScidImport::read(&broken, NAMES, GAMES).is_err());
        let mut broken = INDEX.to_vec(); broken[182+10..182+12].copy_from_slice(&99u16.to_be_bytes());
        assert!(ScidImport::read(&broken, NAMES, GAMES).is_err());
        let mut broken = GAMES.to_vec(); broken[2] = 0x0e;
        assert!(ScidImport::read(INDEX, NAMES, &broken).unwrap().pgn(0).is_err());
    }

    #[test]
    fn scid_native_requires_matching_parts_without_modifying_them() {
        let files = vec![("sample.si4".into(), INDEX.to_vec()), ("sample.sn4".into(), NAMES.to_vec()), ("sample.sg4".into(), GAMES.to_vec())];
        let original = files.clone();
        let text = read_files(&files).unwrap();
        assert_eq!(text.matches("[White \"Ada\"]").count(), 2);
        assert_eq!(files, original);
        assert!(read_files(&files[..1]).is_err());
        let mut wrong = files.clone(); wrong[1].0 = "other.sn4".into();
        assert!(read_files(&wrong).is_err());
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl ScidImport {
    #[wasm_bindgen(constructor)]
    pub fn new(si4: &[u8], sn4: &[u8], sg4: &[u8]) -> Result<ScidImport, JsValue> {
        Self::read(si4, sn4, sg4).map_err(|e| JsValue::from_str(&e))
    }
    pub fn game_count(&self) -> usize { self.count() }
    pub fn game_pgn(&self, n: usize) -> Result<String, JsValue> {
        self.pgn(n).map_err(|e| JsValue::from_str(&e))
    }
}

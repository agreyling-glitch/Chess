//! Owner-local analysis runs, stored separately from original game documents.
use crate::archive::{self, Result, invalid};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub id: String,
    pub revision: u64,
    pub created_at: i64,
    pub metadata: Value,
    pub payload: Value,
}
pub fn initialize(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS analysis_runs(game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,id TEXT NOT NULL,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,metadata TEXT NOT NULL,payload TEXT NOT NULL,PRIMARY KEY(game_id,id));CREATE INDEX IF NOT EXISTS analysis_runs_recent ON analysis_runs(game_id,created_at DESC,id);")?;
    Ok(())
}
fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Run> {
    let parse = |i| {
        serde_json::from_str(&row.get::<_, String>(i)?).map_err(|_| rusqlite::Error::InvalidQuery)
    };
    Ok(Run {
        id: row.get(0)?,
        revision: row.get(1)?,
        created_at: row.get(2)?,
        metadata: parse(3)?,
        payload: parse(4)?,
    })
}
pub fn all(db: &Connection, game: &str) -> Result<Vec<Run>> {
    Ok(db.prepare("SELECT id,revision,created_at,metadata,payload FROM analysis_runs WHERE game_id=?1 ORDER BY created_at DESC,id")?.query_map([game],read)?.collect::<std::result::Result<_,_>>()?)
}
pub fn list(db: &Connection, game: &str) -> Result<Value> {
    if !db.query_row(
        "SELECT EXISTS(SELECT 1 FROM games WHERE id=?1)",
        [game],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(archive::Error("not_found", "Archive game was not found"));
    }
    let rows=db.prepare("SELECT id,revision,created_at,metadata FROM analysis_runs WHERE game_id=?1 ORDER BY created_at DESC,id")?.query_map([game],|r|Ok(json!({"id":r.get::<_,String>(0)?,"revision":r.get::<_,u64>(1)?,"created_at":r.get::<_,i64>(2)?,"metadata":serde_json::from_str::<Value>(&r.get::<_,String>(3)?).map_err(|_|rusqlite::Error::InvalidQuery)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(json!({"runs":rows}))
}
pub fn get(db: &Connection, game: &str, id: &str) -> Result<Run> {
    db.query_row("SELECT id,revision,created_at,metadata,payload FROM analysis_runs WHERE game_id=?1 AND id=?2",params![game,id],read).optional()?.ok_or(archive::Error("not_found","Analysis run was not found"))
}
fn validate(id: &str, metadata: &Value, payload: &Value) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || !metadata.is_object()
        || serde_json::to_vec(metadata)?.len() > 65536
        || serde_json::to_vec(payload)?.len() > 8 * 1024 * 1024
    {
        return Err(invalid("Invalid analysis run"));
    }
    if !payload["game_analysis"].is_array()
        || payload["game_analysis"].as_array().unwrap().len() > 10001
        || !metadata["engine_name"].is_string()
    {
        return Err(invalid("Invalid analysis results or engine details"));
    }
    Ok(())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Save {
    pub expected_revision: u64,
    pub metadata: Value,
    pub payload: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Delete { pub runs: Vec<(String, u64)> }

pub fn delete(db: &mut Connection, game: &str, request: Delete) -> Result<Value> {
    if request.runs.is_empty() || request.runs.len() > 1000 { return Err(invalid("Select between 1 and 1000 analysis runs")); }
    let ids: std::collections::HashSet<_> = request.runs.iter().map(|(id,_)|id.as_str()).collect();
    if ids.len()!=request.runs.len() { return Err(invalid("Duplicate analysis run IDs")); }
    let tx=db.transaction()?;
    let mut original=archive::get(&tx,game)?;
    for (id,revision) in &request.runs {
        if get(&tx,game,id)?.revision!=*revision { return Err(archive::Error("conflict","Analysis run changed; refresh before deleting")); }
    }
    for (id,_) in &request.runs { tx.execute("DELETE FROM analysis_runs WHERE game_id=?1 AND id=?2",params![game,id])?; }
    // The game document can also contain the currently displayed results. Clear
    // only that analysis cache when its run is deleted, retaining every annotation.
    if let Some(record)=original.document.record.as_mut().and_then(Value::as_object_mut) {
        if let Some(raw)=record.get("json").and_then(Value::as_str) {
            let mut saved:Value=serde_json::from_str(raw)?;
            let current=saved["analysis_run"]["id"].as_str();
            if current.is_some_and(|id|ids.contains(id)) || (current.is_none()&&ids.contains(format!("legacy-{game}").as_str())) {
                let count=saved["game_analysis"].as_array().map_or(0,Vec::len);
                saved["analysis_run"]=Value::Null;
                saved["game_analysis"]=json!(vec![Value::Null;count]);
                for field in ["verification_targets","verification_results"] { saved[field]=json!([]); }
                for field in ["verification_nodes","verification_time_ms"] { saved[field]=json!(0); }
                saved["game_analysis_running"]=json!(false);saved["game_analysis_paused"]=json!(false);
                record.insert("json".into(),json!(serde_json::to_string(&saved)?));
                tx.execute("UPDATE games SET document=?1,fingerprint=?2,revision=revision+1 WHERE id=?3",params![serde_json::to_string(&original.document)?,original.document.fingerprint()?,game])?;
            }
        }
    }
    let updated=archive::get(&tx,game)?;
    tx.commit()?;
    Ok(json!({"deleted":request.runs.len(),"game":updated}))
}
pub fn save(db: &mut Connection, game: &str, id: &str, request: Save) -> Result<Run> {
    validate(id, &request.metadata, &request.payload)?;
    let tx = db.transaction()?;
    archive::get(&tx, game)?;
    let existing = tx
        .query_row(
            "SELECT revision FROM analysis_runs WHERE game_id=?1 AND id=?2",
            params![game, id],
            |r| r.get::<_, u64>(0),
        )
        .optional()?;
    if existing.unwrap_or(0) != request.expected_revision {
        return Err(archive::Error(
            "conflict",
            "Analysis run changed; reload before saving",
        ));
    }
    let revision = request
        .expected_revision
        .checked_add(1)
        .filter(|r| *r <= i64::MAX as u64)
        .ok_or(invalid("Invalid analysis revision"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    tx.execute("INSERT INTO analysis_runs VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(game_id,id) DO UPDATE SET revision=excluded.revision,metadata=excluded.metadata,payload=excluded.payload",params![game,id,revision,stamp,serde_json::to_string(&request.metadata)?,serde_json::to_string(&request.payload)?])?;
    let result = get(&tx, game, id)?;
    tx.commit()?;
    Ok(result)
}
pub fn merge(db: &Connection, game: &str, runs: &[Run]) -> Result<()> {
    for run in runs {
        validate(&run.id, &run.metadata, &run.payload)?;
        let existing = db
            .query_row(
                "SELECT metadata,payload FROM analysis_runs WHERE game_id=?1 AND id=?2",
                params![game, run.id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?;
        let metadata = serde_json::to_string(&run.metadata)?;
        let payload = serde_json::to_string(&run.payload)?;
        if let Some((a, b)) = existing {
            if a != metadata || b != payload {
                return Err(archive::Error(
                    "conflict",
                    "An analysis run ID already has different results",
                ));
            }
            continue;
        }
        if run.revision == 0 || run.revision > i64::MAX as u64 {
            return Err(invalid("Invalid analysis revision"));
        }
        db.execute(
            "INSERT INTO analysis_runs VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                game,
                run.id,
                run.revision,
                run.created_at,
                metadata,
                payload
            ],
        )?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn database() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE games(id TEXT PRIMARY KEY,revision INTEGER,created_at INTEGER,updated_at INTEGER,document TEXT,fingerprint TEXT UNIQUE);CREATE TABLE collections(id TEXT PRIMARY KEY,name TEXT,revision INTEGER);CREATE TABLE memberships(collection_id TEXT,game_id TEXT);").unwrap();
        initialize(&db).unwrap();
        db
    }
    fn request(engine: &str, score: i32) -> Save {
        Save {
            expected_revision: 0,
            metadata: json!({"engine_name":engine,"settings_label":"5 seconds per position","engine_options":{"Threads":"2"},"status":"Complete"}),
            payload: json!({"game_analysis":[{"eval_cp":score,"depth":10,"nodes":100,"pv":"e4"}],"analysis_run":{"id":engine}}),
        }
    }
    #[test]
    fn deletion_is_atomic_and_preserves_game_annotations_and_other_runs() {
        let mut db=database();
        let mut docs=crate::archive_pgn::documents("1. e4 e5 *").unwrap();
        let saved=json!({"board":"startpos","history":[],"analysis_run":{"id":"lc0"},"game_analysis":[{"eval_cp":70},null],"move_notes":["Keep this note"],"game_notes":"Opening research","board_marks":["e4"],"verification_targets":[0]});
        docs[0].record=Some(json!({"id":"original","json":serde_json::to_string(&saved).unwrap(),"favorite":true}));
        let imported=archive::import(&mut db,docs,None).unwrap();let game=imported["ids"][0].as_str().unwrap();
        let original=archive::get(&db,game).unwrap();
        save(&mut db,game,"lc0",request("Lc0",70)).unwrap();
        let sf=save(&mut db,game,"sf",request("Stockfish",20)).unwrap();
        assert!(delete(&mut db,game,Delete{runs:vec![("lc0".into(),1),("sf".into(),99)]}).is_err());
        assert_eq!(all(&db,game).unwrap().len(),2);
        delete(&mut db,game,Delete{runs:vec![("lc0".into(),1)]}).unwrap();
        assert!(get(&db,game,"lc0").is_err());assert_eq!(get(&db,game,"sf").unwrap().payload,sf.payload);
        let updated=archive::get(&db,game).unwrap();assert_eq!(updated.document.pgn,original.document.pgn);
        let remaining:Value=serde_json::from_str(updated.document.record.as_ref().unwrap()["json"].as_str().unwrap()).unwrap();
        for field in ["move_notes","game_notes","board_marks"] {assert_eq!(remaining[field],saved[field]);}
        assert!(remaining["analysis_run"].is_null());assert_eq!(remaining["game_analysis"],json!([null,null]));
        assert_eq!(updated.document.record.as_ref().unwrap()["favorite"],true);
        delete(&mut db,game,Delete{runs:vec![("sf".into(),1)]}).unwrap();assert!(all(&db,game).unwrap().is_empty());
        assert!(archive::get(&db,game).is_ok());
    }
    #[test]
    fn runs_are_separate_revision_checked_and_survive_backup_restore() {
        let mut db = database();
        let docs = crate::archive_pgn::documents("1. e4 *").unwrap();
        let result = archive::import(&mut db, docs, None).unwrap();
        let game = result["ids"][0].as_str().unwrap();
        let original = serde_json::to_value(archive::get(&db, game).unwrap()).unwrap();
        let a = save(&mut db, game, "stockfish", request("Stockfish 19", 31)).unwrap();
        let b = save(&mut db, game, "lc0", request("Lc0 v0.32.1", 70)).unwrap();
        assert_eq!(a.payload["game_analysis"][0]["eval_cp"], 31);
        assert_eq!(b.payload["game_analysis"][0]["eval_cp"], 70);
        assert_eq!(
            serde_json::to_value(archive::get(&db, game).unwrap()).unwrap(),
            original
        );
        assert!(save(&mut db, game, "stockfish", request("Stockfish 19", 99)).is_err());
        assert_eq!(get(&db, game, "stockfish").unwrap().payload, a.payload);
        let catalog = list(&db, game).unwrap();
        assert_eq!(catalog["runs"].as_array().unwrap().len(), 2);
        assert!(catalog["runs"][0].get("payload").is_none());
        assert!(
            archive::delete_with_runs(&mut db, game, 1, Some(vec![("stockfish".into(), 1)]))
                .is_err()
        );
        assert_eq!(all(&db, game).unwrap().len(), 2);
        let backup = archive::backup(&mut db).unwrap();
        assert_eq!(backup.games[0].document.analysis_runs.len(), 2);
        let mut target = database();
        archive::restore(&mut target, backup).unwrap();
        assert_eq!(all(&target, game).unwrap().len(), 2);
        archive::delete(&mut target, game, 1).unwrap();
        assert_eq!(
            target
                .query_row("SELECT count(*) FROM analysis_runs", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn transfer_merges_runs_without_duplicate_games_or_overwriting_results() {
        let mut db = database();
        let result = archive::import(
            &mut db,
            crate::archive_pgn::documents("1. d4 *").unwrap(),
            None,
        )
        .unwrap();
        let game = result["ids"][0].as_str().unwrap();
        save(&mut db, game, "run", request("Lc0", 12)).unwrap();
        let mut doc = archive::get(&db, game).unwrap().document;
        doc.analysis_runs = all(&db, game).unwrap();
        let result = archive::import(&mut db, vec![doc.clone()], None).unwrap();
        assert_eq!(result["duplicates"], 1);
        assert_eq!(all(&db, game).unwrap().len(), 1);
        doc.analysis_runs[0].payload["game_analysis"][0]["eval_cp"] = json!(90);
        assert!(archive::import(&mut db, vec![doc], None).is_err());
        assert_eq!(
            get(&db, game, "run").unwrap().payload["game_analysis"][0]["eval_cp"],
            12
        );
    }
}

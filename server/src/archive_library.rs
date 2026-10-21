//! Rebuildable, compact library index. Complete documents remain in games.
use crate::archive::{Document, Result, invalid};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};

pub fn initialize(db: &Connection) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    tx.execute_batch("CREATE TABLE IF NOT EXISTS library_cards(id TEXT PRIMARY KEY REFERENCES games(id) ON DELETE CASCADE,revision INTEGER NOT NULL,updated INTEGER NOT NULL,moves INTEGER NOT NULL,category TEXT NOT NULL,favorite INTEGER NOT NULL,white TEXT NOT NULL,black TEXT NOT NULL,profile TEXT NOT NULL,analysis TEXT NOT NULL,search TEXT NOT NULL,summary TEXT NOT NULL);
    CREATE INDEX IF NOT EXISTS library_recent ON library_cards(updated DESC,id);
    CREATE INDEX IF NOT EXISTS library_moves ON library_cards(moves,updated DESC,id);
    CREATE INDEX IF NOT EXISTS library_category ON library_cards(category,updated DESC,id);
    CREATE TABLE IF NOT EXISTS library_dirty(id TEXT PRIMARY KEY);
    CREATE TABLE IF NOT EXISTS library_cache_state(id INTEGER PRIMARY KEY CHECK(id=1),revision INTEGER NOT NULL);
    INSERT OR IGNORE INTO library_cache_state VALUES(1,1);
    CREATE TABLE IF NOT EXISTS library_query_cache(key TEXT PRIMARY KEY,value TEXT NOT NULL);
    CREATE TRIGGER IF NOT EXISTS library_cache_insert AFTER INSERT ON games BEGIN UPDATE library_cache_state SET revision=revision+1 WHERE id=1; DELETE FROM library_query_cache; END;
    CREATE TRIGGER IF NOT EXISTS library_cache_update AFTER UPDATE ON games BEGIN UPDATE library_cache_state SET revision=revision+1 WHERE id=1; DELETE FROM library_query_cache; END;
    CREATE TRIGGER IF NOT EXISTS library_cache_delete AFTER DELETE ON games BEGIN UPDATE library_cache_state SET revision=revision+1 WHERE id=1; DELETE FROM library_query_cache; END;
    CREATE TRIGGER IF NOT EXISTS library_insert AFTER INSERT ON games BEGIN INSERT OR IGNORE INTO library_dirty VALUES(new.id); END;
    CREATE TRIGGER IF NOT EXISTS library_update AFTER UPDATE ON games BEGIN INSERT OR IGNORE INTO library_dirty VALUES(new.id); END;
    CREATE TRIGGER IF NOT EXISTS library_delete AFTER DELETE ON games BEGIN DELETE FROM library_cards WHERE id=old.id; DELETE FROM library_dirty WHERE id=old.id; END;
    INSERT OR IGNORE INTO library_dirty SELECT g.id FROM games g LEFT JOIN library_cards c ON c.id=g.id WHERE c.id IS NULL OR c.revision<>g.revision;")?;
    tx.commit()?;
    Ok(())
}

fn summary(id: &str, revision: u64, updated: i64, document: &Document) -> Result<Value> {
    let parsed = crate::archive_pgn::parse(&document.pgn)?
        .into_iter()
        .next()
        .ok_or(invalid("Missing PGN game"))?;
    let record = document.record.as_ref().unwrap_or(&Value::Null);
    let saved: Value = record["json"]
        .as_str()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null);
    let tag = |key: &str| {
        document
            .metadata
            .get(key)
            .or_else(|| parsed.tags.get(key))
            .cloned()
            .unwrap_or_default()
    };
    let training = &saved["training"];
    let profile = training["profile_id"].as_str().unwrap_or("");
    let profile_name = training["profile_name"].as_str().unwrap_or("You");
    let engine_kind = training["engine"]["kind"].as_str().unwrap_or("stockfish");
    let opponent = if engine_kind == "lc0" {
        format!(
            "{} · Level {} · {} nodes",
            training["engine"]["name"].as_str().unwrap_or("lc0"),
            training["difficulty_level"],
            training["node_budget"]
        )
    } else {
        format!("Stockfish {} Elo", training["opponent_elo"])
    };
    let mut white = tag("White");
    let mut black = tag("Black");
    if !profile.is_empty() {
        (white, black) = if training["side"] == "Black" {
            (opponent, profile_name.into())
        } else {
            (profile_name.into(), opponent)
        };
    }
    if white.is_empty() {
        white = "Unknown".into();
    }
    if black.is_empty() {
        black = "Unknown".into();
    }
    let puzzle = tag("Event") == "Lichess puzzle";
    let category = if puzzle {
        "puzzles"
    } else if !profile.is_empty() {
        "training"
    } else {
        record["category"].as_str().unwrap_or("imported")
    };
    let moves = saved["live_moves"]
        .as_array()
        .filter(|v| !v.is_empty())
        .map(Vec::len)
        .unwrap_or(parsed.moves);
    let analyzed = saved["game_analysis"]
        .as_array()
        .map(|a| a.iter().filter(|v| !v.is_null()).count())
        .unwrap_or(0);
    let analysis = if analyzed == 0 {
        "Not analyzed"
    } else if analyzed >= moves + 1 {
        "Complete"
    } else {
        "Partial"
    };
    let date = tag("Date");
    let site = tag("Site");
    let puzzle_id = tag("PuzzleId");
    let played = training["started_at"]
        .as_str()
        .map(|v| v.chars().take(10).collect::<String>())
        .unwrap_or(date);
    let played = if played == "????.??.??" {
        String::new()
    } else {
        played
    };
    let venue = if site == "-" || site == "?" {
        String::new()
    } else {
        site
    };
    let title = if puzzle {
        format!(
            "Lichess puzzle{}",
            if puzzle_id.is_empty() {
                String::new()
            } else {
                format!(" {puzzle_id}")
            }
        )
    } else {
        format!("{white} vs {black}")
    };
    let final_fen = saved["final_board"]
        .as_str()
        .or_else(|| saved["board"].as_str())
        .unwrap_or(&parsed.final_fen);
    Ok(
        json!({"id":id,"revision":revision,"updatedAt":record["updatedAt"].as_i64().unwrap_or(updated),"title":title,"white":white,"black":black,
        "whiteFideId":tag("WhiteFideId"),"blackFideId":tag("BlackFideId"),"playedAt":played,"venue":venue,
        "result":tag("Result"),"moves":moves,"analyzed":analyzed,"analysisStatus":analysis,"category":category,
        "favorite":record["favorite"].as_bool().unwrap_or(false),"source":record["source"],"finalFen":final_fen,
        "startingNote":saved["move_notes"][0].as_str().unwrap_or("").trim(),"puzzleId":puzzle_id,
        "trainingProfileId":profile,"trainingProfileName":profile_name,"trainingSide":training["side"],
        "trainingEngineKind":engine_kind,"trainingOpponentElo":training["opponent_elo"],
        "ratingBefore":training["rating_before"],"ratingAfter":training["rating_after"]}),
    )
}

pub fn synchronize(db: &mut Connection) -> Result<()> {
    let tx = db.transaction()?;
    // Stream one dirty document at a time; never collect the complete archive.
    {
        let mut stmt=tx.prepare("SELECT g.id,g.revision,g.updated_at,g.document FROM library_dirty d CROSS JOIN games g ON g.id=d.id")?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let revision: u64 = row.get(1)?;
            let updated: i64 = row.get(2)?;
            let doc: Document = serde_json::from_str(&row.get::<_, String>(3)?)?;
            let card = summary(&id, revision, updated, &doc)?;
            let search = ["title", "white", "black", "source", "venue", "playedAt"]
                .map(|key| card[key].as_str().unwrap_or(""))
                .join(" ")
                .to_lowercase();
            tx.execute("INSERT OR REPLACE INTO library_cards VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![id,revision,card["updatedAt"].as_i64().unwrap_or(updated),card["moves"].as_u64().unwrap_or(0),card["category"].as_str(),card["favorite"].as_bool(),card["white"].as_str(),card["black"].as_str(),card["trainingProfileId"].as_str(),card["analysisStatus"].as_str(),search,serde_json::to_string(&card)?])?;
        }
    }
    tx.execute("DELETE FROM library_dirty", [])?;
    tx.commit()?;
    Ok(())
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Filter {
    pub category: String,
    pub player: String,
    pub profile: String,
    pub search: String,
    pub analysis: String,
    pub sort: String,
    pub page: u32,
    pub limit: u32,
    pub selection: bool,
    pub facets_revision: Option<u64>,
}
pub fn query(db: &mut Connection, filter: Filter) -> Result<Value> {
    if [
        &filter.category,
        &filter.player,
        &filter.profile,
        &filter.search,
        &filter.analysis,
        &filter.sort,
    ]
    .iter()
    .any(|v| v.len() > 256)
        || filter.limit > 100
        || filter.page > 10_000_000
        || ![
            "",
            "all",
            "mine",
            "training",
            "puzzles",
            "observed",
            "imported",
            "favorites",
        ]
        .contains(&filter.category.as_str())
        || !["", "*", "complete", "partial", "unanalyzed"].contains(&filter.analysis.as_str())
    {
        return Err(invalid("Invalid library filter"));
    }
    let order = match filter.sort.as_str() {
        "" | "recent" => "updated DESC,id",
        "moves-asc" => "moves ASC,updated DESC,id",
        "moves-desc" => "moves DESC,updated DESC,id",
        _ => return Err(invalid("Invalid library sort")),
    };
    synchronize(db)?;
    let tx = db.transaction()?;
    let category = if filter.category == "all" {
        ""
    } else {
        &filter.category
    };
    let player = if filter.player == "*" {
        ""
    } else {
        &filter.player
    };
    let profile = if filter.profile == "*" {
        ""
    } else {
        &filter.profile
    };
    let search = filter.search.trim().to_lowercase();
    let analysis = match filter.analysis.as_str() {
        "complete" => "Complete",
        "partial" => "Partial",
        "unanalyzed" => "Not analyzed",
        _ => "",
    };
    let where_sql = "(?1='' OR (?1='favorites' AND favorite=1) OR category=?1) AND (?2='' OR white=?2 OR black=?2 OR (?2='?' AND (white='Unknown' OR black='Unknown'))) AND (?3='' OR profile=?3) AND (?4='' OR instr(search,?4)>0) AND (?5='' OR analysis=?5)";
    let args = params![category, player, profile, search, analysis];
    let revision: u64 = tx.query_row(
        "SELECT revision FROM library_cache_state WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    let count_key = serde_json::to_string(&json!([
        "total", category, player, profile, search, analysis
    ]))?;
    let total = cached(&tx, &count_key, || {
        Ok(json!(tx.query_row(
            &format!("SELECT count(*) FROM library_cards WHERE {where_sql}"),
            args,
            |r| r.get::<_, u64>(0)
        )?))
    })?
    .as_u64()
    .ok_or(invalid("Invalid cached archive count"))?;
    let limit = if filter.limit == 0 { 10 } else { filter.limit };
    let page = (filter.page as u64).min(total.saturating_sub(1) / limit as u64);
    let mut cards = Vec::new();
    let sql = if filter.selection {
        format!(
            "SELECT json_object('id',id,'revision',revision,'title',json_extract(summary,'$.title')) FROM library_cards WHERE {where_sql} ORDER BY {order} LIMIT 100 OFFSET ?6"
        )
    } else {
        format!(
            "SELECT summary FROM library_cards WHERE {where_sql} ORDER BY {order} LIMIT {limit} OFFSET ?6"
        )
    };
    let offset = if filter.selection {
        filter.page as u64 * 100
    } else {
        page * limit as u64
    };
    for row in tx.prepare(&sql)?.query_map(
        params![category, player, profile, search, analysis, offset],
        |r| r.get::<_, String>(0),
    )? {
        cards.push(serde_json::from_str::<Value>(&row?)?);
    }
    let mut result = json!({"games":cards,"total":total,"page":page,"revision":revision});
    if !filter.selection && filter.facets_revision != Some(revision) {
        let facets = cached(
            &tx,
            &serde_json::to_string(&json!(["facets", category]))?,
            || facets(&tx, category),
        )?;
        for key in ["counts", "players", "profiles"] {
            result[key] = facets[key].clone();
        }
    }
    tx.commit()?;
    Ok(result)
}

fn cached(db: &Connection, key: &str, calculate: impl FnOnce() -> Result<Value>) -> Result<Value> {
    if let Some(value) = db
        .query_row(
            "SELECT value FROM library_query_cache WHERE key=?1",
            [key],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(serde_json::from_str(&value)?);
    }
    let value = calculate()?;
    let bytes = serde_json::to_string(&value)?;
    if bytes.len() > crate::archive::MAX_BODY {
        return Err(invalid("Library filter choices exceed response limits"));
    }
    db.execute("DELETE FROM library_query_cache WHERE rowid IN (SELECT rowid FROM library_query_cache ORDER BY rowid LIMIT max(0,(SELECT count(*) FROM library_query_cache)-127))",[])?;
    db.execute(
        "INSERT INTO library_query_cache VALUES(?1,?2)",
        params![key, bytes],
    )?;
    Ok(value)
}
fn facets(db: &Connection, category: &str) -> Result<Value> {
    let mut counts = serde_json::Map::new();
    counts.insert(
        "all".into(),
        json!(db.query_row("SELECT count(*) FROM library_cards", [], |r| {
            r.get::<_, u64>(0)
        })?),
    );
    for row in db
        .prepare("SELECT category,count(*) FROM library_cards GROUP BY category")?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?)))?
    {
        let (k, v) = row?;
        counts.insert(k, json!(v));
    }
    counts.insert(
        "favorites".into(),
        json!(db.query_row(
            "SELECT count(*) FROM library_cards WHERE favorite=1",
            [],
            |r| r.get::<_, u64>(0)
        )?),
    );
    let players=db.prepare("SELECT white FROM library_cards WHERE (?1='' OR (?1='favorites' AND favorite=1) OR category=?1) UNION SELECT black FROM library_cards WHERE (?1='' OR (?1='favorites' AND favorite=1) OR category=?1) ORDER BY 1")?.query_map([category],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?;
    let profiles=db.prepare("SELECT profile,max(json_extract(summary,'$.trainingProfileName')) FROM library_cards WHERE profile<>'' GROUP BY profile ORDER BY 2,1")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(json!({"counts":counts,"players":players,"profiles":profiles}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archive;
    fn database() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE games(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,document TEXT NOT NULL,fingerprint TEXT UNIQUE);CREATE TABLE collections(id TEXT PRIMARY KEY,name TEXT,revision INTEGER);CREATE TABLE memberships(collection_id TEXT,game_id TEXT,PRIMARY KEY(collection_id,game_id));").unwrap();
        crate::analysis_runs::initialize(&db).unwrap();
        initialize(&db).unwrap();
        db
    }
    fn document(i: usize) -> Document {
        let mut doc=crate::archive_pgn::documents(&format!("[White \"Player {i}\"]\n[Black \"Opponent\"]\n[Date \"2026.10.20\"]\n[Site \"Test venue\"]\n\n1. e4 {{note}} (1. d4 d5) e5 *")).unwrap().remove(0);
        doc.record = Some(
            json!({"id":format!("local-{i}"),"category":if i%2==0 {"mine"}else{"imported"},"favorite":i%3==0,"updatedAt":i+1,"source":"test source",
            "json":serde_json::to_string(&json!({"board":"8/8/8/8/8/8/8/K6k w - - 0 1","history":[],"live_moves":vec!["e4";i%5+1],"game_analysis":if i%2==0{vec![json!({"score":0});i%5+2]}else{vec![]}})).unwrap()}),
        );
        doc
    }
    #[test]
    fn pages_filters_facets_sort_and_selection_never_return_documents() {
        let mut db = database();
        archive::import(&mut db, (0..125).map(document).collect(), None).unwrap();
        let all = query(&mut db, Filter::default()).unwrap();
        assert_eq!(all["total"], 125);
        assert_eq!(all["games"].as_array().unwrap().len(), 10);
        assert_eq!(all["counts"]["mine"], 63);
        assert_eq!(all["counts"]["favorites"], 42);
        assert_eq!(all["players"].as_array().unwrap().len(), 126);
        assert_eq!(all["games"][0]["white"], "Player 124");
        assert!(all["games"][0].get("document").is_none());
        assert!(all["games"][0].get("json").is_none());
        let second = query(
            &mut db,
            Filter {
                page: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(second["games"][0]["white"], "Player 114");
        let search = query(
            &mut db,
            Filter {
                category: "mine".into(),
                search: "PLAYER 12".into(),
                analysis: "complete".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(search["total"], 4);
        let player = query(
            &mut db,
            Filter {
                player: "Player 12".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(player["total"], 1);
        let asc = query(
            &mut db,
            Filter {
                sort: "moves-asc".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(asc["games"][0]["moves"], 1);
        let desc = query(
            &mut db,
            Filter {
                sort: "moves-desc".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(desc["games"][0]["moves"], 5);
        let selected = query(
            &mut db,
            Filter {
                selection: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(selected["games"].as_array().unwrap().len(), 100);
        assert!(selected["games"][0].get("finalFen").is_none());
        let rest = query(
            &mut db,
            Filter {
                selection: true,
                page: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(rest["games"].as_array().unwrap().len(), 25);
        assert!(
            query(
                &mut db,
                Filter {
                    sort: "updated;DROP TABLE games".into(),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    #[test]
    fn counts_and_facets_are_cached_separately_and_mutations_invalidate_both() {
        let mut db = database();
        archive::import(&mut db, (0..25).map(document).collect(), None).unwrap();
        let first = query(&mut db, Filter::default()).unwrap();
        let revision = first["revision"].as_u64().unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM library_query_cache", [], |r| r
                .get::<_, u64>(0))
                .unwrap(),
            2
        );
        let second = query(
            &mut db,
            Filter {
                page: 1,
                sort: "moves-desc".into(),
                facets_revision: Some(revision),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(second.get("players").is_none());
        assert!(second.get("counts").is_none());
        assert_eq!(second["total"], 25);
        assert_eq!(
            db.query_row("SELECT count(*) FROM library_query_cache", [], |r| r
                .get::<_, u64>(0))
                .unwrap(),
            2
        );
        query(
            &mut db,
            Filter {
                search: "Player 1".into(),
                facets_revision: Some(revision),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM library_query_cache", [], |r| r
                .get::<_, u64>(0))
                .unwrap(),
            3
        );
        let id = first["games"][0]["id"].as_str().unwrap();
        let mut game = archive::get(&db, id).unwrap();
        game.document.record.as_mut().unwrap()["favorite"] = json!(false);
        archive::update(&mut db, id, game.revision, game.document).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM library_query_cache", [], |r| r
                .get::<_, u64>(0))
                .unwrap(),
            0
        );
        let fresh = query(
            &mut db,
            Filter {
                facets_revision: Some(revision),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(fresh["revision"].as_u64().unwrap() > revision);
        assert!(fresh["players"].is_array());
        assert_eq!(fresh["counts"]["favorites"], 8);
        for i in 0..140 {
            query(
                &mut db,
                Filter {
                    search: format!("unique {i}"),
                    facets_revision: fresh["revision"].as_u64(),
                    ..Default::default()
                },
            )
            .unwrap();
        }
        assert!(
            db.query_row("SELECT count(*) FROM library_query_cache", [], |r| r
                .get::<_, u64>(0))
                .unwrap()
                <= 128
        );
    }

    #[test]
    fn training_unknown_players_analysis_and_empty_pages_match_controls() {
        let mut db = database();
        let mut training = document(2);
        let record = training.record.as_mut().unwrap();
        let mut saved: Value = serde_json::from_str(record["json"].as_str().unwrap()).unwrap();
        saved["training"] = json!({"profile_id":"profile","profile_name":"Dad","side":"White","engine":{"kind":"lc0","name":"Lc0"},"difficulty_level":2,"node_budget":10,"rating_before":1,"rating_after":2});
        saved["game_analysis"] = json!([{"score":0}]);
        record["json"] = json!(serde_json::to_string(&saved).unwrap());
        let unknown = crate::archive_pgn::documents("1. d4 *").unwrap().remove(0);
        archive::import(&mut db, vec![training, unknown], None).unwrap();
        let result = query(
            &mut db,
            Filter {
                category: "training".into(),
                profile: "profile".into(),
                player: "Dad".into(),
                analysis: "partial".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result["total"], 1);
        assert_eq!(result["games"][0]["black"], "Lc0 · Level 2 · 10 nodes");
        assert_eq!(result["profiles"][0], json!(["profile", "Dad"]));
        assert_eq!(result["games"][0]["ratingAfter"], 2);
        let result = query(
            &mut db,
            Filter {
                player: "?".into(),
                analysis: "unanalyzed".into(),
                page: 999,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result["total"], 1);
        assert_eq!(result["page"], 0);
        let empty = query(
            &mut db,
            Filter {
                search: "no match".into(),
                page: 999,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(empty["total"], 0);
        assert_eq!(empty["page"], 0);
        assert_eq!(empty["games"], json!([]));
    }

    #[test]
    fn index_backfill_updates_deletes_and_pgn_only_cards_are_consistent() {
        let mut db = database();
        let doc = document(1);
        let result = archive::import(&mut db, vec![doc.clone()], None).unwrap();
        let id = result["ids"][0].as_str().unwrap();
        let original = archive::get(&db, id).unwrap().document;
        let first = query(&mut db, Filter::default()).unwrap();
        assert_eq!(first["games"][0]["category"], "imported");
        let mut changed = doc;
        changed.record.as_mut().unwrap()["favorite"] = json!(true);
        archive::update(&mut db, id, 1, changed).unwrap();
        let favorite = query(
            &mut db,
            Filter {
                category: "favorites".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(favorite["total"], 1);
        assert_eq!(favorite["games"][0]["revision"], 2);
        archive::delete(&mut db, id, 2).unwrap();
        assert_eq!(query(&mut db, Filter::default()).unwrap()["total"], 0);
        let mut plain = original;
        plain.record = None;
        archive::import(&mut db, vec![plain], None).unwrap();
        // A missing derived index is rebuilt without changing original documents.
        db.execute("DELETE FROM library_dirty", []).unwrap();
        db.execute("DELETE FROM library_cards", []).unwrap();
        crate::analysis_runs::initialize(&db).unwrap();
        initialize(&db).unwrap();
        let rebuilt = query(&mut db, Filter::default()).unwrap();
        assert_eq!(rebuilt["games"][0]["moves"], 2);
        assert!(
            !rebuilt["games"][0]["finalFen"]
                .as_str()
                .unwrap()
                .contains(" b ")
        );
        let raw: String = db
            .query_row("SELECT document FROM games", [], |r| r.get(0))
            .unwrap();
        assert!(
            serde_json::from_str::<Document>(&raw)
                .unwrap()
                .record
                .is_none()
        );
    }
}

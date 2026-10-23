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
    let migrated: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='library_openings_v1')",[],|row|row.get(0))?;
    if !migrated {
        tx.execute_batch("CREATE TABLE library_openings_v1(id INTEGER PRIMARY KEY); INSERT OR IGNORE INTO library_dirty SELECT id FROM games; DELETE FROM library_query_cache; UPDATE library_cache_state SET revision=revision+1 WHERE id=1;")?;
    }
    tx.commit()?;
    Ok(())
}

fn summary(id: &str, revision: u64, updated: i64, document: &Document) -> Result<Value> {
    let parsed = crate::archive_pgn::parse(&document.pgn)?
        .into_iter()
        .next()
        .ok_or(invalid("Missing PGN game"))?;
    summary_parsed(id,revision,updated,document,&parsed)
}
pub(crate) fn summary_parsed(id:&str,revision:u64,updated:i64,document:&Document,parsed:&crate::archive_pgn::Parsed)->Result<Value> {
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
    let mut eco=tag("ECO");
    let mut opening=tag("Opening");
    if (eco.trim().is_empty() || eco=="?" || opening.trim().is_empty() || opening=="?") && tag("FEN").is_empty() && ["","Standard","Chess"].contains(&tag("Variant").as_str()) {
        if let Some((classified_eco,classified_opening))=crate::opening_classification::classify(&parsed.sans) {
            if eco.trim().is_empty() || eco=="?" {eco=classified_eco;}
            if opening.trim().is_empty() || opening=="?" {opening=classified_opening;}
        }
    }
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
        "eco":eco,"opening":opening,
        "favorite":record["favorite"].as_bool().unwrap_or(false),"source":record["source"],"finalFen":final_fen,
        "startingNote":saved["move_notes"][0].as_str().unwrap_or("").trim(),"puzzleId":puzzle_id,
        "trainingProfileId":profile,"trainingProfileName":profile_name,"trainingSide":training["side"],
        "trainingEngineKind":engine_kind,"trainingOpponentElo":training["opponent_elo"],
        "ratingBefore":training["rating_before"],"ratingAfter":training["rating_after"]}),
    )
}

pub(crate) fn store_card(db:&Connection,id:&str,revision:u64,updated:i64,card:&Value)->Result<()> {
    let search=["title","white","black","source","venue","playedAt"].map(|key|card[key].as_str().unwrap_or("")).join(" ").to_lowercase();
    db.execute("INSERT OR REPLACE INTO library_cards VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![id,revision,card["updatedAt"].as_i64().unwrap_or(updated),card["moves"].as_u64().unwrap_or(0),card["category"].as_str(),card["favorite"].as_bool(),card["white"].as_str(),card["black"].as_str(),card["trainingProfileId"].as_str(),card["analysisStatus"].as_str(),search,serde_json::to_string(card)?])?;
    db.execute("DELETE FROM library_dirty WHERE id=?1",[id])?;
    Ok(())
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
    pub source: String,
    pub date: String,
    pub analysis: String,
    pub eco: String,
    pub opening: String,
    pub opening_choice: String,
    pub sort: String,
    pub page: u32,
    pub limit: u32,
    pub selection: bool,
    pub facets_revision: Option<u64>,
}
const SOURCE_VALUE: &str = "coalesce(nullif((CASE WHEN trim(coalesce(json_extract(summary,'$.source'),'')) IN ('?','-','????.??.??','????-??-??') THEN '' ELSE trim(coalesce(json_extract(summary,'$.source'),'')) END),''),(CASE WHEN trim(coalesce(json_extract(summary,'$.venue'),'')) IN ('?','-','????.??.??','????-??-??') THEN '' ELSE trim(coalesce(json_extract(summary,'$.venue'),'')) END))";
const DATE_VALUE: &str = "CASE WHEN trim(coalesce(json_extract(summary,'$.playedAt'),'')) IN ('?','-','????.??.??','????-??-??') THEN '' ELSE replace(trim(coalesce(json_extract(summary,'$.playedAt'),'')),'.','-') END";
const ECO_VALUE: &str = "CASE WHEN trim(coalesce(json_extract(summary,'$.eco'),'')) IN ('?','-') THEN '' ELSE upper(trim(coalesce(json_extract(summary,'$.eco'),''))) END";
const OPENING_VALUE: &str = "CASE WHEN trim(coalesce(json_extract(summary,'$.opening'),'')) IN ('?','-') THEN '' ELSE trim(coalesce(json_extract(summary,'$.opening'),'')) END";
pub fn query(db: &mut Connection, filter: Filter) -> Result<Value> {
    if [
        &filter.category,
        &filter.player,
        &filter.profile,
        &filter.search,
        &filter.source,
        &filter.date,
        &filter.analysis,
        &filter.eco,
        &filter.opening,
        &filter.sort,
    ]
    .iter()
    .any(|v| v.len() > 256)
        || filter.opening_choice.len()>1024
        || filter.limit > if filter.selection { 1000 } else { 100 }
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
    let source=if filter.source=="*" { "" } else { filter.source.trim() };
    let date=if filter.date=="*" { String::new() } else { filter.date.trim().replace('.',"-") };
    let analysis = match filter.analysis.as_str() {
        "complete" => "Complete",
        "partial" => "Partial",
        "unanalyzed" => "Not analyzed",
        _ => "",
    };
    let eco=filter.eco.trim().to_uppercase();
    let opening=filter.opening.trim().to_lowercase();
    let (choice_mode,choice_eco,choice_name)=match filter.opening_choice.as_str() {
        ""|"*"=>(0,String::new(),String::new()),
        "?"=>(2,String::new(),String::new()),
        value=>{let pair:[String;2]=serde_json::from_str(value).map_err(|_|invalid("Invalid opening choice"))?;
            if pair.iter().any(|part|part.len()>256){return Err(invalid("Invalid opening choice"));}
            (1,pair[0].trim().to_uppercase(),pair[1].trim().to_owned())}
    };
    let where_sql = format!("(?1='' OR (?1='favorites' AND favorite=1) OR category=?1) AND (?2='' OR white=?2 OR black=?2 OR (?2='?' AND (white='Unknown' OR black='Unknown'))) AND (?3='' OR profile=?3) AND (?4='' OR instr(search,?4)>0) AND (?5='' OR analysis=?5) AND (?6='' OR substr(upper(json_extract(summary,'$.eco')),1,length(?6))=?6) AND (?7='' OR instr(lower(json_extract(summary,'$.opening')),?7)>0) AND (?8='' OR ({SOURCE_VALUE})=?8 OR (?8='?' AND ({SOURCE_VALUE})='')) AND (?9='' OR ({DATE_VALUE})=?9 OR (?9='?' AND ({DATE_VALUE})='')) AND (?12=0 OR (?12=1 AND ({ECO_VALUE})=?10 AND ({OPENING_VALUE})=?11) OR (?12=2 AND ({ECO_VALUE})='' AND ({OPENING_VALUE})=''))");
    let args = params![category, player, profile, search, analysis, eco, opening,source,date,choice_eco,choice_name,choice_mode];
    let revision: u64 = tx.query_row(
        "SELECT revision FROM library_cache_state WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    let count_key = serde_json::to_string(&json!([
        "total-source-date-v2", category, player, profile, search, analysis, eco, opening,source,date,filter.opening_choice
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
    let limit = if filter.limit == 0 { if filter.selection { 100 } else { 10 } } else { filter.limit };
    let page = (filter.page as u64).min(total.saturating_sub(1) / limit as u64);
    let mut cards = Vec::new();
    let sql = if filter.selection {
        format!(
            "SELECT json_object('id',id,'revision',revision,'title',json_extract(summary,'$.title')) FROM library_cards WHERE {where_sql} ORDER BY {order} LIMIT {limit} OFFSET ?13"
        )
    } else {
        format!(
            "SELECT summary FROM library_cards WHERE {where_sql} ORDER BY {order} LIMIT {limit} OFFSET ?13"
        )
    };
    let offset = if filter.selection {
        filter.page as u64 * limit as u64
    } else {
        page * limit as u64
    };
    for row in tx.prepare(&sql)?.query_map(
        params![category, player, profile, search, analysis, eco, opening, source,date,choice_eco,choice_name,choice_mode,offset],
        |r| r.get::<_, String>(0),
    )? {
        cards.push(serde_json::from_str::<Value>(&row?)?);
    }
    let mut result = json!({"games":cards,"total":total,"page":page,"revision":revision});
    if !filter.selection && filter.facets_revision != Some(revision) {
        let facets = cached(
            &tx,
            &serde_json::to_string(&json!(["facets-openings-v3", category]))?,
            || facets(&tx, category),
        )?;
        for key in ["counts", "players", "profiles", "sources", "dates", "openings"] {
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
    let choices=|expression:&str,descending:bool|->Result<Vec<String>> {
        let sql=format!("SELECT DISTINCT ({expression}) AS value FROM library_cards WHERE (?1='' OR (?1='favorites' AND favorite=1) OR category=?1) AND ({expression})<>'' ORDER BY value {}",if descending {"DESC"} else {"ASC"});
        Ok(db.prepare(&sql)?.query_map([category],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?)
    };
    let sources=choices(SOURCE_VALUE,false)?;
    let dates=choices(DATE_VALUE,true)?;
    let openings=db.prepare(&format!("SELECT DISTINCT ({ECO_VALUE}),({OPENING_VALUE}) FROM library_cards WHERE (?1='' OR (?1='favorites' AND favorite=1) OR category=?1) AND (({ECO_VALUE})<>'' OR ({OPENING_VALUE})<>'') ORDER BY 1,2"))?.query_map([category],|r|Ok(json!({"eco":r.get::<_,String>(0)?,"opening":r.get::<_,String>(1)?})))?.collect::<std::result::Result<Vec<_>,_>>()?;
    Ok(json!({"counts":counts,"players":players,"profiles":profiles,"sources":sources,"dates":dates,"openings":openings}))
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
    fn source_date_choices_filtering_and_selection_agree() {
        let mut db=database();
        let mut a=document(0);a.record.as_mut().unwrap()["source"]=json!("Lichess");
        let mut b=document(1);b.record.as_mut().unwrap()["source"]=json!("Local");b.metadata.insert("Date".into(),"2026-10-20".into());
        let mut c=document(2);c.record.as_mut().unwrap()["source"]=Value::Null;c.metadata.insert("Date".into(),"????.??.??".into());c.metadata.insert("Site".into(),"-".into());
        archive::import(&mut db,vec![a,b,c],None).unwrap();
        let all=query(&mut db,Filter::default()).unwrap();
        assert_eq!(all["sources"],json!(["Lichess","Local"]));assert_eq!(all["dates"],json!(["2026-10-20"]));
        let chosen=query(&mut db,Filter{source:"Lichess".into(),date:"2026.10.20".into(),player:"Player 0".into(),selection:true,..Default::default()}).unwrap();assert_eq!(chosen["total"],1);assert_eq!(chosen["games"][0]["title"],"Player 0 vs Opponent");
        let date=query(&mut db,Filter{date:"2026-10-20".into(),..Default::default()}).unwrap();assert_eq!(date["total"],2);
        let unknown=query(&mut db,Filter{source:"?".into(),date:"?".into(),..Default::default()}).unwrap();assert_eq!(unknown["total"],1);assert_eq!(unknown["games"][0]["white"],"Player 2");
        let category=query(&mut db,Filter{category:"imported".into(),..Default::default()}).unwrap();assert_eq!(category["sources"],json!(["Local"]));
        let wrong=query(&mut db,Filter{source:"Lichess".into(),date:"?".into(),..Default::default()}).unwrap();assert_eq!(wrong["total"],0);
        assert_eq!(query(&mut db,Filter::default()).unwrap()["total"],3);
        let mut d=document(3);d.record.as_mut().unwrap()["source"]=Value::Null;archive::import(&mut db,vec![d],None).unwrap();
        let site=query(&mut db,Filter{source:"Test venue".into(),..Default::default()}).unwrap();assert_eq!(site["total"],1);assert_eq!(site["games"][0]["white"],"Player 3");

    }

    #[test]
    fn combined_opening_choices_match_exact_variations_and_unknowns() {
        let mut db=database();
        let docs=crate::archive_pgn::documents("[White \"Base\"]\n[ECO \"B20\"]\n[Opening \"Sicilian Defense\"]\n\n1. e4 c5 *\n\n[White \"Variation\"]\n[ECO \"B20\"]\n[Opening \"Sicilian Defense: Variation\"]\n\n1. e4 c5 *\n\n[White \"Unknown\"]\n\n*").unwrap();
        archive::import(&mut db,docs,None).unwrap();
        let all=query(&mut db,Filter::default()).unwrap();assert_eq!(all["openings"].as_array().unwrap().len(),2);
        let selected=query(&mut db,Filter{opening_choice:serde_json::to_string(&["B20","Sicilian Defense"]).unwrap(),selection:true,..Default::default()}).unwrap();assert_eq!(selected["total"],1);assert_eq!(selected["games"][0]["title"],"Base vs Unknown");
        let variation=query(&mut db,Filter{opening_choice:serde_json::to_string(&["B20","Sicilian Defense: Variation"]).unwrap(),..Default::default()}).unwrap();assert_eq!(variation["total"],1);assert_eq!(variation["games"][0]["white"],"Variation");
        let unknown=query(&mut db,Filter{opening_choice:"?".into(),..Default::default()}).unwrap();assert_eq!(unknown["total"],1);assert_eq!(unknown["games"][0]["white"],"Unknown");
        assert!(query(&mut db,Filter{opening_choice:"invalid".into(),..Default::default()}).is_err());
    }

    #[test]
    fn opening_filters_classify_missing_tags_preserve_tags_and_rebuild_old_cards() {
        let mut db=database();
        let docs=crate::archive_pgn::documents("[White \"Sicilian\"]\n\n1. e4 c5 *\n\n[White \"Tagged\"]\n[ECO \"A99\"]\n[Opening \"Custom tag\"]\n\n1. e4 c5 *\n\n[White \"French\"]\n\n1. d4 e6 2. e4 d5 *").unwrap();
        archive::import(&mut db,docs,None).unwrap();
        let sicilian=query(&mut db,Filter{eco:"b2".into(),opening:"SICILIAN".into(),..Default::default()}).unwrap();
        assert_eq!(sicilian["total"],1);assert_eq!(sicilian["games"][0]["eco"],"B20");
        let tagged=query(&mut db,Filter{opening:"custom tag".into(),..Default::default()}).unwrap();assert_eq!(tagged["games"][0]["eco"],"A99");
        let french=query(&mut db,Filter{opening:"French".into(),selection:true,..Default::default()}).unwrap();assert_eq!(french["total"],1);
        db.execute_batch("UPDATE library_cards SET summary=json_remove(summary,'$.eco','$.opening'); DROP TABLE library_openings_v1;").unwrap();
        initialize(&db).unwrap();
        assert_eq!(query(&mut db,Filter{eco:"B20".into(),..Default::default()}).unwrap()["total"],1);
        assert_eq!(db.query_row("SELECT count(*) FROM games",[],|row|row.get::<_,u64>(0)).unwrap(),3);
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
        let bulk=query(&mut db,Filter{selection:true,limit:1000,..Default::default()}).unwrap();
        assert_eq!(bulk["games"].as_array().unwrap().len(),125);
        assert_eq!(bulk["games"],json!(selected["games"].as_array().unwrap().iter().chain(rest["games"].as_array().unwrap()).cloned().collect::<Vec<_>>()));
        assert!(bulk["games"][0].get("record").is_none());
        assert!(query(&mut db,Filter{limit:1000,..Default::default()}).is_err());
        assert!(query(&mut db,Filter{selection:true,limit:1001,..Default::default()}).is_err());

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

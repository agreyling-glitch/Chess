//! Rebuildable standard-chess position index. Source games remain authoritative.
use crate::archive::{self, Document, Error, Result, invalid};
use pgn_reader::{Outcome, RawTag, Reader, SanPlus, Skip, Visitor};
use rusqlite::{Connection, OptionalExtension, named_params, params};
use serde::Deserialize;
use serde_json::{Value, json};
use shakmaty::{CastlingMode, Chess, EnPassantMode, Position, fen::Fen, san::San, uci::UciMove};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::ControlFlow,
    time::Duration,
};

const VERSION: i64 = 1;
pub fn initialize(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS explorer_state(id INTEGER PRIMARY KEY CHECK(id=1),version INTEGER NOT NULL,generation INTEGER NOT NULL); INSERT OR IGNORE INTO explorer_state VALUES(1,1,1);
    CREATE TABLE IF NOT EXISTS explorer_games(game_id TEXT PRIMARY KEY REFERENCES games(id) ON DELETE CASCADE,revision INTEGER NOT NULL,status TEXT NOT NULL,white TEXT,black TEXT,white_key TEXT,black_key TEXT,played_date TEXT,white_elo INTEGER,black_elo INTEGER,result TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS explorer_positions(game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,position TEXT NOT NULL,uci TEXT NOT NULL,san TEXT NOT NULL,PRIMARY KEY(game_id,position,uci)) WITHOUT ROWID;
    CREATE INDEX IF NOT EXISTS explorer_lookup ON explorer_positions(position,uci,game_id);
    CREATE TABLE IF NOT EXISTS explorer_queue(game_id TEXT PRIMARY KEY REFERENCES games(id) ON DELETE CASCADE);
    CREATE TRIGGER IF NOT EXISTS explorer_game_insert AFTER INSERT ON games BEGIN UPDATE explorer_state SET generation=generation+1; INSERT OR IGNORE INTO explorer_queue VALUES(NEW.id); END;
    CREATE TRIGGER IF NOT EXISTS explorer_game_update AFTER UPDATE ON games BEGIN UPDATE explorer_state SET generation=generation+1; INSERT OR IGNORE INTO explorer_queue VALUES(NEW.id); END;
    CREATE TRIGGER IF NOT EXISTS explorer_game_delete AFTER DELETE ON games BEGIN UPDATE explorer_state SET generation=generation+1; END;
    CREATE TRIGGER IF NOT EXISTS explorer_member_insert AFTER INSERT ON memberships BEGIN UPDATE explorer_state SET generation=generation+1; END;
    CREATE TRIGGER IF NOT EXISTS explorer_member_delete AFTER DELETE ON memberships BEGIN UPDATE explorer_state SET generation=generation+1; END;
    CREATE TRIGGER IF NOT EXISTS explorer_collection_update AFTER UPDATE ON collections BEGIN UPDATE explorer_state SET generation=generation+1; END;
    CREATE TRIGGER IF NOT EXISTS explorer_collection_delete AFTER DELETE ON collections BEGIN UPDATE explorer_state SET generation=generation+1; END;
    INSERT OR IGNORE INTO explorer_queue SELECT g.id FROM games g LEFT JOIN explorer_games i ON i.game_id=g.id WHERE i.revision IS NULL OR i.revision<>g.revision;")?;
    let version: i64 = db.query_row("SELECT version FROM explorer_state WHERE id=1", [], |r| {
        r.get(0)
    })?;
    if version != VERSION {
        db.execute_batch("DELETE FROM explorer_positions;DELETE FROM explorer_games;INSERT OR IGNORE INTO explorer_queue SELECT id FROM games;UPDATE explorer_state SET version=1,generation=generation+1;")?;
    }
    Ok(())
}
pub fn position_key(position: &Chess) -> String {
    Fen::from_position(position, EnPassantMode::Legal)
        .to_string()
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
}
fn position(fen: &str) -> Result<Chess> {
    if fen.split_whitespace().count() != 6 {
        return Err(invalid("Explorer FEN requires all six fields"));
    }
    fen.parse::<Fen>()
        .ok()
        .and_then(|f| f.into_position(CastlingMode::Standard).ok())
        .ok_or(invalid("Invalid standard-chess explorer position"))
}

#[derive(Default)]
struct Indexer;
pub struct Indexed {
    tags: BTreeMap<String, String>,
    entries: BTreeSet<(String, String, String)>,
    status: &'static str,
}
struct Line {
    data: Indexed,
    position: Chess,
    plies: usize,
}
impl Visitor for Indexer {
    type Tags = BTreeMap<String, String>;
    type Movetext = Line;
    type Output = Result<Indexed>;
    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> {
        ControlFlow::Continue(BTreeMap::new())
    }
    fn tag(
        &mut self,
        tags: &mut Self::Tags,
        name: &[u8],
        value: RawTag<'_>,
    ) -> ControlFlow<Self::Output> {
        let (Ok(name), Ok(value)) = (std::str::from_utf8(name), value.decode_utf8()) else {
            return ControlFlow::Break(Err(invalid("Invalid indexed PGN tags")));
        };
        tags.insert(name.into(), value.into_owned());
        ControlFlow::Continue(())
    }
    fn begin_movetext(&mut self, tags: Self::Tags) -> ControlFlow<Self::Output, Self::Movetext> {
        if tags
            .get("Variant")
            .is_some_and(|v| !matches!(v.as_str(), "Standard" | "Chess" | "From Position"))
        {
            return ControlFlow::Continue(Line {
                data: Indexed {
                    tags,
                    entries: BTreeSet::new(),
                    status: "unsupported_variant",
                },
                position: Chess::default(),
                plies: 0,
            });
        }
        let pos = match tags.get("FEN") {
            Some(f) => match position(f) {
                Ok(p) => p,
                Err(e) => return ControlFlow::Break(Err(e)),
            },
            None => Chess::default(),
        };
        let mut data = Indexed {
            tags,
            entries: BTreeSet::new(),
            status: "ready",
        };
        data.entries
            .insert((position_key(&pos), String::new(), String::new()));
        ControlFlow::Continue(Line {
            data,
            position: pos,
            plies: 0,
        })
    }
    fn san(&mut self, line: &mut Line, san: SanPlus) -> ControlFlow<Self::Output> {
        if line.data.status == "unsupported_variant" {
            return ControlFlow::Continue(());
        }
        if line.plies >= 10000 {
            return ControlFlow::Break(Err(invalid("Indexed game exceeds move limit")));
        }
        let Ok(m) = san.san.to_move(&line.position) else {
            return ControlFlow::Break(Err(invalid("Illegal indexed PGN move")));
        };
        line.data.entries.insert((
            position_key(&line.position),
            UciMove::from_standard(m).to_string(),
            San::from_move(&line.position, m).to_string(),
        ));
        line.position.play_unchecked(m);
        line.plies += 1;
        line.data
            .entries
            .insert((position_key(&line.position), String::new(), String::new()));
        ControlFlow::Continue(())
    }
    fn begin_variation(&mut self, _: &mut Line) -> ControlFlow<Self::Output, Skip> {
        ControlFlow::Continue(Skip(true))
    }
    fn outcome(&mut self, line: &mut Line, outcome: Outcome) -> ControlFlow<Self::Output> {
        line.data
            .tags
            .entry("Result".into())
            .or_insert_with(|| outcome.to_string());
        ControlFlow::Continue(())
    }
    fn end_game(&mut self, line: Line) -> Self::Output {
        Ok(line.data)
    }
}
pub fn index(document: &Document) -> Result<Indexed> {
    let mut reader = Reader::new(document.pgn.as_bytes());
    let mut indexed = reader
        .read_game(&mut Indexer)
        .map_err(|_| invalid("Cannot index malformed PGN"))?
        .ok_or(invalid("Cannot index empty PGN"))??;
    if reader
        .read_game(&mut Indexer)
        .map_err(|_| invalid("Cannot index malformed PGN"))?
        .is_some()
    {
        return Err(invalid("Indexed document must contain one game"));
    }
    // Archive search metadata overrides tags, without rewriting the source PGN.
    indexed.tags.extend(document.metadata.clone());
    if indexed
        .tags
        .get("Variant")
        .is_some_and(|v| !matches!(v.as_str(), "Standard" | "Chess" | "From Position"))
    {
        indexed.status = "unsupported_variant";
        indexed.entries.clear();
    }
    Ok(indexed)
}
fn name(value: Option<&String>) -> Option<String> {
    value
        .filter(|v| !v.trim().is_empty() && !matches!(v.as_str(), "?" | "-"))
        .cloned()
}
fn rating(value: Option<&String>) -> Option<u32> {
    value
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|v| (1..=5000).contains(v))
}
fn date(value: &str) -> Option<String> {
    let text = value.replace('-', ".");
    let parts = text.split('.').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return None;
    }
    let (y, m, d) = (
        parts[0].parse::<u32>().ok()?,
        parts[1].parse::<u32>().ok()?,
        parts[2].parse::<u32>().ok()?,
    );
    let days = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => return None,
    };
    (y > 0 && (1..=days).contains(&d)).then_some(text)
}
struct Job {
    id: String,
    revision: u64,
    fingerprint: String,
    document: Document,
}
fn next(db: &Connection) -> Result<Option<Job>> {
    let candidate: Option<String> = db
        .query_row(
            "SELECT game_id FROM explorer_queue ORDER BY game_id LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    candidate
        .map(|id| {
            let game = archive::get(db, &id)?;
            let fingerprint =
                db.query_row("SELECT fingerprint FROM games WHERE id=?1", [&id], |r| {
                    r.get(0)
                })?;
            Ok(Job {
                id,
                revision: game.revision,
                fingerprint,
                document: game.document,
            })
        })
        .transpose()
}
fn apply(
    db: &mut Connection,
    id: &str,
    revision: u64,
    fingerprint: &str,
    result: Result<Indexed>,
) -> Result<()> {
    let tx = db.transaction()?;
    let current: Option<(u64, String)> = tx
        .query_row(
            "SELECT revision,fingerprint FROM games WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if current
        .as_ref()
        .is_none_or(|(r, f)| *r != revision || f != fingerprint)
    {
        return Ok(());
    }
    let data = result.unwrap_or_else(|_| Indexed {
        tags: BTreeMap::new(),
        entries: BTreeSet::new(),
        status: "failed",
    });
    let white = name(data.tags.get("White"));
    let black = name(data.tags.get("Black"));
    tx.execute("DELETE FROM explorer_positions WHERE game_id=?1", [id])?;
    tx.execute("INSERT INTO explorer_games VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(game_id) DO UPDATE SET revision=excluded.revision,status=excluded.status,white=excluded.white,black=excluded.black,white_key=excluded.white_key,black_key=excluded.black_key,played_date=excluded.played_date,white_elo=excluded.white_elo,black_elo=excluded.black_elo,result=excluded.result",params![id,revision,data.status,white,black,white.as_ref().map(|v|v.to_lowercase()),black.as_ref().map(|v|v.to_lowercase()),data.tags.get("Date").and_then(|v|date(v)),rating(data.tags.get("WhiteElo")),rating(data.tags.get("BlackElo")),data.tags.get("Result").map(String::as_str).unwrap_or("*")])?;
    {
        let mut statement = tx.prepare("INSERT INTO explorer_positions VALUES(?1,?2,?3,?4)")?;
        for (key, uci, san) in data.entries {
            statement.execute(params![id, key, uci, san])?;
        }
    }
    tx.execute("DELETE FROM explorer_queue WHERE game_id=?1", [id])?;
    tx.execute(
        "UPDATE explorer_state SET generation=generation+1 WHERE id=1",
        [],
    )?;
    tx.commit()?;
    Ok(())
}
pub fn spawn(archive: archive::Archive, control: crate::Control) {
    let permit = control
        .background
        .clone()
        .try_acquire_owned()
        .expect("One explorer worker per server");
    tokio::spawn(async move {
        let _permit = permit;
        loop {
            if control.stopping() {
                break;
            }
            let worked = async {
                let Some(Job {
                    id,
                    revision,
                    fingerprint,
                    document,
                }) = archive.run(|db| next(db)).await?
                else {
                    return Ok::<_, Error>(false);
                };
                // PGN replay is outside the archive connection lock, so reads remain available.
                let indexed = tokio::task::spawn_blocking(move || index(&document))
                    .await
                    .map_err(|_| Error("archive_failed", "Index worker failed"))?;
                archive
                    .run(move |db| apply(db, &id, revision, &fingerprint, indexed))
                    .await?;
                Ok(true)
            };
            let result = worked.await;
            if !matches!(result, Ok(true)) {
                tokio::select! {_ = control.cancelled()=>break,_=tokio::time::sleep(Duration::from_millis(500))=>{}}
            } else {
                tokio::task::yield_now().await;
            }
        }
    });
}
pub fn status(db: &Connection) -> Result<Value> {
    let generation: u64 = db.query_row(
        "SELECT generation FROM explorer_state WHERE id=1",
        [],
        |r| r.get(0),
    )?;
    let total: u64 = db.query_row("SELECT count(*) FROM games", [], |r| r.get(0))?;
    let pending: u64 = db.query_row("SELECT count(*) FROM explorer_queue", [], |r| r.get(0))?;
    let mut ready = 0u64;
    let mut unsupported = 0u64;
    let mut failed = 0u64;
    let mut stmt=db.prepare("SELECT i.status,count(*) FROM explorer_games i JOIN games g ON g.id=i.game_id AND g.revision=i.revision GROUP BY i.status")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?)))? {
        let (state, count) = row?;
        match state.as_str() {
            "ready" => ready = count,
            "unsupported_variant" => unsupported = count,
            "pending" => {}
            _ => failed += count,
        }
    }
    Ok(
        json!({"generation":generation,"version":VERSION,"total":total,"indexed":ready,"unsupported_variants":unsupported,"failed":failed,"pending":pending,"complete":pending==0&&failed==0}),
    )
}
pub fn rebuild(db: &mut Connection) -> Result<Value> {
    let tx = db.transaction()?;
    // Hide old rows immediately, but reclaim/replace positions one game at a time.
    // A large index must not be deleted under the archive lock in one request.
    tx.execute_batch("UPDATE explorer_games SET status='pending';INSERT OR IGNORE INTO explorer_queue SELECT id FROM games;UPDATE explorer_state SET generation=generation+1;")?;
    tx.commit()?;
    status(db)
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Query {
    pub fen: String,
    pub variant: String,
    pub collection: String,
    pub player: String,
    pub color: String,
    pub date_from: String,
    pub date_to: String,
    pub white_elo_min: Option<u32>,
    pub white_elo_max: Option<u32>,
    pub black_elo_min: Option<u32>,
    pub black_elo_max: Option<u32>,
    pub game_after: String,
    pub game_limit: u32,
    pub uci: String,
}
const MATCHED: &str = "WITH matched AS (SELECT i.* FROM explorer_games i JOIN games g ON g.id=i.game_id AND g.revision=i.revision JOIN explorer_positions p ON p.game_id=i.game_id AND p.position=:key AND p.uci='' WHERE i.status='ready' AND (:collection='' OR EXISTS(SELECT 1 FROM memberships c WHERE c.game_id=i.game_id AND c.collection_id=:collection)) AND (:player='' OR (:color IN('','either','white') AND instr(i.white_key,:player)>0) OR (:color IN('','either','black') AND instr(i.black_key,:player)>0)) AND (:from='' OR i.played_date>=:from) AND (:to='' OR i.played_date<=:to) AND (:wmin IS NULL OR i.white_elo>=:wmin) AND (:wmax IS NULL OR i.white_elo<=:wmax) AND (:bmin IS NULL OR i.black_elo>=:bmin) AND (:bmax IS NULL OR i.black_elo<=:bmax)) ";
const COUNTS: &str = "count(*),coalesce(sum(result='1-0'),0),coalesce(sum(result='1/2-1/2'),0),coalesce(sum(result='0-1'),0)";
fn counts(total: u64, white: u64, draws: u64, black: u64) -> Value {
    let completed = white + draws + black;
    let percentages = if completed == 0 {
        Value::Null
    } else {
        json!({"white":100.0*white as f64/completed as f64,"draws":100.0*draws as f64/completed as f64,"black":100.0*black as f64/completed as f64})
    };
    json!({"total":total,"white":white,"draws":draws,"black":black,"unknown":total-completed,"completed":completed,"percentages":percentages})
}
pub fn query(db: &mut Connection, q: Query) -> Result<Value> {
    if !matches!(q.variant.as_str(), "" | "standard") {
        return Err(invalid("Archive explorer supports standard chess only"));
    }
    if q.player.len() > 256
        || q.collection.len() > 128
        || q.game_after.len() > 128
        || q.game_limit > 50
        || !matches!(q.color.as_str(), "" | "either" | "white" | "black")
        || [
            q.white_elo_min,
            q.white_elo_max,
            q.black_elo_min,
            q.black_elo_max,
        ]
        .into_iter()
        .flatten()
        .any(|v| !(1..=5000).contains(&v))
        || q.white_elo_min
            .zip(q.white_elo_max)
            .is_some_and(|(a, b)| a > b)
        || q.black_elo_min
            .zip(q.black_elo_max)
            .is_some_and(|(a, b)| a > b)
    {
        return Err(invalid("Invalid explorer filter"));
    }
    let from = if q.date_from.is_empty() {
        String::new()
    } else {
        date(&q.date_from).ok_or(invalid("Use a valid YYYY-MM-DD explorer date"))?
    };
    let to = if q.date_to.is_empty() {
        String::new()
    } else {
        date(&q.date_to).ok_or(invalid("Use a valid YYYY-MM-DD explorer date"))?
    };
    if !from.is_empty() && !to.is_empty() && from > to {
        return Err(invalid("Explorer date range is reversed"));
    }
    let pos = position(&q.fen)?;
    if !q.uci.is_empty()
        && q.uci
            .parse::<UciMove>()
            .ok()
            .and_then(|m| m.to_move(&pos).ok())
            .is_none()
    {
        return Err(invalid(
            "Matching-game move must be legal in the queried position",
        ));
    }
    let key = position_key(&pos);
    let player = q.player.to_lowercase();
    let limit = if q.game_limit == 0 { 10 } else { q.game_limit };
    let values = named_params! {":key":key,":collection":q.collection,":player":player,":color":q.color,":from":from,":to":to,":wmin":q.white_elo_min,":wmax":q.white_elo_max,":bmin":q.black_elo_min,":bmax":q.black_elo_max};
    let tx = db.transaction()?;
    let progress = status(&tx)?;
    let (total,white,draws,black,missing_white,missing_black,missing_date,missing_white_elo,missing_black_elo)=tx.query_row(&format!("{MATCHED} SELECT {COUNTS},coalesce(sum(white IS NULL),0),coalesce(sum(black IS NULL),0),coalesce(sum(played_date IS NULL),0),coalesce(sum(white_elo IS NULL),0),coalesce(sum(black_elo IS NULL),0) FROM matched"),values,|r|Ok((r.get::<_,u64>(0)?,r.get::<_,u64>(1)?,r.get::<_,u64>(2)?,r.get::<_,u64>(3)?,r.get::<_,u64>(4)?,r.get::<_,u64>(5)?,r.get::<_,u64>(6)?,r.get::<_,u64>(7)?,r.get::<_,u64>(8)?)))?;
    let mut answer = counts(total, white, draws, black);
    let moves = {
        let mut stmt=tx.prepare(&format!("{MATCHED} SELECT m.uci,m.san,{COUNTS} FROM matched i JOIN explorer_positions m ON m.game_id=i.game_id AND m.position=:key AND m.uci<>'' GROUP BY m.uci,m.san ORDER BY count(*) DESC,m.san"))?;
        stmt.query_map(values, |r| {
            let mut v = counts(r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?);
            v["uci"] = json!(r.get::<_, String>(0)?);
            v["san"] = json!(r.get::<_, String>(1)?);
            Ok(v)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };
    let games = {
        let mut stmt=tx.prepare(&format!("{MATCHED} SELECT game_id,white,black,result,played_date,white_elo,black_elo,revision FROM matched WHERE game_id>:after AND (:uci='' OR EXISTS(SELECT 1 FROM explorer_positions m WHERE m.game_id=matched.game_id AND m.position=:key AND m.uci=:uci)) ORDER BY game_id LIMIT :limit"))?;
        let mut games=stmt.query_map(named_params!{":key":key,":collection":q.collection,":player":player,":color":q.color,":from":from,":to":to,":wmin":q.white_elo_min,":wmax":q.white_elo_max,":bmin":q.black_elo_min,":bmax":q.black_elo_max,":after":q.game_after,":uci":q.uci,":limit":limit+1},|r|{let result:String=r.get(3)?;Ok(json!({"id":r.get::<_,String>(0)?,"white":{"name":r.get::<_,Option<String>>(1)?.unwrap_or_else(||"Unknown".into())},"black":{"name":r.get::<_,Option<String>>(2)?.unwrap_or_else(||"Unknown".into())},"result":result,"winner":match result.as_str(){"1-0"=>Some("white"),"0-1"=>Some("black"),_=>None},"month":r.get::<_,Option<String>>(4)?.unwrap_or_default(),"white_elo":r.get::<_,Option<u32>>(5)?,"black_elo":r.get::<_,Option<u32>>(6)?,"revision":r.get::<_,u64>(7)?}))})?.collect::<std::result::Result<Vec<_>,_>>()?;
        let next = if games.len() > limit as usize {
            games.pop();
            games.last().map(|g| g["id"].clone())
        } else {
            None
        };
        answer["next"] = json!(next);
        games
    };
    answer["moves"] = json!(moves);
    answer["recentGames"] = json!(games);
    answer["generation"] = progress["generation"].clone();
    answer["indexing"] = json!(progress["pending"].as_u64().unwrap_or(0) > 0);
    answer["partial"] = json!(progress["complete"] != true);
    answer["index"] = progress;
    answer["position_key"] = json!(key);
    answer["missing_metadata"] = json!({"white_player":missing_white,"black_player":missing_black,"date":missing_date,"white_elo":missing_white_elo,"black_elo":missing_black_elo});
    tx.commit()?;
    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::*;
    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    fn database() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE games(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,document TEXT NOT NULL,fingerprint TEXT UNIQUE);CREATE TABLE collections(id TEXT PRIMARY KEY,name TEXT NOT NULL,revision INTEGER NOT NULL);CREATE TABLE memberships(collection_id TEXT REFERENCES collections(id) ON DELETE CASCADE,game_id TEXT REFERENCES games(id) ON DELETE CASCADE,PRIMARY KEY(collection_id,game_id));").unwrap();
        initialize(&db).unwrap();
        crate::analysis_runs::initialize(&db).unwrap();
        db
    }
    fn add(db: &mut Connection, pgn: &str) -> String {
        let docs = crate::archive_pgn::documents(pgn).unwrap();
        archive::import(db, docs, None).unwrap()["ids"][0]
            .as_str()
            .unwrap()
            .into()
    }
    fn drain(db: &mut Connection) {
        while let Some(Job {
            id,
            revision,
            fingerprint,
            document,
        }) = next(db).unwrap()
        {
            apply(db, &id, revision, &fingerprint, index(&document)).unwrap();
        }
    }
    fn ask(db: &mut Connection, fen: &str) -> Value {
        query(
            db,
            Query {
                fen: fen.into(),
                ..Query::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn canonical_keys_include_rights_and_legal_ep_but_ignore_counters() {
        let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        assert_ne!(
            position_key(&position(fen).unwrap()),
            position_key(&position(&fen.replace("KQkq", "-")).unwrap())
        );
        let ep = "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2";
        assert_ne!(
            position_key(&position(ep).unwrap()),
            position_key(&position(&ep.replace("d6", "-")).unwrap())
        );
        let pinned = "k3r3/8/8/3pP3/8/8/8/4K3 w - d6 0 2";
        assert_eq!(
            position_key(&position(pinned).unwrap()),
            position_key(&position(&pinned.replace("d6", "-")).unwrap())
        );
        assert_eq!(
            position_key(&position(START).unwrap()),
            position_key(&position(&START.replace("0 1", "83 91")).unwrap())
        );
    }
    #[test]
    fn transpositions_repetitions_unknown_results_and_duplicates_have_independent_counts() {
        let mut db = database();
        let one = "[White \"Ada\"]\n[Black \"Bob\"]\n[WhiteElo \"1800\"]\n[BlackElo \"1700\"]\n[Date \"2026.10.17\"]\n\n1. Nf3 Nf6 2. g3 g6 3. Bg2 1-0";
        let first = add(&mut db, one);
        add(
            &mut db,
            "[White \"Grace\"]\n\n1. g3 g6 2. Nf3 Nf6 3. Bg2 0-1",
        );
        add(&mut db, "1. Nf3 Nf6 2. Ng1 Ng8 3. Nf3 Nf6 4. g3 g6 *");
        add(&mut db, "1. Nf3 d5 1/2-1/2");
        add(&mut db, one);
        assert_eq!(status(&db).unwrap()["pending"], 4);
        assert_eq!(ask(&mut db, START)["partial"], true);
        drain(&mut db);
        let root = ask(&mut db, START);
        assert_eq!(root["total"], 4);
        assert_eq!(root["unknown"], 1);
        assert_eq!(root["completed"], 3);
        assert!((root["percentages"]["white"].as_f64().unwrap() - 100.0 / 3.0).abs() < 1e-9);
        let nf3 = root["moves"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["uci"] == "g1f3")
            .unwrap();
        assert_eq!(nf3["total"], 3);
        assert_eq!(nf3["unknown"], 1);
        assert_eq!(nf3["percentages"]["white"], 50.0);
        assert_eq!(nf3["percentages"]["draws"], 50.0);
        let mut pos = Chess::default();
        for uci in ["g1f3", "g8f6", "g2g3", "g7g6"] {
            let m = uci.parse::<UciMove>().unwrap().to_move(&pos).unwrap();
            pos.play_unchecked(m);
        }
        let trans = ask(
            &mut db,
            &Fen::from_position(&pos, EnPassantMode::Legal).to_string(),
        );
        assert_eq!(trans["total"], 3);
        assert_eq!(trans["moves"][0]["uci"], "f1g2");
        assert_eq!(trans["moves"][0]["total"], 2);
        assert_eq!(root["missing_metadata"]["date"], 3);
        assert_eq!(root["missing_metadata"]["white_elo"], 3);
        let filtered = query(
            &mut db,
            Query {
                fen: START.into(),
                player: "ada".into(),
                color: "white".into(),
                white_elo_min: Some(1750),
                date_from: "2026-10-01".into(),
                ..Query::default()
            },
        )
        .unwrap();
        assert_eq!(filtered["total"], 1);
        assert_eq!(filtered["recentGames"][0]["id"], first);
        let before = root["generation"].as_u64().unwrap();
        let mut document = archive::get(&db, &first).unwrap().document;
        document.metadata.insert("Result".into(), "*".into());
        archive::update(&mut db, &first, 1, document).unwrap();
        let pending = ask(&mut db, START);
        assert_eq!(pending["total"], 3);
        assert_eq!(pending["partial"], true);
        drain(&mut db);
        let changed = ask(&mut db, START);
        assert_eq!(changed["unknown"], 2);
        assert!(changed["generation"].as_u64().unwrap() > before);
        rebuild(&mut db).unwrap();
        assert_eq!(ask(&mut db, START)["total"], 0);
        assert_eq!(status(&db).unwrap()["failed"], 0);
        drain(&mut db);
        let rebuilt = ask(&mut db, START);
        assert_eq!(rebuilt["moves"], changed["moves"]);
        assert_eq!(rebuilt["total"], changed["total"]);
        archive::delete(&mut db, &first, 2).unwrap();
        assert_eq!(ask(&mut db, START)["total"], 3);
    }
    #[test]
    fn setup_variants_matching_pages_and_collection_invalidation() {
        let mut db = database();
        let id = add(&mut db, "[Variant \"Chess960\"]\n\n1. e4 *");
        add(
            &mut db,
            "[SetUp \"1\"]\n[FEN \"8/8/8/8/8/8/4K3/7k w - - 47 63\"]\n\n63. Kf3 *",
        );
        let normal = add(&mut db, "1. e4 *");
        let another = add(&mut db, "1. d4 *");
        drain(&mut db);
        assert_eq!(status(&db).unwrap()["unsupported_variants"], 1);
        assert_eq!(ask(&mut db, START)["total"], 2);
        let unknown = ask(&mut db, "8/8/8/8/8/8/4K3/7k w - - 0 1");
        assert_eq!(unknown["total"], 1);
        assert_eq!(unknown["percentages"], Value::Null);
        let group = archive::collection_edit(&mut db, None, 0, "create", "subset", &[]).unwrap();
        let c = group["id"].as_str().unwrap();
        archive::collection_edit(
            &mut db,
            Some(c),
            1,
            "add",
            "",
            std::slice::from_ref(&normal),
        )
        .unwrap();
        let subset = query(
            &mut db,
            Query {
                fen: START.into(),
                collection: c.into(),
                ..Query::default()
            },
        )
        .unwrap();
        assert_eq!(subset["total"], 1);
        let first = query(
            &mut db,
            Query {
                fen: START.into(),
                game_limit: 1,
                ..Query::default()
            },
        )
        .unwrap();
        let second = query(
            &mut db,
            Query {
                fen: START.into(),
                game_limit: 1,
                game_after: first["next"].as_str().unwrap().into(),
                ..Query::default()
            },
        )
        .unwrap();
        assert_ne!(
            first["recentGames"][0]["id"],
            second["recentGames"][0]["id"]
        );
        assert_eq!(second["next"], Value::Null);
        assert_eq!(
            query(
                &mut db,
                Query {
                    fen: START.into(),
                    uci: "e2e4".into(),
                    ..Query::default()
                }
            )
            .unwrap()["recentGames"][0]["id"],
            normal
        );
        let generation = subset["generation"].as_u64().unwrap();
        archive::collection_edit(
            &mut db,
            Some(c),
            2,
            "remove",
            "",
            std::slice::from_ref(&normal),
        )
        .unwrap();
        let empty = query(
            &mut db,
            Query {
                fen: START.into(),
                collection: c.into(),
                ..Query::default()
            },
        )
        .unwrap();
        assert_eq!(empty["total"], 0);
        assert!(empty["generation"].as_u64().unwrap() > generation);
        assert!(
            query(
                &mut db,
                Query {
                    fen: START.into(),
                    date_from: "2026-02-30".into(),
                    ..Query::default()
                }
            )
            .is_err()
        );
        assert!(
            query(
                &mut db,
                Query {
                    fen: START.into(),
                    variant: "chess960".into(),
                    ..Query::default()
                }
            )
            .is_err()
        );
        // A job captured before an edit/delete may not publish stale entries.
        let captured = archive::get(&db, &another).unwrap();
        archive::delete(&mut db, &another, 1).unwrap();
        apply(
            &mut db,
            &another,
            1,
            "deleted-fingerprint",
            index(&captured.document),
        )
        .unwrap();
        assert_eq!(ask(&mut db, START)["total"], 1);
        assert!(archive::get(&db, &id).is_ok());
    }
    #[test]
    fn restored_id_and_revision_cannot_accept_an_old_jobs_content() {
        let mut db = database();
        let id = add(&mut db, "1. e4 *");
        let job = next(&db).unwrap().unwrap();
        let mut backup = archive::backup(&mut db).unwrap();
        archive::delete(&mut db, &id, 1).unwrap();
        backup.games[0].document = crate::archive_pgn::documents("1. d4 *").unwrap().remove(0);
        archive::restore(&mut db, backup).unwrap();
        apply(
            &mut db,
            &id,
            job.revision,
            &job.fingerprint,
            index(&job.document),
        )
        .unwrap();
        assert_eq!(status(&db).unwrap()["pending"], 1);
        assert_eq!(ask(&mut db, START)["total"], 0);
        drain(&mut db);
        assert_eq!(ask(&mut db, START)["moves"][0]["uci"], "d2d4");
    }
}

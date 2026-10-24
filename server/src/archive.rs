//! Transactional, owner-local archive. SQLite work runs outside Tokio's workers.
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

pub const SCHEMA: u32 = 3;
pub const MAX_BODY: usize = 16 * 1024 * 1024;
pub const MAX_BATCH: usize = 1000;
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub struct Error(pub &'static str, pub &'static str);
impl From<rusqlite::Error> for Error {
    fn from(_: rusqlite::Error) -> Self {
        Self(
            "archive_failed",
            "Archive operation failed; no partial transaction was committed",
        )
    }
}
impl From<serde_json::Error> for Error {
    fn from(_: serde_json::Error) -> Self {
        invalid("Invalid archive JSON")
    }
}
pub fn invalid(message: &'static str) -> Error {
    Error("invalid_request", message)
}
fn conflict() -> Error {
    Error("conflict", "Revision changed; refresh before editing")
}
fn missing() -> Error {
    Error("not_found", "Archive resource not found")
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
fn random_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| Error("random_unavailable", "OS randomness unavailable"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[derive(Clone)]
pub struct Archive {
    connection: Arc<Mutex<Connection>>,
    slots: Arc<tokio::sync::Semaphore>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub format: String,
    pub version: u32,
    /// The complete original IndexedDB record, including the unchanged JSON string.
    pub record: Option<Value>,
    pub pgn: String,
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    pub analysis_runs: Vec<crate::analysis_runs::Run>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    pub id: String,
    pub revision: u64,
    pub created_at: i64,
    pub updated_at: i64,
    pub document: Document,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub revision: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    pub format: String,
    pub schema_version: u32,
    pub games: Vec<Game>,
    pub collections: Vec<Collection>,
    pub memberships: Vec<(String, String)>,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Filter {
    pub after: String,
    pub limit: u32,
    pub white: String,
    pub black: String,
    pub date_from: String,
    pub date_to: String,
    pub result: String,
    pub collection: String,
    pub opening: String,
    pub profile: String,
}

impl Document {
    pub fn validate(&self) -> Result<()> { self.validated().map(|_| ()) }
    fn validate_fields(&self) -> Result<()> {
        if self.format != "ironwood-companion-game" || self.version != 1 {
            return Err(Error(
                "unsupported_version",
                "Unsupported game transfer version",
            ));
        }
        if self.pgn.len() > 2 * 1024 * 1024
            || self.pgn.trim().is_empty()
            || self.metadata.len() > 64
            || self
                .metadata
                .iter()
                .any(|(k, v)| k.len() > 128 || v.len() > 4096)
        {
            return Err(invalid("Game metadata or PGN exceeds supported limits"));
        }
        if let Some(record) = &self.record {
            if !record.is_object() || !record["id"].is_string() {
                return Err(invalid("Invalid browser record"));
            }
            let raw = record["json"]
                .as_str()
                .ok_or(invalid("Missing browser game JSON"))?;
            if raw.len() > 4 * 1024 * 1024 {
                return Err(invalid("Game record exceeds 4 MiB"));
            }
            let game: Value = serde_json::from_str(raw)?;
            if !game["board"].is_string() || !game["history"].is_array() {
                return Err(invalid("Unsupported browser game record"));
            }
            if game
                .get("schema_version")
                .is_some_and(|v| v.as_u64() != Some(1))
            {
                return Err(Error(
                    "unsupported_version",
                    "Unsupported browser game schema",
                ));
            }
        }
        Ok(())
    }
    fn validated(&self) -> Result<crate::archive_pgn::Parsed> {
        self.validate_fields()?;
        let mut games=crate::archive_pgn::parse(&self.pgn)?;
        if games.len()!=1 {return Err(invalid("A document must contain exactly one PGN game"));}
        Ok(games.remove(0))
    }
    pub(crate) fn fingerprint(&self) -> Result<String> {
        let mut value = serde_json::to_value(self)?;
        value.as_object_mut().unwrap().remove("analysis_runs");
        // Transport-local identifiers/timestamps do not define an annotated version.
        if let Some(record) = value["record"].as_object_mut() {
            record.remove("id");
            record.remove("updatedAt");
        }
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&value)?)))
    }
}

impl Archive {
    pub fn open(path: &Path) -> std::result::Result<Self, Box<dyn std::error::Error>> {
        if path.is_symlink() {
            return Err("Archive database cannot be a symlink".into());
        }
        let mut connection = Connection::open(path)?;
        let version: u32 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > SCHEMA {
            return Err(
                "Archive schema is newer than this Companion; database was not changed".into(),
            );
        }
        if version == 0 {
            let tables:i64=connection.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",[],|r|r.get(0))?;
            if tables != 0 {
                return Err("Unrecognized archive database; no changes made".into());
            }
        }
        if version > 0 && version < SCHEMA {
            let destination = path.with_extension(format!(
                "pre-v{version}-{}.sqlite3",
                random_id().map_err(|_| "Randomness unavailable")?
            ));
            connection.backup("main", &destination, None)?;
        }
        let tx = connection.transaction()?;
        if version == 0 {
            tx.execute_batch("CREATE TABLE games(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,document TEXT NOT NULL,fingerprint TEXT NOT NULL UNIQUE); CREATE TABLE collections(id TEXT PRIMARY KEY,name TEXT NOT NULL,revision INTEGER NOT NULL);")?;
        }
        if version < 2 {
            tx.execute_batch("CREATE TABLE memberships(collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,PRIMARY KEY(collection_id,game_id)); CREATE INDEX membership_games ON memberships(game_id); PRAGMA user_version=2;")?;
        }
        tx.prepare(
            "SELECT id,revision,created_at,updated_at,document,fingerprint FROM games LIMIT 0",
        )?;
        tx.prepare("SELECT id,name,revision FROM collections LIMIT 0")?;
        tx.prepare("SELECT collection_id,game_id FROM memberships LIMIT 0")?;
        if version<3 {crate::analysis_runs::initialize(&tx).map_err(|e|e.1)?;tx.pragma_update(None,"user_version",3)?;}
        tx.prepare("SELECT game_id,id,revision,created_at,metadata,payload FROM analysis_runs LIMIT 0")?;
        tx.commit()?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=2000;")?;
        initialize_statistics(&connection)?;
        crate::explorer::initialize(&connection)?;
        crate::archive_library::initialize(&connection).map_err(|e| e.1)?;
        crate::archive_library::synchronize(&mut connection).map_err(|e| e.1)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            slots: Arc::new(tokio::sync::Semaphore::new(4)),
        })
    }
    /// CPU validation shares admission control, but never acquires the database mutex.
    pub(crate) fn idle(&self) -> bool { self.slots.available_permits()==4 && self.connection.try_lock().is_ok() }
    pub async fn prepare<T: Send + 'static>(&self, operation: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
        let permit=self.slots.clone().try_acquire_owned().map_err(|_|Error("busy","Archive is busy; retry shortly"))?;
        tokio::task::spawn_blocking(move || {let _permit=permit;operation()}).await
            .map_err(|_|Error("archive_failed","Archive preparation failed"))?
    }
    pub async fn run<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error("busy", "Archive is busy; retry shortly"))?;
        let connection = self.connection.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let mut db = connection
                .lock()
                .map_err(|_| Error("archive_failed", "Archive lock unavailable"))?;
            operation(&mut db)
        })
        .await
        .map_err(|_| Error("archive_failed", "Archive worker failed"))?
    }
    /// A single bounded background queue waits for admission instead of failing a file under foreground load.
    pub(crate) async fn run_background<T:Send+'static>(&self,operation:impl FnOnce(&mut Connection)->Result<T>+Send+'static)->Result<T>{
        let permit=self.slots.clone().acquire_owned().await.map_err(|_|Error("busy","Archive is stopping"))?;
        let connection=self.connection.clone();
        tokio::task::spawn_blocking(move||{let _permit=permit;let mut db=connection.lock().map_err(|_|Error("archive_failed","Archive lock unavailable"))?;operation(&mut db)}).await.map_err(|_|Error("archive_failed","Archive worker failed"))?
    }
}

fn read_game(row: &rusqlite::Row<'_>) -> rusqlite::Result<Game> {
    let text: String = row.get(4)?;
    let document = serde_json::from_str(&text).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(Game {
        id: row.get(0)?,
        revision: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
        document,
    })
}
pub fn get(db: &Connection, id: &str) -> Result<Game> {
    get_optional(db, id)?.ok_or_else(missing)
}
pub fn get_optional(db: &Connection, id: &str) -> Result<Option<Game>> {
    Ok(db
        .query_row(
            "SELECT id,revision,created_at,updated_at,document FROM games WHERE id=?1",
            [id],
            read_game,
        )
        .optional()?)
}
const STATS_CATEGORY: &str = "CASE WHEN json_extract(document,'$.record.category')='puzzles' OR json_extract(document,'$.metadata.Event')='Lichess puzzle' THEN 'puzzles' WHEN json_extract(document,'$.metadata.TrainingProfileId') IS NOT NULL OR json_extract(document,'$.record.category')='training' THEN 'training' WHEN json_extract(document,'$.record.category') IN ('mine','observed','imported') THEN json_extract(document,'$.record.category') ELSE 'imported' END";
fn initialize_statistics(db: &Connection) -> rusqlite::Result<()> {
    db.execute_batch(&format!("CREATE INDEX IF NOT EXISTS archive_stats_category ON games(({STATS_CATEGORY}));
        CREATE INDEX IF NOT EXISTS archive_stats_favorites ON games(id) WHERE json_extract(document,'$.record.favorite')=1;"))
}

/// Archive-local usage; this does not represent free space on the host volume.
pub fn stats(db: &Connection) -> Result<Value> {
    let games: u64 = db.query_row("SELECT count(*) FROM games", [], |r| r.get(0))?;
    let collections: u64 = db.query_row("SELECT count(*) FROM collections", [], |r| r.get(0))?;
    let memberships: u64 = db.query_row("SELECT count(*) FROM memberships", [], |r| r.get(0))?;
    let favorites: u64 = db.query_row(
        "SELECT count(*) FROM games WHERE json_extract(document,'$.record.favorite')=1",
        [],
        |r| r.get(0),
    )?;
    let mut categories = serde_json::Map::new();
    // These expression/partial indexes preserve legacy category precedence without
    // fetching or parsing every complete document while holding the archive lock.
    let mut statement = db.prepare(&format!("SELECT {STATS_CATEGORY} AS category,count(*) FROM games GROUP BY category"))?;
    for row in statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u64>(1)?)))? {
        let (category, count) = row?;
        categories.insert(category, json!(count));
    }
    let page_size: u64 = db.pragma_query_value(None, "page_size", |r| r.get(0))?;
    let page_count: u64 = db.pragma_query_value(None, "page_count", |r| r.get(0))?;
    let free_pages: u64 = db.pragma_query_value(None, "freelist_count", |r| r.get(0))?;
    let size = |suffix: &str| -> Option<u64> {
        let mut path = std::path::PathBuf::from(db.path()?).into_os_string();
        path.push(suffix);
        match std::fs::symlink_metadata(std::path::PathBuf::from(path)) {
            Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => Some(meta.len()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(0),
            _ => None,
        }
    };
    let database_bytes = size("");
    let wal_bytes = size("-wal");
    let shared_memory_bytes = size("-shm");
    let total_bytes = database_bytes
        .zip(wal_bytes)
        .zip(shared_memory_bytes)
        .map(|((db, wal), shm)| db + wal + shm);
    Ok(
        json!({"schema_version":SCHEMA,"games":games,"categories":categories,"favorites":favorites,"collections":collections,"memberships":memberships,"database_bytes":database_bytes,"wal_bytes":wal_bytes,"shared_memory_bytes":shared_memory_bytes,"total_bytes":total_bytes,"allocated_bytes":page_size*page_count,"reusable_bytes":page_size*free_pages}),
    )
}
pub fn list(db: &Connection, filter: Filter) -> Result<Value> {
    if filter.limit > 100
        || [
            &filter.white,
            &filter.black,
            &filter.opening,
            &filter.profile,
        ]
        .iter()
        .any(|v| v.len() > 256)
    {
        return Err(invalid("Invalid search filter or page limit"));
    }
    let limit = if filter.limit == 0 { 50 } else { filter.limit };
    let mut stmt=db.prepare("SELECT g.id,g.revision,g.created_at,g.updated_at,g.document FROM games g WHERE g.id>?1 AND (?2='' OR instr(lower(json_extract(document,'$.metadata.White')),lower(?2))>0) AND (?3='' OR instr(lower(json_extract(document,'$.metadata.Black')),lower(?3))>0) AND (?4='' OR json_extract(document,'$.metadata.Date')>=?4) AND (?5='' OR json_extract(document,'$.metadata.Date')<=?5) AND (?6='' OR json_extract(document,'$.metadata.Result')=?6) AND (?7='' OR EXISTS(SELECT 1 FROM memberships m WHERE m.game_id=g.id AND m.collection_id=?7)) AND (?8='' OR instr(lower(coalesce(json_extract(document,'$.metadata.Opening'),'')||' '||coalesce(json_extract(document,'$.metadata.ECO'),'')),lower(?8))>0) AND (?9='' OR json_extract(document,'$.metadata.TrainingProfileId')=?9) ORDER BY g.id LIMIT ?10")?;
    let rows = stmt.query_map(
        params![
            filter.after,
            filter.white,
            filter.black,
            filter.date_from,
            filter.date_to,
            filter.result,
            filter.collection,
            filter.opening,
            filter.profile,
            limit + 1
        ],
        read_game,
    )?;
    let mut games = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    let next = if games.len() > limit as usize {
        games.pop();
        games.last().map(|g| g.id.clone())
    } else {
        None
    };
    // Lists omit large complete records; detail fetches are explicit.
    let summaries:Vec<_>=games.into_iter().map(|g|json!({"id":g.id,"revision":g.revision,"created_at":g.created_at,"updated_at":g.updated_at,"metadata":g.document.metadata,"has_record":g.document.record.is_some()})).collect();
    Ok(json!({"games":summaries,"next":next}))
}
pub struct PreparedImport { games: Vec<PreparedGame>, discarded: usize }
struct PreparedGame { id:String,stamp:i64,document:String,fingerprint:String,runs:Vec<crate::analysis_runs::PreparedRun>,card:Value }
pub(crate) fn prepare_parsed(games:Vec<(Document,crate::archive_pgn::Parsed)>) -> Result<PreparedImport> {
    let mut prepared=Vec::with_capacity(games.len());
    for (mut doc,parsed) in games {
        doc.validate_fields()?;
        let runs=crate::analysis_runs::prepare_merge(&std::mem::take(&mut doc.analysis_runs))?;
        let fingerprint=doc.fingerprint()?;
        let id=random_id()?;let stamp=now();
        let card=crate::archive_library::summary_parsed(&id,1,stamp,&doc,&parsed)?;
        prepared.push(PreparedGame{id,stamp,card,runs,fingerprint,document:serde_json::to_string(&doc)?});
    }
    Ok(PreparedImport{games:prepared,discarded:0})
}
pub fn prepare_import(documents:Vec<Document>) -> Result<PreparedImport> {
    if documents.is_empty()||documents.len()>MAX_BATCH{return Err(invalid("Import requires 1 to 1000 games"));}
    let mut parsed=Vec::with_capacity(documents.len());
    for document in documents {let game=document.validated()?;parsed.push((document,game));}
    prepare_parsed(parsed)
}
pub(crate) fn prepare_pgn_parsed(games: Vec<(Document,crate::archive_pgn::Parsed)>) -> Result<PreparedImport> {
    let count = games.len();
    let games: Vec<_> = games.into_iter().filter(|(_,game)|game.moves >= 4).collect();
    let discarded = count - games.len();
    let mut prepared = prepare_parsed(games)?;
    prepared.discarded = discarded;
    Ok(prepared)
}
pub fn prepare_pgn_import(pgn:&str) -> Result<PreparedImport> {
    let games=crate::archive_pgn::parse(pgn)?.into_iter().map(|game|{
        let document=Document{format:"ironwood-companion-game".into(),version:1,record:None,analysis_runs:Vec::new(),metadata:game.tags.clone(),pgn:game.pgn.clone()};
        (document,game)
    }).collect();
    prepare_pgn_parsed(games)
}
pub fn import(db:&mut Connection,documents:Vec<Document>,collection:Option<String>) -> Result<Value> {
    commit_import(db,prepare_import(documents)?,collection)
}
pub fn commit_import(db:&mut Connection,prepared:PreparedImport,collection:Option<String>) -> Result<Value> {
    let tx = db.transaction()?;
    let result=commit_import_rows(&tx,prepared,collection)?;
    tx.commit()?;
    Ok(result)
}
pub(crate) fn commit_import_rows(tx:&Connection,prepared:PreparedImport,collection:Option<String>) -> Result<Value> {
    if let Some(c) = &collection
        && !tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM collections WHERE id=?1)",
            [c],
            |r| r.get::<_, bool>(0),
        )?
    {
        return Err(missing());
    }
    let mut imported = 0;
    let mut duplicates = 0;
    let mut ids = Vec::new();
    for game in prepared.games {
        let fingerprint=&game.fingerprint;
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM games WHERE fingerprint=?1",
                [&fingerprint],
                |r| r.get(0),
            )
            .optional()?;
        let game_id = if let Some(id) = existing {
            duplicates += 1;
            id
        } else {
            tx.execute("INSERT INTO games VALUES(?1,1,?2,?2,?3,?4)",params![game.id,game.stamp,game.document,fingerprint])?;
            crate::archive_library::store_card(&tx,&game.id,1,game.stamp,&game.card)?;
            imported += 1;
            game.id
        };
        crate::analysis_runs::merge_prepared(tx,&game_id,&game.runs)?;
        if let Some(c) = &collection {
            let changed = tx.execute(
                "INSERT OR IGNORE INTO memberships VALUES(?1,?2)",
                params![c, game_id],
            )?;
            if changed > 0 {
                tx.execute(
                    "UPDATE collections SET revision=revision+1 WHERE id=?1",
                    [c],
                )?;
            }
        }
        ids.push(game_id);
    }
    Ok(json!({"imported":imported,"duplicates":duplicates,"discarded":prepared.discarded,"ids":ids}))
}
pub fn update(db: &mut Connection, id: &str, revision: u64, mut doc: Document) -> Result<Game> {
    doc.validate()?;
    let tx = db.transaction()?;
    let current = get_optional(&tx, id)?.ok_or_else(missing)?;
    if current.revision != revision {
        return Err(conflict());
    }
    let runs=std::mem::take(&mut doc.analysis_runs);
    crate::analysis_runs::merge(&tx,id,&runs)?;
    let fingerprint = doc.fingerprint()?;
    if tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM games WHERE fingerprint=?1 AND id<>?2)",
        params![fingerprint, id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(Error(
            "conflict",
            "An identical annotated version already exists",
        ));
    }
    tx.execute(
        "UPDATE games SET document=?1,fingerprint=?2,revision=revision+1,updated_at=?3 WHERE id=?4",
        params![serde_json::to_string(&doc)?, fingerprint, now(), id],
    )?;
    let updated = get(&tx, id)?;
    tx.commit()?;
    Ok(updated)
}
pub fn delete(db: &mut Connection, id: &str, revision: u64) -> Result<Value> {
    delete_with_runs(db,id,revision,None)
}
pub fn delete_with_runs(db:&mut Connection,id:&str,revision:u64,expected_runs:Option<Vec<(String,u64)>>)->Result<Value>{
    let tx = db.transaction()?;
    if let Some(mut expected)=expected_runs {
        expected.sort();
        let actual=tx.prepare("SELECT id,revision FROM analysis_runs WHERE game_id=?1 ORDER BY id")?.query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,u64>(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
        if expected!=actual {return Err(Error("conflict","Analysis runs changed during transfer; source game was kept"));}
    }
    let current = get_optional(&tx, id)?.ok_or_else(missing)?;
    if current.revision != revision {
        return Err(conflict());
    }
    tx.execute("UPDATE collections SET revision=revision+1 WHERE id IN(SELECT collection_id FROM memberships WHERE game_id=?1)",[id])?;
    tx.execute("DELETE FROM games WHERE id=?1", [id])?;
    tx.commit()?;
    Ok(json!({"deleted":id}))
}
pub fn collections(db: &Connection) -> Result<Vec<Collection>> {
    Ok(db
        .prepare("SELECT id,name,revision FROM collections ORDER BY name,id")?
        .query_map([], |r| {
            Ok(Collection {
                id: r.get(0)?,
                name: r.get(1)?,
                revision: r.get(2)?,
            })
        })?
        .collect::<std::result::Result<_, _>>()?)
}
pub fn collection_edit(
    db: &mut Connection,
    id: Option<&str>,
    revision: u64,
    action: &str,
    name: &str,
    games: &[String],
) -> Result<Value> {
    if name.len() > 200 || games.len() > MAX_BATCH {
        return Err(invalid("Collection limits exceeded"));
    }
    let tx = db.transaction()?;
    let collection_id = if let Some(id) = id {
        let actual: Option<u64> = tx
            .query_row("SELECT revision FROM collections WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        if actual.ok_or_else(missing)? != revision {
            return Err(conflict());
        }
        id.to_owned()
    } else {
        if action != "create" || name.trim().is_empty() {
            return Err(invalid("Collection name required"));
        }
        let id = random_id()?;
        tx.execute("INSERT INTO collections VALUES(?1,?2,1)", params![id, name])?;
        id
    };
    match action {
        "create" => {}
        "rename" if !name.trim().is_empty() => {
            tx.execute(
                "UPDATE collections SET name=?1,revision=revision+1 WHERE id=?2",
                params![name, collection_id],
            )?;
        }
        "delete" => {
            tx.execute("DELETE FROM collections WHERE id=?1", [&collection_id])?;
        }
        "add" | "remove" => {
            for game in games {
                if get_optional(&tx, game)?.is_none() {
                    return Err(missing());
                }
                tx.execute(
                    if action == "add" {
                        "INSERT OR IGNORE INTO memberships VALUES(?1,?2)"
                    } else {
                        "DELETE FROM memberships WHERE collection_id=?1 AND game_id=?2"
                    },
                    params![collection_id, game],
                )?;
            }
            tx.execute(
                "UPDATE collections SET revision=revision+1 WHERE id=?1",
                [&collection_id],
            )?;
        }
        _ => return Err(invalid("Invalid collection action")),
    }
    tx.commit()?;
    Ok(json!({"id":collection_id,"collections":collections(db)?}))
}
pub fn backup(db: &mut Connection) -> Result<Backup> {
    let tx = db.transaction()?;
    let mut games = tx
        .prepare("SELECT id,revision,created_at,updated_at,document FROM games ORDER BY id")?
        .query_map([], read_game)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for game in &mut games {game.document.analysis_runs=crate::analysis_runs::all(&tx,&game.id)?;}
    let collections = collections(&tx)?;
    let memberships = tx
        .prepare("SELECT collection_id,game_id FROM memberships ORDER BY collection_id,game_id")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    tx.commit()?;
    Ok(Backup {
        format: "ironwood-companion-archive".into(),
        schema_version: SCHEMA,
        games,
        collections,
        memberships,
    })
}
pub fn restore(db: &mut Connection, backup: Backup) -> Result<Value> {
    if backup.format != "ironwood-companion-archive" || !matches!(backup.schema_version,2|3) {
        return Err(Error(
            "unsupported_version",
            "Unsupported archive backup schema",
        ));
    }
    if backup.games.len() > 10000
        || backup.collections.len() > 1000
        || backup.memberships.len() > 100000
    {
        return Err(invalid("Restore exceeds supported limits"));
    }
    for g in &backup.games {
        g.document.validate()?;
        if g.id.is_empty() || g.id.len() > 128 || g.revision == 0 || g.revision > i64::MAX as u64 {
            return Err(invalid("Invalid backup game identity"));
        }
    }
    let tx = db.transaction()?;
    let count: i64 = tx.query_row(
        "SELECT (SELECT count(*) FROM games)+(SELECT count(*) FROM collections)",
        [],
        |r| r.get(0),
    )?;
    if count != 0 {
        return Err(Error(
            "conflict",
            "Restore requires an empty archive; existing games were not changed",
        ));
    }
    for g in &backup.games {
        let mut document=g.document.clone();let _runs=std::mem::take(&mut document.analysis_runs);
        tx.execute(
            "INSERT INTO games VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                g.id,
                g.revision,
                g.created_at,
                g.updated_at,
                serde_json::to_string(&document)?,
                g.document.fingerprint()?
            ],
        )?;
    }
    for g in &backup.games {crate::analysis_runs::merge(&tx,&g.id,&g.document.analysis_runs)?;}
    for c in &backup.collections {
        if c.id.is_empty()
            || c.id.len() > 128
            || c.name.trim().is_empty()
            || c.name.len() > 200
            || c.revision == 0
            || c.revision > i64::MAX as u64
        {
            return Err(invalid("Invalid backup collection"));
        }
        tx.execute(
            "INSERT INTO collections VALUES(?1,?2,?3)",
            params![c.id, c.name, c.revision],
        )?;
    }
    for (c, g) in &backup.memberships {
        tx.execute("INSERT INTO memberships VALUES(?1,?2)", params![c, g])?;
    }
    tx.commit()?;
    Ok(json!({"restored":backup.games.len()}))
}

/// Consistent SQLite snapshot, also suitable for archives larger than HTTP limits.
/// The caller holds the data-directory lock; existing destinations are preserved.
pub fn snapshot(
    source: &Path,
    destination: &Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    if source.is_symlink() || destination.exists() || destination.is_symlink() {
        return Err("Snapshot requires a real source and a new destination file".into());
    }
    let db = Connection::open_with_flags(source, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let version: u32 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if !(1..=SCHEMA).contains(&version) {
        return Err("Unsupported archive snapshot schema; no changes made".into());
    }
    let check: String = db.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if check != "ok" {
        return Err("Archive snapshot failed integrity validation".into());
    }
    let foreign = db.prepare("PRAGMA foreign_key_check")?.exists([])?;
    if foreign {
        return Err("Archive snapshot has invalid collection references".into());
    }
    db.prepare("SELECT id,revision,created_at,updated_at,document,fingerprint FROM games LIMIT 0")?;
    db.prepare("SELECT id,name,revision FROM collections LIMIT 0")?;
    if version >= 2 {
        db.prepare("SELECT collection_id,game_id FROM memberships LIMIT 0")?;
    }
    if version>=3 {db.prepare("SELECT game_id,id,revision,created_at,metadata,payload FROM analysis_runs LIMIT 0")?;}
    // Reserve exclusively before the backup API opens its destination.
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    drop(options.open(destination)?);
    db.backup("main", destination, None)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn memory() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;CREATE TABLE games(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,created_at INTEGER NOT NULL,updated_at INTEGER NOT NULL,document TEXT NOT NULL,fingerprint TEXT UNIQUE);CREATE TABLE collections(id TEXT PRIMARY KEY,name TEXT NOT NULL,revision INTEGER NOT NULL);CREATE TABLE memberships(collection_id TEXT REFERENCES collections(id) ON DELETE CASCADE,game_id TEXT REFERENCES games(id) ON DELETE CASCADE,PRIMARY KEY(collection_id,game_id));").unwrap();
        crate::analysis_runs::initialize(&db).unwrap();
        crate::archive_library::initialize(&db).unwrap();
        initialize_statistics(&db).unwrap();
        db
    }
    #[test]
    fn storage_statistics_indexes_preserve_documents_and_track_category_changes() {
        let mut db=memory();
        let mut documents=crate::archive_pgn::documents("[White \"Notes\"]\n\n1. e4 {keep note} e5 *").unwrap();
        let doc=&mut documents[0];doc.record=Some(json!({"id":"stats-test","category":"mine","favorite":true,"json":json!({"board":"8/8/8/8/8/8/8/K6k w - - 0 1","history":[]}).to_string()}));
        let result=import(&mut db,documents,None).unwrap();let id=result["ids"][0].as_str().unwrap();
        let original=get(&db,id).unwrap();
        initialize_statistics(&db).unwrap();initialize_statistics(&db).unwrap();
        assert_eq!(serde_json::to_value(&original.document).unwrap(),serde_json::to_value(&get(&db,id).unwrap().document).unwrap());
        let first=stats(&db).unwrap();assert_eq!(first["games"],1);assert_eq!(first["favorites"],1);assert_eq!(first["categories"]["mine"],1);
        let mut document=original.document;document.metadata.insert("TrainingProfileId".into(),"metadata-only".into());
        document.record.as_mut().unwrap()["favorite"]=json!(false);
        let updated=update(&mut db,id,original.revision,document).unwrap();
        let second=stats(&db).unwrap();assert_eq!(second["favorites"],0);assert_eq!(second["categories"]["training"],1);
        let mut document=updated.document;document.metadata.insert("Event".into(),"Lichess puzzle".into());
        update(&mut db,id,updated.revision,document).unwrap();assert_eq!(stats(&db).unwrap()["categories"]["puzzles"],1);
        for (sql,index) in [(format!("SELECT {STATS_CATEGORY},count(*) FROM games GROUP BY 1"),"archive_stats_category"),("SELECT count(*) FROM games WHERE json_extract(document,'$.record.favorite')=1".into(),"archive_stats_favorites")] {
            let plan=db.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap().query_map([],|row|row.get::<_,String>(3)).unwrap().collect::<std::result::Result<Vec<_>,_>>().unwrap().join(" ");
            assert!(plan.contains(index),"{plan}");
        }
    }
    #[test]
    fn pgn_import_discards_short_mainlines_without_affecting_record_transfers() {
        let mut db=memory();
        for pgn in ["*", "1. e4 *", "1. e4 e5 *", "1. e4 e5 2. Nf3 *", "1. e4 (1. d4 d5 2. c4 e6) e5 *"] {
            let result=commit_import(&mut db,prepare_pgn_import(pgn).unwrap(),None).unwrap();
            assert_eq!(result["discarded"],1);assert_eq!(result["imported"],0);
        }
        let result=commit_import(&mut db,prepare_pgn_import("1. e4 e5 2. Nf3 Nc6 *").unwrap(),None).unwrap();
        assert_eq!(result["discarded"],0);assert_eq!(result["imported"],1);
        let documents=crate::archive_pgn::documents("1. d4 *").unwrap();
        assert_eq!(import(&mut db,documents,None).unwrap()["imported"],1,"Saved records still transfer intact");
    }
    #[test]
    fn prepared_pgn_import_reuses_validation_for_library_cards() {
        let mut db=memory();
        crate::archive_pgn::PARSE_COUNT.with(|count|count.set(0));
        let prepared=prepare_pgn_import("[White \"Cached\"]\n\n1. e4 {keep note} c5 2. Nf3 Nc6 *").unwrap();
        assert_eq!(crate::archive_pgn::PARSE_COUNT.with(|count|count.get()),1);
        let imported=commit_import(&mut db,prepared,None).unwrap();
        crate::archive_library::synchronize(&mut db).unwrap();
        assert_eq!(crate::archive_pgn::PARSE_COUNT.with(|count|count.get()),1);
        assert_eq!(db.query_row("SELECT count(*) FROM library_dirty",[],|row|row.get::<_,usize>(0)).unwrap(),0);
        let game=get(&db,imported["ids"][0].as_str().unwrap()).unwrap();assert!(game.document.pgn.contains("keep note"));
        assert_eq!(crate::archive_library::query(&mut db,Default::default()).unwrap()["games"][0]["eco"],"B30");
    }
    #[tokio::test]
    async fn preparation_does_not_hold_database_lock() {
        let archive=Archive{connection:Arc::new(Mutex::new(memory())),slots:Arc::new(tokio::sync::Semaphore::new(4))};
        let worker=archive.clone();
        let (started,wait_started)=tokio::sync::oneshot::channel();
        let (release,wait_release)=std::sync::mpsc::channel();
        let validation=tokio::spawn(async move{worker.prepare(move||{started.send(()).unwrap();wait_release.recv().unwrap();prepare_pgn_import("1. e4 e5 *")}).await});
        wait_started.await.unwrap();
        let read=tokio::time::timeout(std::time::Duration::from_secs(1),archive.run(|db|Ok(db.query_row("SELECT count(*) FROM games",[],|row|row.get::<_,usize>(0))?))).await;
        release.send(()).unwrap();
        assert_eq!(read.unwrap().unwrap(),0);
        let prepared=validation.await.unwrap().unwrap();archive.run(move|db|commit_import(db,prepared,None)).await.unwrap();
    }
    fn doc() -> Document {
        let mut d = crate::archive_pgn::documents(
            "[White \"Ada\"]\n[Black \"Grace\"]\n\n1. e4 {note [%cal Ge2e4]} (1. d4 d5) e5 $1 *",
        )
        .unwrap()
        .remove(0);
        d.record = Some(
            json!({"id":"browser-one","json":"{\"board\":\"starting FEN is retained by original browser record\",\"history\":[],\"move_notes\":[\"note\"],\"board_marks\":[[{\"color\":\"Gold\"}]],\"game_analysis\":[{\"score\":23}],\"engine_config\":{\"companion_name\":\"Owner engine\"}}","favorite":true,"updatedAt":123,"source":"custom metadata"}),
        );
        d
    }
    #[test]
    fn round_trip_versions_dedup_and_failed_import_are_atomic() {
        let mut db = memory();
        let d = doc();
        let result = import(&mut db, vec![d.clone()], None).unwrap();
        let id = result["ids"][0].as_str().unwrap();
        assert_eq!(get(&db, id).unwrap().document.record, d.record);
        let mut repeat = d.clone();
        repeat.record.as_mut().unwrap()["id"] = json!("other-browser");
        repeat.record.as_mut().unwrap()["updatedAt"] = json!(999);
        assert_eq!(
            import(&mut db, vec![repeat], None).unwrap()["duplicates"],
            1
        );
        let mut bad = d.clone();
        bad.version = 99;
        assert_eq!(
            import(&mut db, vec![d, bad], None).unwrap_err().0,
            "unsupported_version"
        );
        assert_eq!(backup(&mut db).unwrap().games.len(), 1);
        let mut changed = doc();
        changed.record.as_mut().unwrap()["source"] = json!("new annotation version");
        assert_eq!(import(&mut db, vec![changed], None).unwrap()["imported"], 1);
    }
    #[test]
    fn revisions_memberships_backup_and_restore_preserve_boundaries() {
        let mut db = memory();
        let imported = import(&mut db, vec![doc()], None).unwrap();
        let id = imported["ids"][0].as_str().unwrap();
        let group = collection_edit(&mut db, None, 0, "create", "Training archive", &[]).unwrap();
        let c = group["id"].as_str().unwrap();
        collection_edit(&mut db, Some(c), 1, "add", "", &[id.into()]).unwrap();
        assert_eq!(
            collection_edit(&mut db, Some(c), 1, "rename", "lost update", &[])
                .unwrap_err()
                .0,
            "conflict"
        );
        let mut edited = doc();
        edited.metadata.insert("Opening".into(), "King pawn".into());
        update(&mut db, id, 1, edited.clone()).unwrap();
        assert_eq!(update(&mut db, id, 1, edited).unwrap_err().0, "conflict");
        let original = serde_json::to_value(backup(&mut db).unwrap()).unwrap();
        let mut target = memory();
        let mut broken = original.clone();
        broken["memberships"]
            .as_array_mut()
            .unwrap()
            .push(json!([c, "missing-game"]));
        assert!(restore(&mut target, serde_json::from_value(broken).unwrap()).is_err());
        assert!(backup(&mut target).unwrap().games.is_empty());
        restore(
            &mut target,
            serde_json::from_value(original.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(backup(&mut target).unwrap()).unwrap(),
            original
        );
        assert_eq!(
            restore(&mut target, serde_json::from_value(original).unwrap())
                .unwrap_err()
                .0,
            "conflict"
        );
        collection_edit(&mut db, Some(c), 2, "delete", "", &[]).unwrap();
        assert!(get_optional(&db, id).unwrap().is_some());
        assert!(backup(&mut db).unwrap().memberships.is_empty());
        assert_eq!(delete(&mut db, id, 1).unwrap_err().0, "conflict");
        delete(&mut db, id, 2).unwrap();
        assert!(backup(&mut db).unwrap().games.is_empty());
    }
    #[test]
    fn malformed_pgn_variants_counters_and_variations() {
        for bad in [
            "1. e5 *",
            "1. e4",
            "[Variant \"Atomic\"]\n\n*",
            "[FEN \"8/8/8/8/8/8/8/8 w - -\"]\n\n*",
        ] {
            assert!(crate::archive_pgn::parse(bad).is_err(), "{bad}");
        }
        let pgn = "[SetUp \"1\"]\n[FEN \"8/8/8/8/8/8/4K3/7k w - - 47 63\"]\n\n63. Kf3 {preserve counters} *";
        let parsed = crate::archive_pgn::documents(pgn).unwrap();
        assert!(parsed[0].pgn.contains("47 63"));
        assert!(doc().pgn.contains("( 1. d4 1... d5 )"));
    }
    #[test]
    fn migrations_snapshot_and_newer_schema_rejection() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tmp");
        std::fs::create_dir_all(&root).unwrap();
        let dir = root.join(format!("companion-archive-{}", random_id().unwrap()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("archive.sqlite3");
        {
            let db = memory();
            db.backup("main", &path, None).unwrap();
            let disk = Connection::open(&path).unwrap();
            disk.execute_batch("DROP TABLE memberships;PRAGMA user_version=1;")
                .unwrap();
        }
        let store = Archive::open(&path).unwrap();
        drop(store);
        let backups = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("pre-v1-"))
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        let old = Connection::open(backups[0].path()).unwrap();
        assert_eq!(
            old.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                .unwrap(),
            1
        );
        drop(old);
        let copy = dir.join("snapshot.sqlite3");
        snapshot(&path, &copy).unwrap();
        assert!(snapshot(&path, &copy).is_err());
        {
            let db = Connection::open(&path).unwrap();
            db.pragma_update(None, "user_version", 99).unwrap();
        }
        let before = std::fs::read(&path).unwrap();
        assert!(Archive::open(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(snapshot(&path, &dir.join("future.sqlite3")).is_err());
        let broken = dir.join("broken-v1.sqlite3");
        {
            let db = Connection::open(&broken).unwrap();
            db.execute_batch("CREATE TABLE games(id TEXT);CREATE TABLE collections(id TEXT);PRAGMA user_version=1;").unwrap();
        }
        assert!(Archive::open(&broken).is_err());
        {
            let db = Connection::open(&broken).unwrap();
            assert_eq!(
                db.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
                    .unwrap(),
                1
            );
            assert_eq!(
                db.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE name='memberships'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
        }
        let resolved = std::fs::canonicalize(&dir).unwrap();
        assert!(resolved.starts_with(std::fs::canonicalize(root).unwrap()));
        std::fs::remove_dir_all(resolved).unwrap();
    }
}

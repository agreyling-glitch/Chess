//! Private, owner-local import queue. Files are claimed atomically and removed only after a durable report.
use crate::{
    Control,
    archive::{self, Archive, Document, PreparedImport},
    archive_pgn::ImportReader,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, String>;
const BATCH: usize = 128;
#[derive(Clone)]
pub struct Manager {
    archive: Arc<Mutex<Option<Archive>>>,
    status: Arc<Mutex<String>>,
    work: Arc<tokio::sync::Semaphore>,
    worker: Arc<tokio::sync::Semaphore>,
}
impl Default for Manager {
    fn default() -> Self {
        Self {
            archive: Arc::default(),
            status: Arc::new(Mutex::new("Not started".into())),
            work: Arc::new(tokio::sync::Semaphore::new(1)),
            worker: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }
}
impl Manager {
    pub(crate) fn configure(&self, archive: Option<Archive>) {
        *self.archive.lock().unwrap() = archive;
    }
    pub fn status(&self) -> String {
        self.status.lock().unwrap().clone()
    }
    fn message(&self, text: impl Into<String>) {
        *self.status.lock().unwrap() = text.into();
    }
    pub(crate) fn busy(&self) -> bool {
        self.work.available_permits() == 0
    }
    pub(crate) fn reserve(&self) -> Option<tokio::sync::OwnedSemaphorePermit> {
        self.work.clone().try_acquire_owned().ok()
    }
    pub(crate) async fn drained(&self) {
        let _permit = self.worker.clone().acquire_owned().await;
    }
    pub fn start(
        &self,
        control: Control,
        root: PathBuf,
        enabled: bool,
        startup_marker: Option<PathBuf>,
    ) -> Result<()> {
        if !enabled {
            self.message("Disabled in configuration");
            return Ok(());
        }
        let Some(archive) = self.archive.lock().unwrap().clone() else {
            self.message("Archive is disabled");
            return Ok(());
        };
        let permit = self
            .worker
            .clone()
            .try_acquire_owned()
            .map_err(|_| "Import queue is already running")?;
        let queue = Queue::new(root)?;
        let manager = self.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let mut queue = queue;
            loop {
                if control.stopping() {
                    break;
                }
                if startup_marker.as_ref().is_some_and(|path| path.exists()) {
                    manager.message("Waiting for update validation");
                } else if !idle(&control, &archive) {
                    manager.message("Waiting for idle Companion");
                } else {
                    match queue.next() {
                        Ok(Some(mut job)) => {
                            manager.message(format!("Importing {}", job.filename));
                            let result = execute(&manager, &archive, &control, &mut job).await;
                            if control.stopping() {
                                manager.message("Interrupted; source retained for restart");
                                break;
                            }
                            let stats = totals(&archive, job.id.clone()).await;
                            match stats.and_then(|stats| queue.finish(&job, &stats, result.err())) {
                                Ok(()) => manager
                                    .message(format!("Finished {}; report saved", job.filename)),
                                Err(error) => manager.message(format!("Source retained: {error}")),
                            }
                        }
                        Ok(None) => {
                            manager.message("Idle — waiting for PGN or ZIP files in Import")
                        }
                        Err(error) => manager.message(format!("Queue paused: {error}")),
                    }
                }
                tokio::select! {_=control.cancelled()=>break,_=tokio::time::sleep(Duration::from_secs(2))=>{}}
            }
        });
        Ok(())
    }
}
fn idle(control: &Control, archive: &Archive) -> bool {
    !control.stopping()
        && control.engine_idle()
        && control.registrations.available_permits() > 0
        && archive.idle()
}
async fn reserve(
    manager: &Manager,
    archive: &Archive,
    control: &Control,
) -> Result<tokio::sync::OwnedSemaphorePermit> {
    loop {
        if control.stopping() {
            return Err("Interrupted; source retained".into());
        }
        if idle(control, archive) {
            if let Some(permit) = manager.reserve() {
                return Ok(permit);
            }
        }
        manager.message("Paused — waiting for idle Companion");
        tokio::select! {_=control.cancelled()=>return Err("Interrupted; source retained".into()),_=tokio::time::sleep(Duration::from_millis(250))=>{}}
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Job {
    id: String,
    filename: String,
    kind: String,
    started: u64,
    hash: String,
    #[serde(skip)]
    directory: PathBuf,
}
impl Job {
    fn source(&self) -> PathBuf {
        self.directory.join(format!("source.{}", self.kind))
    }
    fn save(&self) -> Result<()> {
        atomic(
            &self.directory.join("job.json"),
            &serde_json::to_vec(self).map_err(|e| e.to_string())?,
        )
    }
}
#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq)]
struct Stats {
    imported: u64,
    duplicates: u64,
    failed: u64,
    #[serde(default)]
    discarded: u64,
}
impl Stats {
    fn count(&self) -> u64 {
        self.imported + self.duplicates + self.failed + self.discarded
    }
    fn add(&mut self, other: &Self) {
        self.imported += other.imported;
        self.duplicates += other.duplicates;
        self.failed += other.failed;
        self.discarded += other.discarded;
    }
}
struct Queue {
    root: PathBuf,
    imports: PathBuf,
    reports: PathBuf,
    processing: PathBuf,
    failed: PathBuf,
    seen: HashMap<PathBuf, (u64, SystemTime)>,
}
impl Queue {
    fn new(root: PathBuf) -> Result<Self> {
        let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
        let imports = folder(&root, "Import")?;
        let reports = folder(&root, "Reports")?;
        let processing = folder(&imports, ".processing")?;
        let failed = folder(&imports, "Failed")?;
        Ok(Self {
            root,
            imports,
            reports,
            processing,
            failed,
            seen: HashMap::new(),
        })
    }
    fn check(&self) -> Result<()> {
        for path in [
            &self.root,
            &self.imports,
            &self.reports,
            &self.processing,
            &self.failed,
        ] {
            real(path, true)?;
        }
        Ok(())
    }
    fn next(&mut self) -> Result<Option<Job>> {
        self.check()?;
        let mut pending = fs::read_dir(&self.processing)
            .map_err(|e| e.to_string())?
            .filter_map(|entry| entry.ok())
            .collect::<Vec<_>>();
        pending.sort_by_key(|entry| entry.file_name());
        for entry in pending {
            real(&entry.path(), true)?;
            let manifest = entry.path().join("job.json");
            if !manifest.exists() {
                continue;
            }
            let mut job: Job = serde_json::from_slice(&read(&manifest, 16384)?)
                .map_err(|_| "Invalid import recovery record; source retained")?;
            if !safe_id(&job.id)
                || entry.file_name() != std::ffi::OsStr::new(&job.id)
                || !matches!(job.kind.as_str(), "pgn" | "zip")
            {
                return Err("Invalid import recovery record; source retained".into());
            }
            job.directory = entry.path();
            if job.source().exists() {
                real(&job.source(), false)?;
                return Ok(Some(job));
            }
        }
        let mut files = fs::read_dir(&self.imports)
            .map_err(|e| e.to_string())?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| {
                        value.eq_ignore_ascii_case("pgn") || value.eq_ignore_ascii_case("zip")
                    })
            })
            .collect::<Vec<_>>();
        files.sort_by_key(|entry| {
            (
                entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .unwrap_or(UNIX_EPOCH),
                entry.file_name(),
            )
        });
        self.seen.retain(|path, _| path.exists());
        for entry in files {
            let path = entry.path();
            real(&path, false)?;
            let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
            let stamp = (
                metadata.len(),
                metadata.modified().map_err(|e| e.to_string())?,
            );
            if self.seen.insert(path.clone(), stamp) != Some(stamp) {
                continue;
            }
            let id = new_id()?;
            let directory = folder(&self.processing, &id)?;
            let kind = path
                .extension()
                .unwrap()
                .to_string_lossy()
                .to_ascii_lowercase();
            let job = Job {
                id,
                filename: entry.file_name().to_string_lossy().into_owned(),
                kind,
                started: millis(),
                hash: String::new(),
                directory,
            };
            job.save()?;
            if let Err(error) = fs::rename(&path, job.source()) {
                let _ = fs::remove_file(job.directory.join("job.json"));
                let _ = fs::remove_dir(&job.directory);
                self.seen.remove(&path);
                return Err(format!("File may still be copying: {error}"));
            }
            self.seen.remove(&path);
            return Ok(Some(job));
        }
        Ok(None)
    }
    fn finish(&self, job: &Job, stats: &Stats, error: Option<String>) -> Result<()> {
        let stopped_early = error.is_some();
        self.check()?;
        real(&job.directory, true)?;
        real(&job.source(), false)?;
        let error = error.or_else(|| {
            if stats.failed > 0 {
                Some("Some PGN games failed validation; source retained for review".into())
            } else {
                None
            }
        });
        let complete = error.is_none();
        let retained = self
            .failed
            .join(format!("{}-{}", job.id, safe_filename(&job.filename)));
        let report = format!(
            "Ironwood Companion automatic import\n\nFile: {:?}\nStarted (UTC): {}\nFinished (UTC): {}\nStatus: {}\n\nGames processed: {}\nImported: {}\nAlready saved (duplicates): {}\nFailed games: {}\nDiscarded (fewer than 2 full moves): {}\nNot attempted: {}\n\n{}\nSource: {}\n",
            job.filename,
            utc(job.started),
            utc(millis()),
            if complete {
                "Complete"
            } else {
                "Needs attention"
            },
            stats.count(),
            stats.imported,
            stats.duplicates,
            stats.failed,
            stats.discarded,
            if stopped_early {
                "Unknown — import stopped before completion"
            } else {
                "0"
            },
            error
                .as_deref()
                .unwrap_or("Import committed and report saved. Source file removed."),
            if complete {
                "Removed after successful import".into()
            } else {
                retained.display().to_string()
            }
        );
        // The report must be on disk before the source is removed or moved to Failed.
        atomic(
            &self
                .reports
                .join(format!("{}-{}.txt", job.id, safe_filename(&job.filename))),
            report.as_bytes(),
        )?;
        if complete {
            fs::remove_file(job.source()).map_err(|e| e.to_string())?;
        } else {
            if retained.exists() {
                return Err("Retained source already exists; no files overwritten".into());
            }
            fs::rename(job.source(), retained).map_err(|e| e.to_string())?;
        }
        fs::remove_file(job.directory.join("job.json")).map_err(|e| e.to_string())?;
        fs::remove_dir(&job.directory).map_err(|e| e.to_string())?;
        Ok(())
    }
}
fn prepared(games: Vec<crate::archive_pgn::Parsed>) -> archive::Result<PreparedImport> {
    archive::prepare_pgn_parsed(
        games
            .into_iter()
            .map(|parsed| {
                let doc = Document {
                    format: "ironwood-companion-game".into(),
                    version: 1,
                    pgn: parsed.pgn.clone(),
                    record: None,
                    analysis_runs: vec![],
                    metadata: parsed.tags.clone(),
                };
                (doc, parsed)
            })
            .collect(),
    )
}
async fn execute(
    manager: &Manager,
    archive: &Archive,
    control: &Control,
    job: &mut Job,
) -> Result<()> {
    let source = job.source();
    let kind = job.kind.clone();
    let expected = job.hash.clone();
    let (hash, files) = tokio::task::spawn_blocking(move || {
        let bytes = read(&source, crate::import_zip::MAX_BYTES)?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        if !expected.is_empty() && expected != hash {
            return Err(
                "Source changed since the interrupted import; retained without deletion".into(),
            );
        }
        let files = if kind == "zip" {
            crate::import_zip::pgns(&bytes)?
        } else {
            vec![(String::new(), bytes)]
        };
        Ok::<_, String>((hash, files))
    })
    .await
    .map_err(|_| "Import preparation stopped unexpectedly")??;
    job.hash = hash;
    job.save()?;
    let mut batch = 0u64;
    for (name, bytes) in files {
        let mut reader = ImportReader::new(bytes);
        loop {
            let _reservation = reserve(manager, archive, control).await?;
            manager.message(format!(
                "Importing {} {} — batch {}",
                job.filename,
                name,
                batch + 1
            ));
            let (next, items, failed, count) = tokio::task::spawn_blocking(move || {
                let mut games = Vec::new();
                let mut failed = 0;
                let mut count = 0;
                for _ in 0..BATCH {
                    match reader.next().map_err(|error| error.1.to_owned())? {
                        None => break,
                        Some(Ok(game)) => {
                            games.push(game);
                            count += 1;
                        }
                        Some(Err(_)) => {
                            failed += 1;
                            count += 1;
                        }
                    }
                }
                let prepared = prepared(games).map_err(|error| error.1.to_owned())?;
                Ok::<_, String>((reader, prepared, failed, count))
            })
            .await
            .map_err(|_| "PGN parsing stopped unexpectedly")??;
            reader = next;
            if count == 0 {
                break;
            }
            let id = job.id.clone();
            archive
                .run_background(move |db| commit_batch(db, &id, batch, items, failed))
                .await
                .map_err(|error| error.1.to_owned())?;
            batch += 1;
            tokio::task::yield_now().await;
        }
    }
    let stats = totals(archive, job.id.clone()).await?;
    if stats.count() == 0 {
        return Err("No PGN games found; source retained".into());
    }
    let source = job.source();
    let hash = tokio::task::spawn_blocking(move || {
        read(&source, crate::import_zip::MAX_BYTES)
            .map(|bytes| format!("{:x}", Sha256::digest(&bytes)))
    })
    .await
    .map_err(|_| "Source verification stopped")??;
    if hash != job.hash {
        return Err("Source changed during import; retained without deletion".into());
    }
    Ok(())
}
fn initialize(db: &rusqlite::Connection) -> archive::Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS automatic_import_batches(job TEXT NOT NULL,batch INTEGER NOT NULL,stats TEXT NOT NULL,PRIMARY KEY(job,batch))")?;
    Ok(())
}
fn commit_batch(
    db: &mut rusqlite::Connection,
    job: &str,
    batch: u64,
    prepared: PreparedImport,
    failed: u64,
) -> archive::Result<Stats> {
    initialize(db)?;
    let tx = db.transaction()?;
    let saved: Option<String> = tx
        .query_row(
            "SELECT stats FROM automatic_import_batches WHERE job=?1 AND batch=?2",
            params![job, batch],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(saved) = saved {
        return Ok(serde_json::from_str(&saved)?);
    }
    let value = archive::commit_import_rows(&tx, prepared, None)?;
    let stats = Stats {
        imported: value["imported"].as_u64().unwrap_or(0),
        duplicates: value["duplicates"].as_u64().unwrap_or(0),
        failed,
        discarded: value["discarded"].as_u64().unwrap_or(0),
    };
    tx.execute(
        "INSERT INTO automatic_import_batches VALUES(?1,?2,?3)",
        params![job, batch, serde_json::to_string(&stats)?],
    )?;
    tx.commit()?;
    Ok(stats)
}
async fn totals(archive: &Archive, job: String) -> Result<Stats> {
    archive
        .run_background(move |db| {
            initialize(db)?;
            let mut statement =
                db.prepare("SELECT stats FROM automatic_import_batches WHERE job=?1")?;
            let values = statement.query_map([job], |row| row.get::<_, String>(0))?;
            let mut stats = Stats::default();
            for value in values {
                stats.add(&serde_json::from_str::<Stats>(&value?)?);
            }
            Ok(stats)
        })
        .await
        .map_err(|error| error.1.to_owned())
}
fn link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}
fn real(path: &Path, directory: bool) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if link(&metadata)
        || if directory {
            !metadata.is_dir()
        } else {
            !metadata.is_file()
        }
    {
        return Err(format!(
            "Queue paths must be real files/directories: {}",
            path.display()
        ));
    }
    Ok(())
}
fn folder(parent: &Path, name: &str) -> Result<PathBuf> {
    real(parent, true)?;
    let path = parent.join(name);
    if !path.exists() {
        #[allow(unused_mut)]
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path).map_err(|e| e.to_string())?;
    }
    real(&path, true)?;
    Ok(path)
}
fn read(path: &Path, limit: usize) -> Result<Vec<u8>> {
    real(path, false)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > limit as u64 {
        return Err("Input exceeds the supported size limit".into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("Input exceeds the supported size limit".into());
    }
    Ok(bytes)
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("Missing report directory")?;
    real(parent, true)?;
    if path.exists() {
        real(path, false)?;
    }
    let temp = parent.join(format!(".{}.tmp", new_id()?));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temp).map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temp, path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        fs::File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() < 80 && id.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
}
fn new_id() -> Result<String> {
    let mut bytes = [0u8; 12];
    getrandom::fill(&mut bytes).map_err(|_| "Randomness unavailable")?;
    Ok(format!(
        "{}-{}",
        millis(),
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}
fn safe_filename(name: &str) -> String {
    name.chars()
        .take(100)
        .map(|c| {
            if c.is_alphanumeric() || ".-_".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}
fn millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn utc(ms: u64) -> String {
    let seconds = ms / 1000;
    let mut days = seconds / 86400;
    let mut year = 1970;
    let leap = |year: u64| year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    loop {
        let length = if leap(year) { 366 } else { 365 };
        if days < length {
            break;
        }
        days -= length;
        year += 1;
    }
    let months = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for length in months {
        if days < length {
            break;
        }
        days -= length;
        month += 1;
    }
    format!(
        "{year:04}-{month:02}-{:02}T{:02}:{:02}:{:02}Z",
        days + 1,
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    const PGN: &str = "[Event \"Queue\"]\n[White \"Alice\"]\n[Black \"Bob\"]\n\n1. e4 e5 2. Nf3 Nc6 *\n";
    struct Temp(PathBuf);
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn setup() -> (Temp, Archive, Control) {
        let root = std::env::temp_dir().join(format!("ironwood-queue-{}", new_id().unwrap()));
        fs::create_dir(&root).unwrap();
        let archive = Archive::open(&root.join("archive.sqlite3")).unwrap();
        let mut config = crate::Config::defaults(false);
        config.archive = None;
        config.token = "a".repeat(64);
        let (_, control) = crate::app_with_control(config).unwrap();
        (Temp(root), archive, control)
    }
    fn claim(queue: &mut Queue, name: &str, bytes: &[u8]) -> Job {
        fs::write(queue.imports.join(name), bytes).unwrap();
        assert!(queue.next().unwrap().is_none());
        queue.next().unwrap().unwrap()
    }
    async fn run(queue: &Queue, archive: &Archive, control: &Control, job: &mut Job) -> Stats {
        let result = execute(&control.imports, archive, control, job).await;
        let stats = totals(archive, job.id.clone()).await.unwrap();
        queue.finish(job, &stats, result.err()).unwrap();
        stats
    }
    #[tokio::test]
    async fn short_games_are_discarded_reported_and_counted_as_processed() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(&mut queue,"short.pgn",b"[White \"Short\"]\n\n1. e4 e5 2. Nf3 *\n");
        let stats=run(&queue,&archive,&control,&mut job).await;
        assert_eq!(stats.discarded,1);assert_eq!(stats.failed,0);assert_eq!(stats.imported,0);
        assert!(!job.source().exists());
        let report=fs::read_to_string(queue.reports.join(format!("{}-short.pgn.txt",job.id))).unwrap();
        assert!(report.contains("Discarded (fewer than 2 full moves): 1"));
    }
    #[tokio::test]
    async fn imports_large_pgn_in_batches_and_writes_report_before_removing_source() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(&mut queue, "collection.pgn", PGN.repeat(1001).as_bytes());
        let id = job.id.clone();
        let stats = run(&queue, &archive, &control, &mut job).await;
        assert_eq!(
            stats,
            Stats {
                imported: 1,
                duplicates: 1000,
                failed: 0, discarded: 0
            }
        );
        assert!(!job.source().exists());
        assert!(!queue.imports.join("collection.pgn").exists());
        let report =
            fs::read_to_string(queue.reports.join(format!("{id}-collection.pgn.txt"))).unwrap();
        assert!(report.contains("Imported: 1"));
        assert!(report.contains("Already saved (duplicates): 1000"));
        assert!(report.contains("Not attempted: 0"));
        assert!(report.contains("collection.pgn"));
    }
    #[tokio::test]
    async fn failed_report_preserves_source_and_restart_reuses_committed_batch_statistics() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(&mut queue, "restart.pgn", PGN.repeat(129).as_bytes());
        execute(&control.imports, &archive, &control, &mut job)
            .await
            .unwrap();
        let expected = totals(&archive, job.id.clone()).await.unwrap();
        let report = queue.reports.join(format!("{}-restart.pgn.txt", job.id));
        fs::create_dir(&report).unwrap();
        assert!(queue.finish(&job, &expected, None).is_err());
        assert!(job.source().exists());
        fs::remove_dir(&report).unwrap();
        let mut restarted = Queue::new(temp.0.clone()).unwrap();
        let mut recovered = restarted.next().unwrap().unwrap();
        assert_eq!(recovered.id, job.id);
        let stats = run(&restarted, &archive, &control, &mut recovered).await;
        assert_eq!(stats, expected);
        assert_eq!(
            stats,
            Stats {
                imported: 1,
                duplicates: 128,
                failed: 0, discarded: 0
            }
        );
    }
    #[tokio::test]
    async fn queue_preserves_existing_annotated_versions_and_saved_analysis_runs() {
        let (temp, archive, control) = setup();
        let mut document = crate::archive_pgn::documents(PGN).unwrap().remove(0);
        document.pgn = document.pgn.replace("e4", "e4 {Original note}");
        document.record = Some(
            json!({"id":"original","json":json!({"board":"kept","history":[],"move_notes":["Original note"]}).to_string()}),
        );
        document.analysis_runs.push(crate::analysis_runs::Run {
            id: "kept-run".into(),
            revision: 1,
            created_at: 10,
            metadata: json!({"engine_name":"Stockfish"}),
            payload: json!({"game_analysis":[]}),
        });
        let result = archive
            .run(move |db| archive::import(db, vec![document], None))
            .await
            .unwrap();
        let id = result["ids"][0].as_str().unwrap().to_owned();
        let original = archive
            .run({
                let id = id.clone();
                move |db| archive::get(db, &id)
            })
            .await
            .unwrap();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(&mut queue, "duplicate.pgn", PGN.as_bytes());
        assert_eq!(run(&queue, &archive, &control, &mut job).await.imported, 1);
        let after = archive.run(move |db| archive::get(db, &id)).await.unwrap();
        assert_eq!(
            serde_json::to_value(after).unwrap(),
            serde_json::to_value(original).unwrap()
        );
    }
    #[tokio::test]
    async fn exact_duplicate_preserves_saved_analysis_runs() {
        let (temp, archive, control) = setup();
        let mut document = crate::archive_pgn::documents(PGN).unwrap().remove(0);
        document.analysis_runs.push(crate::analysis_runs::Run {
            id: "kept-run".into(),
            revision: 1,
            created_at: 10,
            metadata: json!({"engine_name":"Stockfish"}),
            payload: json!({"game_analysis":[]}),
        });
        let result = archive
            .run(move |db| archive::import(db, vec![document], None))
            .await
            .unwrap();
        let id = result["ids"][0].as_str().unwrap().to_owned();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(&mut queue, "duplicate.pgn", PGN.as_bytes());
        assert_eq!(
            run(&queue, &archive, &control, &mut job).await.duplicates,
            1
        );
        let runs = archive
            .run(move |db| crate::analysis_runs::all(db, &id))
            .await
            .unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, "kept-run");
    }
    #[tokio::test]
    async fn invalid_games_are_reported_and_source_is_retained_in_failed() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let invalid = "[White \"Bad\"]\n\n1. e5 *\n";
        let mut job = claim(
            &mut queue,
            "partial.pgn",
            format!("{invalid}\n{PGN}").as_bytes(),
        );
        let stats = run(&queue, &archive, &control, &mut job).await;
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.imported, 1);
        assert!(
            queue
                .failed
                .join(format!("{}-partial.pgn", job.id))
                .exists()
        );
        let report =
            fs::read_to_string(queue.reports.join(format!("{}-partial.pgn.txt", job.id))).unwrap();
        assert!(report.contains("Needs attention"));
    }
    #[tokio::test]
    async fn zip_queue_import_and_corrupt_zip_never_extract_paths_or_delete_failed_sources() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let bytes = zip("../../nested/game.pgn", PGN.as_bytes());
        let mut job = claim(&mut queue, "games.ZIP", &bytes);
        assert_eq!(run(&queue, &archive, &control, &mut job).await.imported, 1);
        assert!(!temp.0.join("nested").exists());
        let mut bad = claim(&mut queue, "broken.zip", b"bad zip");
        let stats = run(&queue, &archive, &control, &mut bad).await;
        assert_eq!(stats.count(), 0);
        assert!(queue.failed.join(format!("{}-broken.zip", bad.id)).exists());
    }
    #[tokio::test]
    async fn active_engines_pause_work_and_update_reservations_exclude_import_commits() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(&mut queue, "idle.pgn", PGN.as_bytes());
        let engine = control.engines.clone().try_acquire_owned().unwrap();
        assert!(
            idle(&control, &archive),
            "A loaded but idle engine must not block imports"
        );
        let mut activity = control.engine_activity();
        activity.set(true);
        assert!(!idle(&control, &archive));
        let manager = control.imports.clone();
        let worker_archive = archive.clone();
        let worker_control = control.clone();
        let running = tokio::spawn(async move {
            execute(&manager, &worker_archive, &worker_control, &mut job).await
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!running.is_finished());
        drop(activity);
        running.await.unwrap().unwrap();
        drop(engine);
        let import = control.imports.reserve().unwrap();
        assert!(control.reserve_update().is_none());
        drop(import);
        let update = control.reserve_update().unwrap();
        assert!(control.imports.reserve().is_none());
        drop(update);
        assert!(control.imports.reserve().is_some());
    }
    #[tokio::test]
    async fn worker_waits_for_startup_validation_and_drains_on_shutdown() {
        let (temp, archive, control) = setup();
        control.imports.configure(Some(archive.clone()));
        let marker = temp.0.join("update.pending");
        fs::write(&marker, "pending").unwrap();
        control
            .imports
            .start(control.clone(), temp.0.clone(), true, Some(marker.clone()))
            .unwrap();
        fs::write(temp.0.join("Import/queued.pgn"), PGN).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(temp.0.join("Import/queued.pgn").exists());
        assert!(control.imports.status().contains("validation"));
        fs::remove_file(marker).unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while fs::read_dir(temp.0.join("Reports"))
            .unwrap()
            .next()
            .is_none()
        {
            assert!(
                tokio::time::Instant::now() < deadline,
                "queue did not finish"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        control.shutdown();
        control.drained().await;
        assert!(!temp.0.join("Import/queued.pgn").exists());
    }
    #[test]
    fn changed_files_wait_and_non_import_files_are_ignored() {
        let (temp, _, _) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        fs::write(queue.imports.join("notes.txt"), "ignore").unwrap();
        fs::write(queue.imports.join("game.pgn"), PGN).unwrap();
        assert!(queue.next().unwrap().is_none());
        fs::write(queue.imports.join("game.pgn"), PGN.repeat(2)).unwrap();
        assert!(queue.next().unwrap().is_none());
        assert!(queue.next().unwrap().is_some());
        assert!(queue.imports.join("notes.txt").exists());
        assert_eq!(utc(0), "1970-01-01T00:00:00Z");
    }
    #[test]
    fn stopped_files_report_unknown_remainder_even_if_some_games_already_failed() {
        let (temp, _, _) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let job = claim(&mut queue, "stopped.pgn", PGN.as_bytes());
        queue
            .finish(
                &job,
                &Stats {
                    imported: 0,
                    duplicates: 0,
                    failed: 1,
                    discarded: 0,
                },
                Some("Malformed PGN stream".into()),
            )
            .unwrap();
        let report =
            fs::read_to_string(queue.reports.join(format!("{}-stopped.pgn.txt", job.id))).unwrap();
        assert!(report.contains("Not attempted: Unknown"));
        assert!(report.contains("Failed games: 1"));
    }
    #[test]
    fn native_zip_rejects_encryption_corruption_and_expansion_limits() {
        let base = zip("game.pgn", PGN.as_bytes());
        let end = base.len() - 22;
        let central = u32::from_le_bytes(base[end + 16..end + 20].try_into().unwrap()) as usize;
        let mut encrypted = base.clone();
        short(&mut encrypted, 6, 1);
        short(&mut encrypted, central + 8, 1);
        assert!(
            crate::import_zip::pgns(&encrypted)
                .unwrap_err()
                .contains("Password")
        );
        let mut corrupt = base.clone();
        corrupt[central + 16] ^= 1;
        assert!(
            crate::import_zip::pgns(&corrupt)
                .unwrap_err()
                .contains("integrity")
        );
        let mut bomb = base.clone();
        put(&mut bomb, central + 24, 65 * 1024 * 1024);
        assert!(
            crate::import_zip::pgns(&bomb)
                .unwrap_err()
                .contains("64 MiB")
        );
        let mut files = base.clone();
        short(&mut files, end + 8, 1001);
        short(&mut files, end + 10, 1001);
        assert!(
            crate::import_zip::pgns(&files)
                .unwrap_err()
                .contains("1,000")
        );
    }
    #[tokio::test]
    async fn source_changes_after_interruption_are_retained_instead_of_skipping_new_data() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(&mut queue, "changed.pgn", PGN.as_bytes());
        execute(&control.imports, &archive, &control, &mut job)
            .await
            .unwrap();
        fs::write(job.source(), PGN.replace("Alice", "Changed")).unwrap();
        let error = execute(&control.imports, &archive, &control, &mut job)
            .await
            .unwrap_err();
        assert!(error.contains("Source changed"));
        assert!(job.source().exists());
        let stats = totals(&archive, job.id.clone()).await.unwrap();
        assert_eq!(stats.imported, 1);
        queue.finish(&job, &stats, Some(error)).unwrap();
        assert!(
            queue
                .failed
                .join(format!("{}-changed.pgn", job.id))
                .exists()
        );
    }
    fn zip(name: &str, bytes: &[u8]) -> Vec<u8> {
        use flate2::{Compression, write::DeflateEncoder};
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes).unwrap();
        let data = encoder.finish().unwrap();
        let crc = crate::import_zip::test_crc(bytes);
        let mut output = vec![0u8; 30];
        put(&mut output, 0, 0x04034b50);
        short(&mut output, 4, 20);
        short(&mut output, 8, 8);
        put(&mut output, 14, crc);
        put(&mut output, 18, data.len() as u32);
        put(&mut output, 22, bytes.len() as u32);
        short(&mut output, 26, name.len() as u16);
        output.extend(name.as_bytes());
        output.extend(data);
        let offset = output.len();
        let mut central = vec![0u8; 46];
        put(&mut central, 0, 0x02014b50);
        short(&mut central, 6, 20);
        short(&mut central, 10, 8);
        put(&mut central, 16, crc);
        put(&mut central, 20, (offset - 30 - name.len()) as u32);
        put(&mut central, 24, bytes.len() as u32);
        short(&mut central, 28, name.len() as u16);
        output.extend(central);
        output.extend(name.as_bytes());
        let size = output.len() - offset;
        let mut end = vec![0u8; 22];
        put(&mut end, 0, 0x06054b50);
        short(&mut end, 8, 1);
        short(&mut end, 10, 1);
        put(&mut end, 12, size as u32);
        put(&mut end, 16, offset as u32);
        output.extend(end);
        output
    }
    fn put(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn short(bytes: &mut [u8], at: usize, value: u16) {
        bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }
}

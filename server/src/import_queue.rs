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
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, String>;
const BATCH: usize = 128;
#[derive(Clone, Default, Serialize)]
pub struct Progress {
    active: bool,
    filename: Option<String>,
    phase: String,
    message: String,
    total_games: Option<u64>,
    processed: u64,
    stats: Stats,
    games_per_second: f64,
    eta_seconds: Option<u64>,
}
#[derive(Default)]
struct Tracking {
    value: Progress,
    started: Option<Instant>,
    elapsed: f64,
    baseline: u64,
}
#[derive(Clone)]
pub struct Manager {
    archive: Arc<Mutex<Option<Archive>>>,
    status: Arc<Mutex<String>>,
    progress: Arc<Mutex<Tracking>>,
    work: Arc<tokio::sync::Semaphore>,
    worker: Arc<tokio::sync::Semaphore>,
}
impl Default for Manager {
    fn default() -> Self {
        Self {
            archive: Arc::default(),
            status: Arc::new(Mutex::new("Not started".into())),
            progress: Arc::new(Mutex::new(Tracking::default())),
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
    pub fn activity_label(&self) -> Option<&'static str> {
        let p = self.progress();
        if !p.active {
            return None;
        }
        Some(match p.phase.as_str() {
            "waiting" => "Import waiting",
            "resuming" => "Resuming import",
            "unpacking" => "Unpacking import",
            "counting" | "preparing" => "Preparing import",
            "verifying" => "Finishing import",
            _ => "Importing games",
        })
    }
    pub fn status_details(&self) -> String {
        let p = self.progress();
        let Some(filename) = p.filename else {
            return self.status();
        };
        let count = |n: u64| {
            let digits = n.to_string();
            let mut text = String::new();
            for (i, c) in digits.chars().enumerate() {
                if i > 0 && (digits.len() - i) % 3 == 0 {
                    text.push(',');
                }
                text.push(c);
            }
            text
        };
        let total = p
            .total_games
            .map(|n| format!(" of {}", count(n)))
            .unwrap_or_default();
        let speed = if p.games_per_second > 0.0 {
            format!("{:.1} games/s", p.games_per_second)
        } else {
            "Speed available when importing".into()
        };
        let eta = if let Some(seconds) = p.eta_seconds {
            let minutes = (seconds + 59) / 60;
            if minutes >= 1440 {
                format!(
                    "About {}d {}h remaining",
                    minutes / 1440,
                    (minutes % 1440) / 60
                )
            } else if minutes >= 60 {
                format!("About {}h {}m remaining", minutes / 60, minutes % 60)
            } else if seconds >= 60 {
                format!("About {minutes}m remaining")
            } else {
                format!("About {seconds}s remaining")
            }
        } else {
            match p.phase.as_str() {
                "waiting" => "Waiting — estimate paused",
                "complete" => "Complete",
                "failed" => "Needs attention",
                "unpacking" => "Unpacking before import",
                "verifying" => "Verifying source",
                _ => "Estimating time…",
            }
            .into()
        };
        format!(
            "{}\r\nFile: {}\r\nRead {}{} · Saved {} · Duplicates {}\r\nShort {} · Deleted {} · Failed {}\r\n{} · {}",
            p.message,
            filename,
            count(p.processed),
            total,
            count(p.stats.imported),
            count(p.stats.duplicates),
            count(p.stats.discarded),
            count(p.stats.deleted),
            count(p.stats.failed),
            speed,
            eta
        )
    }
    fn message(&self, text: impl Into<String>) {
        let text = text.into();
        *self.status.lock().unwrap() = text.clone();
        let mut tracking = self.progress.lock().unwrap();
        if tracking.value.active || tracking.value.filename.is_none() {
            tracking.value.phase = if text.starts_with("Paused") || text.starts_with("Waiting") {
                "waiting"
            } else if text.starts_with("Importing") {
                "importing"
            } else if text.starts_with("Verifying and unpacking") {
                "unpacking"
            } else if text.starts_with("Resuming") {
                "resuming"
            } else if text.starts_with("Counting") {
                "counting"
            } else if text.starts_with("Verifying source") {
                "verifying"
            } else {
                "idle"
            }
            .into();
            tracking.value.message = text;
        }
    }
    pub fn progress(&self) -> Progress {
        let tracking = self.progress.lock().unwrap();
        let mut value = tracking.value.clone();
        let elapsed = tracking
            .started
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        let elapsed = if value.active {
            elapsed
        } else {
            tracking.elapsed
        };
        value.games_per_second = if elapsed > 0.0 {
            value.stats.count().saturating_sub(tracking.baseline) as f64 / elapsed
        } else {
            0.0
        };
        value.eta_seconds =
            if value.active && value.phase == "importing" && value.games_per_second > 0.0 {
                value.total_games.map(|total| {
                    (total.saturating_sub(value.stats.count()) as f64 / value.games_per_second)
                        .ceil() as u64
                })
            } else {
                None
            };
        value.processed = value.stats.count();
        value
    }
    fn begin(&self, job: &Job, stats: Stats) {
        *self.progress.lock().unwrap() = Tracking {
            baseline: stats.count(),
            value: Progress {
                active: true,
                filename: Some(job.filename.clone()),
                phase: "preparing".into(),
                message: "Preparing folder import".into(),
                stats,
                ..Default::default()
            },
            ..Default::default()
        };
    }
    fn ready(&self, total: u64) {
        let mut p = self.progress.lock().unwrap();
        p.value.total_games = Some(total);
        p.started = Some(Instant::now());
    }
    fn committed(&self, stats: &Stats) {
        let mut p = self.progress.lock().unwrap();
        if stats.count() >= p.baseline {
            p.value.stats = stats.clone();
        }
    }
    fn finished(&self, error: Option<&str>) {
        let mut p = self.progress.lock().unwrap();
        p.elapsed = p.started.map(|t| t.elapsed().as_secs_f64()).unwrap_or(0.0);
        p.value.active = false;
        p.value.phase = if error.is_some() {
            "failed"
        } else {
            "complete"
        }
        .into();
        p.value.message = error.unwrap_or("Complete — report saved").into();
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
                } else if let Some(reason) = wait_reason(&control, &archive) {
                    manager.message(format!("Waiting for {reason}"));
                } else {
                    match queue.next() {
                        Ok(Some(mut job)) => {
                            manager.message(format!("Importing {}", job.filename));
                            let result = execute(&manager, &archive, &control, &mut job).await;
                            if control.stopping() {
                                manager.message("Interrupted; source retained for restart");
                                break;
                            }
                            let mut failure = result.as_ref().err().cloned();
                            let stats = totals(&archive, job.id.clone()).await;
                            if failure.is_none() && stats.as_ref().is_ok_and(|s| s.failed > 0) {
                                failure = Some(
                                    "Some games failed validation; source retained for review"
                                        .into(),
                                );
                            }
                            let finished =
                                stats.and_then(|stats| queue.finish(&job, &stats, result.err()));
                            manager.finished(
                                finished
                                    .as_ref()
                                    .err()
                                    .or(failure.as_ref())
                                    .map(String::as_str),
                            );
                            match finished {
                                Ok(()) => manager
                                    .message(format!("Finished {}; report saved", job.filename)),
                                Err(error) => manager.message(format!("Source retained: {error}")),
                            }
                        }
                        Ok(None) => {
                            manager.message("Idle — waiting for PGN, ZIP or 7z files in Import")
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
    wait_reason(control, archive).is_none()
}
fn engine_wait_reason(control: &Control) -> Option<&'static str> {
    if control.stopping() {
        Some("Companion shutdown")
    } else if control.registrations.available_permits() == 0 {
        Some("engine setup or validation to finish")
    } else if !control.engine_idle() {
        if control
            .engine_work
            .count
            .load(std::sync::atomic::Ordering::SeqCst)
            > 0
        {
            Some("a chess engine to finish searching")
        } else {
            Some("the two-second quiet period after engine activity")
        }
    } else {
        None
    }
}
fn wait_reason(control: &Control, archive: &Archive) -> Option<&'static str> {
    engine_wait_reason(control).or_else(|| {
        if archive.idle() {
            None
        } else {
            Some("an archive request to finish")
        }
    })
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
        if let Some(reason) = wait_reason(control, archive) {
            manager.message(format!("Paused — waiting for {reason}"));
        } else {
            if let Some(permit) = manager.reserve() {
                return Ok(permit);
            }
            manager.message("Paused — waiting for a Companion update or another import batch");
        }
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
    #[serde(default)]
    deleted: u64,
}
impl Stats {
    fn count(&self) -> u64 {
        self.imported + self.duplicates + self.failed + self.discarded + self.deleted
    }
    fn add(&mut self, other: &Self) {
        self.imported += other.imported;
        self.duplicates += other.duplicates;
        self.failed += other.failed;
        self.discarded += other.discarded;
        self.deleted += other.deleted;
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
                || !matches!(job.kind.as_str(), "pgn" | "zip" | "7z")
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
                        value.eq_ignore_ascii_case("pgn")
                            || value.eq_ignore_ascii_case("zip")
                            || value.eq_ignore_ascii_case("7z")
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
                Some("Some games failed validation; source retained for review".into())
            } else {
                None
            }
        });
        let complete = error.is_none();
        let retained = self
            .failed
            .join(format!("{}-{}", job.id, safe_filename(&job.filename)));
        let report = format!(
            "Ironwood Companion automatic import\n\nFile: {:?}\nStarted (UTC): {}\nFinished (UTC): {}\nStatus: {}\n\nGames processed: {}\nImported: {}\nAlready saved (duplicates): {}\nFailed games: {}\nDiscarded (fewer than 2 full moves): {}\nDeleted Scid games skipped: {}\nNot attempted: {}\n\n{}\nSource: {}\n",
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
            stats.deleted,
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
        crate::import_sevenz::cleanup(&job.directory)?;
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
    manager.begin(job, totals(archive, job.id.clone()).await?);
    let receipts = Arc::new(saved_batches(archive, job.id.clone()).await?);
    if !receipts.is_empty() && job.hash.is_empty() {
        return Err("Saved batches have no verified source fingerprint; source retained".into());
    }
    if job.kind == "7z" {
        return execute_sevenz(manager, archive, control, job, receipts).await;
    }
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
    manager.message("Counting games in folder import");
    let (c, m) = (control.clone(), manager.clone());
    let (total, files) = tokio::task::spawn_blocking(move || {
        let total = files.iter().try_fold(0u64, |sum, (_, bytes)| {
            crate::archive_pgn::count_games(std::io::Cursor::new(bytes), &mut || {
                preparation_check(&c, &m)
            })
            .map(|n| sum + n)
        })?;
        Ok::<_, String>((total, files))
    })
    .await
    .map_err(|_| "Counting games stopped unexpectedly")??;
    manager.ready(total);
    let mut committed = Stats::default();
    let mut batch = 0u64;
    let mut rate_started = receipts.is_empty();
    for (name, bytes) in files {
        let mut reader = ImportReader::new(bytes);
        let (c, m, saved) = (control.clone(), manager.clone(), receipts.clone());
        let cursor = (batch as usize).min(saved.len());
        if cursor < saved.len() {
            manager.message("Resuming from the last saved batch — locating the next PGN game");
        }
        let (next, skipped) = tokio::task::spawn_blocking(move || {
            let recovered = skip_saved_batches(
                &saved[cursor..],
                |count| reader.skip_games(count).map_err(|e| e.1.to_owned()),
                &mut || preparation_check(&c, &m),
            )?;
            Ok::<_, String>((reader, recovered))
        })
        .await
        .map_err(|_| "PGN checkpoint recovery stopped")??;
        reader = next;
        batch += skipped.0;
        committed.add(&skipped.1);
        manager.committed(&committed);
        if !rate_started && batch as usize >= receipts.len() {
            manager.ready(total);
            rate_started = true;
        }
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
            let saved = archive
                .run_background(move |db| commit_batch(db, &id, batch, items, failed, 0))
                .await
                .map_err(|error| error.1.to_owned())?;
            committed.add(&saved);
            manager.committed(&committed);
            batch += 1;
            tokio::task::yield_now().await;
        }
    }
    if (batch as usize) < receipts.len() {
        return Err("Saved checkpoint exceeds the verified source; source retained".into());
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
fn preparation_check(control: &Control, manager: &Manager) -> Result<()> {
    let mut prior = None;
    loop {
        if control.stopping() {
            return Err("Interrupted; source retained for restart".into());
        }
        if engine_wait_reason(control).is_none() {
            if let Some(message) = prior {
                manager.message(message);
            }
            return Ok(());
        }
        prior.get_or_insert_with(|| manager.status());
        manager.message(format!(
            "Paused — waiting for {}",
            engine_wait_reason(control).unwrap_or("engine activity to settle")
        ));
        std::thread::sleep(Duration::from_millis(250));
    }
}
enum NativeReader {
    Pgn(ImportReader),
    Scid(crate::import_sevenz::ScidReader),
}
impl NativeReader {
    fn skip_games(&mut self, count: u64) -> Result<u64> {
        match self {
            Self::Pgn(reader) => reader.skip_games(count).map_err(|e| e.1.to_owned()),
            Self::Scid(reader) => reader.skip_games(count),
        }
    }
    fn open(input: crate::import_sevenz::Input) -> Result<Self> {
        use crate::import_sevenz::{Input, ScidReader};
        match input {
            Input::Pgn { path, .. } => {
                real(&path, false)?;
                Ok(Self::Pgn(ImportReader::from_reader(
                    fs::File::open(path).map_err(|e| e.to_string())?,
                )))
            }
            Input::Scid {
                index,
                names,
                games,
                version,
                ..
            } => Ok(Self::Scid(ScidReader::open(
                &index, &names, &games, version,
            )?)),
        }
    }
    fn batch(&mut self) -> Result<(PreparedImport, u64, u64, u64)> {
        let (mut games, mut failed, mut deleted, mut count) = (Vec::new(), 0, 0, 0);
        for _ in 0..BATCH {
            match self {
                Self::Pgn(reader) => match reader.next().map_err(|e| e.1.to_owned())? {
                    None => break,
                    Some(Ok(game)) => games.push(game),
                    Some(Err(_)) => failed += 1,
                },
                Self::Scid(reader) => match reader.next()? {
                    None => break,
                    Some(crate::import_sevenz::Game::Deleted) => deleted += 1,
                    Some(crate::import_sevenz::Game::Failed) => failed += 1,
                    Some(crate::import_sevenz::Game::Pgn(pgn)) => {
                        match crate::archive_pgn::parse(&pgn) {
                            Ok(mut parsed) if parsed.len() == 1 => games.push(parsed.remove(0)),
                            _ => failed += 1,
                        }
                    }
                },
            }
            count += 1;
        }
        Ok((
            prepared(games).map_err(|e| e.1.to_owned())?,
            failed,
            deleted,
            count,
        ))
    }
}
async fn execute_sevenz(
    manager: &Manager,
    archive: &Archive,
    control: &Control,
    job: &mut Job,
    receipts: Arc<Vec<Stats>>,
) -> Result<()> {
    let permit = reserve(manager, archive, control).await?;
    manager.message(format!(
        "Verifying and unpacking {} — large archives can take several minutes",
        job.filename
    ));
    let (source, directory, expected) = (job.source(), job.directory.clone(), job.hash.clone());
    let (c, m) = (control.clone(), manager.clone());
    let (hash, inputs) = tokio::task::spawn_blocking(move || {
        let mut check = || preparation_check(&c, &m);
        let hash = crate::import_sevenz::hash(&source, &mut check)?;
        if !expected.is_empty() && hash != expected {
            return Err("Source changed since interrupted import; source retained".into());
        }
        m.message("Resuming saved import — verifying extracted files");
        let inputs = if let Some(inputs) =
            crate::import_sevenz::cached_inputs(&directory, &hash, &mut check).unwrap_or(None)
        {
            inputs
        } else {
            check()?;
            m.message("Verifying and unpacking source archive");
            let inputs = crate::import_sevenz::extract(&source, &directory, &mut check)?;
            m.message("Resuming saved import — verifying extraction checkpoint");
            crate::import_sevenz::cache_inputs(&directory, &hash, &inputs, &mut check)?;
            inputs
        };
        m.message("Counting games in folder import");
        let total = inputs.iter().try_fold(0u64, |sum, input| {
            input.game_count(&mut check).map(|n| sum + n)
        })?;
        m.ready(total);
        Ok::<_, String>((hash, inputs))
    })
    .await
    .map_err(|_| "7z preparation stopped unexpectedly")??;
    job.hash = hash;
    job.save()?;
    drop(permit);
    let mut batch = 0;
    let mut committed = Stats::default();
    let mut rate_started = receipts.is_empty();
    for input in inputs {
        let name = input.name().to_owned();
        let mut reader = tokio::task::spawn_blocking(move || NativeReader::open(input))
            .await
            .map_err(|_| "Import reader stopped")??;
        let (c, m, saved) = (control.clone(), manager.clone(), receipts.clone());
        let cursor = (batch as usize).min(saved.len());
        if cursor < saved.len() {
            manager.message("Resuming from the last saved batch — skipping committed games");
        }
        let (next, skipped) = tokio::task::spawn_blocking(move || {
            let recovered = skip_saved_batches(
                &saved[cursor..],
                |count| reader.skip_games(count),
                &mut || preparation_check(&c, &m),
            )?;
            Ok::<_, String>((reader, recovered))
        })
        .await
        .map_err(|_| "Import checkpoint recovery stopped")??;
        reader = next;
        batch += skipped.0;
        committed.add(&skipped.1);
        manager.committed(&committed);
        if !rate_started && batch as usize >= receipts.len() {
            let total = manager.progress().total_games.unwrap_or(0);
            manager.ready(total);
            rate_started = true;
        }
        loop {
            let _permit = reserve(manager, archive, control).await?;
            manager.message(format!(
                "Importing {} / {} — batch {}",
                job.filename,
                name,
                batch + 1
            ));
            let (next, (items, failed, deleted, count)) = tokio::task::spawn_blocking(move || {
                let items = reader.batch()?;
                Ok::<_, String>((reader, items))
            })
            .await
            .map_err(|_| "Import parsing stopped")??;
            reader = next;
            if count == 0 {
                break;
            }
            let id = job.id.clone();
            let saved = archive
                .run_background(move |db| commit_batch(db, &id, batch, items, failed, deleted))
                .await
                .map_err(|e| e.1.to_owned())?;
            committed.add(&saved);
            manager.committed(&committed);
            batch += 1;
            tokio::task::yield_now().await;
        }
    }
    if (batch as usize) < receipts.len() {
        return Err("Saved checkpoint exceeds the verified source; source retained".into());
    }
    if totals(archive, job.id.clone()).await?.count() == 0 {
        return Err("No games found; source retained".into());
    }
    let _permit = reserve(manager, archive, control).await?;
    manager.message("Verifying source before final report");
    let (source, c, m) = (job.source(), control.clone(), manager.clone());
    let hash = tokio::task::spawn_blocking(move || {
        crate::import_sevenz::hash(&source, &mut || preparation_check(&c, &m))
    })
    .await
    .map_err(|_| "Source verification stopped")??;
    if hash != job.hash {
        return Err("Source changed during import; source retained".into());
    }
    Ok(())
}
// Receipts are committed atomically with their games. Only the contiguous saved
// prefix can be skipped, after verifying that the claimed source is unchanged.
fn skip_saved_batches(
    receipts: &[Stats],
    mut skip: impl FnMut(u64) -> Result<u64>,
    check: &mut impl FnMut() -> Result<()>,
) -> Result<(u64, Stats)> {
    let (mut batches, mut stats) = (0, Stats::default());
    for saved in receipts {
        check()?;
        let count = saved.count();
        if count == 0 || count > BATCH as u64 {
            return Err("Invalid saved batch size; source retained".into());
        }
        let skipped = skip(count)?;
        if skipped == 0 {
            break;
        }
        if skipped != count {
            return Err("Saved batch exceeds the source file; source retained".into());
        }
        batches += 1;
        stats.add(saved);
        if count < BATCH as u64 {
            if skip(1)? != 0 {
                return Err(
                    "Partial saved batch does not end its source file; source retained".into(),
                );
            }
            break;
        }
    }
    Ok((batches, stats))
}
async fn saved_batches(archive: &Archive, job: String) -> Result<Vec<Stats>> {
    archive
        .run_background(move |db| {
            initialize(db)?;
            let mut query = db.prepare(
                "SELECT batch,stats FROM automatic_import_batches WHERE job=?1 ORDER BY batch",
            )?;
            let rows = query.query_map([job], |row| {
                Ok((row.get::<_, u64>(0)?, row.get::<_, String>(1)?))
            })?;
            let mut saved = Vec::new();
            for row in rows {
                let (batch, json) = row?;
                if batch != saved.len() as u64 {
                    return Err(archive::invalid("Import checkpoint has a missing batch"));
                }
                let stats: Stats = serde_json::from_str(&json)?;
                if stats.count() == 0 || stats.count() > BATCH as u64 {
                    return Err(archive::invalid("Invalid import checkpoint batch size"));
                }
                saved.push(stats);
            }
            Ok(saved)
        })
        .await
        .map_err(|e| e.1.to_owned())
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
    deleted: u64,
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
        deleted,
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
pub(crate) fn real(path: &Path, directory: bool) -> Result<()> {
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
pub(crate) fn folder(parent: &Path, name: &str) -> Result<PathBuf> {
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
    #[test]
    fn resume_crosses_full_and_partial_file_boundaries_without_revalidating_saved_moves() {
        let receipts = [
            Stats {
                imported: 1,
                duplicates: 127,
                ..Default::default()
            },
            Stats {
                failed: 3,
                ..Default::default()
            },
            Stats {
                duplicates: 128,
                ..Default::default()
            },
        ];
        let mut cursor = 0;
        let mut recovered = Stats::default();
        for count in [128, 3, 129] {
            let mut reader = ImportReader::new(PGN.repeat(count).into_bytes());
            let (batches, stats) = skip_saved_batches(
                &receipts[cursor..],
                |n| reader.skip_games(n).map_err(|e| e.1.to_owned()),
                &mut || Ok(()),
            )
            .unwrap();
            cursor += batches as usize;
            recovered.add(&stats);
            if count == 129 {
                assert!(reader.next().unwrap().unwrap().is_ok());
            }
            assert!(reader.next().unwrap().is_none());
        }
        assert_eq!(cursor, 3);
        assert_eq!(recovered.count(), 259);
        assert_eq!(recovered.failed, 3);
        let mut reader=ImportReader::new(b"[Variant \"Unsupported saved variant\"]\n\n1. e4 e5 2. Nf3 Nc6 *\n\n[White \"Next game\"]\n\n1. d4 d5 2. c4 e6 *\n".to_vec());
        assert_eq!(reader.skip_games(1).unwrap(), 1);
        assert_eq!(
            reader.next().unwrap().unwrap().unwrap().tags["White"],
            "Next game"
        );
    }
    #[test]
    fn resume_rejects_partial_batches_that_do_not_end_a_file() {
        let mut reader = ImportReader::new(PGN.repeat(4).into_bytes());
        let result = skip_saved_batches(
            &[Stats {
                duplicates: 3,
                ..Default::default()
            }],
            |n| reader.skip_games(n).map_err(|e| e.1.to_owned()),
            &mut || Ok(()),
        );
        assert!(result.unwrap_err().contains("does not end"));
    }
    #[tokio::test]
    async fn sevenz_cache_reuses_verified_files_and_rebuilds_them_if_modified() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(
            &mut queue,
            "cached.7z",
            include_bytes!("../../tests/fixtures/scid/sample-si5.7z"),
        );
        execute(&control.imports, &archive, &control, &mut job)
            .await
            .unwrap();
        let before = totals(&archive, job.id.clone()).await.unwrap();
        let inputs = crate::import_sevenz::cached_inputs(&job.directory, &job.hash, &mut || Ok(()))
            .unwrap()
            .unwrap();
        let crate::import_sevenz::Input::Scid { games, .. } = &inputs[0] else {
            panic!("Scid fixture")
        };
        let stamp = UNIX_EPOCH + Duration::from_secs(60);
        fs::OpenOptions::new()
            .write(true)
            .open(games)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(stamp))
            .unwrap();
        execute(&control.imports, &archive, &control, &mut job)
            .await
            .unwrap();
        assert_eq!(
            fs::metadata(games).unwrap().modified().unwrap(),
            stamp,
            "Recovery must reuse verified extraction instead of decompressing again"
        );
        assert!(
            crate::import_sevenz::cached_inputs(&job.directory, "different source", &mut || Ok(()))
                .unwrap()
                .is_none()
        );
        let original = fs::read(games).unwrap();
        let mut damaged = original.clone();
        damaged[0] ^= 128;
        fs::write(games, damaged).unwrap();
        assert!(
            crate::import_sevenz::cached_inputs(&job.directory, &job.hash, &mut || Ok(()))
                .unwrap()
                .is_none()
        );
        execute(&control.imports, &archive, &control, &mut job)
            .await
            .unwrap();
        assert_eq!(fs::read(games).unwrap(), original);
        assert_eq!(totals(&archive, job.id.clone()).await.unwrap(), before);
        let marker = job.directory.join("unpacked/entry-cache.json");
        let mut json: serde_json::Value =
            serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
        json["files"][0]["staged"] = "../outside.si5".into();
        fs::write(marker, serde_json::to_vec(&json).unwrap()).unwrap();
        assert!(
            crate::import_sevenz::cached_inputs(&job.directory, &job.hash, &mut || Ok(())).is_err()
        );
    }
    #[tokio::test]
    async fn recovery_accepts_legacy_receipts_and_rejects_gaps_and_missing_source_hashes() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(&mut queue, "resume.pgn", PGN.repeat(129).as_bytes());
        let id = job.id.clone();
        archive
            .run_background(move |db| {
                initialize(db)?;
                db.execute(
                    "INSERT INTO automatic_import_batches VALUES(?1,0,?2)",
                    params![id, r#"{"imported":0,"duplicates":128,"failed":0}"#],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(
            saved_batches(&archive, job.id.clone()).await.unwrap()[0].count(),
            128
        );
        assert!(
            execute(&control.imports, &archive, &control, &mut job)
                .await
                .unwrap_err()
                .contains("no verified source")
        );
        job.hash = format!("{:x}", Sha256::digest(PGN.repeat(129).as_bytes()));
        job.save().unwrap();
        execute(&control.imports, &archive, &control, &mut job)
            .await
            .unwrap();
        let stats = totals(&archive, job.id.clone()).await.unwrap();
        assert_eq!(stats.duplicates, 128);
        assert_eq!(stats.imported, 1);
        let id = job.id.clone();
        archive
            .run_background(move |db| {
                db.execute(
                    "DELETE FROM automatic_import_batches WHERE job=?1 AND batch=0",
                    [id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        assert!(
            saved_batches(&archive, job.id.clone())
                .await
                .unwrap_err()
                .contains("missing batch")
        );
    }
    #[test]
    fn progress_uses_committed_counts_freezes_completion_and_excludes_replayed_receipts() {
        let manager = Manager::default();
        let job = Job {
            id: "telemetry".into(),
            filename: "sample.7z".into(),
            kind: "7z".into(),
            started: 0,
            hash: String::new(),
            directory: PathBuf::new(),
        };
        manager.begin(
            &job,
            Stats {
                imported: 128,
                ..Default::default()
            },
        );
        manager.ready(1000);
        manager.progress.lock().unwrap().started = Some(Instant::now() - Duration::from_secs(2));
        manager.committed(&Stats {
            imported: 64,
            ..Default::default()
        });
        assert_eq!(manager.progress().processed, 128);
        manager.committed(&Stats {
            imported: 240,
            duplicates: 16,
            ..Default::default()
        });
        manager.message("Importing sample.7z");
        let value = manager.progress();
        assert_eq!(value.processed, 256);
        assert_eq!(manager.activity_label(), Some("Importing games"));
        assert!(manager.status_details().contains("Read 256 of 1,000"));
        assert!(manager.status_details().contains("Duplicates 16"));
        assert!(manager.status_details().contains("games/s"));
        assert!((value.games_per_second - 64.0).abs() < 1.0);
        assert!(value.eta_seconds.is_some());
        manager.message("Paused — waiting for idle Companion");
        assert!(manager.progress().eta_seconds.is_none());
        assert_eq!(manager.activity_label(), Some("Import waiting"));
        assert!(manager.status_details().contains("estimate paused"));
        manager.finished(None);
        let rate = manager.progress().games_per_second;
        manager.message("Idle — waiting for files");
        assert_eq!(manager.progress().phase, "complete");
        assert_eq!(manager.activity_label(), None);
        assert_eq!(manager.progress().games_per_second, rate);
    }
    use serde_json::json;
    const PGN: &str =
        "[Event \"Queue\"]\n[White \"Alice\"]\n[Black \"Bob\"]\n\n1. e4 e5 2. Nf3 Nc6 *\n";
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
    #[ignore = "Uses an owner-supplied ZIP via IRONWOOD_TEST_IMPORT_ZIP"]
    async fn owner_zip_import_reproduction() {
        let bytes=fs::read(std::env::var("IRONWOOD_TEST_IMPORT_ZIP").unwrap()).unwrap();
        let (temp,archive,control)=setup();
        let mut queue=Queue::new(temp.0.clone()).unwrap();
        let mut job=claim(&mut queue,"owner.zip",&bytes);
        execute(&control.imports,&archive,&control,&mut job).await.unwrap();
        let stats=totals(&archive,job.id.clone()).await.unwrap();
        eprintln!("Imported {}, duplicates {}, failed {}, short {}",stats.imported,stats.duplicates,stats.failed,stats.discarded);
        assert!(stats.imported>0);
    }
    #[tokio::test]
    async fn sevenz_scid_and_pgn_imports_keep_restart_statistics_and_cleanup() {
        for kind in ["si4", "si5", "pgn"] {
            let (temp, archive, control) = setup();
            let mut queue = Queue::new(temp.0.clone()).unwrap();
            let bytes = fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(format!("../tests/fixtures/scid/sample-{kind}.7z")),
            )
            .unwrap();
            let mut job = claim(&mut queue, "database.7z", &bytes);
            execute(&control.imports, &archive, &control, &mut job)
                .await
                .unwrap();
            let before = totals(&archive, job.id.clone()).await.unwrap();
            assert_eq!(before.imported, 1);
            assert_eq!(before.failed, 0);
            assert_eq!(before.deleted, if kind == "pgn" { 0 } else { 1 });
            assert_eq!(before.duplicates, if kind == "pgn" { 0 } else { 1 });
            let mut recovery = Queue::new(temp.0.clone()).unwrap();
            let mut recovered = recovery.next().unwrap().unwrap();
            execute(&control.imports, &archive, &control, &mut recovered)
                .await
                .unwrap();
            let after = totals(&archive, job.id.clone()).await.unwrap();
            assert_eq!(before, after);
            queue.finish(&recovered, &after, None).unwrap();
            assert!(!recovered.directory.exists());
        }
    }
    #[tokio::test]
    async fn corrupt_sevenz_preserves_original_without_importing_partial_data() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut bytes = include_bytes!("../../tests/fixtures/scid/sample-si5.7z").to_vec();
        bytes[40] ^= 0x80;
        let mut job = claim(&mut queue, "bad.7z", &bytes);
        let result = execute(&control.imports, &archive, &control, &mut job).await;
        assert!(result.is_err());
        let stats = totals(&archive, job.id.clone()).await.unwrap();
        assert_eq!(stats.count(), 0);
        queue.finish(&job, &stats, result.err()).unwrap();
        assert_eq!(
            fs::read(queue.failed.join(format!("{}-bad.7z", job.id))).unwrap(),
            bytes
        );
    }
    #[tokio::test]
    async fn short_games_are_discarded_reported_and_counted_as_processed() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(
            &mut queue,
            "short.pgn",
            b"[White \"Short\"]\n\n1. e4 e5 2. Nf3 *\n",
        );
        let stats = run(&queue, &archive, &control, &mut job).await;
        assert_eq!(stats.discarded, 1);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.imported, 0);
        assert!(!job.source().exists());
        let report =
            fs::read_to_string(queue.reports.join(format!("{}-short.pgn.txt", job.id))).unwrap();
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
                failed: 0,
                discarded: 0,
                deleted: 0
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
                failed: 0,
                discarded: 0,
                deleted: 0
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
    async fn background_indexing_does_not_block_import_admission_but_foreground_requests_do() {
        let (_temp, archive, control) = setup();
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let background = archive.clone();
        let work = tokio::spawn(async move {
            background
                .run_background(move |_| {
                    let _ = entered.send(());
                    wait.recv().unwrap();
                    Ok(())
                })
                .await
        });
        started.await.unwrap();
        assert!(
            idle(&control, &archive),
            "The background indexer may hold the database without stopping import admission"
        );
        let permit = tokio::time::timeout(
            Duration::from_secs(1),
            reserve(&control.imports, &archive, &control),
        )
        .await
        .unwrap()
        .unwrap();
        drop(permit);
        release.send(()).unwrap();
        work.await.unwrap().unwrap();
        let (entered, started) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let foreground = archive.clone();
        let work = tokio::spawn(async move {
            foreground
                .run(move |_| {
                    let _ = entered.send(());
                    wait.recv().unwrap();
                    Ok(())
                })
                .await
        });
        started.await.unwrap();
        assert_eq!(
            wait_reason(&control, &archive),
            Some("an archive request to finish")
        );
        release.send(()).unwrap();
        work.await.unwrap().unwrap();
        assert!(idle(&control, &archive));
    }
    #[tokio::test]
    async fn continuous_indexing_and_folder_import_both_make_progress() {
        let (temp, archive, control) = setup();
        let mut queue = Queue::new(temp.0.clone()).unwrap();
        let mut job = claim(&mut queue, "parallel.pgn", PGN.repeat(1025).as_bytes());
        let background = archive.clone();
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done = stop.clone();
        let work = tokio::spawn(async move {
            let mut rounds = 0;
            while !done.load(std::sync::atomic::Ordering::SeqCst) {
                background
                    .run_background(|db| {
                        db.query_row("SELECT count(*) FROM games", [], |r| r.get::<_, u64>(0))?;
                        Ok(())
                    })
                    .await
                    .unwrap();
                rounds += 1;
                tokio::task::yield_now().await;
            }
            rounds
        });
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            execute(&control.imports, &archive, &control, &mut job),
        )
        .await;
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(work.await.unwrap() > 0);
        result.unwrap().unwrap();
        let stats = totals(&archive, job.id.clone()).await.unwrap();
        assert_eq!(stats.imported, 1);
        assert_eq!(stats.duplicates, 1024);
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
        assert_eq!(
            wait_reason(&control, &archive),
            Some("a chess engine to finish searching")
        );
        let manager = control.imports.clone();
        let worker_archive = archive.clone();
        let worker_control = control.clone();
        let running = tokio::spawn(async move {
            execute(&manager, &worker_archive, &worker_control, &mut job).await
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(!running.is_finished());
        drop(activity);
        assert_eq!(
            wait_reason(&control, &archive),
            Some("the two-second quiet period after engine activity")
        );
        running.await.unwrap().unwrap();
        drop(engine);
        let import = control.imports.reserve().unwrap();
        assert!(control.reserve_update().is_none());
        drop(import);
        let update = control.reserve_update().unwrap();
        assert_eq!(
            wait_reason(&control, &archive),
            Some("engine setup or validation to finish")
        );
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
                    deleted: 0,
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

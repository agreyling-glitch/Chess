//! Publisher-direct, bounded downloads. Browser requests contain catalog IDs only.
use crate::{
    Shared, failure,
    import_queue::{folder, real},
};
use axum::{
    Json, Router,
    extract::{Path as RoutePath, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::io::AsyncWriteExt;
type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Serialize)]
struct Package {
    id: &'static str,
    name: &'static str,
    version: &'static str,
    platform: &'static str,
    url: &'static str,
    sha256: &'static str,
    bytes: u64,
    executable: &'static str,
    license: &'static str,
    source: &'static str,
    networks: &'static str,
}
const PACKAGES: [Package; 5] = [
    Package {
        id: "stockfish-19-windows-x64",
        name: "Stockfish",
        version: "19",
        platform: "windows-x64",
        url: "https://github.com/official-stockfish/Stockfish/releases/download/sf_19/stockfish-windows-x86-64-universal.zip",
        sha256: "3c8bf1f9ea66a09350a40df4f632288285ac206d99f33ab5842c408fc30b48a7",
        bytes: 81431614,
        executable: "stockfish/stockfish-windows-x86-64-universal.exe",
        license: "GPL-3.0",
        source: "https://github.com/official-stockfish/Stockfish/releases/tag/sf_19",
        networks: "Included in the publisher's engine package",
    },
    Package {
        id: "stockfish-19-windows-arm64",
        name: "Stockfish",
        version: "19",
        platform: "windows-arm64",
        url: "https://github.com/official-stockfish/Stockfish/releases/download/sf_19/stockfish-windows-arm64-universal.zip",
        sha256: "8372ad3f0d7276deb2c70f801f541ec7db463219fc6d9c7592864e542aa4f401",
        bytes: 80190536,
        executable: "stockfish/stockfish-windows-arm64-universal.exe",
        license: "GPL-3.0",
        source: "https://github.com/official-stockfish/Stockfish/releases/tag/sf_19",
        networks: "Included in the publisher's engine package",
    },
    Package {
        id: "arasan-26-1-windows-x64", name: "Arasan", version: "26.1", platform: "windows-x64",
        url: "https://github.com/jdart1/arasan-chess/releases/download/v26.1/arasan26.1.zip",
        sha256: "57af9ca4d8a9f57662421c1b54652b246b02cce011f9e2ed26c2252813420690",
        bytes: 29832657, executable: "arasanx-64.exe", license: "MIT",
        source: "https://github.com/jdart1/arasan-chess/releases/tag/v26.1",
        networks: "Includes the publisher's NNUE network and opening book; standard x64/SSE2 build",
    },
    Package {
        id: "weiss-2-0-windows-x64", name: "Weiss", version: "2.0", platform: "windows-x64",
        url: "https://github.com/TerjeKir/weiss/releases/download/v2.0/Weiss-2.0-windows-collection.zip",
        sha256: "2bbc728d8eb9ac1e60c2cc57b843ca43a4887854cc790db8f8fe238873c255ca",
        bytes: 471176, executable: "weiss-nopopcnt.exe", license: "GPL-3.0",
        source: "https://github.com/TerjeKir/weiss/tree/v2.0",
        networks: "Classical evaluation; no neural-network download required; compatible x64 build",
    },
    Package {
        id: "lc0-0-32-1-cpu-windows-x64", name: "Lc0 (CPU)", version: "0.32.1", platform: "windows-x64",
        url: "https://github.com/LeelaChessZero/lc0/releases/download/v0.32.1/lc0-v0.32.1-windows-cpu-dnnl.zip",
        sha256: "b9cfcfbd3dabffbfd452f6e8e087c22273721bef48eba19640b6d92006c142f5",
        bytes: 24001097, executable: "lc0.exe", license: "GPL-3.0-or-later; T1 network: creator permission",
        source: "https://github.com/LeelaChessZero/lc0/releases/tag/v0.32.1",
        networks: "Includes T1-256x10-distilled-swa-2432500 by masterkni6; CPU version, no GPU required",
    },
];
const T1_NAME: &str = "t1-256x10-distilled-swa-2432500.pb.gz";
const T1_PERMISSION: &str = "T1-256x10-distilled-swa-2432500\nCreator: masterkni6\nSource: https://storage.lczero.org/files/networks-contrib/t1-256x10-distilled-swa-2432500.pb.gz\nPermission: https://github.com/orgs/LeelaChessZero/discussions/2430\nThe creator stated on 2026-08-05: I did make that net. Anyone can use it however they want\nThis permission applies to this specific network, not all Lc0 networks.\n";
fn network(p: &Package) -> Option<Package> {
    if p.id != "lc0-0-32-1-cpu-windows-x64" { return None; }
    Some(Package { url: "https://storage.lczero.org/files/networks-contrib/t1-256x10-distilled-swa-2432500.pb.gz",
        sha256: "bc27a6cae8ad36f2b9a80a6ad9dabb0d6fda25b1e7f481a79bc359e14f563406", bytes: 37118673, ..p.clone() })
}
fn total_bytes(p: &Package) -> u64 { p.bytes + network(p).map_or(0,|n|n.bytes) }
pub(crate) fn installed_args(id: &str, executable: &Path) -> Vec<String> {
    if package(id).and_then(network).is_some() {
        vec![format!("--weights={}",executable.parent().unwrap().join(T1_NAME).display()), "--backend=blas".into()]
    } else { Vec::new() }
}
fn random_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "OS randomness unavailable")?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn platform() -> &'static str {
    if cfg!(all(windows, target_arch = "x86_64")) {
        "windows-x64"
    } else if cfg!(all(windows, target_arch = "aarch64")) {
        "windows-arm64"
    } else {
        "unsupported"
    }
}
fn package(id: &str) -> Option<&'static Package> {
    PACKAGES.iter().find(|p| p.id == id)
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Job {
    pub id: String,
    pub phase: String,
    pub downloaded: u64,
    pub total: u64,
    pub speed: f64,
    pub eta_seconds: Option<u64>,
    pub message: String,
    pub engine_id: Option<String>,
    cancelled: bool,
}
#[derive(Clone)]
pub(crate) struct Manager {
    root: Option<PathBuf>,
    jobs: Arc<Mutex<BTreeMap<String, Job>>>,
    running: Arc<AtomicBool>,
    notify: Arc<tokio::sync::Notify>,
}
impl Manager {
    pub(crate) fn owned_install(&self, engine: &crate::config::EngineConfig) -> Option<PathBuf> {
        let id = engine.id.strip_prefix("download-")?;
        let p = package(id)?;
        let root = fs::canonicalize(self.root.as_ref()?).ok()?;
        let directory = root.join(id);
        real(&directory, true).ok()?;
        let directory = fs::canonicalize(directory).ok()?;
        if directory.parent() != Some(root.as_path()) || fs::canonicalize(directory.join(p.executable)).ok()? != engine.executable { return None; }
        real(&directory.join(".ironwood-package.json"), false).ok()?;
        Some(directory)
    }
    pub(crate) fn uninstall(&self, engine: &crate::config::EngineConfig, commit: impl FnOnce() -> Result<()>) -> Result<String> {
        if self.running.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() { return Err("Wait for the current engine download to finish.".into()); }
        let result = (|| {
            let directory = self.owned_install(engine).ok_or("Only engines installed by Companion can have their files removed")?;
            let p = package(engine.id.strip_prefix("download-").unwrap()).unwrap();
            verify_install(p, &directory)?;
            let receipt: Receipt = serde_json::from_slice(&fs::read(directory.join(".ironwood-package.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            // Refuse extra files and links: user-added files must never be deleted.
            fn check_tree(base: &Path, path: &Path, files: &BTreeMap<String,String>) -> Result<()> {
                real(path, true)?;
                for entry in fs::read_dir(path).map_err(|e|e.to_string())? {
                    let path=entry.map_err(|e|e.to_string())?.path();
                    let meta=fs::symlink_metadata(&path).map_err(|e|e.to_string())?;
                    if meta.is_dir() { check_tree(base,&path,files)?; }
                    else { real(&path,false)?;let name=path.strip_prefix(base).map_err(|e|e.to_string())?.to_string_lossy().replace('\\',"/");
                        if name!=".ironwood-package.json" && !files.contains_key(&name) {return Err("The engine folder contains additional files. Remove the registration only to keep them safe.".into());}
                    }
                } Ok(())
            }
            check_tree(&directory,&directory,&receipt.files)?;
            let root=directory.parent().ok_or("Invalid installation directory")?;
            let quarantine=root.join(format!(".uninstall-{}",random_id()?));
            fs::rename(&directory,&quarantine).map_err(|_|"Could not move engine files. Close the engine and retry.")?;
            if let Err(error)=commit(){let _=fs::rename(&quarantine,&directory);return Err(error);}
            // Both absolute targets are verified direct children of the engine root.
            let checked=fs::canonicalize(&quarantine).map_err(|e|e.to_string())?;
            if checked.parent()!=Some(root){return Ok("Engine removed; installation files were retained for review.".into());}
            Ok(if fs::remove_dir_all(&checked).is_ok(){"Engine uninstalled."}else{"Engine removed; some installation files could not be deleted."}.into())
        })();
        self.running.store(false,Ordering::SeqCst);result
    }
    pub fn new(root: Option<PathBuf>) -> Self {
        let mut jobs = BTreeMap::new();
        if let Some(root) = &root {
            for p in &PACKAGES {
                let path = root.join(".downloads").join(p.id).join("job.json");
                if real(&path, false).is_ok() && fs::metadata(&path).is_ok_and(|m| m.len() < 65536)
                {
                    if let Ok(bytes) = fs::read(path) {
                        if let Ok(mut job) = serde_json::from_slice::<Job>(&bytes) {
                            if job.id == p.id {
                                if job.phase != "ready" {
                                    job.phase = "paused".into();
                                    job.speed = 0.0;
                                    job.eta_seconds = None;
                                    job.message="Interrupted by restart. Resume continues from the saved download.".into();
                                    job.cancelled = false;
                                }
                                jobs.insert(job.id.clone(), job);
                            }
                        }
                    }
                }
            }
        }
        Self {
            root,
            jobs: Arc::new(Mutex::new(jobs)),
            running: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }
    fn directory(&self, id: &str) -> Result<PathBuf> {
        if package(id).is_none() {
            return Err("Unknown engine package".into());
        }
        let root = self.root.as_ref().ok_or("Engine directory unavailable")?;
        fs::create_dir_all(root).map_err(|_| "Could not create the engine directory")?;
        real(root, true)?;
        folder(&folder(root, ".downloads")?, id)
    }
    fn save(&self, job: &Job) -> Result<()> {
        let directory = self.directory(&job.id)?;
        let path = directory.join("job.json");
        let temporary = directory.join("job.next");
        if temporary.exists() {
            real(&temporary, false)?;
            fs::remove_file(&temporary).map_err(|e| e.to_string())?;
        }
        crate::config::private_write_new(
            &temporary,
            &serde_json::to_vec(job).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(temporary, path).map_err(|e| e.to_string())
    }
    fn update(
        &self,
        id: &str,
        phase: &str,
        message: &str,
        progress: Option<(u64, f64)>,
    ) -> Result<()> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.get_mut(id).ok_or("Download no longer exists")?;
        job.phase = phase.into();
        job.message = message.into();
        if let Some((done, speed)) = progress {
            job.downloaded = done;
            job.speed = speed;
            job.eta_seconds = if speed > 0.0 {
                Some(((job.total.saturating_sub(done)) as f64 / speed).ceil() as u64)
            } else {
                None
            };
        }
        if phase != "downloading" {
            job.speed = 0.0;
            job.eta_seconds = None;
        }
        self.save(job)
    }
    fn cancelled(&self, id: &str) -> bool {
        self.jobs
            .lock()
            .unwrap()
            .get(id)
            .is_none_or(|j| j.cancelled)
    }
    fn check(&self, id: &str, control: &crate::Control) -> Result<()> {
        if control.stopping() {
            Err("Companion stopped. Resume continues from the saved download.".into())
        } else if self.cancelled(id) {
            Err("Download cancelled. The partial file is kept for Resume.".into())
        } else {
            Ok(())
        }
    }
    pub fn commit_registration(
        &self,
        id: &str,
        operation: impl FnOnce() -> Result<String>,
    ) -> Result<String> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.get_mut(id).ok_or("Download no longer exists")?;
        if job.cancelled {
            return Err("Download cancelled before engine registration".into());
        }
        let engine_id = operation()?;
        job.phase = "ready".into();
        job.message = "Installed and ready to use".into();
        job.engine_id = Some(engine_id.clone());
        job.speed = 0.0;
        job.eta_seconds = None;
        let _ = self.save(job);
        Ok(engine_id)
    }
}
pub(crate) fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/v1/engine-download-catalog", get(catalog))
        .route("/api/v1/engine-downloads", get(jobs).post(start))
        .route("/api/v1/engine-downloads/{id}/cancel", post(cancel))
}
async fn catalog(State(shared): State<Shared>) -> Json<serde_json::Value> {
    Json(
        json!({"platform":platform(),"packages":PACKAGES.iter().filter(|p|p.platform==platform()).map(|p|{let mut value=serde_json::to_value(p).unwrap();value["bytes"]=total_bytes(p).into();if let Some(net)=network(p){value["network_source"]=net.url.into();value["network_permission"]="https://github.com/orgs/LeelaChessZero/discussions/2430".into();}value}).collect::<Vec<_>>(),"available":shared.registry.downloads.root.is_some()}),
    )
}
async fn jobs(State(shared): State<Shared>) -> Json<serde_json::Value> {
    let mut jobs = shared
        .registry
        .downloads
        .jobs
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect::<Vec<_>>();
    let registry = shared.registry.engines.lock().unwrap();
    for job in &mut jobs {
        if job.phase == "ready"
            && !registry
                .iter()
                .any(|e| Some(&e.id) == job.engine_id.as_ref())
        {
            job.phase = "paused".into();
            job.message =
                "Engine configuration changed. Resume to verify and register the installation."
                    .into();
        }
    }
    Json(json!({"jobs":jobs}))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Start {
    package: String,
}
async fn start(State(shared): State<Shared>, Json(command): Json<Start>) -> Response {
    let Some(p) = package(&command.package).filter(|p| p.platform == platform()) else {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Choose a supported package from the engine catalog",
        );
    };
    let manager = shared.registry.downloads.clone();
    if manager.root.is_none() {
        return failure(
            StatusCode::CONFLICT,
            "configuration",
            "Engine downloads require a persistent Companion configuration",
        );
    }
    if manager
        .running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return failure(
            StatusCode::CONFLICT,
            "busy",
            "Another engine download is running",
        );
    }
    let previous = manager.jobs.lock().unwrap().get(p.id).cloned();
    if previous.as_ref().is_some_and(|job| {
        job.phase == "ready"
            && shared
                .registry
                .engines
                .lock()
                .unwrap()
                .iter()
                .any(|e| Some(&e.id) == job.engine_id.as_ref())
    }) {
        manager.running.store(false, Ordering::SeqCst);
        return Json(previous.unwrap()).into_response();
    }
    let job = Job {
        id: p.id.into(),
        phase: "queued".into(),
        downloaded: previous.map_or(0, |j| j.downloaded),
        total: total_bytes(p),
        speed: 0.0,
        eta_seconds: None,
        message: "Preparing publisher-direct download…".into(),
        engine_id: None,
        cancelled: false,
    };
    if let Err(e) = manager.save(&job) {
        manager.running.store(false, Ordering::SeqCst);
        return failure(StatusCode::CONFLICT, "download_unavailable", &e);
    }
    manager
        .jobs
        .lock()
        .unwrap()
        .insert(p.id.into(), job.clone());
    tokio::spawn(async move {
        let result = run(&shared, p).await;
        if let Err(message) = result {
            let phase = if manager.cancelled(p.id) {
                "cancelled"
            } else {
                "paused"
            };
            let _ = manager.update(p.id, phase, &message, None);
        }
        manager.running.store(false, Ordering::SeqCst);
    });
    (StatusCode::ACCEPTED, Json(job)).into_response()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
async fn cancel(
    State(shared): State<Shared>,
    RoutePath(id): RoutePath<String>,
    Json(_): Json<Empty>,
) -> Response {
    let manager = &shared.registry.downloads;
    let mut jobs = manager.jobs.lock().unwrap();
    let Some(job) = jobs.get_mut(&id) else {
        return failure(StatusCode::NOT_FOUND, "not_found", "Download not found");
    };
    if job.phase != "ready" {
        job.cancelled = true;
        job.message = "Cancelling download…".into();
        let _ = manager.save(job);
        manager.notify.notify_waiters();
    }
    Json(job.clone()).into_response()
}
fn http_client() -> Result<reqwest::Client> {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client.clone());
    }
    let client = reqwest::Client::builder()
        .https_only(true)
        .user_agent("Ironwood-Companion/0.1.3")
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            if attempt.previous().len() < 6
                && url.scheme() == "https"
                && matches!(
                    url.host_str(),
                    Some(
                        "github.com"
                            | "release-assets.githubusercontent.com"
                            | "objects.githubusercontent.com"
                            | "storage.lczero.org"
                    )
                )
                && url.port_or_known_default() == Some(443)
            {
                attempt.follow()
            } else {
                attempt.error("Publisher redirect is not approved")
            }
        }))
        .build()
        .map_err(|_| "Could not initialize HTTPS downloads")?;
    let _ = CLIENT.set(client.clone());
    Ok(client)
}
#[cfg(test)]
async fn transfer(
    manager: &Manager,
    p: &Package,
    path: &Path,
    client: &reqwest::Client,
    control: &crate::Control,
) -> Result<()> {
    transfer_part(manager,p,path,client,control,0).await
}
async fn transfer_part(manager:&Manager,p:&Package,path:&Path,client:&reqwest::Client,control:&crate::Control,offset:u64)->Result<()> {
    use tokio::fs::OpenOptions;
    let mut done = if path.exists() {
        real(path, false)?;
        fs::metadata(path).map_err(|e| e.to_string())?.len()
    } else {
        0
    };
    if done > p.bytes {
        return Err("Partial download exceeds the publisher package size".into());
    }
    if done == p.bytes {
        return Ok(());
    }
    manager.check(p.id, control)?;
    let mut request = client.get(p.url).header("Accept-Encoding", "identity");
    if done > 0 {
        request = request.header("Range", format!("bytes={done}-"));
    }
    let mut response = tokio::select! {_=control.cancelled()=>return Err("Companion stopped; resume the saved download".into()),_=manager.notify.notified()=>return Err("Download cancelled; resume to continue".into()),response=request.send()=>response.map_err(|_|"Could not reach the publisher. Resume to retry.")?};
    if response.status() == reqwest::StatusCode::OK {
        done = 0;
    } else if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
        let expected = format!("bytes {done}-{}/{}", p.bytes - 1, p.bytes);
        if response
            .headers()
            .get("content-range")
            .and_then(|v| v.to_str().ok())
            != Some(expected.as_str())
        {
            return Err(
                "Publisher returned an unexpected download range; partial data was kept".into(),
            );
        }
    } else {
        return Err(format!(
            "Publisher returned HTTP {}. Resume to retry.",
            response.status().as_u16()
        ));
    }
    if response
        .content_length()
        .is_some_and(|n| n != p.bytes - done)
    {
        return Err("Publisher package size changed; download was not installed".into());
    }
    let mut output = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(done == 0)
        .append(done > 0)
        .open(path)
        .await
        .map_err(|_| "Could not create the partial download")?;
    let baseline = done;
    let started = Instant::now();
    let mut last = Instant::now() - Duration::from_secs(2);
    manager.update(
        p.id,
        "downloading",
        if offset>0 {"Downloading the T1 neural network…"} else {"Downloading directly from the publisher…"},
        Some((offset + done, 0.0)),
    )?;
    loop {
        manager.check(p.id, control)?;
        let chunk = tokio::select! {_=control.cancelled()=>return Err("Companion stopped; resume the saved download".into()),_=manager.notify.notified()=>return Err("Download cancelled; resume to continue".into()),chunk=response.chunk()=>chunk.map_err(|_|"Connection interrupted. Resume continues from the saved download.")?};
        let Some(chunk) = chunk else {
            break;
        };
        if done + chunk.len() as u64 > p.bytes {
            return Err("Publisher response exceeded its declared size".into());
        }
        output
            .write_all(&chunk)
            .await
            .map_err(|_| "Could not save download; check disk space")?;
        done += chunk.len() as u64;
        if last.elapsed() >= Duration::from_millis(500) {
            manager.update(
                p.id,
                "downloading",
                if offset>0 {"Downloading the T1 neural network…"} else {"Downloading directly from the publisher…"},
                Some((
                    offset + done,
                    (done - baseline) as f64 / started.elapsed().as_secs_f64().max(0.001),
                )),
            )?;
            last = Instant::now();
        }
    }
    output
        .sync_all()
        .await
        .map_err(|_| "Could not finish saving download")?;
    manager.update(
        p.id,
        "verifying",
        "Verifying the publisher checksum…",
        Some((offset + done, 0.0)),
    )?;
    if done != p.bytes {
        return Err("Download was interrupted before completion. Resume to continue.".into());
    }
    Ok(())
}
fn hash(path: &Path) -> Result<String> {
    real(path, false)?;
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
#[derive(Serialize, Deserialize)]
struct Receipt {
    package: String,
    sha256: String,
    files: BTreeMap<String, String>,
}
fn extract(
    p: &Package,
    path: &Path,
    destination: &Path,
    check: impl Fn() -> Result<()>,
) -> Result<Receipt> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if archive.len() > 4096 {
        return Err("Engine archive has too many entries".into());
    }
    let mut seen = HashSet::new();
    let mut total = 0u64;
    let mut files = BTreeMap::new();
    for i in 0..archive.len() {
        check()?;
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().map_err(|e| e.to_string())?.replace('\\', "/");
        if name.starts_with('/')
            || name.contains(':')
            || name.chars().any(char::is_control)
            || name
                .split('/')
                .any(|s| s == ".." || s == "." || s.ends_with(' ') || s.ends_with('.'))
            || !seen.insert(name.to_lowercase())
            || entry
                .unix_mode()
                .is_some_and(|m| !matches!(m & 0xf000, 0 | 0x4000 | 0x8000))
        {
            return Err("Unsafe engine archive entry".into());
        }
        if entry.enclosed_name().is_none() {
            return Err("Unsafe engine archive path".into());
        }
        let target = destination.join(&name);
        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Engine archive size overflow")?;
        if total > 512 * 1024 * 1024 || entry.size() > 256 * 1024 * 1024 {
            return Err("Engine archive exceeds its extraction limits".into());
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // The publisher's default net has separate provenance; install only our explicitly permitted T1 net.
        if network(p).is_some() && name == "791556.pb.gz" { continue; }
        let expected = entry.size();
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| e.to_string())?;
        let mut count = 0;
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            check()?;
            let n = entry.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > expected {
                return Err("Engine archive entry exceeded its size".into());
            }
            output.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            digest.update(&buffer[..n]);
        }
        if count != expected {
            return Err("Truncated engine archive entry".into());
        }
        output.sync_all().map_err(|e| e.to_string())?;
        files.insert(name, format!("{:x}", digest.finalize()));
    }
    if !files.contains_key(p.executable) {
        return Err("The publisher package does not contain the expected engine".into());
    }
    Ok(Receipt {
        package: p.id.into(),
        sha256: p.sha256.into(),
        files,
    })
}
fn verify_install(p: &Package, directory: &Path) -> Result<()> {
    real(directory, true)?;
    let receipt_path = directory.join(".ironwood-package.json");
    real(&receipt_path, false)?;
    if fs::metadata(&receipt_path)
        .map_err(|e| e.to_string())?
        .len()
        > 1024 * 1024
    {
        return Err("Invalid installation receipt".into());
    }
    let receipt: Receipt =
        serde_json::from_slice(&fs::read(receipt_path).map_err(|e| e.to_string())?)
            .map_err(|_| "Invalid installation receipt")?;
    if receipt.package != p.id
        || receipt.sha256 != p.sha256
        || !receipt.files.contains_key(p.executable)
    {
        return Err("Installation does not match the publisher package".into());
    }
    if let Some(net)=network(p) {
        if receipt.files.get(T1_NAME).map(String::as_str)!=Some(net.sha256) || receipt.files.contains_key("791556.pb.gz") {
            return Err("Installation must contain the verified T1 network only".into());
        }
    }
    if receipt.files.len() > 4096 {
        return Err("Invalid installation receipt".into());
    }
    for (name, expected) in receipt.files {
        let name = name.replace('\\', "/");
        if name.contains(':')
            || name.starts_with('/')
            || name.split('/').any(|s| s == ".." || s == ".")
        {
            return Err("Invalid installation receipt path".into());
        }
        if hash(&directory.join(name))? != expected {
            return Err("Installed files changed. Keep this installation for review and use a clean engine folder.".into());
        }
    }
    Ok(())
}
fn clear_incomplete_stages(directory: &Path) -> Result<()> {
    let parent = fs::canonicalize(directory).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(&parent).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name
            .strip_prefix("stage-")
            .is_some_and(|n| n.len() == 32 && n.bytes().all(|c| c.is_ascii_hexdigit()))
        {
            real(&entry.path(), true)?;
            let resolved = fs::canonicalize(entry.path()).map_err(|e| e.to_string())?;
            if resolved.parent() != Some(parent.as_path()) {
                return Err(
                    "Refusing to remove a staging directory outside this download job".into(),
                );
            }
            fs::remove_dir_all(resolved).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
async fn transfer_verified(manager:&Manager,p:&Package,archive:&Path,client:&reqwest::Client,control:&crate::Control,offset:u64)->Result<()> {
    for attempt in 0..3 {
        match transfer_part(manager, p, archive, client, control, offset).await {
            Ok(()) => break,
            Err(message) => {
                manager.check(p.id, control)?;
                if attempt == 2 {
                    return Err(message);
                }
                let seconds = 1u64 << attempt;
                manager.update(
                    p.id,
                    "retrying",
                    &format!("Connection interrupted. Retrying in {seconds} seconds…"),
                    None,
                )?;
                tokio::select! {_=control.cancelled()=>return Err("Companion stopped; resume after restart".into()),_=manager.notify.notified()=>return Err("Download cancelled; partial file kept".into()),_=tokio::time::sleep(Duration::from_secs(seconds))=>{}}
            }
        }
    }
    manager.check(p.id, control)?;
    let archive_hash = {
        let path = archive.to_path_buf();
        tokio::task::spawn_blocking(move || hash(&path))
            .await
            .map_err(|_| "Checksum worker failed")??
    };
    if archive_hash != p.sha256 {
        fs::remove_file(&archive)
            .map_err(|_| "Checksum failed; could not clear the corrupt partial file")?;
        return Err("Publisher checksum did not match. The corrupt download was discarded; retry starts fresh.".into());
    }
    Ok(())
}
async fn run(shared: &Shared, p: &'static Package) -> Result<()> {
    let manager = &shared.registry.downloads;
    let directory = manager.directory(p.id)?;
    let archive = directory.join("package.zip.part");
    let client = http_client()?;
    transfer_verified(manager,p,&archive,&client,&shared.control,0).await?;
    let root = manager
        .root
        .as_ref()
        .ok_or("Engine directory unavailable")?;
    let installed = root.join(p.id);
    if !installed.exists() {
        clear_incomplete_stages(&directory)?;
        manager.update(p.id, "extracting", "Extracting verified files…", None)?;
        let stage = directory.join(format!("stage-{}", random_id()?));
        fs::create_dir(&stage).map_err(|e| e.to_string())?;
        let p2 = p.clone();
        let path = archive.clone();
        let stage2 = stage.clone();
        let manager2 = manager.clone();
        let control = shared.control.clone();
        let mut receipt = match tokio::task::spawn_blocking(move || {
            extract(&p2, &path, &stage2, || manager2.check(p2.id, &control))
        })
        .await
        .map_err(|_| "Extraction worker failed")?
        {
            Ok(receipt) => receipt,
            Err(error) => {
                let _ = clear_incomplete_stages(&directory);
                return Err(error);
            }
        };
        if let Some(net)=network(p) {
            let source=directory.join("network.pb.gz.part");
            transfer_verified(manager,&net,&source,&client,&shared.control,p.bytes).await?;
            fs::copy(&source,stage.join(T1_NAME)).map_err(|e|e.to_string())?;
            receipt.files.insert(T1_NAME.into(),net.sha256.into());
            crate::config::private_write_new(&stage.join("NETWORK-PERMISSION.txt"),T1_PERMISSION.as_bytes()).map_err(|e|e.to_string())?;
            receipt.files.insert("NETWORK-PERMISSION.txt".into(),hash(&stage.join("NETWORK-PERMISSION.txt"))?);
        }
        crate::config::private_write_new(
            &stage.join(".ironwood-package.json"),
            &serde_json::to_vec(&receipt).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        manager.check(p.id, &shared.control)?;
        fs::rename(stage, &installed).map_err(
            |_| "Could not publish the engine installation; existing files were not replaced",
        )?;
    }
    manager.update(
        p.id,
        "validating",
        "Checking the installed engine's UCI response…",
        None,
    )?;
    {
        let path = installed.clone();
        let p = p.clone();
        tokio::task::spawn_blocking(move || verify_install(&p, &path))
            .await
            .map_err(|_| "Installation verification failed")??;
    }
    manager.check(p.id, &shared.control)?;
    crate::registration::register_downloaded(
        shared,
        p.id,
        installed.join(p.executable),
        &format!("{} {}", p.name, p.version),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lc0_installs_only_selected_network_and_launches_with_explicit_weights() {
        let (root,_,_)=setup();let p=package("lc0-0-32-1-cpu-windows-x64").unwrap();
        assert_eq!(total_bytes(p),61119770);
        let archive=root.join("fixture.zip");
        let mut zip=zip::ZipWriter::new(fs::File::create(&archive).unwrap());
        for (name,contents) in [("lc0.exe","engine"),("COPYING","GPL"),("791556.pb.gz","unselected net")] {
            zip.start_file(name,zip::write::SimpleFileOptions::default()).unwrap();zip.write_all(contents.as_bytes()).unwrap();
        }
        zip.finish().unwrap();let stage=root.join("stage");fs::create_dir(&stage).unwrap();
        let receipt=extract(p,&archive,&stage,||Ok(())).unwrap();
        assert!(!stage.join("791556.pb.gz").exists());assert!(!receipt.files.contains_key("791556.pb.gz"));
        assert!(stage.join("COPYING").exists());
        let args=installed_args(p.id,&stage.join(p.executable));
        assert!(args[0].ends_with(T1_NAME));assert_eq!(args[1],"--backend=blas");
        assert!(verify_install(p,&stage).is_err(),"an incomplete network installation cannot pass verification");
        cleanup(&root);
    }
    #[test]
    fn uninstall_checks_ownership_extra_files_and_rolls_back_failed_commit() {
        let (root,manager,_)=setup();let p=&PACKAGES[0];let directory=root.join(p.id);
        fs::create_dir_all(directory.join(Path::new(p.executable).parent().unwrap())).unwrap();
        fs::write(directory.join(p.executable),b"fixture").unwrap();
        let receipt=Receipt{package:p.id.into(),sha256:p.sha256.into(),files:BTreeMap::from([(p.executable.into(),hash(&directory.join(p.executable)).unwrap())])};
        fs::write(directory.join(".ironwood-package.json"),serde_json::to_vec(&receipt).unwrap()).unwrap();
        let engine=crate::config::EngineConfig{id:format!("download-{}",p.id),label:"test".into(),executable:fs::canonicalize(directory.join(p.executable)).unwrap(),args:vec![],options:Default::default()};
        assert!(manager.owned_install(&engine).is_some());
        fs::write(directory.join("my-file"),b"keep").unwrap();
        assert!(manager.uninstall(&engine,||panic!("must not commit with extra files")).is_err());
        fs::remove_file(directory.join("my-file")).unwrap();
        assert!(manager.uninstall(&engine,||Err("conflict".into())).is_err());assert!(directory.exists());
        let mut manual=engine.clone();manual.id="manual".into();assert!(manager.uninstall(&manual,||panic!("manual engine")).is_err());
        assert_eq!(manager.uninstall(&engine,||Ok(())).unwrap(),"Engine uninstalled.");assert!(!directory.exists());cleanup(&root);
    }
    fn setup() -> (PathBuf, Manager, crate::Control) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../output")
            .join(format!("engine-download-test-{}", random_id().unwrap()));
        fs::create_dir_all(&root).unwrap();
        let manager = Manager::new(Some(root.clone()));
        let p = &PACKAGES[0];
        manager.jobs.lock().unwrap().insert(
            p.id.into(),
            Job {
                id: p.id.into(),
                phase: "queued".into(),
                downloaded: 0,
                total: 6,
                speed: 0.0,
                eta_seconds: None,
                message: String::new(),
                engine_id: None,
                cancelled: false,
            },
        );
        let mut config = crate::Config::defaults(false);
        config.archive = None;
        config.token = "a".repeat(64);
        let (_, control) = crate::app_with_control(config).unwrap();
        (root, manager, control)
    }
    fn cleanup(root: &Path) {
        let resolved = fs::canonicalize(root).unwrap();
        let parent =
            fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../output")).unwrap();
        assert!(resolved.starts_with(parent));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("engine-download-test-")
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    async fn range_resume_and_ignored_range_never_append_duplicate_bytes() {
        for ignore in [false, true] {
            let (root, manager, control) = setup();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let app = Router::new().route(
                "/package",
                get(move |headers: axum::http::HeaderMap| async move {
                    assert_eq!(headers.get("range").unwrap(), "bytes=3-");
                    if ignore {
                        (StatusCode::OK, [("content-range", "bytes 0-5/6")], "abcdef")
                    } else {
                        (
                            StatusCode::PARTIAL_CONTENT,
                            [("content-range", "bytes 3-5/6")],
                            "def",
                        )
                    }
                }),
            );
            let server = tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            let mut p = PACKAGES[0].clone();
            p.url = Box::leak(format!("http://{address}/package").into_boxed_str());
            p.bytes = 6;
            let path = manager.directory(p.id).unwrap().join("package.zip.part");
            fs::write(&path, b"abc").unwrap();
            transfer(&manager, &p, &path, &reqwest::Client::new(), &control)
                .await
                .unwrap();
            assert_eq!(fs::read(&path).unwrap(), b"abcdef");
            server.abort();
            cleanup(&root);
        }
    }
    #[tokio::test]
    async fn wrong_ranges_are_rejected_without_changing_the_partial_file() {
        let (root, manager, control) = setup();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().route(
                    "/package",
                    get(|| async {
                        (
                            StatusCode::PARTIAL_CONTENT,
                            [("content-range", "bytes 2-4/6")],
                            "cde",
                        )
                    }),
                ),
            )
            .await
            .unwrap();
        });
        let mut p = PACKAGES[0].clone();
        p.url = Box::leak(format!("http://{address}/package").into_boxed_str());
        p.bytes = 6;
        let path = manager.directory(p.id).unwrap().join("package.zip.part");
        fs::write(&path, b"abc").unwrap();
        assert!(
            transfer(&manager, &p, &path, &reqwest::Client::new(), &control)
                .await
                .unwrap_err()
                .contains("range")
        );
        assert_eq!(fs::read(&path).unwrap(), b"abc");
        server.abort();
        cleanup(&root);
    }
    #[test]
    fn restart_restores_paused_jobs_and_cancel_excludes_registration_commit() {
        let (root, manager, _) = setup();
        let p = &PACKAGES[0];
        manager
            .update(p.id, "downloading", "Downloading", Some((3, 1.0)))
            .unwrap();
        let restored = Manager::new(Some(root.clone()));
        let job = restored.jobs.lock().unwrap().get(p.id).cloned().unwrap();
        assert_eq!(job.phase, "paused");
        assert_eq!(job.downloaded, 3);
        assert_eq!(job.speed, 0.0);
        restored
            .jobs
            .lock()
            .unwrap()
            .get_mut(p.id)
            .unwrap()
            .cancelled = true;
        assert!(
            restored
                .commit_registration(p.id, || panic!("cancelled jobs cannot register"))
                .is_err()
        );
        cleanup(&root);
    }
    #[test]
    fn extraction_rejects_traversal_and_retains_original_license_files() {
        let (root, manager, _) = setup();
        let p = &PACKAGES[0];
        let path = manager.directory(p.id).unwrap().join("test.zip");
        for unsafe_path in [false, true] {
            let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
            zip.start_file(
                if unsafe_path {
                    "../outside.exe"
                } else {
                    p.executable
                },
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(b"fixture engine").unwrap();
            zip.start_file(
                "stockfish/Copying.txt",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
            zip.write_all(b"GPL license retained").unwrap();
            zip.finish().unwrap();
            let stage =
                manager
                    .directory(p.id)
                    .unwrap()
                    .join(if unsafe_path { "bad" } else { "good" });
            fs::create_dir(&stage).unwrap();
            let result = extract(p, &path, &stage, || Ok(()));
            if unsafe_path {
                assert!(result.is_err());
                assert!(!root.join("outside.exe").exists());
            } else {
                let receipt = result.unwrap();
                assert!(receipt.files.contains_key(p.executable));
                assert_eq!(
                    fs::read(stage.join("stockfish/Copying.txt")).unwrap(),
                    b"GPL license retained"
                );
            }
        }
        cleanup(&root);
    }
}

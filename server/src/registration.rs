//! Authenticated engine browsing, UCI validation and explicit persistence.
use crate::{
    Shared,
    config::{self, EngineConfig},
    engine::Engine,
    failure,
};
use axum::{
    Json, Router,
    extract::{Path as RoutePath, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct Validated {
    engine: EngineConfig,
    original: Vec<u8>,
}
#[derive(Clone)]
pub(crate) struct Registry {
    pub downloads: crate::engine_downloads::Manager,
    pub engines: Arc<Mutex<Vec<EngineConfig>>>,
    path: Option<PathBuf>,
    jobs: Arc<Mutex<HashMap<String, Value>>>,
    validated: Arc<Mutex<HashMap<String, Validated>>>,
    browse_root: Option<PathBuf>,
}
impl Registry {
    pub(crate) fn configuration_path(&self)->Option<PathBuf>{self.path.clone()}
    pub fn new(engines: Vec<EngineConfig>, path: Option<PathBuf>) -> Self {
        let browse_root = std::env::var_os("IRONWOOD_ENGINE_DIRECTORY")
            .map(PathBuf::from)
            .or_else(|| {
                path.as_ref()
                    .and_then(|p| p.parent())
                    .map(|p| p.join("engines"))
            });
        Self {
            downloads: crate::engine_downloads::Manager::new(browse_root.clone()),
            engines: Arc::new(Mutex::new(engines)),
            path,
            jobs: Arc::default(),
            validated: Arc::default(),
            browse_root,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Proposal {
    #[serde(default)]
    label: String,
    executable: PathBuf,
    #[serde(default)]
    args: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    #[serde(default)]
    label: String,
}
#[derive(Deserialize)]
struct Browse {
    path: Option<PathBuf>,
}
fn valid_request(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn safe_text(value: &str) -> bool {
    !value.chars().any(char::is_control)
}
fn same_launch(engine: &EngineConfig, executable: &Path, args: &[String]) -> bool {
    engine.args == args
        && fs::canonicalize(&engine.executable).unwrap_or_else(|_| engine.executable.clone())
            == executable
}
fn read_small(path: &Path) -> Result<Vec<u8>, config::Error> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 65536 {
        return Err("Invalid configuration file".into());
    }
    Ok(fs::read(path)?)
}
fn executable(path: &Path) -> bool {
    #[cfg(windows)]
    {
        path.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
    }
}
fn permitted(registry: &Registry, path: &Path) -> bool {
    registry
        .browse_root
        .as_ref()
        .is_some_and(|root| fs::canonicalize(root).is_ok_and(|root| path.strip_prefix(root).is_ok_and(|relative| !relative.components().next().is_some_and(|part|part.as_os_str().to_string_lossy().eq_ignore_ascii_case(".downloads")))))
}
pub(crate) fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/v1/engine-files", get(browse))
        .route("/api/v1/engine-registrations", post(propose))
        .route("/api/v1/engine-registrations/{id}", get(status))
        .route("/api/v1/engine-registrations/{id}/save", post(save))
        .route("/api/v1/engine-registrations/{id}/cancel", post(cancel))
}
async fn browse(State(shared): State<Shared>, Query(query): Query<Browse>) -> Response {
    let registry = &shared.registry;
    let initial = registry.browse_root.clone();
    let Some(path) = query
        .path
        .or(initial)
        .and_then(|p| fs::canonicalize(p).ok())
    else {
        return failure(
            StatusCode::NOT_FOUND,
            "not_found",
            "The engines folder is unavailable. Restart the updated Companion to create it.",
        );
    };
    if !permitted(registry, &path) {
        return failure(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Choose an engine inside the engines folder",
        );
    }
    let Ok(directory) = fs::read_dir(&path) else {
        return failure(
            StatusCode::BAD_REQUEST,
            "unavailable",
            "Cannot read this folder",
        );
    };
    let mut entries = Vec::new();
    for (index, entry) in directory.take(4097).enumerate() {
        if index >= 4096 || entries.len() >= 2000 {
            return failure(
                StatusCode::BAD_REQUEST,
                "too_many_files",
                "This folder is too large. Open a smaller folder using its path.",
            );
        }
        let Ok(entry) = entry else { continue };
        let Ok(target) = fs::canonicalize(entry.path()) else {
            continue;
        };
        if !permitted(registry, &target) {
            continue;
        }
        let directory = target.is_dir();
        if directory || (target.is_file() && executable(&target)) {
            entries.push(json!({"name":entry.file_name().to_string_lossy(),"path":target,"directory":directory}));
        }
    }
    entries.sort_by(|a, b| {
        b["directory"]
            .as_bool()
            .cmp(&a["directory"].as_bool())
            .then_with(|| {
                a["name"]
                    .as_str()
                    .unwrap_or("")
                    .to_lowercase()
                    .cmp(&b["name"].as_str().unwrap_or("").to_lowercase())
            })
    });
    let parent = path.parent().filter(|p| permitted(registry, p));
    let root = registry
        .browse_root
        .as_ref()
        .and_then(|p| fs::canonicalize(p).ok());
    let relative = root.as_ref().and_then(|r| path.strip_prefix(r).ok());
    let folder = relative.map(|p| {
        if p.as_os_str().is_empty() {
            PathBuf::from("engines")
        } else {
            PathBuf::from("engines").join(p)
        }
    });
    Json(json!({"path":path,"parent":parent,"root":root,"folder":folder,"entries":entries}))
        .into_response()
}
async fn cancel(State(shared): State<Shared>, RoutePath(id): RoutePath<String>) -> Response {
    let mut jobs = shared.registry.jobs.lock().unwrap();
    let Some(value) = jobs.get_mut(&id) else {
        return failure(StatusCode::NOT_FOUND, "not_found", "Request not found");
    };
    if value["status"] != "ready" {
        *value = json!({"request":id,"status":"cancelled"});
    }
    shared.registry.validated.lock().unwrap().remove(&id);
    Json(value.clone()).into_response()
}
async fn status(State(shared): State<Shared>, RoutePath(id): RoutePath<String>) -> Response {
    if !valid_request(&id) {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Invalid request ID",
        );
    }
    match shared.registry.jobs.lock().unwrap().get(&id) {
        Some(value) => Json(value.clone()).into_response(),
        None => failure(
            StatusCode::NOT_FOUND,
            "not_found",
            "Engine validation not found; select the engine again",
        ),
    }
}
async fn propose(State(shared): State<Shared>, Json(proposal): Json<Proposal>) -> Response {
    let Some(path) = shared.registry.path.clone() else {
        return failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
            "Engine registration requires a writable configuration",
        );
    };
    if shared.control.stopping() {
        return failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "stopping",
            "Server is stopping",
        );
    }
    let Ok(permit) = shared.control.registrations.clone().try_acquire_owned() else {
        return failure(
            StatusCode::CONFLICT,
            "busy",
            "Another engine is being validated. Try again shortly.",
        );
    };
    if !proposal.executable.is_absolute()
        || !safe_text(&proposal.label)
        || proposal
            .executable
            .to_str()
            .is_none_or(|p| p.len() > 4096 || !safe_text(p))
        || proposal.args.iter().any(|a| !safe_text(a))
    {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Choose an executable and valid name and arguments",
        );
    }
    let Ok(executable) = fs::canonicalize(&proposal.executable) else {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Executable does not exist",
        );
    };
    if !executable.is_file() || !permitted(&shared.registry, &executable) {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Choose an executable inside the engine folder",
        );
    }
    if shared.registry.engines.lock().unwrap().iter()
        .any(|engine| same_launch(engine, &executable, &proposal.args)) {
        return failure(StatusCode::CONFLICT, "already_registered",
            "This engine is already added. Select it in Engine Settings instead.");
    }
    let mut random = [0u8; 16];
    if getrandom::fill(&mut random).is_err() {
        return failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "unavailable",
            "OS randomness unavailable",
        );
    }
    let id: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let label = if proposal.label.trim().is_empty() {
        executable
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .take(128)
            .collect()
    } else {
        proposal.label
    };
    let config = EngineConfig {
        id: format!("engine-{id}"),
        label,
        executable,
        args: proposal.args,
        options: Default::default(),
    };
    let mut check = shared.config.clone();
    check.engines = shared.registry.engines.lock().unwrap().clone();
    check.engines.push(config.clone());
    if check.validate().is_err() {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Invalid engine settings; at most 64 engines are supported",
        );
    }
    let Ok(original) = read_small(&path) else {
        return failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "configuration",
            "Cannot read configuration",
        );
    };
    let value = json!({"request":id,"status":"validating"});
    {
        let mut jobs = shared.registry.jobs.lock().unwrap();
        if jobs.len() >= 16 {
            jobs.retain(|_, v| v["status"] == "validating");
            shared.registry.validated.lock().unwrap().clear();
        }
        jobs.insert(id.clone(), value.clone());
    }
    tokio::spawn(async move {
        let _permit = permit;
        let result = validate(&shared, &config).await;
        let mut jobs = shared.registry.jobs.lock().unwrap();
        if jobs.get(&id).is_none_or(|v| v["status"] == "cancelled") {
            return;
        }
        let value = match result {
            Ok(name) => {
                let mut engine = config;
                engine.label = name.clone();
                shared
                    .registry
                    .validated
                    .lock()
                    .unwrap()
                    .insert(id.clone(), Validated { engine, original });
                json!({"request":id,"status":"validated","name":name})
            }
            Err(message) => json!({"request":id,"status":"failed","message":message}),
        };
        jobs.insert(id, value);
    });
    (StatusCode::ACCEPTED, Json(value)).into_response()
}
async fn validate(shared: &Shared, config: &EngineConfig) -> Result<String, &'static str> {
    let permit = shared
        .engines
        .clone()
        .try_acquire_owned()
        .map_err(|_| "Engine process limit reached. Close another engine and try again.")?;
    let mut engine = Engine::spawn(
        config,
        shared.config.engine_limits.clone(),
        permit,
        &shared.config.tablebase_directories,
    )?;
    let result = tokio::select! { _=shared.control.cancelled()=>Err("Server stopped during validation"), result=engine.handshake()=>result };
    let mut name = engine.details.name.clone();
    while name.len() > 128 {
        name.pop();
    }
    engine.close().await;
    result?;
    Ok(if name.trim().is_empty() || !safe_text(&name) {
        config.label.clone()
    } else {
        name
    })
}
pub(crate) async fn register_downloaded(shared:&Shared,id:&str,path:PathBuf,label:&str)->Result<String,String>{
    let _permit=shared.control.registrations.clone().try_acquire_owned().map_err(|_|"Another engine is being validated; resume this download when it finishes.")?;
    let path=fs::canonicalize(path).map_err(|_|"Installed executable is unavailable")?;
    if !permitted(&shared.registry,&path){return Err("Installed engine is outside the engine directory".into());}
    let config=EngineConfig{id:format!("download-{id}"),label:label.into(),args:crate::engine_downloads::installed_args(id,&path),executable:path,options:Default::default()};
    if let Some(existing)=shared.registry.engines.lock().unwrap().iter().find(|e|same_launch(e,&config.executable,&config.args)).map(|e|e.id.clone()){
        return shared.registry.downloads.commit_registration(id,||Ok(existing));
    }
    let original=read_small(shared.registry.path.as_ref().ok_or("Configuration unavailable")?).map_err(|_|"Could not read engine configuration")?;
    let name=validate(shared,&config).await.map_err(str::to_owned)?;
    let mut engine=config;engine.label=name;
    shared.registry.downloads.commit_registration(id,||{
        if shared.control.stopping(){return Err("Companion is stopping; resume after restart".into());}
        let engine_id=engine.id.clone();persist(shared,id,&Validated{engine,original}).map_err(str::to_owned)?;Ok(engine_id)
    })
}
async fn save(
    State(shared): State<Shared>,
    RoutePath(id): RoutePath<String>,
    Json(save): Json<Save>,
) -> Response {
    let Ok(_permit) = shared.control.registrations.clone().try_acquire_owned() else {
        return failure(
            StatusCode::CONFLICT,
            "busy",
            "An engine is being validated. Try Save again shortly.",
        );
    };
    // Serialize cancellation with the actual configuration commit.
    let mut jobs = shared.registry.jobs.lock().unwrap();
    if shared.control.stopping() || jobs.get(&id).is_none_or(|v| v["status"] != "validated") {
        return failure(
            StatusCode::CONFLICT,
            "cancelled",
            "Engine validation is no longer available",
        );
    }
    let Some(mut prepared) = shared.registry.validated.lock().unwrap().get(&id).cloned() else {
        return failure(
            StatusCode::CONFLICT,
            "not_validated",
            "Validate this engine before saving",
        );
    };
    if !save.label.trim().is_empty() {
        prepared.engine.label = save.label.trim().into();
    }
    let result = persist(&shared, &id, &prepared);
    match result {
        Ok(()) => {
            shared.registry.validated.lock().unwrap().remove(&id);
            let value = json!({"request":id,"status":"ready","engineId":prepared.engine.id,"name":prepared.engine.label});
            jobs.insert(id, value.clone());
            Json(value).into_response()
        }
        Err(message) => failure(StatusCode::CONFLICT, "configuration", message),
    }
}
fn persist(shared: &Shared, id: &str, prepared: &Validated) -> Result<(), &'static str> {
    let path = shared
        .registry
        .path
        .as_ref()
        .ok_or("Configuration unavailable")?;
    let mut registry = shared.registry.engines.lock().unwrap();
    if registry.iter().any(|engine| same_launch(engine, &prepared.engine.executable, &prepared.engine.args)) {
        return Err("This engine is already added. Select it in Engine Settings instead.");
    }
    let mut check = shared.config.clone();
    check.engines = registry.clone();
    check.engines.push(prepared.engine.clone());
    if !safe_text(&prepared.engine.label) {
        return Err("Invalid engine name");
    }
    check
        .validate()
        .map_err(|_| "Invalid engine name or too many engines")?;
    if read_small(path).map_err(|_| "Cannot read configuration")? != prepared.original {
        return Err(
            "Configuration changed during validation. Select the engine again; no settings were overwritten.",
        );
    }
    if !prepared.engine.executable.is_file()
        || !permitted(&shared.registry, &prepared.engine.executable)
    {
        return Err("Selected executable is no longer available");
    }
    let mut value: Value =
        serde_json::from_slice(&prepared.original).map_err(|_| "Invalid configuration JSON")?;
    value
        .as_object_mut()
        .ok_or("Invalid configuration")?
        .entry("engines")
        .or_insert(json!([]))
        .as_array_mut()
        .ok_or("Invalid engines configuration")?
        .push(serde_json::to_value(&prepared.engine).map_err(|_| "Cannot serialize engine")?);
    let temporary = path.with_file_name(format!(".engine-config-{id}"));
    let bytes = serde_json::to_vec_pretty(&value).map_err(|_| "Cannot serialize configuration")?;
    if bytes.len() > 65536 {
        return Err("Configuration is too large; engine was not saved");
    }
    config::private_write_new(&temporary, &bytes).map_err(|_| "Cannot write configuration")?;
    if fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err("Cannot replace configuration; engine was not saved");
    }
    #[cfg(unix)]
    {
        let _ = fs::File::open(path.parent().unwrap()).and_then(|f| f.sync_all());
    }
    registry.push(prepared.engine.clone());
    Ok(())
}

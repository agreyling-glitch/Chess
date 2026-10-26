//! Owner-only editing of registered engines. Existing running sessions are unchanged.
use crate::{
    Shared,
    config::{self, EngineConfig},
    engine::Engine,
    failure,
};
use axum::{
    Json, Router,
    extract::{Path as RoutePath, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs};
type Result<T> = std::result::Result<T, String>;
pub(crate) fn routes() -> Router<Shared> {
    Router::new().route("/api/v1/engine-management/{id}", get(read).post(save))
        .route("/api/v1/engine-management/{id}/remove", post(remove))
}
fn text(value: &str) -> bool {
    !value.chars().any(char::is_control)
}
fn configuration(shared: &Shared) -> Result<(std::path::PathBuf, Vec<u8>, String)> {
    let path = shared
        .registry
        .configuration_path()
        .ok_or("Engine management requires a persistent Companion configuration")?;
    let meta = fs::symlink_metadata(&path).map_err(|_| "Could not read Companion configuration")?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 65536 {
        return Err("Invalid Companion configuration file".into());
    }
    let bytes = fs::read(&path).map_err(|_| "Could not read Companion configuration")?;
    let token = format!("{:x}", Sha256::digest(&bytes));
    Ok((path, bytes, token))
}
fn registered(shared: &Shared, id: &str) -> Result<EngineConfig> {
    shared
        .registry
        .engines
        .lock()
        .unwrap()
        .iter()
        .find(|e| e.id == id)
        .cloned()
        .ok_or("Registered engine not found".into())
}
fn matching_entry(bytes: &[u8], engine: &EngineConfig) -> Result<()> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| "Invalid Companion configuration")?;
    let entry = value["engines"]
        .as_array()
        .and_then(|entries| {
            entries
                .iter()
                .find(|e| e["id"].as_str() == Some(engine.id.as_str()))
        })
        .ok_or("The engine registration changed. Restart Companion before editing.")?;
    let saved: EngineConfig =
        serde_json::from_value(entry.clone()).map_err(|_| "Invalid engine configuration")?;
    if serde_json::to_value(saved).map_err(|e| e.to_string())?
        != serde_json::to_value(engine).map_err(|e| e.to_string())?
    {
        return Err("The engine was edited outside Companion. Restart Companion before editing its settings.".into());
    }
    Ok(())
}
async fn inspect(
    shared: &Shared,
    config: &EngineConfig,
) -> Result<ironwood_protocol::EngineDetails> {
    let permit = shared
        .engines
        .clone()
        .try_acquire_owned()
        .map_err(|_| "Engine slots are busy. Close another engine and try again.")?;
    let mut engine = Engine::spawn(
        config,
        shared.config.engine_limits.clone(),
        permit,
        &shared.config.tablebase_directories,
    )
    .map_err(str::to_owned)?;
    let result = tokio::select! {_=shared.control.cancelled()=>Err("Companion is stopping"),result=engine.handshake()=>result};
    let details = engine.details.clone();
    engine.close().await;
    result.map_err(str::to_owned)?;
    Ok(details)
}
async fn read(State(shared): State<Shared>, RoutePath(id): RoutePath<String>) -> Response {
    let Ok(_permit) = shared.control.registrations.clone().try_acquire_owned() else {
        return failure(
            StatusCode::CONFLICT,
            "busy",
            "Another engine is being checked. Try again shortly.",
        );
    };
    let result=async{
  let engine=registered(&shared,&id)?;let (_,bytes,token)=configuration(&shared)?;matching_entry(&bytes,&engine)?;
  Ok::<_,String>(json!({"id":engine.id,"label":engine.label,"executable":engine.executable,"args":engine.args,"values":engine.options,"can_uninstall":shared.registry.downloads.owned_install(&engine).is_some(),"token":token}))
 }.await;
    match result {
        Ok(value) => Json(value).into_response(),
        Err(message) => failure(StatusCode::BAD_REQUEST, "unavailable", &message),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Remove { token: String, delete_files: bool }
async fn remove(State(shared): State<Shared>, RoutePath(id): RoutePath<String>, Json(command): Json<Remove>) -> Response {
    let result=(|| -> Result<Value> {
        let _registration=shared.control.registrations.clone().try_acquire_owned().map_err(|_|"Another engine operation is in progress. Try again shortly.")?;
        let _engines=shared.engines.clone().try_acquire_many_owned(shared.config.engine_limits.max_processes as u32).map_err(|_|"An engine is running. Close engine sessions before removing an engine.")?;
        let original=registered(&shared,&id)?;
        let (path,bytes,token)=configuration(&shared)?;
        if token!=command.token{return Err("Configuration changed. Reload before removing the engine.".into());}
        matching_entry(&bytes,&original)?;
        let mut registry=shared.registry.engines.lock().unwrap();
        if command.delete_files {
            if let Some(directory)=shared.registry.downloads.owned_install(&original) {
                if registry.iter().any(|engine|engine.id!=id && engine.executable.starts_with(&directory)) {
                    return Err("Another registered engine uses this installation. Remove the registration only, or remove the other registration first.".into());
                }
            }
        }
        let commit=|| -> Result<()> {
            if shared.control.stopping() {return Err("Companion is stopping".into());}
            let mut value:Value=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
            let entries=value["engines"].as_array_mut().ok_or("Missing engine list")?;
            entries.retain(|e|e["id"].as_str()!=Some(&id));
            let updated=serde_json::to_vec_pretty(&value).map_err(|e|e.to_string())?;
            let mut nonce=[0u8;8];getrandom::fill(&mut nonce).map_err(|e|e.to_string())?;
            let name:String=nonce.iter().map(|b|format!("{b:02x}")).collect();
            let temporary=path.with_file_name(format!(".engine-remove-{name}"));
            config::private_write_new(&temporary,&updated).map_err(|e|e.to_string())?;
            if fs::read(&path).map_err(|e|e.to_string())?!=bytes {let _=fs::remove_file(&temporary);return Err("Configuration changed. Reload before removing the engine.".into());}
            if let Err(error)=fs::rename(&temporary,&path){let _=fs::remove_file(&temporary);return Err(error.to_string());}
            registry.retain(|engine|engine.id!=id);Ok(())
        };
        let message=if command.delete_files {shared.registry.downloads.uninstall(&original,commit)?} else {let mut commit=commit;commit()?;"Engine removed from Companion. Files were kept.".into()};
        Ok(json!({"removed":true,"message":message}))
    })();
    match result {Ok(value)=>Json(value).into_response(),Err(message)=>failure(StatusCode::CONFLICT,"remove_failed",&message)}
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Save {
    token: String,
    label: String,
    args: Vec<String>,
    options: BTreeMap<String, String>,
}
async fn save(
    State(shared): State<Shared>,
    RoutePath(id): RoutePath<String>,
    Json(command): Json<Save>,
) -> Response {
    let Ok(_permit) = shared.control.registrations.clone().try_acquire_owned() else {
        return failure(
            StatusCode::CONFLICT,
            "busy",
            "Another engine is being checked. Try again shortly.",
        );
    };
    if command.label.trim().is_empty()
        || command.label.len() > 128
        || !text(&command.label)
        || command.args.len() > 32
        || command.args.iter().any(|a| a.len() > 4096 || !text(a))
        || command.options.len() > 128
        || command
            .options
            .iter()
            .any(|(k, v)| k.len() > 256 || v.len() > 4096 || !text(k) || !text(v))
    {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "Enter a valid engine name, arguments and option values",
        );
    }
    let result=async{
  let original=registered(&shared,&id).map_err(|e|("configuration",e))?;let (path,bytes,token)=configuration(&shared).map_err(|e|("configuration",e))?;if token!=command.token{return Err(("conflict","Companion configuration changed. Reload the engine settings before saving.".into()));}matching_entry(&bytes,&original).map_err(|e|("conflict",e))?;
  let mut engine=original.clone();engine.label=command.label.trim().into();engine.args=command.args;engine.options=command.options;
  let details=inspect(&shared,&engine).await.map_err(|e|("invalid_options",e))?;
  if engine.options.keys().any(|name|details.options.iter().any(|o|o.name==*name&&o.kind=="button")){return Err(("invalid_options","Engine action buttons cannot be saved as startup defaults".into()));}
  // Serialize the final comparison and file replacement with registration and
  // download installation. Preserve unknown fields and every other engine.
  let mut registry=shared.registry.engines.lock().unwrap();
  if shared.control.stopping(){return Err(("unavailable","Companion is stopping".into()));}
  let index=registry.iter().position(|e|e.id==id).ok_or(("conflict","The engine was removed. Reload the settings.".into()))?;
  if fs::read(&path).map_err(|_|("configuration","Could not read Companion configuration".into()))?!=bytes{return Err(("conflict","Companion configuration changed. Reload before saving.".into()));}
  let mut check=shared.config.clone();check.engines=registry.clone();check.engines[index]=engine.clone();check.validate().map_err(|_|("invalid_options","Engine configuration is outside Companion limits".into()))?;
  let mut value:Value=serde_json::from_slice(&bytes).map_err(|_|("configuration","Invalid Companion configuration JSON".into()))?;
  let entries=value["engines"].as_array_mut().ok_or(("configuration","Missing engine configuration list".into()))?;
  let entry=entries.iter_mut().find(|e|e["id"].as_str()==Some(&id)).ok_or(("conflict","The engine was removed. Reload before saving.".into()))?;
  entry["label"]=engine.label.clone().into();entry["args"]=json!(engine.args);entry["options"]=json!(engine.options);
  let updated=serde_json::to_vec_pretty(&value).map_err(|_|("configuration","Could not encode engine configuration".into()))?;if updated.len()>65536{return Err(("configuration","Companion configuration exceeds its size limit".into()));}
  let mut random=[0u8;8];getrandom::fill(&mut random).map_err(|_|("configuration","OS randomness unavailable".into()))?;let nonce:String=random.iter().map(|v|format!("{v:02x}")).collect();
  let temporary=path.with_file_name(format!(".engine-settings-{id}-{nonce}"));
  config::private_write_new(&temporary,&updated).map_err(|_|("configuration","Could not save engine configuration".into()))?;
  if fs::read(&path).map_err(|_|("configuration","Could not read Companion configuration".into()))?!=bytes{let _=fs::remove_file(&temporary);return Err(("conflict","Companion configuration changed. Reload before saving.".into()));}
  if fs::rename(&temporary,&path).is_err(){let _=fs::remove_file(temporary);return Err(("configuration","Could not replace engine configuration; existing settings were retained".into()));}
  registry[index]=engine;Ok::<_,(&'static str,String)>(json!({"id":id,"token":format!("{:x}",Sha256::digest(updated)),"message":"Saved. New engine sessions will use these defaults."}))
 }.await;
    match result {
        Ok(value) => Json(value).into_response(),
        Err((code, message)) => failure(
            if code == "conflict" {
                StatusCode::CONFLICT
            } else {
                StatusCode::BAD_REQUEST
            },
            code,
            &message,
        ),
    }
}

//! Disk-based SQLite snapshots. Jobs never serialize the game collection.
use crate::{Shared, archive::{self, Error, Result}, failure};
use axum::{Json, Router, extract::State, routing::get, response::{Response, IntoResponse}, http::StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::{Path,PathBuf}, sync::{Arc, Mutex}, time::{SystemTime, UNIX_EPOCH}};
use rusqlite::{Connection, OpenFlags, backup::{Backup, StepResult}};

#[derive(Clone)]
pub struct Manager {
    folder: Arc<Mutex<Option<PathBuf>>>,
    job: Arc<Mutex<Value>>,
    gate: Arc<tokio::sync::Semaphore>,
}
impl Manager {
    pub fn new(folder:Option<PathBuf>)->Self {Self{folder:Arc::new(Mutex::new(folder)),job:Arc::new(Mutex::new(json!({"state":"idle"}))),gate:Arc::new(tokio::sync::Semaphore::new(1))}}
    pub fn busy(&self)->bool {self.gate.available_permits()==0}
    pub async fn drained(&self){let _permit=self.gate.acquire().await;}
    pub fn folder(&self)->Option<PathBuf>{self.folder.lock().unwrap().clone()}
    pub fn configure(&self,value:&str,path:&Path,config_lock:&Mutex<Vec<crate::config::EngineConfig>>)->Result<PathBuf>{
        let _permit=self.gate.try_acquire().map_err(|_|Error("busy","Wait for the current backup or restore to finish"))?;
        if value.trim().is_empty()||value.len()>4096||value.contains(['\0','\r','\n']){return Err(err("Enter a valid absolute backup folder path on the Companion machine"));}
        let folder=PathBuf::from(value.trim());
        if !folder.is_absolute(){return Err(err("Backup folder must be an absolute path on the Companion machine"));}
        io(std::fs::create_dir_all(&folder))?;
        let folder=io(std::fs::canonicalize(folder))?;
        let probe=folder.join(format!(".ironwood-write-check-{}",std::process::id()));
        crate::config::private_write_new(&probe,b"").map_err(|_|err("Backup folder is not writable"))?;io(std::fs::remove_file(&probe))?;
        let _config_lock=config_lock.lock().unwrap();
        let mut value:Value=serde_json::from_slice(&io(std::fs::read(path))?)?;
        value["backup_directory"]=json!(folder);
        let temporary=path.with_extension("backup-settings.tmp");
        crate::config::private_write_new(&temporary,&serde_json::to_vec_pretty(&value)?).map_err(|_|err("Cannot save backup folder setting"))?;
        if std::fs::rename(&temporary,path).is_err(){let _=std::fs::remove_file(temporary);return Err(err("Cannot save Companion configuration"));}
        *self.folder.lock().unwrap()=Some(folder.clone());Ok(folder)
    }
    fn directory(&self)->Result<PathBuf>{
        let folder=self.folder.lock().unwrap().clone().ok_or(Error("configuration","Configure a backup folder in Companion settings first"))?;
        std::fs::create_dir_all(&folder).map_err(|_|Error("backup_failed","Cannot create backup folder on Companion"))?;
        std::fs::canonicalize(folder).map_err(|_|Error("backup_failed","Backup folder is unavailable on Companion"))
    }
}
fn err(message:&'static str)->Error{Error("backup_failed",message)}
fn io<T>(result:std::io::Result<T>)->Result<T>{result.map_err(|_|err("Cannot read or write Companion backup folder; check permissions and free disk space"))}
fn response(result:Result<Value>)->Response{match result{Ok(value)=>Json(value).into_response(),Err(Error(code,message))=>failure(if code=="busy"{StatusCode::CONFLICT}else{StatusCode::BAD_REQUEST},code,message)}}
pub fn routes()->Router<Shared>{Router::new().route("/api/v1/archive/backups",get(list).post(start)).route("/api/v1/archive/backups/settings",get(settings).post(save_settings))}
async fn settings(State(shared):State<Shared>)->Response {Json(json!({"folder":shared.control.backups.folder.lock().unwrap().as_ref().map(|p|p.to_string_lossy()),"server_side":true})).into_response()}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {folder:String}
async fn save_settings(State(shared):State<Shared>,Json(request):Json<Settings>)->Response {
    let result=tokio::task::spawn_blocking(move||shared.registry.configuration_path().ok_or(err("Companion configuration is not writable")).and_then(|path|
        shared.control.backups.configure(&request.folder,&path,&shared.registry.engines))).await
        .map_err(|_|err("Backup settings worker failed")).and_then(|r|r);
    response(result.map(|folder|json!({"folder":folder,"server_side":true})))
}

fn safe_name(name:&str)->bool {name.starts_with("ironwood-")&&name.ends_with(".sqlite3")&&name.len()<160&&name.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'-'|b'.'))}
async fn list(State(shared):State<Shared>)->Response {
    let manager=shared.control.backups.clone();
    let result=tokio::task::spawn_blocking(move||{
        let directory=manager.directory()?;
        let mut files=Vec::new();
        for entry in io(std::fs::read_dir(&directory))? {
            let entry=io(entry)?;let name=entry.file_name().to_string_lossy().into_owned();
            if !safe_name(&name)||!io(entry.file_type())?.is_file(){continue;}
            let metadata=io(entry.metadata())?;
            files.push(json!({"name":name,"bytes":metadata.len()}));
            if files.len()>1000{return Err(err("More than 1000 backups in this folder; move older backups to another folder"));}
        }
        files.sort_by(|a,b|b["name"].as_str().cmp(&a["name"].as_str()));
        Ok(json!({"folder":directory,"files":files,"job":manager.job.lock().unwrap().clone()}))
    }).await.map_err(|_|err("Backup folder worker failed")).and_then(|r|r);
    response(result)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {action:String,#[serde(default)] file:String}
async fn start(State(shared):State<Shared>,Json(request):Json<Request>)->Response {
    let manager=shared.control.backups.clone();
    let permit=match manager.gate.clone().try_acquire_owned(){Ok(p)=>p,Err(_)=>return response(Err(Error("busy","A backup or restore is already running")))};
    if shared.control.stopping(){return response(Err(err("Companion is stopping")));}
    let restore=request.action=="restore";
    if request.action!="backup"&&!restore{return response(Err(err("Choose backup or restore")));}
    if restore&&!safe_name(&request.file){return response(Err(err("Choose a listed Companion backup file")));}
    let Some(archive)=shared.archive.clone() else{return response(Err(err("Archive is disabled")))};
    let Some(import_permit)=shared.control.imports.reserve() else{return response(Err(Error("busy","Wait for imports to finish before backup or restore")))};
    *manager.job.lock().unwrap()=json!({"state":"running","action":request.action,"phase":"Waiting for archive","percent":0});
    let control=shared.control.clone();
    tokio::spawn(async move{
        let _permit=permit;let _imports=import_permit;
        let job_manager=manager.clone();
        let result=archive.run_background(move|db|execute(db,&job_manager,&request,||control.stopping())).await;
        let mut job=manager.job.lock().unwrap();
        match result {Ok(file)=>{job["state"]=json!("complete");job["file"]=json!(file);job["percent"]=json!(100);job["phase"]=json!("Complete");},Err(Error(_,message))=>{job["state"]=json!("failed");job["message"]=json!(message);}}
    });
    Json(json!({"started":true})).into_response()
}
fn validate(db:&Connection)->Result<()> {
    let version:u32=db.pragma_query_value(None,"user_version",|r|r.get(0))?;
    if version!=archive::SCHEMA{return Err(err("Backup schema is incompatible with this Companion version"));}
    if db.query_row("PRAGMA integrity_check",[],|r|r.get::<_,String>(0))?!="ok" {return Err(err("Backup failed SQLite integrity validation"));}
    if db.prepare("PRAGMA foreign_key_check")?.exists([])?{return Err(err("Backup contains invalid references"));}
    for sql in ["SELECT id,revision,document FROM games LIMIT 0","SELECT id,name FROM collections LIMIT 0","SELECT collection_id,game_id FROM memberships LIMIT 0","SELECT game_id,id,metadata,payload FROM analysis_runs LIMIT 0"] {db.prepare(sql)?;}
    Ok(())
}
fn copy(from:&Connection,to:&mut Connection,manager:&Manager,stopping:impl Fn()->bool)->Result<()> {
    let backup=Backup::new(from,to)?;
    loop {
        if stopping(){return Err(err("Backup or restore stopped before completion"));}
        let result=backup.step(1024)?;let progress=backup.progress();
        {let mut job=manager.job.lock().unwrap();job["phase"]=json!("Copying database pages");job["percent"]=json!(if progress.pagecount>0{((progress.pagecount-progress.remaining) as f64*100.0/progress.pagecount as f64).min(99.0)}else{0.0});}
        match result {StepResult::Done=>break,StepResult::Busy|StepResult::Locked=>std::thread::sleep(std::time::Duration::from_millis(50)),_=>{}}
    }
    Ok(())
}
fn execute(db:&mut Connection,manager:&Manager,request:&Request,stopping:impl Fn()->bool)->Result<String>{
    let folder=manager.directory()?;
    if request.action=="restore" {
        if db.query_row("SELECT EXISTS(SELECT 1 FROM games) OR EXISTS(SELECT 1 FROM collections)",[],|r|r.get::<_,bool>(0))? {return Err(err("Restore requires an empty Companion archive. Existing games were not changed"));}
        let path=folder.join(&request.file);
        if !io(std::fs::symlink_metadata(&path))?.file_type().is_file()||io(std::fs::canonicalize(&path))?.parent()!=Some(folder.as_path()){return Err(err("Choose a regular backup file in the configured folder"));}
        let source=Connection::open_with_flags(path,OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        manager.job.lock().unwrap()["phase"]=json!("Validating backup");validate(&source)?;
        copy(&source,db,manager,stopping)?;
        // Cached cursors must see a new revision even when restoring an old snapshot.
        crate::archive_library::initialize(db)?;
        db.execute_batch("DELETE FROM library_query_cache; UPDATE library_cache_state SET revision=revision+1 WHERE id=1;")?;
        Ok(request.file.clone())
    } else {
        let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
        let mut random=[0u8;8];getrandom::fill(&mut random).map_err(|_|err("Cannot generate backup name"))?;
        let name=format!("ironwood-{stamp}-{}.sqlite3",random.iter().map(|b|format!("{b:02x}")).collect::<String>());
        let temporary=folder.join(format!(".{name}.partial"));
        crate::config::private_write_new(&temporary,b"").map_err(|_|err("Cannot create backup file"))?;
        let result=(||{
            let mut target=Connection::open(&temporary)?;copy(db,&mut target,manager,stopping)?;
            // Publish a self-contained file, with no WAL/shared-memory dependency on a NAS.
            target.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL;")?;
            manager.job.lock().unwrap()["phase"]=json!("Validating backup");validate(&target)?;drop(target);
            io(io(std::fs::OpenOptions::new().read(true).write(true).open(&temporary))?.sync_all())?;
            io(std::fs::rename(&temporary,folder.join(&name)))?;
            Ok(name)
        })();
        if result.is_err(){let _=std::fs::remove_file(&temporary);}result
    }
}

#[cfg(test)]
mod tests {
 use super::*;
 fn root()->PathBuf{let path=std::env::temp_dir().join(format!("ironwood-backups-test-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));std::fs::create_dir_all(&path).unwrap();path}
 #[tokio::test]
 async fn snapshots_roundtrip_packed_games_and_reject_overwrite_or_corruption(){
  let root=root();let archive=archive::Archive::open(&root.join("source.sqlite3")).unwrap();let manager=Manager::new(Some(root.join("backups")));
  archive.run(|db|{let docs=crate::archive_pgn::documents("[White \"Backup\"]\n\n1. e4 e5 2. Nf3 Nc6 *")?;archive::import(db,docs,None)?;Ok(())}).await.unwrap();
  let job=manager.clone();let name=archive.run(move|db|execute(db,&job,&Request{action:"backup".into(),file:String::new()},||false)).await.unwrap();
  assert!(safe_name(&name));assert!(!safe_name("../ironwood-test.sqlite3"));assert!(!safe_name("C:\\ironwood-test.sqlite3"));
  let target=archive::Archive::open(&root.join("target.sqlite3")).unwrap();let job=manager.clone();let selected=name.clone();
  target.run(move|db|execute(db,&job,&Request{action:"restore".into(),file:selected},||false)).await.unwrap();
  target.run(|db|{assert_eq!(archive::backup(db)?.games.len(),1);Ok(())}).await.unwrap();
  let job=manager.clone();let selected=name.clone();assert!(target.run(move|db|execute(db,&job,&Request{action:"restore".into(),file:selected},||false)).await.is_err());
  std::fs::write(root.join("backups/ironwood-broken.sqlite3"),b"broken").unwrap();let empty=archive::Archive::open(&root.join("empty.sqlite3")).unwrap();let job=manager.clone();
  assert!(empty.run(move|db|execute(db,&job,&Request{action:"restore".into(),file:"ironwood-broken.sqlite3".into()},||false)).await.is_err());
  empty.run(|db|{assert_eq!(archive::backup(db)?.games.len(),0);Ok(())}).await.unwrap();
  let job=manager.clone();assert!(archive.run(move|db|execute(db,&job,&Request{action:"backup".into(),file:String::new()},||true)).await.is_err());
  assert!(!std::fs::read_dir(root.join("backups")).unwrap().any(|entry|entry.unwrap().file_name().to_string_lossy().ends_with("partial")));
  drop((archive,target,empty));std::fs::remove_dir_all(root).unwrap();
 }
 #[test]
 fn settings_persist_folder_and_preserve_other_configuration(){
  let root=root();let path=root.join("config.json");std::fs::write(&path,br#"{"engines":[{"id":"keep"}],"listen":"127.0.0.1:8790"}"#).unwrap();let manager=Manager::new(None);let lock=Mutex::new(vec![]);
  let folder=manager.configure(root.join("nas").to_str().unwrap(),&path,&lock).unwrap();let saved:Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();assert_eq!(saved["engines"][0]["id"],"keep");assert_eq!(saved["backup_directory"],json!(folder));assert!(manager.configure("relative/path",Path::new("missing"),&lock).is_err());
  std::fs::remove_dir_all(root).unwrap();
 }
 #[test]
 fn interrupted_page_copy_rolls_back_destination(){
  let source=Connection::open_in_memory().unwrap();source.execute_batch("CREATE TABLE large(data BLOB);INSERT INTO large VALUES(zeroblob(8388608));").unwrap();
  let mut target=Connection::open_in_memory().unwrap();target.execute_batch("CREATE TABLE original(value TEXT);INSERT INTO original VALUES('keep');").unwrap();
  let steps=std::sync::atomic::AtomicUsize::new(0);let manager=Manager::new(None);
  assert!(copy(&source,&mut target,&manager,||steps.fetch_add(1,std::sync::atomic::Ordering::SeqCst)>0).is_err());
  assert_eq!(target.query_row("SELECT value FROM original",[],|r|r.get::<_,String>(0)).unwrap(),"keep");
 }
 #[tokio::test]
 async fn backup_endpoints_require_owner_and_exact_origin(){
  use axum::{body::Body,http::Request as HttpRequest};use tower::ServiceExt;
  for (token,origin,status) in [("bad","http://localhost:8788",401),("good","http://evil.invalid",403)] {
   let mut config=crate::Config::defaults(false);config.token="a".repeat(64);config.archive=None;
   let response=crate::app(config).unwrap().oneshot(HttpRequest::builder().uri("/api/v1/archive/backups").header("origin",origin).header("authorization",format!("Bearer {}",if token=="good"{"a".repeat(64)}else{token.into()})).header("X-Ironwood-Protocol","1").body(Body::empty()).unwrap()).await.unwrap();assert_eq!(response.status().as_u16(),status);
  }
 }
}

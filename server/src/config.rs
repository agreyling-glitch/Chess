use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
};

pub type Error = Box<dyn std::error::Error>;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineConfig {
    pub id: String,
    pub label: String,
    pub executable: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub options: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EngineLimits {
    pub max_processes: usize,
    pub handshake_ms: u64,
    pub stop_ms: u64,
    pub max_search_ms: u64,
    pub max_threads: i64,
    pub max_hash_mib: i64,
}
impl Default for EngineLimits {
    fn default() -> Self {
        Self {
            max_processes: 4,
            handshake_ms: 3000,
            stop_ms: 1000,
            max_search_ms: 300000,
            max_threads: 8,
            max_hash_mib: 1024,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub max_connections: u32,
    pub max_pending_tickets: usize,
    pub requests_per_minute: u32,
    pub archive_reads_per_minute: u32,
    pub preflight_requests_per_minute: u32,
    pub max_request_bytes: usize,
    pub request_timeout_seconds: u64,
    pub authentication_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub shutdown_timeout_seconds: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_connections: 16,
            max_pending_tickets: 64,
            requests_per_minute: 240,
            archive_reads_per_minute: 1200,
            preflight_requests_per_minute: 2400,
            max_request_bytes: 4096,
            request_timeout_seconds: 10,
            authentication_timeout_seconds: 5,
            idle_timeout_seconds: 30,
            shutdown_timeout_seconds: 5,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub listen: String,
    pub allowed_origins: Vec<String>,
    pub token_file: String,
    #[serde(default)]
    pub allow_non_loopback: bool,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default)]
    pub engines: Vec<EngineConfig>,
    #[serde(default)]
    pub engine_limits: EngineLimits,
    #[serde(default = "default_archive")]
    pub archive: Option<PathBuf>,
    #[serde(default)]
    pub tablebase_directories: Vec<PathBuf>,
    #[serde(default)]
    pub backup_directory: Option<PathBuf>,
    #[serde(default = "automatic_import_default", skip_serializing_if = "automatic_import_enabled")]
    pub automatic_import: bool,
    #[serde(skip)]
    pub(crate) token: String,
}

impl Config {
    /// For the local Windows tray only. Never serialize or log this value.
    #[cfg(windows)]
    pub fn owner_credential(&self) -> &str { &self.token }
    pub fn defaults(docker: bool) -> Self {
        Self {
            listen: if docker {
                "0.0.0.0:8790"
            } else {
                "127.0.0.1:8790"
            }
            .into(),
            allowed_origins: vec![
                "https://ironwoodchess.com".into(),
                "http://localhost:8788".into(),
                "http://127.0.0.1:8788".into(),
            ],
            token_file: "owner-token".into(),
            allow_non_loopback: docker,
            limits: Limits::default(),
            engines: vec![],
            engine_limits: EngineLimits::default(),
            archive: default_archive(),
            tablebase_directories: vec![],
            backup_directory: None,
            automatic_import: true,
            token: String::new(),
        }
    }

    pub fn read(path: &Path) -> Result<Self, Error> {
        let bytes = fs::read(path).map_err(|_| "Cannot read configuration file")?;
        if bytes.len() > 65536 {
            return Err("Configuration file is too large".into());
        }
        let mut config: Self = serde_json::from_slice(&bytes)
            .map_err(|_| "Invalid configuration JSON or unknown setting")?;
        if let Some(archive) = &mut config.archive {
            if archive.is_relative() {
                *archive =
                    fs::canonicalize(path.parent().unwrap_or(Path::new(".")))?.join(&archive);
            }
            if archive.is_symlink() {
                return Err("Archive database cannot be a symlink".into());
            }
        }
        if let Some(directory) = &mut config.backup_directory {
            if directory.is_relative() {
                *directory = fs::canonicalize(path.parent().unwrap_or(Path::new(".")))?.join(&directory);
            }
        }
        for directory in &mut config.tablebase_directories {
            if directory.is_relative() {
                *directory =
                    fs::canonicalize(path.parent().unwrap_or(Path::new(".")))?.join(&directory);
            }
        }
        for engine in &mut config.engines {
            if engine.executable.is_relative() {
                engine.executable = fs::canonicalize(
                    path.parent()
                        .unwrap_or(Path::new("."))
                        .join(&engine.executable),
                )
                .map_err(|_| "Configured engine executable does not exist")?;
            }
        }
        let token_path = path
            .parent()
            .unwrap_or(Path::new("."))
            .join(&config.token_file);
        if fs::metadata(&token_path)?.len() > 1024 {
            return Err("Credential file is too large".into());
        }
        config.token = fs::read_to_string(token_path)
            .map_err(|_| "Cannot read owner credential")?
            .trim()
            .into();
        config.validate()?;
        Ok(config)
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.tablebase_directories.len() > 16 {
            return Err("At most 16 tablebase directories are supported".into());
        }
        if self.tablebase_directories.iter().any(|p| {
            p.to_str().is_none_or(|s| {
                s.len() > 4096
                    || s.contains(['\r', '\n', '\0'])
                    || s.contains(if cfg!(windows) { ';' } else { ':' })
            })
        }) {
            return Err("Invalid tablebase directory path".into());
        }
        let address: SocketAddr = self
            .listen
            .parse()
            .map_err(|_| "Listen must be an IP socket address")?;
        if !address.ip().is_loopback() && !self.allow_non_loopback {
            return Err("Non-loopback binding requires allow_non_loopback".into());
        }
        if !(32..=1024).contains(&self.token.len())
            || !self.token.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            return Err("Invalid owner credential format".into());
        }
        if self.allowed_origins.is_empty() || self.allowed_origins.len() > 32 {
            return Err("Configure 1 to 32 allowed origins".into());
        }
        for origin in &self.allowed_origins {
            let uri: axum::http::Uri = origin.parse().map_err(|_| "Invalid allowed origin")?;
            if !matches!(uri.scheme_str(), Some("http" | "https"))
                || uri.authority().is_none()
                || uri.path() != "/"
                || uri.query().is_some()
                || origin.ends_with('/')
                || origin.contains('@')
                || axum::http::HeaderValue::from_str(origin).is_err()
            {
                return Err(
                    "Origins must be exact HTTP(S) origins without a trailing slash".into(),
                );
            }
        }
        let l = &self.limits;
        let e = &self.engine_limits;
        if !(1..=32).contains(&e.max_processes)
            || !(100..=10000).contains(&e.handshake_ms)
            || !(100..=2000).contains(&e.stop_ms)
            || !(1000..=3600000).contains(&e.max_search_ms)
            || !(1..=256).contains(&e.max_threads)
            || !(1..=65536).contains(&e.max_hash_mib)
            || self.engines.len() > 64
        {
            return Err("Engine resource limits are outside supported ranges".into());
        }
        let mut ids = std::collections::HashSet::new();
        for engine in &self.engines {
            if engine.id.is_empty()
                || engine.id.len() > 64
                || !engine
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                || !ids.insert(&engine.id)
                || engine.label.is_empty()
                || engine.label.len() > 128
                || !engine.executable.is_absolute()
                || engine.args.len() > 64
                || engine
                    .args
                    .iter()
                    .any(|s| s.len() > 4096 || s.contains('\0'))
                || engine.options.len() > 128
            {
                return Err("Invalid owner engine registration".into());
            }
        }
        if !(1..=256).contains(&l.max_connections)
            || !(1..=4096).contains(&l.max_pending_tickets)
            || !(1..=10000).contains(&l.requests_per_minute)
            || !(1..=10000).contains(&l.archive_reads_per_minute)
            || !(1..=10000).contains(&l.preflight_requests_per_minute)
            || !(1..=65536).contains(&l.max_request_bytes)
            || [
                l.request_timeout_seconds,
                l.authentication_timeout_seconds,
                l.idle_timeout_seconds,
                l.shutdown_timeout_seconds,
            ]
            .iter()
            .any(|v| !(1..=300).contains(v))
        {
            return Err("Resource limits are outside supported ranges".into());
        }
        Ok(())
    }
}

fn automatic_import_default() -> bool { true }
fn automatic_import_enabled(value:&bool)->bool { *value }
fn default_archive() -> Option<PathBuf> {
    Some(PathBuf::from("archive/archive.sqlite3"))
}

#[derive(Debug)]
pub struct DataDirectoryInUse;
impl std::fmt::Display for DataDirectoryInUse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Data directory is in use; stop Companion before rotating credentials or starting another instance")
    }
}
impl std::error::Error for DataDirectoryInUse {}

/// Held for the process lifetime. Locks are released by the OS after crashes.
pub struct DataDirectory {
    pub path: PathBuf,
    _lock: File,
}

impl DataDirectory {
    pub fn open(path: &Path) -> Result<Self, Error> {
        if !path.exists() {
            private_directory(path)?;
        }
        if !path.is_dir() || fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err("Data directory must be a real directory".into());
        }
        let path = fs::canonicalize(path)?;
        let lock_path = path.join("companion.lock");
        if lock_path.is_symlink() {
            return Err("Lock file cannot be a symlink".into());
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(lock_path)?;
        lock.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => Box::new(DataDirectoryInUse) as Error,
            std::fs::TryLockError::Error(error) => Box::new(error) as Error,
        })?;
        Ok(Self { path, _lock: lock })
    }

    pub fn initialize(&self, docker: bool) -> Result<bool, Error> {
        let path = self.path.join("config.json");
        if path.exists() {
            Config::read(&path)?;
            self.prepare_layout(&path)?;
            return Ok(false);
        }
        // Never overwrite or adopt an unrelated directory for provisioning.
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            let name = entry.file_name();
            let folder = entry.file_type()?.is_dir();
            let allowed = name == "companion.lock"
                || (name == "engines" && folder)
                || ((name == "archive" || name == "Import" || name == "Reports") && folder && fs::read_dir(entry.path())?.next().is_none());
            if !allowed {
                return Err("Initialization requires an empty dedicated data directory (an engines folder is allowed)".into());
            }
        }
        secure_directory(&self.path)?;
        self.ensure_folders()?;
        let mut config = Config::defaults(docker);
        config.token = random_token()?;
        private_write_new(&self.path.join("owner-token"), config.token.as_bytes())?;
        let bytes = serde_json::to_vec_pretty(&config)?;
        private_write_new(&path, &bytes)?;
        Ok(true)
    }

    fn ensure_folders(&self) -> Result<(), Error> {
        for name in ["engines", "archive", "Import", "Reports"] {
            let path = self.path.join(name);
            if path.exists() {
                if !path.is_dir() || fs::symlink_metadata(&path)?.file_type().is_symlink() {
                    return Err(format!(
                        "{name} must be a real directory inside the Companion data directory"
                    )
                    .into());
                }
            } else {
                private_directory(&path)?;
            }
        }
        Ok(())
    }

    /// Called while holding the data-directory lock, before opening the archive.
    /// The journal makes a crash between the database rename and config commit recoverable.
    pub fn prepare_layout(&self, config_path: &Path) -> Result<(), Error> {
        use sha2::{Digest, Sha256};
        Config::read(config_path)?;
        self.ensure_folders()?;
        let original = fs::read(config_path)?;
        let mut value: serde_json::Value = serde_json::from_slice(&original)?;
        let source = self.path.join("archive.sqlite3");
        let target = self.path.join("archive/archive.sqlite3");
        let journal = self.path.join(".archive-layout-migration.json");
        let configured = value.get("archive");
        let legacy = configured.is_none()
            || configured.and_then(|v| v.as_str()).is_some_and(|p| {
                let candidate = self.path.join(p);
                candidate
                    .file_name()
                    .is_some_and(|name| name == "archive.sqlite3")
                    && candidate
                        .parent()
                        .and_then(|parent| fs::canonicalize(parent).ok())
                        .as_ref()
                        == Some(&self.path)
            });
        let configured_target =
            configured.and_then(|v| v.as_str()) == Some("archive/archive.sqlite3");
        let digest = format!("{:x}", Sha256::digest(&original));
        if journal.is_symlink() {
            return Err("Archive migration journal cannot be a symlink".into());
        }
        if journal.exists() {
            if journal.is_symlink() || fs::metadata(&journal)?.len() > 65536 {
                return Err("Invalid archive migration journal".into());
            }
            let marker: serde_json::Value = serde_json::from_slice(&fs::read(&journal)?)?;
            if marker["version"] != 1 || marker["config"] != config_path.to_string_lossy().as_ref()
            {
                return Err("Archive migration journal does not match this configuration".into());
            }
            if configured_target && target.is_file() && !source.exists() {
                fs::remove_file(&journal)?;
                return Ok(());
            }
            if !legacy || marker["config_sha256"] != digest {
                return Err("Configuration changed during archive migration; both archive locations have been preserved".into());
            }
        } else if !legacy {
            return Ok(()); // Preserve disabled archives and explicit custom database paths.
        }
        if source.is_symlink() || target.is_symlink() {
            return Err("Archive migration does not follow database symlinks".into());
        }
        if source.exists() && target.exists() {
            return Err(
                "Both legacy and archive-folder databases exist; neither was overwritten".into(),
            );
        }
        if source.exists() {
            // Include committed WAL contents before moving the database. Refuse a move
            // while another SQLite client still has sidecars open.
            let db = rusqlite::Connection::open_with_flags(
                &source,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
            )?;
            let check: String = db.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
            if check != "ok" {
                return Err("Archive integrity check failed; database was not moved".into());
            }
            let busy: i64 = db.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0))?;
            if busy != 0 {
                return Err("Close other archive database clients before migrating".into());
            }
            drop(db);
            for suffix in ["-wal", "-shm", "-journal"] {
                if self.path.join(format!("archive.sqlite3{suffix}")).exists() {
                    return Err("Close other archive database clients before migrating".into());
                }
            }
        } else if target.exists() && !journal.exists() {
            return Err(
                "Archive-folder database already exists; legacy configuration was not changed"
                    .into(),
            );
        }
        if source.exists() && !journal.exists() {
            private_write_new(
                &journal,
                &serde_json::to_vec(&serde_json::json!({
                    "version":1,"config":config_path.to_string_lossy(),"config_sha256":digest
                }))?,
            )?;
        }
        if source.exists() {
            fs::rename(&source, &target)?;
        }
        value["archive"] = serde_json::json!("archive/archive.sqlite3");
        let temporary = self
            .path
            .join(format!(".config-layout-{}", random_token()?));
        private_write_new(&temporary, &serde_json::to_vec_pretty(&value)?)?;
        if let Err(error) = fs::rename(&temporary, config_path) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        #[cfg(unix)]
        {
            File::open(self.path.join("archive"))?.sync_all()?;
            File::open(&self.path)?.sync_all()?;
        }
        if journal.exists() {
            fs::remove_file(journal)?;
        }
        Ok(())
    }

    pub fn rotate(&self) -> Result<(), Error> {
        let config_path = self.path.join("config.json");
        let config = Config::read(&config_path)?;
        // Rotation only manages a file directly inside this dedicated directory.
        if Path::new(&config.token_file).components().count() != 1
            || config.token_file == "."
            || config.token_file == ".."
        {
            return Err(
                "Rotation requires token_file to be a filename in the data directory".into(),
            );
        }
        secure_directory(&self.path)?;
        let temporary = self.path.join(format!(".owner-token-{}", random_token()?));
        private_write_new(&temporary, random_token()?.as_bytes())?;
        let target = self.path.join(&config.token_file);
        if target.is_symlink() {
            let _ = fs::remove_file(&temporary);
            return Err("Credential file cannot be a symlink".into());
        }
        if let Err(error) = fs::rename(&temporary, target) {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }
        #[cfg(unix)]
        File::open(&self.path)?.sync_all()?;
        Ok(())
    }
}

fn random_token() -> Result<String, Error> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "OS randomness unavailable")?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

pub(crate) fn private_write_new(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn private_directory(path: &Path) -> Result<(), Error> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    secure_directory(path)
}

#[cfg(unix)]
fn secure_directory(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(windows)]
fn secure_directory(path: &Path) -> Result<(), Error> {
    use std::os::windows::process::CommandExt;
    // Invoke OS tools directly with argument lists, never a shell. Suppress output
    // and helper windows; credentials are not arguments to either tool.
    let system = std::env::var_os("SystemRoot").ok_or("SystemRoot unavailable")?;
    let system = PathBuf::from(system).join("System32");
    let output = std::process::Command::new(system.join("whoami.exe"))
        .args(["/user", "/fo", "csv", "/nh"])
        .creation_flags(0x08000000)
        .output()?;
    if !output.status.success() {
        return Err("Cannot determine Windows owner SID".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let sid = text
        .split([',', '"', '\r', '\n'])
        .find(|part| part.starts_with("S-1-"))
        .ok_or("Cannot determine Windows owner SID")?;
    let status = std::process::Command::new(system.join("icacls.exe"))
        .arg(path)
        .args(["/inheritance:r", "/grant:r", &format!("*{sid}:(OI)(CI)F")])
        .creation_flags(0x08000000)
        .output()?;
    if !status.status.success() {
        return Err("Cannot protect Windows data-directory ACL".into());
    }
    Ok(())
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use serde_json::json;
    use sha2::{Digest, Sha256};

    fn fixture() -> (PathBuf, DataDirectory, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("ironwood-layout-{}", random_token().unwrap()));
        let storage = DataDirectory::open(&root).unwrap();
        // A bind mount / pre-extracted engine folder must not block first initialization.
        fs::create_dir(storage.path.join("engines")).unwrap();
        fs::write(
            storage.path.join("engines/owner-installed-engine.txt"),
            "fixture",
        )
        .unwrap();
        storage.initialize(false).unwrap();
        let path = storage.path.join("config.json");
        (root, storage, path)
    }
    fn legacy(path: &Path) -> Vec<u8> {
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        value["archive"] = json!("archive.sqlite3");
        let bytes = serde_json::to_vec_pretty(&value).unwrap();
        fs::write(path, &bytes).unwrap();
        bytes
    }
    #[test]
    fn folders_and_legacy_archive_migration_preserve_committed_wal_and_credentials() {
        let (root, storage, path) = fixture();
        assert!(storage.path.join("engines").is_dir());
        assert!(storage.path.join("archive").is_dir());
        assert_eq!(
            Config::read(&path).unwrap().archive.unwrap(),
            storage.path.join("archive/archive.sqlite3")
        );
        let token = fs::read(storage.path.join("owner-token")).unwrap();
        let old_config = legacy(&path);
        let old = storage.path.join("archive.sqlite3");
        let db = rusqlite::Connection::open(&old).unwrap();
        db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE retained(value TEXT); INSERT INTO retained VALUES('game with annotations');").unwrap();
        assert!(
            storage.prepare_layout(&path).is_err(),
            "another open SQLite client must prevent moving its database"
        );
        assert_eq!(fs::read(&path).unwrap(), old_config);
        drop(db);
        storage.prepare_layout(&path).unwrap();
        assert!(!old.exists());
        let target = storage.path.join("archive/archive.sqlite3");
        let db = rusqlite::Connection::open(&target).unwrap();
        let value: String = db
            .query_row("SELECT value FROM retained", [], |r| r.get(0))
            .unwrap();
        assert_eq!(value, "game with annotations");
        drop(db);
        assert_eq!(fs::read(storage.path.join("owner-token")).unwrap(), token);
        let migrated = fs::read(&path).unwrap();
        storage.prepare_layout(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), migrated);
        assert!(!storage.path.join(".archive-layout-migration.json").exists());
        drop(storage);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn migration_resumes_after_rename_and_never_overwrites_a_conflicting_archive() {
        let (root, storage, path) = fixture();
        let bytes = legacy(&path);
        let old = storage.path.join("archive.sqlite3");
        let target = storage.path.join("archive/archive.sqlite3");
        let db = rusqlite::Connection::open(&old).unwrap();
        db.execute_batch(
            "CREATE TABLE retained(value TEXT); INSERT INTO retained VALUES('original');",
        )
        .unwrap();
        drop(db);
        fs::write(&target, b"unrelated archive").unwrap();
        assert!(
            storage
                .prepare_layout(&path)
                .unwrap_err()
                .to_string()
                .contains("neither was overwritten")
        );
        assert_eq!(fs::read(&target).unwrap(), b"unrelated archive");
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_file(&target).unwrap();
        let journal = storage.path.join(".archive-layout-migration.json");
        private_write_new(&journal, &serde_json::to_vec(&json!({"version":1,"config":path.to_string_lossy(),"config_sha256":format!("{:x}", Sha256::digest(&bytes))})).unwrap()).unwrap();
        fs::rename(&old, &target).unwrap(); // Simulate a crash before the config rename.
        storage.prepare_layout(&path).unwrap();
        assert_eq!(Config::read(&path).unwrap().archive.unwrap(), target);
        assert!(!journal.exists());
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["archive"] = json!("custom.sqlite3");
        let custom = serde_json::to_vec(&value).unwrap();
        fs::write(&path, &custom).unwrap();
        storage.prepare_layout(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), custom);
        value["archive"] = json!(null);
        let disabled = serde_json::to_vec(&value).unwrap();
        fs::write(&path, &disabled).unwrap();
        storage.prepare_layout(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), disabled);
        drop(storage);
        fs::remove_dir_all(root).unwrap();
    }
}

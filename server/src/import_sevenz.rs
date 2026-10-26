//! Private native 7z import. Archive paths are never used as output paths.
use crate::import_queue::{folder, real};
use chess_scid_rw::{entry::IndexEntry, names::NameTables, pgn_build, si4, si5};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::{BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, String>;
pub const MAX_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const MAX_NAMES: u64 = 128 * 1024 * 1024;
#[derive(serde::Serialize, serde::Deserialize)]
struct ExtractionCache {
    version: u8,
    source_hash: String,
    files: Vec<CachedFile>,
}
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedFile {
    name: String,
    staged: String,
    size: u64,
    hash: String,
}

pub fn cached_inputs(
    job: &Path,
    source_hash: &str,
    check: &mut impl FnMut() -> Result<()>,
) -> Result<Option<Vec<Input>>> {
    let stage = job.join("unpacked");
    let marker = stage.join("entry-cache.json");
    if !marker.exists() {
        return Ok(None);
    }
    real(job, true)?;
    real(&stage, true)?;
    real(&marker, false)?;
    let file = File::open(&marker).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Invalid extraction cache size".into());
    }
    let cache: ExtractionCache =
        serde_json::from_slice(&bytes).map_err(|_| "Invalid extraction cache")?;
    if cache.version != 1 || cache.source_hash != source_hash {
        return Ok(None);
    }
    if cache.files.is_empty() || cache.files.len() > 1000 {
        return Err("Invalid extraction cache entries".into());
    }
    let (mut outputs, mut total) = (BTreeMap::new(), 0u64);
    for item in cache.files {
        check()?;
        let name = logical_name(&item.name)?.to_lowercase();
        let staged = Path::new(&item.staged);
        if staged.file_name() != Some(std::ffi::OsStr::new(&item.staged))
            || !item.staged.starts_with("entry-")
            || staged.extension() != Path::new(&name).extension()
        {
            return Err("Invalid extraction cache path".into());
        }
        let path = stage.join(&item.staged);
        real(&path, false)?;
        total = total
            .checked_add(item.size)
            .ok_or("Invalid extraction cache size")?;
        if total > MAX_BYTES
            || fs::metadata(&path).map_err(|e| e.to_string())?.len() != item.size
            || hash(&path, check)? != item.hash
        {
            return Ok(None);
        }
        if outputs.insert(name, path).is_some() {
            return Err("Duplicate extraction cache name".into());
        }
    }
    Ok(Some(plan(&outputs)?))
}
pub fn cache_inputs(
    job: &Path,
    source_hash: &str,
    inputs: &[Input],
    check: &mut impl FnMut() -> Result<()>,
) -> Result<()> {
    let stage = job.join("unpacked");
    real(&stage, true)?;
    let mut files = Vec::new();
    let mut add = |name: String, path: &Path| -> Result<()> {
        real(path, false)?;
        files.push(CachedFile {
            name,
            staged: path
                .file_name()
                .ok_or("Invalid staging path")?
                .to_str()
                .ok_or("Invalid staging path")?
                .to_owned(),
            size: fs::metadata(path).map_err(|e| e.to_string())?.len(),
            hash: hash(path, check)?,
        });
        Ok(())
    };
    for input in inputs {
        match input {
            Input::Pgn { name, path } => add(name.clone(), path)?,
            Input::Scid {
                name,
                index,
                names,
                games,
                version,
            } => {
                add(format!("{name}.si{version}"), index)?;
                add(format!("{name}.sn{version}"), names)?;
                add(format!("{name}.sg{version}"), games)?;
            }
        }
    }
    let bytes = serde_json::to_vec(&ExtractionCache {
        version: 1,
        source_hash: source_hash.into(),
        files,
    })
    .map_err(|e| e.to_string())?;
    let temp = stage.join("entry-cache.tmp");
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    output
        .write_all(&bytes)
        .and_then(|_| output.sync_all())
        .map_err(|e| e.to_string())?;
    drop(output);
    fs::rename(temp, stage.join("entry-cache.json")).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    File::open(&stage)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Clone)]
pub enum Input {
    Pgn {
        name: String,
        path: PathBuf,
    },
    Scid {
        name: String,
        index: PathBuf,
        names: PathBuf,
        games: PathBuf,
        version: u8,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_archive_paths_and_incomplete_or_mixed_scid_sets() {
        for name in [
            "../escape.pgn",
            "C:\\escape.pgn",
            "/escape.pgn",
            "a/./b.pgn",
            "a\0.pgn",
        ] {
            assert!(logical_name(name).is_err());
        }
        let mut files = BTreeMap::new();
        files.insert("nested/db.si5".into(), PathBuf::from("index"));
        assert!(plan(&files).is_err());
        files.insert("nested/db.sn5".into(), PathBuf::from("names"));
        files.insert("nested/db.sg4".into(), PathBuf::from("games"));
        assert!(plan(&files).is_err());
        files.remove("nested/db.sg4");
        files.insert("nested/db.sg5".into(), PathBuf::from("games"));
        assert_eq!(plan(&files).unwrap().len(), 1);
    }
    #[test]
    fn resume_seeks_over_saved_scid_entries_without_decoding_games() {
        for version in [4, 5] {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/scid");
            let mut reader = ScidReader::open(
                &root.join(format!("sample.si{version}")),
                &root.join(format!("sample.sn{version}")),
                &root.join(format!("sample.sg{version}")),
                version,
            )
            .unwrap();
            reader.game_bytes = 0;
            assert_eq!(reader.skip_games(2).unwrap(), 2);
            assert!(matches!(reader.next().unwrap(), Some(Game::Deleted)));
            assert_eq!(reader.skip_games(128).unwrap(), 0);
            assert!(reader.next().unwrap().is_none());
        }
    }
    #[test]
    fn native_scid_fixtures_preserve_annotations_and_deleted_status() {
        for version in [4, 5] {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/scid");
            let mut reader = ScidReader::open(
                &root.join(format!("sample.si{version}")),
                &root.join(format!("sample.sn{version}")),
                &root.join(format!("sample.sg{version}")),
                version,
            )
            .unwrap();
            let Some(Game::Pgn(pgn)) = reader.next().unwrap() else {
                panic!("Missing game")
            };
            assert!(pgn.contains("Nf3"));
            assert!(pgn.contains("Nc3"));
            assert!(pgn.contains('{'));
            assert!(matches!(reader.next().unwrap(), Some(Game::Pgn(_))));
            assert!(matches!(reader.next().unwrap(), Some(Game::Deleted)));
            assert!(reader.next().unwrap().is_none());
        }
    }
    #[test]
    #[ignore = "Owner-provided Scid resume validation; set IRONWOOD_VALIDATE_SCID5"]
    fn owner_resume_seeks_directly_to_next_game() {
        let root=PathBuf::from(std::env::var("IRONWOOD_VALIDATE_SCID5").unwrap());
        let mut reader=ScidReader::open(&root.join("LumbrasGigaBase_OTB.si5"),&root.join("LumbrasGigaBase_OTB.sn5"),&root.join("LumbrasGigaBase_OTB.sg5"),5).unwrap();
        let start=std::time::Instant::now();assert_eq!(reader.skip_games(745472).unwrap(),745472);let elapsed=start.elapsed();
        assert_eq!(reader.index.stream_position().unwrap(),745472*56);
        assert_eq!(reader.games.stream_position().unwrap(),0,"Saved game blobs must not be read");
        let Some(Game::Pgn(pgn))=reader.next().unwrap() else{panic!("Expected next playable game")};crate::archive_pgn::parse(&pgn).unwrap();
        eprintln!("Skipped 745,472 saved games by index seek in {elapsed:?}; decoded only the next game");
    }
    #[test]
    #[ignore = "Owner-provided large archive validation; set IRONWOOD_VALIDATE_7Z"]
    fn owner_large_archive_decodes_bounded_sample() {
        let source = PathBuf::from(std::env::var("IRONWOOD_VALIDATE_7Z").unwrap());
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../output/sevenz-validation");
        fs::create_dir_all(&root).unwrap();
        let inputs = extract(&source, &root, &mut || Ok(())).unwrap();
        let mut count = 0;
        for input in inputs {
            if let Input::Scid {
                index,
                names,
                games,
                version,
                ..
            } = input
            {
                let mut reader = ScidReader::open(&index, &names, &games, version).unwrap();
                for _ in 0..128 {
                    if let Some(Game::Pgn(pgn)) = reader.next().unwrap() {
                        crate::archive_pgn::parse(&pgn).unwrap();
                        count += 1;
                    }
                }
            }
        }
        assert_eq!(count, 128);
        cleanup(&root).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
impl Input {
    pub fn game_count(&self, check: &mut impl FnMut() -> Result<()>) -> Result<u64> {
        check()?;
        match self {
            Self::Pgn { path, .. } => {
                crate::archive_pgn::count_games(File::open(path).map_err(|e| e.to_string())?, check)
            }
            Self::Scid { index, version, .. } => {
                let mut file = File::open(index).map_err(|e| e.to_string())?;
                let size = file.metadata().map_err(|e| e.to_string())?.len();
                if *version == 5 {
                    if size % 56 != 0 {
                        return Err("Truncated Scid 5 index".into());
                    }
                    Ok(size / 56)
                } else {
                    let mut header = [0; 182];
                    file.read_exact(&mut header).map_err(|e| e.to_string())?;
                    let header = si4::index::read_header(&header).map_err(|e| e.to_string())?;
                    if size != 182 + u64::from(header.num_games) * 47 {
                        return Err("Inconsistent Scid index".into());
                    }
                    Ok(u64::from(header.num_games))
                }
            }
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Self::Pgn { name, .. } | Self::Scid { name, .. } => name,
        }
    }
}

pub fn hash(path: &Path, check: &mut impl FnMut() -> Result<()>) -> Result<String> {
    use sha2::{Digest, Sha256};
    real(path, false)?;
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("7z input exceeds 64 GiB".into());
    }
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut total = 0u64;
    loop {
        check()?;
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > MAX_BYTES {
            return Err("7z input exceeds 64 GiB".into());
        }
        hash.update(&buf[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn logical_name(name: &str) -> Result<String> {
    let name = name.replace('\\', "/");
    if name.is_empty()
        || name.len() > 4096
        || name.starts_with('/')
        || name.contains(':')
        || name.chars().any(|c| c.is_control())
        || name.split('/').any(|part| part == ".." || part == ".")
    {
        return Err("Unsafe path in 7z archive".into());
    }
    Ok(name.trim_end_matches('/').to_owned())
}

pub fn cleanup(job: &Path) -> Result<()> {
    let path = job.join("unpacked");
    if !path.exists() {
        return Ok(());
    }
    real(job, true)?;
    real(&path, true)?;
    for entry in fs::read_dir(&path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        real(&entry.path(), false)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("entry-") {
            return Err("Unexpected staging file; source retained".into());
        }
        fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
    }
    fs::remove_dir(path).map_err(|e| e.to_string())
}

pub fn extract(
    source: &Path,
    job: &Path,
    check: &mut impl FnMut() -> Result<()>,
) -> Result<Vec<Input>> {
    check()?;
    real(source, false)?;
    cleanup(job)?;
    let stage = folder(job, "unpacked")?;
    let mut reader = sevenz_rust2::ArchiveReader::open(source, sevenz_rust2::Password::empty())
        .map_err(|e| {
            format!(
                "Could not open 7z (encrypted or unsupported archives must be extracted first): {e}"
            )
        })?;
    reader.set_thread_count(1);
    let mut total = 0u64;
    let mut seen = HashSet::new();
    let mut outputs = BTreeMap::new();
    if reader.archive().files.len() > 1000 {
        return Err("7z archive exceeds 1000 entries".into());
    }
    for (n, entry) in reader.archive().files.iter().enumerate() {
        let name = logical_name(&entry.name)?.to_lowercase();
        if !seen.insert(name.clone()) {
            return Err("Duplicate paths in 7z archive".into());
        }
        let unix_kind = (entry.windows_attributes >> 16) & 0xf000;
        if entry.is_anti_item
            || entry.windows_attributes & 0x400 != 0
            || !matches!(unix_kind, 0 | 0x8000 | 0x4000)
        {
            return Err("Links and special files are not supported in 7z imports".into());
        }
        total = total
            .checked_add(entry.size)
            .ok_or("7z expanded size overflow")?;
        if total > MAX_BYTES {
            return Err("7z expanded data exceeds 64 GiB".into());
        }
        if entry.is_directory {
            continue;
        }
        let ext = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if matches!(ext, "pgn" | "si4" | "sn4" | "sg4" | "si5" | "sn5" | "sg5") {
            if matches!(ext, "sn4" | "sn5") && entry.size > MAX_NAMES {
                return Err("Scid name file exceeds 128 MiB".into());
            }
            let path = stage.join(format!("entry-{n:04}.{ext}"));
            outputs.insert(name, path);
        }
    }
    if outputs.is_empty() {
        return Err("7z contains no PGN or supported Scid 4/5 database".into());
    }
    // Validate complete Scid groups before writing anything to the archive DB.
    let inputs = plan(&outputs)?;
    let mut buffer = [0u8; 65536];
    let mut expanded = 0u64;
    reader
        .for_each_entries(|entry, data| {
            let mut action = || -> Result<()> {
                check()?;
                let name = logical_name(&entry.name)?.to_lowercase();
                let mut output = if let Some(path) = outputs.get(&name) {
                    Some(
                        OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(path)
                            .map_err(|e| e.to_string())?,
                    )
                } else {
                    None
                };
                let mut length = 0u64;
                loop {
                    check()?;
                    let n = data.read(&mut buffer).map_err(|e| e.to_string())?;
                    if n == 0 {
                        break;
                    }
                    length += n as u64;
                    expanded += n as u64;
                    if length > entry.size || expanded > MAX_BYTES {
                        return Err("7z expands beyond its declared size or limit".into());
                    }
                    if let Some(file) = output.as_mut() {
                        file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
                    }
                }
                if length != entry.size {
                    return Err("Truncated 7z entry".into());
                }
                if let Some(file) = output {
                    file.sync_all().map_err(|e| e.to_string())?;
                }
                Ok(())
            };
            action()
                .map(|_| true)
                .map_err(|e| sevenz_rust2::Error::Other(e.into()))
        })
        .map_err(|e| format!("7z extraction/integrity check failed: {e}"))?;
    Ok(inputs)
}

fn plan(files: &BTreeMap<String, PathBuf>) -> Result<Vec<Input>> {
    let mut inputs = BTreeMap::new();
    let mut groups: BTreeMap<String, BTreeMap<String, PathBuf>> = BTreeMap::new();
    for (name, path) in files {
        let (stem, ext) = name.rsplit_once('.').ok_or("Missing file extension")?;
        if ext == "pgn" {
            inputs.insert(
                name.clone(),
                Input::Pgn {
                    name: name.clone(),
                    path: path.clone(),
                },
            );
        } else {
            groups
                .entry(stem.to_owned())
                .or_default()
                .insert(ext.to_owned(), path.clone());
        }
    }
    for (name, parts) in groups {
        let version = if parts.contains_key("si5") { 5 } else { 4 };
        let keys = [
            format!("si{version}"),
            format!("sn{version}"),
            format!("sg{version}"),
        ];
        if parts.len() != 3 || keys.iter().any(|key| !parts.contains_key(key)) {
            return Err(format!(
                "Incomplete or mixed Scid file set: {name}. Include matching .si{version}, .sn{version}, .sg{version} files."
            ));
        }
        inputs.insert(
            name.clone(),
            Input::Scid {
                name,
                index: parts[&keys[0]].clone(),
                names: parts[&keys[1]].clone(),
                games: parts[&keys[2]].clone(),
                version,
            },
        );
    }
    Ok(inputs.into_values().collect())
}

pub struct ScidReader {
    index: BufReader<File>,
    games: File,
    names: NameTables,
    remaining: u64,
    game_bytes: u64,
    version: u8,
}
pub enum Game {
    Pgn(String),
    Deleted,
    Failed,
}
impl ScidReader {
    pub fn skip_games(&mut self, count: u64) -> Result<u64> {
        let skipped = count.min(self.remaining);
        let size = if self.version == 4 { 47 } else { 56 };
        let bytes = i64::try_from(
            skipped
                .checked_mul(size)
                .ok_or("Scid checkpoint overflow")?,
        )
        .map_err(|_| "Scid checkpoint overflow")?;
        self.index.seek_relative(bytes).map_err(|e| e.to_string())?;
        self.remaining -= skipped;
        Ok(skipped)
    }
    pub fn open(index: &Path, names: &Path, games: &Path, version: u8) -> Result<Self> {
        for path in [index, names, games] {
            real(path, false)?;
        }
        let names_file = File::open(names).map_err(|e| e.to_string())?;
        if names_file.metadata().map_err(|e| e.to_string())?.len() > MAX_NAMES {
            return Err("Scid names exceed 128 MiB".into());
        }
        let mut names_data = Vec::new();
        names_file
            .take(MAX_NAMES + 1)
            .read_to_end(&mut names_data)
            .map_err(|e| e.to_string())?;
        if names_data.len() as u64 > MAX_NAMES {
            return Err("Scid names exceed 128 MiB".into());
        }
        if version == 4 {
            if names_data.len() < 36 {
                return Err("Truncated Scid 4 name table".into());
            }
            let count = (12..24)
                .step_by(3)
                .map(|i| {
                    ((names_data[i] as usize) << 16)
                        | ((names_data[i + 1] as usize) << 8)
                        | names_data[i + 2] as usize
                })
                .sum::<usize>();
            if count > 4_000_000 || count.saturating_mul(4) > names_data.len() - 36 {
                return Err("Invalid Scid name counts".into());
            }
        } else {
            // Empty journal records still allocate String slots. Bound their count
            // before calling the shared reader, independently of the byte limit.
            let (mut pos, mut count) = (0usize, 0usize);
            while pos < names_data.len() {
                let mut header = 0u64;
                for shift in (0..=63).step_by(7) {
                    let byte = *names_data.get(pos).ok_or("Truncated Scid name journal")?;
                    pos += 1;
                    if shift == 63 && byte > 1 {
                        return Err("Invalid Scid name journal".into());
                    }
                    header |= u64::from(byte & 127) << shift;
                    if byte & 128 == 0 {
                        break;
                    }
                }
                pos = pos
                    .checked_add(
                        usize::try_from(header >> 3).map_err(|_| "Invalid Scid name length")?,
                    )
                    .ok_or("Invalid Scid name length")?;
                if pos > names_data.len() {
                    return Err("Truncated Scid name journal".into());
                }
                if header & 7 < 4 {
                    count += 1;
                }
                if count > 4_000_000 {
                    return Err("Scid exceeds four million name records".into());
                }
            }
        }
        let names = if version == 4 {
            si4::namebase::read_namebase(&names_data)
        } else {
            si5::namebase::read_namebase(&names_data)
        }
        .map_err(|e| e.to_string())?;
        let mut index = BufReader::new(File::open(index).map_err(|e| e.to_string())?);
        let size = index.get_ref().metadata().map_err(|e| e.to_string())?.len();
        let remaining = if version == 4 {
            let mut header = [0u8; 182];
            index.read_exact(&mut header).map_err(|e| e.to_string())?;
            let header = si4::index::read_header(&header).map_err(|e| e.to_string())?;
            if size != 182 + u64::from(header.num_games) * 47 {
                return Err("Scid 4 index has an inconsistent game count".into());
            }
            u64::from(header.num_games)
        } else {
            if size % 56 != 0 {
                return Err("Truncated Scid 5 index".into());
            }
            size / 56
        };
        let games = File::open(games).map_err(|e| e.to_string())?;
        let game_bytes = games.metadata().map_err(|e| e.to_string())?.len();
        Ok(Self {
            index,
            games,
            names,
            remaining,
            game_bytes,
            version,
        })
    }
    pub fn next(&mut self) -> Result<Option<Game>> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        let mut bytes = [0u8; 56];
        let size = if self.version == 4 { 47 } else { 56 };
        self.index
            .read_exact(&mut bytes[..size])
            .map_err(|e| e.to_string())?;
        let entry = if self.version == 4 {
            si4::index::read_index_entry(&bytes[..size])
        } else {
            si5::index::read_index_entry(&bytes)
        }
        .map_err(|e| e.to_string())?;
        if entry.deleted {
            return Ok(Some(Game::Deleted));
        }
        Ok(Some(match self.pgn(&entry) {
            Ok(pgn) => Game::Pgn(pgn),
            Err(_) => Game::Failed,
        }))
    }
    fn pgn(&mut self, e: &IndexEntry) -> Result<String> {
        if e.length == 0
            || e.length > 131071
            || e.offset
                .checked_add(u64::from(e.length))
                .is_none_or(|end| end > self.game_bytes)
            || e.white_id as usize >= self.names.players.len()
            || e.black_id as usize >= self.names.players.len()
            || e.event_id as usize >= self.names.events.len()
            || e.site_id as usize >= self.names.sites.len()
            || e.round_id as usize >= self.names.rounds.len()
        {
            return Err("Invalid Scid entry".into());
        }
        self.games
            .seek(SeekFrom::Start(e.offset))
            .map_err(|e| e.to_string())?;
        let mut blob = vec![0u8; e.length as usize];
        self.games
            .read_exact(&mut blob)
            .map_err(|e| e.to_string())?;
        pgn_build::build_pgn(e, &self.names, &blob).map_err(|e| e.to_string())
    }
}

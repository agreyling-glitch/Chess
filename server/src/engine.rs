use crate::config::{EngineConfig, EngineLimits};
use ironwood_protocol::*;
use shakmaty::{CastlingMode, Chess, Position, fen::Fen, uci::UciMove};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::{Child, ChildStdin},
    sync::{OwnedSemaphorePermit, mpsc},
    task::JoinHandle,
};

pub type Result<T> = std::result::Result<T, &'static str>;
pub struct SearchRequest {
    pub search_id: String,
    pub position_id: String,
    pub fen: String,
    pub moves: Vec<String>,
    pub chess960: bool,
    pub new_game: bool,
    pub options: BTreeMap<String, String>,
    pub limit: SearchLimit,
}
pub struct Active {
    pub search_id: String,
    pub position_id: String,
    pub position: Chess,
    pub cancelled: bool,
    pub deadline: Instant,
}
pub struct Engine {
    pub session_id: String,
    pub details: EngineDetails,
    pub active: Option<Active>,
    child: Child,
    input: ChildStdin,
    pub output: mpsc::Receiver<Result<String>>,
    tasks: Vec<JoinHandle<()>>,
    pub stderr: Arc<Mutex<VecDeque<u8>>>,
    limits: EngineLimits,
    configured: BTreeMap<String, String>,
    syzygy_path: String,
    used: HashSet<String>,
    _permit: OwnedSemaphorePermit,
}

impl Engine {
    pub fn spawn(
        config: &EngineConfig,
        limits: EngineLimits,
        permit: OwnedSemaphorePermit,
        tablebase_directories: &[std::path::PathBuf],
    ) -> Result<Self> {
        let mut command = tokio::process::Command::new(&config.executable);
        command
            .args(&config.args)
            .current_dir(
                config
                    .executable
                    .parent()
                    .ok_or("Engine folder unavailable")?,
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let mut child = command
            .spawn()
            .map_err(|_| "Engine executable could not be started")?;
        let input = child.stdin.take().ok_or("Engine stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("Engine stdout unavailable")?;
        let stderr_reader = child.stderr.take().ok_or("Engine stderr unavailable")?;
        let (tx, output) = mpsc::channel(64);
        let reader = tokio::spawn(read_lines(stdout, tx));
        let stderr = Arc::new(Mutex::new(VecDeque::new()));
        let buffer = stderr.clone();
        let errors = tokio::spawn(async move {
            let mut reader = stderr_reader;
            let mut bytes = [0u8; 1024];
            while let Ok(count) = reader.read(&mut bytes).await {
                if count == 0 {
                    break;
                }
                let mut buffer = buffer.lock().unwrap();
                for byte in &bytes[..count] {
                    if buffer.len() == 16384 {
                        buffer.pop_front();
                    }
                    buffer.push_back(*byte);
                }
            }
        });
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| "OS randomness unavailable")?;
        Ok(Self {
            session_id: random.iter().map(|b| format!("{b:02x}")).collect(),
            details: EngineDetails {
                id: config.id.clone(),
                name: config.label.clone(),
                author: String::new(),
                options: vec![],
            },
            active: None,
            child,
            input,
            output,
            tasks: vec![reader, errors],
            stderr,
            limits,
            configured: config.options.clone(),
            syzygy_path: tablebase_directories
                .iter()
                .map(|p| p.to_string_lossy())
                .collect::<Vec<_>>()
                .join(if cfg!(windows) { ";" } else { ":" }),
            used: HashSet::new(),
            _permit: permit,
        })
    }

    async fn command(&mut self, value: &str) -> Result<()> {
        if value.len() > 65536 || value.contains(['\r', '\n', '\0']) {
            return Err("Invalid UCI command");
        }
        tokio::time::timeout(Duration::from_secs(1), async {
            self.input.write_all(value.as_bytes()).await?;
            self.input.write_all(b"\n").await?;
            self.input.flush().await
        })
        .await
        .map_err(|_| "Engine input timed out")?
        .map_err(|_| "Engine input closed")
    }

    pub async fn handshake(&mut self) -> Result<()> {
        let deadline =
            tokio::time::Instant::now() + Duration::from_millis(self.limits.handshake_ms);
        self.command("uci").await?;
        loop {
            let line = tokio::time::timeout_at(deadline, self.output.recv())
                .await
                .map_err(|_| "Engine UCI handshake timed out")?
                .ok_or("Engine exited during UCI handshake")??;
            if line == "uciok" {
                break;
            }
            if let Some(name) = line.strip_prefix("id name ") {
                self.details.name = name.chars().take(128).collect();
            }
            if let Some(author) = line.strip_prefix("id author ") {
                self.details.author = author.chars().take(128).collect();
            }
            if line.starts_with("option ") {
                if self.details.options.len() >= 256 {
                    return Err("Engine advertises too many options");
                }
                if let Some(option) = parse_option(&line) {
                    if self.details.options.iter().any(|o| o.name == option.name) {
                        return Err("Engine advertises duplicate options");
                    }
                    self.details.options.push(option);
                }
            }
        }
        // Expose effective resource ranges, and apply safe defaults before any go.
        let mut defaults = Vec::new();
        for option in &mut self.details.options {
            let cap = match option.name.as_str() {
                "Threads" => Some(self.limits.max_threads),
                "Hash" => Some(self.limits.max_hash_mib),
                "MultiPV" => Some(32),
                _ => None,
            };
            if option.kind == "spin"
                && let Some(cap) = cap
            {
                option.max = Some(option.max.unwrap_or(cap).min(cap));
                if option.min.is_some_and(|min| min > cap) {
                    return Err("Engine resource minimum exceeds server budget");
                }
                let value = option
                    .default
                    .parse::<i64>()
                    .unwrap_or(option.min.unwrap_or(1))
                    .min(option.max.unwrap());
                option.default = value.to_string();
                if !self.configured.contains_key(&option.name) {
                    defaults.push((option.name.clone(), option.default.clone()));
                }
            }
        }
        for (name, value) in defaults {
            self.set_option(&name, &value, true).await?;
        }
        if !self.syzygy_path.is_empty()
            && !self
                .configured
                .keys()
                .any(|n| n.eq_ignore_ascii_case("SyzygyPath"))
            && let Some(option) = self
                .details
                .options
                .iter()
                .find(|o| o.name.eq_ignore_ascii_case("SyzygyPath") && o.kind == "string")
        {
            self.set_option(&option.name.clone(), &self.syzygy_path.clone(), true)
                .await?;
        }
        for (name, value) in self.configured.clone() {
            self.set_option(&name, &value, true).await?;
            if let Some(option) = self
                .details
                .options
                .iter_mut()
                .find(|option| option.name == name)
            {
                option.default = value;
            }
        }
        if self
            .details
            .options
            .iter()
            .any(|o| o.name == "Ponder" && o.kind == "check")
        {
            self.set_option("Ponder", "false", true).await?;
        }
        self.ready().await
    }

    async fn ready(&mut self) -> Result<()> {
        self.command("isready").await?;
        let deadline =
            tokio::time::Instant::now() + Duration::from_millis(self.limits.handshake_ms);
        loop {
            let line = tokio::time::timeout_at(deadline, self.output.recv())
                .await
                .map_err(|_| "Engine readiness timed out")?
                .ok_or("Engine exited while preparing search")??;
            if line == "readyok" {
                return Ok(());
            }
        }
    }

    pub fn validate_option(&self, name: &str, value: &str, owner: bool) -> Result<()> {
        if !owner && (name == "UCI_Chess960" || (name == "Ponder" && value != "false")) {
            return Err("This protocol controls Chess960 and disables pondering");
        }
        if name.len() > 256
            || value.len() > 4096
            || name.contains(['\r', '\n', '\0'])
            || value.contains(['\r', '\n', '\0'])
        {
            return Err("Invalid option text");
        }
        let option = self
            .details
            .options
            .iter()
            .find(|o| o.name == name)
            .ok_or("Engine does not advertise this option")?;
        match option.kind.as_str() {
            "spin" => {
                let number: i64 = value.parse().map_err(|_| "Option requires an integer")?;
                if option.min.is_some_and(|v| number < v)
                    || option.max.is_some_and(|v| number > v)
                    || (name == "Threads" && number > self.limits.max_threads)
                    || (name == "Hash" && number > self.limits.max_hash_mib)
                {
                    return Err("Option is outside the engine or server range");
                }
            }
            "check" if matches!(value, "true" | "false") => {}
            "combo" if option.choices.iter().any(|choice| choice == value) => {}
            "button" if value.is_empty() => {}
            "string" => {}
            _ => return Err("Option value is invalid or is restricted to owner configuration"),
        }
        Ok(())
    }

    async fn set_option(&mut self, name: &str, value: &str, owner: bool) -> Result<()> {
        self.validate_option(name, value, owner)?;
        if self
            .details
            .options
            .iter()
            .any(|o| o.name == name && o.kind == "button")
        {
            self.command(&format!("setoption name {name}")).await
        } else {
            self.command(&format!("setoption name {name} value {value}"))
                .await
        }
    }

    pub async fn set_idle_option(&mut self, name: &str, value: &str) -> Result<()> {
        if self.active.is_some() {
            return Err("Stop the current search before running an engine action");
        }
        self.set_option(name, value, false).await
    }

    pub async fn search(&mut self, request: SearchRequest) -> Result<()> {
        let SearchRequest {
            search_id,
            position_id,
            fen,
            moves,
            chess960,
            new_game,
            options,
            limit,
        } = request;
        if self.active.is_some() {
            return Err("Engine is busy; cancel and drain the previous search first");
        }
        if !valid_id(&search_id)
            || !valid_id(&position_id)
            || self.used.contains(&search_id)
            || self.used.len() >= 4096
        {
            return Err(
                "Invalid or repeated search identity; reopen the session if its search budget is exhausted",
            );
        }
        let position = position(&fen, &moves, chess960)?;
        if options.len() > 128 {
            return Err("Too many search options");
        }
        for (name, value) in &options {
            self.validate_option(name, value, false)?;
        }
        let command = search_command(&limit, self.limits.max_search_ms)?;
        if chess960
            && !self
                .details
                .options
                .iter()
                .any(|o| o.name == "UCI_Chess960" && o.kind == "check")
        {
            return Err("Engine does not advertise Chess960");
        }
        if new_game {
            self.command("ucinewgame").await?;
        }
        if self
            .details
            .options
            .iter()
            .any(|o| o.name == "UCI_Chess960" && o.kind == "check")
        {
            self.set_option(
                "UCI_Chess960",
                if chess960 { "true" } else { "false" },
                true,
            )
            .await?;
        }
        for (name, value) in options {
            self.set_option(&name, &value, false).await?;
        }
        self.ready().await?;
        self.command(&format!(
            "position fen {fen}{}",
            if moves.is_empty() {
                String::new()
            } else {
                format!(" moves {}", moves.join(" "))
            }
        ))
        .await?;
        self.used.insert(search_id.clone());
        self.active = Some(Active {
            search_id,
            position_id,
            position,
            cancelled: false,
            deadline: Instant::now() + Duration::from_millis(self.limits.max_search_ms),
        });
        self.command(&command).await
    }

    pub async fn cancel(&mut self, search_id: &str) -> Result<bool> {
        if let Some(active) = &mut self.active {
            if active.search_id != search_id {
                return Err("Cancel does not match the active search");
            }
            if !active.cancelled {
                active.cancelled = true;
                active.deadline = Instant::now() + Duration::from_millis(self.limits.stop_ms);
                self.command("stop").await?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn close(&mut self) {
        self.active = None;
        let _ = self.command("stop").await;
        let _ = self.command("quit").await;
        if !matches!(
            tokio::time::timeout(
                Duration::from_millis(self.limits.stop_ms),
                self.child.wait()
            )
            .await,
            Ok(Ok(_))
        ) {
            let _ = self.child.kill().await;
            let _ = self.child.wait().await;
        }
        for task in &self.tasks {
            task.abort();
        }
        self.stderr.lock().unwrap().clear();
    }

    pub fn output_message(&mut self, line: &str) -> Result<Option<ServerMessage>> {
        let Some(active) = &self.active else {
            return Ok(None);
        };
        let session_id = self.session_id.clone();
        if line.starts_with("bestmove ") {
            let active = self.active.take().unwrap();
            if active.cancelled {
                return Ok(Some(ServerMessage::Cancelled {
                    session_id,
                    search_id: active.search_id,
                }));
            }
            let text = line.split_whitespace().nth(1).ok_or("Malformed bestmove")?;
            let bestmove = validated_bestmove(text, &active.position)?;
            return Ok(Some(ServerMessage::Bestmove {
                session_id,
                search_id: active.search_id,
                position_id: active.position_id,
                bestmove,
            }));
        }
        if !active.cancelled
            && let Some(info) = parse_info(line, &active.position)
        {
            return Ok(Some(ServerMessage::Info {
                session_id,
                search_id: active.search_id.clone(),
                position_id: active.position_id.clone(),
                info,
            }));
        }
        Ok(None)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

async fn read_lines(mut reader: impl AsyncRead + Unpin, sender: mpsc::Sender<Result<String>>) {
    let mut buffer = [0u8; 1024];
    let mut line = Vec::new();
    loop {
        match reader.read(&mut buffer).await {
            Ok(0) | Err(_) => {
                let _ = sender.send(Err("Engine output closed")).await;
                break;
            }
            Ok(count) => {
                for byte in &buffer[..count] {
                    if *byte == b'\n' {
                        let text = String::from_utf8_lossy(&line)
                            .trim_end_matches('\r')
                            .to_owned();
                        line.clear();
                        if sender.send(Ok(text)).await.is_err() {
                            return;
                        }
                    } else {
                        if line.len() >= 16384 {
                            let _ = sender.send(Err("Engine output line exceeds limit")).await;
                            return;
                        }
                        line.push(*byte);
                    }
                }
            }
        }
    }
}

pub fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_:-.".contains(&b))
}

fn position(fen: &str, moves: &[String], chess960: bool) -> Result<Chess> {
    if fen.len() > 256
        || fen.split_whitespace().count() != 6
        || fen.contains(['\r', '\n', '\0'])
        || moves.len() > 1024
    {
        return Err("Invalid position");
    }
    let mode = if chess960 {
        CastlingMode::Chess960
    } else {
        CastlingMode::Standard
    };
    let mut position: Chess = fen
        .parse::<Fen>()
        .map_err(|_| "Invalid FEN")?
        .into_position(mode)
        .map_err(|_| "Illegal FEN")?;
    for text in moves {
        let uci = text
            .parse::<UciMove>()
            .map_err(|_| "Invalid history move")?;
        let mv = uci.to_move(&position).map_err(|_| "Illegal history move")?;
        position.play_unchecked(mv);
    }
    Ok(position)
}

fn search_command(limit: &SearchLimit, max_ms: u64) -> Result<String> {
    Ok(match *limit {
        SearchLimit::Time { milliseconds } if (1..=max_ms).contains(&milliseconds) => {
            format!("go movetime {milliseconds}")
        }
        SearchLimit::Depth { plies } if (1..=128).contains(&plies) => format!("go depth {plies}"),
        SearchLimit::Nodes { nodes } if (1..=1_000_000_000_000).contains(&nodes) => {
            format!("go nodes {nodes}")
        }
        SearchLimit::Infinite => "go infinite".into(),
        SearchLimit::Clock {
            white_ms,
            black_ms,
            white_increment_ms,
            black_increment_ms,
        } if white_ms > 0
            && black_ms > 0
            && [white_ms, black_ms, white_increment_ms, black_increment_ms]
                .iter()
                .all(|v| *v <= 604800000) =>
        {
            format!(
                "go wtime {white_ms} btime {black_ms} winc {white_increment_ms} binc {black_increment_ms}"
            )
        }
        _ => return Err("Search limit is outside supported ranges"),
    })
}

fn parse_option(line: &str) -> Option<UciOption> {
    let (name, rest) = line.strip_prefix("option name ")?.split_once(" type ")?;
    if name.is_empty() || name.len() > 256 {
        return None;
    }
    let mut words = rest.split_whitespace();
    let kind = words.next()?.to_owned();
    if !matches!(
        kind.as_str(),
        "spin" | "check" | "combo" | "button" | "string"
    ) {
        return None;
    }
    let mut option = UciOption {
        name: name.into(),
        kind,
        default: String::new(),
        min: None,
        max: None,
        choices: vec![],
    };
    let words: Vec<_> = words.collect();
    let mut i = 0;
    while i < words.len() {
        let key = words[i];
        i += 1;
        let start = i;
        while i < words.len() && !matches!(words[i], "default" | "min" | "max" | "var") {
            i += 1;
        }
        let value = words[start..i].join(" ");
        match key {
            "default" => option.default = value,
            "min" => option.min = value.parse().ok(),
            "max" => option.max = value.parse().ok(),
            "var" => option.choices.push(value),
            _ => {}
        }
    }
    Some(option)
}

fn validated_bestmove(text: &str, position: &Chess) -> Result<Option<String>> {
    // Some engines emit a coordinate placeholder rather than 0000 at mate or
    // stalemate. There is no move to validate or play in either terminal state.
    if position.legal_moves().is_empty() { return Ok(None); }
    if matches!(text, "0000" | "(none)") {
        return Err("Engine returned no move in a playable position");
    }
    let uci = text.parse::<UciMove>().map_err(|_| "Engine returned malformed bestmove")?;
    uci.to_move(position).map_err(|_| "Engine returned an illegal move")?;
    Ok(Some(text.to_owned()))
}

fn parse_info(line: &str, root: &Chess) -> Option<SearchInfo> {
    let words: Vec<_> = line.strip_prefix("info ")?.split_whitespace().collect();
    if words.first() == Some(&"string") {
        return None;
    }
    let mut info = SearchInfo::default();
    let mut i = 0;
    while i < words.len() {
        match words[i] {
            "depth" => info.depth = words.get(i + 1).and_then(|s| s.parse().ok()),
            "nodes" => info.nodes = words.get(i + 1).and_then(|s| s.parse().ok()),
            "multipv" => info.multipv = words.get(i + 1).and_then(|s| s.parse().ok()),
            "score" if matches!(words.get(i + 1), Some(&"cp" | &"mate")) => {
                if let Some(value) = words.get(i + 2).and_then(|s| s.parse().ok()) {
                    info.score = Some(Score {
                        kind: words[i + 1].into(),
                        value,
                        perspective: "side_to_move".into(),
                        bound: words
                            .get(i + 3)
                            .filter(|v| matches!(**v, "lowerbound" | "upperbound"))
                            .map(|v| (*v).into()),
                    });
                }
            }
            "pv" => {
                let mut position = root.clone();
                for text in words[i + 1..].iter().take(256) {
                    let Ok(uci) = text.parse::<UciMove>() else {
                        break;
                    };
                    let Ok(mv) = uci.to_move(&position) else {
                        break;
                    };
                    position.play_unchecked(mv);
                    info.pv.push((*text).into());
                }
                break;
            }
            _ => {}
        }
        i += 1;
    }
    if info.depth.is_none() && info.score.is_none() && info.nodes.is_none() {
        None
    } else {
        Some(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_positions_have_no_bestmove_but_playable_moves_remain_validated() {
        for fen in ["7k/6Q1/6K1/8/8/8/8/8 b - - 0 1", "7k/5Q2/6K1/8/8/8/8/8 b - - 0 1"] {
            let root = position(fen, &[], false).unwrap();
            for placeholder in ["a1a1", "e2e4", "0000", "(none)"] {
                assert_eq!(validated_bestmove(placeholder, &root).unwrap(), None);
            }
        }
        let root = Chess::default();
        assert_eq!(validated_bestmove("e2e4", &root).unwrap(), Some("e2e4".into()));
        for invalid in ["a1a1", "e2e5", "0000", "(none)"] { assert!(validated_bestmove(invalid, &root).is_err()); }
    }
    #[test]
    fn option_names_choices_and_score_semantics() {
        let option = parse_option(
            "option name Playing Style type combo default Normal var Normal var Very Aggressive",
        )
        .unwrap();
        assert_eq!(option.name, "Playing Style");
        assert_eq!(option.choices, ["Normal", "Very Aggressive"]);
        let info = parse_info(
            "info depth 9 score mate -3 lowerbound nodes 100 pv e2e4 e7e5 bad",
            &Chess::default(),
        )
        .unwrap();
        assert_eq!(info.pv, ["e2e4", "e7e5"]);
        let score = info.score.unwrap();
        assert_eq!(score.perspective, "side_to_move");
        assert_eq!(score.value, -3);
        assert_eq!(score.bound.as_deref(), Some("lowerbound"));
    }
    #[test]
    fn position_and_limits_reject_injection_and_illegal_moves() {
        assert!(position("startpos\ngo infinite", &[], false).is_err());
        assert!(search_command(&SearchLimit::Time { milliseconds: 0 }, 1000).is_err());
        assert!(!valid_id("bad\nstop"));
    }
}

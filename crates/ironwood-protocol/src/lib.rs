//! Platform-independent Companion wire types. No networking or UI dependencies.
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: &str = "1";
pub const PROTOCOL_HEADER: &str = "x-ironwood-protocol";

#[derive(Debug, Serialize, Deserialize)]
pub struct Capabilities {
    pub product: String,
    pub server_version: String,
    pub protocol_version: String,
    pub services: Services,
    pub engine_ids: Vec<String>,
    pub archive_schema_version: Option<u32>,
    pub tablebase_coverage: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Services {
    pub uci: bool,
    pub archive: bool,
    pub explorer: bool,
    pub tablebases: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ticket {
    pub ticket: String,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMessage {
    Authenticate {
        ticket: String,
    },
    Ping {
        request_id: String,
    },
    OpenEngine {
        request_id: String,
        engine_id: String,
    },
    Search {
        session_id: String,
        search_id: String,
        position_id: String,
        fen: String,
        #[serde(default)]
        moves: Vec<String>,
        #[serde(default)]
        chess960: bool,
        #[serde(default)]
        new_game: bool,
        #[serde(default)]
        options: std::collections::BTreeMap<String, String>,
        limit: SearchLimit,
    },
    Cancel {
        session_id: String,
        search_id: String,
    },
    CloseEngine {
        session_id: String,
    },
    SetOption {
        session_id: String,
        request_id: String,
        name: String,
        value: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Ready {
        protocol_version: String,
    },
    Pong {
        request_id: String,
    },
    Error {
        code: String,
        message: String,
    },
    EngineOpened {
        request_id: String,
        session_id: String,
        engine: EngineDetails,
    },
    SearchStarted {
        session_id: String,
        search_id: String,
        position_id: String,
    },
    Info {
        session_id: String,
        search_id: String,
        position_id: String,
        info: SearchInfo,
    },
    Bestmove {
        session_id: String,
        search_id: String,
        position_id: String,
        bestmove: Option<String>,
    },
    Cancelled {
        session_id: String,
        search_id: String,
    },
    EngineClosed {
        session_id: String,
    },
    OptionSet {
        session_id: String,
        request_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SearchLimit {
    Time {
        milliseconds: u64,
    },
    Depth {
        plies: u32,
    },
    Nodes {
        nodes: u64,
    },
    Infinite,
    Clock {
        white_ms: u64,
        black_ms: u64,
        white_increment_ms: u64,
        black_increment_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSummary {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineDetails {
    pub id: String,
    pub name: String,
    pub author: String,
    pub options: Vec<UciOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UciOption {
    pub name: String,
    pub kind: String,
    pub default: String,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchInfo {
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
    pub multipv: Option<u32>,
    pub score: Option<Score>,
    pub pv: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub kind: String,
    pub value: i32,
    pub perspective: String,
    pub bound: Option<String>,
}

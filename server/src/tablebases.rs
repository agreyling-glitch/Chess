//! Owner supplied Syzygy files only. No networking, downloads or memory mapping.
use super::{Shared, failure};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};
use shakmaty::{CastlingMode, Chess, Position, fen::Fen, san::SanPlus, uci::UciMove};
use shakmaty_syzygy::{SyzygyError, Tablebase};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct Service {
    directories: Arc<Vec<PathBuf>>,
    gate: Arc<tokio::sync::Semaphore>,
}
impl Service {
    pub fn new(directories: Vec<PathBuf>) -> Self {
        Self {
            directories: Arc::new(directories),
            gate: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }
    pub async fn coverage_names(&self) -> Vec<String> {
        let service = self.clone();
        tokio::task::spawn_blocking(move || {
            service
                .run(None)
                .ok()
                .and_then(|v| {
                    v["materials"].as_object().map(|m| {
                        m.iter()
                            .flat_map(|(name, metrics)| {
                                metrics
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .filter_map(move |metric| {
                                        metric.as_str().map(|metric| format!("{name}.{metric}"))
                                    })
                            })
                            .collect()
                    })
                })
                .unwrap_or_default()
        })
        .await
        .unwrap_or_default()
    }
    pub async fn drained(&self) {
        let _permit = self.gate.acquire().await;
    }
    // Fresh handles on every request deliberately avoid stale results after file replacement.
    fn scan(&self) -> (Tablebase<Chess>, Value) {
        let mut tables = Tablebase::new();
        let mut coverage: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut rejected = 0;
        let mut unavailable = 0;
        let mut files = 0;
        for directory in self.directories.iter() {
            let Ok(entries) = std::fs::read_dir(directory) else {
                unavailable += 1;
                continue;
            };
            for entry in entries.take(10001) {
                files += 1;
                if files > 10000 {
                    rejected += 1;
                    break;
                }
                let Ok(entry) = entry else {
                    rejected += 1;
                    continue;
                };
                let path = entry.path();
                let Some(extension @ ("rtbw" | "rtbz")) = path.extension().and_then(|s| s.to_str())
                else {
                    continue;
                };
                if path.is_symlink() || !path.is_file() || tables.add_file(&path).is_err() {
                    rejected += 1;
                    continue;
                }
                coverage
                    .entry(path.file_stem().unwrap().to_string_lossy().into_owned())
                    .or_default()
                    .push(extension.to_owned());
            }
        }
        let report = json!({"configured": !self.directories.is_empty(), "materials": coverage,
            "rejectedFiles": rejected, "unavailableDirectories": unavailable,
            "validation": "Registered filenames and sizes only; file contents are checked when probed. Successor tables may also be required.",
            "cache": "none"});
        (tables, report)
    }
    fn run(&self, fen: Option<String>) -> Result<Value, (&'static str, String)> {
        let _guard = self
            .gate
            .try_acquire()
            .map_err(|_| ("busy", "Another tablebase request is running".into()))?;
        if self.directories.is_empty() {
            return Err((
                "disabled",
                "Configure tablebase_directories in Companion first".into(),
            ));
        }
        let (tables, coverage) = self.scan();
        let Some(fen) = fen else { return Ok(coverage) };
        if fen.len() > 256 || fen.split_whitespace().count() != 6 {
            return Err(("invalid_position", "Provide a complete standard FEN".into()));
        }
        let pos: Chess = fen
            .parse::<Fen>()
            .ok()
            .and_then(|f| f.into_position(CastlingMode::Standard).ok())
            .ok_or(("invalid_position", "Illegal standard chess position".into()))?;
        if pos.board().occupied().count() > 7 || !pos.castles().is_empty() {
            return Err((
                "unsupported_position",
                "Use at most seven pieces without castling rights".into(),
            ));
        }
        let mut result = probe(&tables, &pos)?;
        let mut moves = Vec::new();
        for m in pos.legal_moves() {
            let mut child = pos.clone();
            child.play_unchecked(m);
            let mut row = probe(&tables, &child)?;
            row["uci"] = json!(UciMove::from_standard(m).to_string());
            row["san"] = json!(SanPlus::from_move(pos.clone(), m).to_string());
            row["zeroing"] = json!(child.halfmoves() == 0);
            moves.push(row);
        }
        // Child categories describe the opponent's perspective, matching the existing UI.
        moves.sort_by_key(|m| {
            let rank = match m["category"].as_str().unwrap_or("") {
                "loss" => 0,
                "maybe-loss" => 1,
                "blessed-loss" => 2,
                "draw" => 3,
                "cursed-win" => 4,
                "maybe-win" => 5,
                _ => 6,
            };
            (
                rank,
                if rank >= 4 {
                    -m["dtz"].as_i64().unwrap_or(0).abs()
                } else {
                    m["dtz"].as_i64().unwrap_or(0).abs()
                },
            )
        });
        result["moves"] = json!(moves);
        result["source"] = json!("companion");
        result["coverage"] = coverage;
        Ok(result)
    }
}
fn probe(tables: &Tablebase<Chess>, pos: &Chess) -> Result<Value, (&'static str, String)> {
    // Checkmate ends the game before a fifty-move claim can be made. Applying
    // the clock to the library's terminal DTZ (-1) would mislabel mate at 100.
    if pos.is_checkmate() {
        return Ok(json!({"category": "loss", "dtz": -1, "dtzRounded": false, "dtm": null}));
    }
    let wdl = tables.probe_wdl(pos).map_err(probe_error)?;
    let dtz = tables.probe_dtz(pos).map_err(probe_error)?;
    // Rounded DTZ=1 at clock 99 can look ambiguous even when a legal move
    // proves immediate mate. That proof gives an exact win, regardless of a
    // possible fifty-move claim in a position that has already ended in mate.
    if dtz.ignore_rounding().0 == 1
        && pos.legal_moves().into_iter().any(|m| {
            let mut child = pos.clone();
            child.play_unchecked(m);
            child.is_checkmate()
        })
    {
        return Ok(json!({"category": "win", "dtz": 1, "dtzRounded": false, "dtm": null}));
    }
    let category = match wdl {
        shakmaty_syzygy::AmbiguousWdl::Loss => "loss",
        shakmaty_syzygy::AmbiguousWdl::MaybeLoss => "maybe-loss",
        shakmaty_syzygy::AmbiguousWdl::BlessedLoss => "blessed-loss",
        shakmaty_syzygy::AmbiguousWdl::Draw => "draw",
        shakmaty_syzygy::AmbiguousWdl::CursedWin => "cursed-win",
        shakmaty_syzygy::AmbiguousWdl::MaybeWin => "maybe-win",
        shakmaty_syzygy::AmbiguousWdl::Win => "win",
    };
    Ok(
        json!({"category": category, "dtz": dtz.ignore_rounding().0, "dtzRounded": dtz.precise().is_none(), "dtm": null}),
    )
}
fn probe_error(error: SyzygyError) -> (&'static str, String) {
    match error {
        SyzygyError::MissingTable { .. } => ("missing_table", error.to_string()),
        SyzygyError::TooManyPieces => {
            ("missing_table", "Required material is not installed".into())
        }
        SyzygyError::Castling => (
            "unsupported_position",
            "Castling rights are unsupported".into(),
        ),
        SyzygyError::ProbeFailed { .. } => (
            "corrupt_table",
            "A required table could not be read or is corrupt; check the installation".into(),
        ),
    }
}
#[derive(Deserialize)]
struct ProbeQuery {
    fen: String,
}
pub fn routes() -> Router<Shared> {
    Router::new()
        .route("/api/v1/tablebases", get(coverage))
        .route("/api/v1/tablebases/probe", get(lookup))
}
async fn execute(service: Service, fen: Option<String>) -> Response {
    match tokio::task::spawn_blocking(move || service.run(fen)).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err((code, message))) => failure(
            match code {
                "busy" => StatusCode::TOO_MANY_REQUESTS,
                "invalid_position" | "unsupported_position" => StatusCode::BAD_REQUEST,
                "disabled" => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::UNPROCESSABLE_ENTITY,
            },
            code,
            &message,
        ),
        Err(_) => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "probe_failed",
            "Tablebase worker failed",
        ),
    }
}
async fn coverage(State(shared): State<Shared>) -> Response {
    execute(shared.tablebases, None).await
}
async fn lookup(State(shared): State<Shared>, Query(query): Query<ProbeQuery>) -> Response {
    execute(shared.tablebases, Some(query.fen)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_draw_and_validation() {
        let service = Service::new(vec![PathBuf::from("nonexistent-test-tablebases")]);
        let draw = service
            .run(Some("8/8/8/8/8/2k5/8/K7 w - - 99 1".into()))
            .unwrap();
        assert_eq!(draw["category"], "draw");
        assert!(!draw["moves"].as_array().unwrap().is_empty());
        assert_eq!(
            service.run(Some("bad".into())).unwrap_err().0,
            "invalid_position"
        );
        assert_eq!(
            service
                .run(Some("8/8/8/8/8/2k5/8/KQ6 w - - 0 1".into()))
                .unwrap_err()
                .0,
            "missing_table"
        );
        assert_eq!(Service::new(vec![]).run(None).unwrap_err().0, "disabled");
    }
    #[test]
    fn corrupt_partial_install_and_replacement_are_not_draws() {
        let mut random = [0; 8];
        getrandom::fill(&mut random).unwrap();
        let dir =
            std::env::temp_dir().join(format!("ironwood-tb-{:x}", u64::from_le_bytes(random)));
        std::fs::create_dir(&dir).unwrap();
        let service = Service::new(vec![dir.clone()]);
        // Original synthetic invalid bytes, not an external dataset or a real table.
        std::fs::write(dir.join("KQvK.rtbw"), [0u8; 80]).unwrap();
        let report = service.run(None).unwrap();
        assert_eq!(report["materials"]["KQvK"], json!(["rtbw"]));
        let fen = "8/8/8/8/8/2k5/8/KQ6 w - - 0 1";
        assert_eq!(
            service.run(Some(fen.into())).unwrap_err().0,
            "corrupt_table"
        );
        std::fs::remove_file(dir.join("KQvK.rtbw")).unwrap();
        assert_eq!(
            service.run(Some(fen.into())).unwrap_err().0,
            "missing_table"
        );
        assert!(
            service.run(None).unwrap()["materials"]
                .as_object()
                .unwrap()
                .is_empty()
        );
        std::fs::write(dir.join("KQvK.rtbw"), [0u8; 3]).unwrap();
        assert_eq!(service.run(None).unwrap()["rejectedFiles"], 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn castling_ep_and_clock_semantics() {
        let service = Service::new(vec![PathBuf::from("missing-test-tables")]);
        assert_eq!(
            service
                .run(Some("4k3/8/8/8/8/8/8/4K2R w K - 0 1".into()))
                .unwrap_err()
                .0,
            "unsupported_position"
        );
        // Legal EP is accepted and passed to the library, not silently removed.
        assert_eq!(
            service
                .run(Some("7k/8/8/3pP3/8/8/8/K7 w - d6 0 1".into()))
                .unwrap_err()
                .0,
            "missing_table"
        );
        use shakmaty_syzygy::{
            AmbiguousWdl as W, Dtz,
            MaybeRounded::{Precise, Rounded},
        };
        for (dtz, clock, expected) in [
            (Precise(Dtz(2)), 0, W::Win),
            (Precise(Dtz(-2)), 0, W::Loss),
            (Precise(Dtz(0)), 99, W::Draw),
            (Precise(Dtz(2)), 99, W::CursedWin),
            (Precise(Dtz(-2)), 99, W::BlessedLoss),
            (Rounded(Dtz(2)), 98, W::MaybeWin),
            (Rounded(Dtz(-2)), 98, W::MaybeLoss),
        ] {
            assert_eq!(W::from_dtz_and_halfmoves(dtz, clock), expected);
        }
    }

    #[test]
    fn checkmate_precedes_fifty_move_claim() {
        let service = Service::new(vec![PathBuf::from("missing-test-tables")]);
        for clock in [0, 99, 100, 150] {
            let result = service
                .run(Some(format!("7k/6Q1/5K2/8/8/8/8/8 b - - {clock} 1")))
                .unwrap();
            assert_eq!(result["category"], "loss");
            assert_eq!(result["dtz"], -1);
            assert_eq!(result["dtzRounded"], false);
            assert!(result["moves"].as_array().unwrap().is_empty());
        }
    }
}

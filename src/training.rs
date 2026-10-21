use serde::{Deserialize, Serialize};

pub const START_RATING: i32 = 1320;
pub const MAX_TARGET: i32 = 3190;
pub const K_FACTOR: f64 = 32.0;
pub const NODE_LEVELS: [u64; 10] = [1, 4, 16, 64, 256, 1024, 4096, 16384, 65536, 262144];
fn default_level() -> u8 { 1 }
fn default_elo() -> i32 { START_RATING }

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrainingEngine {
    #[default]
    Stockfish,
    Lc0 { identity: String, name: String, #[serde(default)] options: std::collections::BTreeMap<String, String> },
}
impl TrainingEngine {
    pub fn is_lc0(&self) -> bool { matches!(self, Self::Lc0 { .. }) }
    pub fn label(&self) -> &str { match self { Self::Stockfish => "Stockfish", Self::Lc0 { name, .. } => name } }
}
#[cfg(target_arch = "wasm32")]
pub const STORAGE_KEY: &str = "ironwood.chess.training.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub fn label(self) -> &'static str {
        match self {
            Self::White => "White",
            Self::Black => "Black",
        }
    }
    pub fn other(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub engine: TrainingEngine,
    #[serde(default)]
    pub node_budget: u64,
    #[serde(default)]
    pub difficulty_level: u8,
    pub id: String,
    pub profile_id: String,
    pub profile_name: String,
    pub side: Side,
    pub opponent_elo: i32,
    pub rating_before: i32,
    pub started_at: String,
    // A result is settled once, keyed by session ID in the profile ledger.
    #[serde(default)]
    pub rating_after: Option<i32>,
    #[serde(default)]
    pub completed_at: Option<String>,
}
impl Session {
    pub fn opponent_label(&self) -> String {
        if self.engine.is_lc0() { format!("{} · Level {} · {} nodes", self.engine.label(), self.difficulty_level, self.node_budget) }
        else { format!("Stockfish {} Elo", self.opponent_elo) }
    }
    pub fn search_command(&self) -> String {
        if self.engine.is_lc0() { format!("go nodes {}", self.node_budget.max(1)) }
        else { "go movetime 1000".into() }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PhaseStats {
    pub moves: usize,
    #[serde(default)]
    pub cp_moves: usize,
    pub loss_cp: u64,
    pub mistakes: usize,
    pub blunders: usize,
}

impl PhaseStats {
    pub fn add(&mut self, loss: Option<i32>, quality: i32) {
        self.moves += 1;
        if let Some(loss) = loss {
            self.loss_cp += loss.max(0) as u64;
            self.cp_moves += 1;
        }
        if quality > 200 {
            self.blunders += 1;
        } else if quality > 100 {
            self.mistakes += 1;
        }
    }
    pub fn merge(&mut self, other: &Self) {
        self.moves += other.moves;
        self.cp_moves += other.cp_moves;
        self.loss_cp += other.loss_cp;
        self.mistakes += other.mistakes;
        self.blunders += other.blunders;
    }
    pub fn mean_loss(&self) -> f64 {
        if self.cp_moves == 0 {
            0.0
        } else {
            self.loss_cp as f64 / self.cp_moves as f64
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AnalysisStats {
    pub total_player_moves: usize,
    pub phases: [PhaseStats; 3],
    pub missed_mates: usize,
    pub allowed_mates: usize,
}

impl AnalysisStats {
    pub fn practice_focus(&self) -> Option<usize> {
        self.phases.iter().enumerate()
            .filter(|(_, phase)| phase.cp_moves >= 20 && phase.mistakes + phase.blunders > 0)
            .max_by(|(_, a), (_, b)| a.mean_loss().total_cmp(&b.mean_loss()))
            .map(|(index, _)| index)
    }

    pub fn analyzed_moves(&self) -> usize {
        self.phases.iter().map(|p| p.moves).sum()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultEntry {
    pub session: Session,
    pub result: String,
    pub score: f64,
    pub timed_out: bool,
    #[serde(default)]
    pub analysis: AnalysisStats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub engine: TrainingEngine,
    #[serde(default = "default_level")]
    pub starting_level: u8,
    #[serde(default = "default_elo")]
    pub starting_elo: i32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub results: Vec<ResultEntry>,
}

impl Profile {
    pub fn initial_rating(&self) -> i32 {
        if self.engine.is_lc0() { self.starting_level.clamp(1, NODE_LEVELS.len() as u8) as i32 }
        else { self.starting_elo.clamp(START_RATING, MAX_TARGET) }
    }
    pub fn rating(&self, side: Side) -> i32 {
        self.results
            .iter()
            .rev()
            .find(|r| r.session.side == side)
            .and_then(|r| r.session.rating_after)
            .unwrap_or_else(|| self.initial_rating())
    }
    pub fn overall(&self) -> i32 {
        self.rating(Side::White).min(self.rating(Side::Black))
    }
    pub fn next_side(&self) -> Side {
        self.results
            .last()
            .map(|r| r.session.side.other())
            .unwrap_or(Side::White)
    }
    pub fn target(&self, side: Side) -> i32 {
        self.rating(side).clamp(START_RATING, MAX_TARGET)
    }
    pub fn session(&self, id: String, started_at: String) -> Session {
        let side = self.next_side();
        Session {
            engine: self.engine.clone(),
            node_budget: if self.engine.is_lc0() { NODE_LEVELS[(self.rating(side).clamp(1, NODE_LEVELS.len() as i32) - 1) as usize] } else { 0 },
            difficulty_level: if self.engine.is_lc0() { self.rating(side).clamp(1, NODE_LEVELS.len() as i32) as u8 } else { 0 },
            id,
            profile_id: self.id.clone(),
            profile_name: self.name.clone(),
            side,
            opponent_elo: if self.engine.is_lc0() { 0 } else { self.target(side) },
            rating_before: self.rating(side),
            started_at,
            rating_after: None,
            completed_at: None,
        }
    }
    /// Returns false for unfinished games or an already settled session.
    pub fn settle(
        &mut self,
        session: &Session,
        result: &str,
        completed_at: String,
        timed_out: bool,
    ) -> bool {
        if session.profile_id != self.id || session.engine != self.engine || self.results.iter().any(|r| r.session.id == session.id)
        {
            return false;
        }
        let score = match (result, session.side) {
            ("1/2-1/2", _) => 0.5,
            ("1-0", Side::White) | ("0-1", Side::Black) => 1.0,
            ("1-0", Side::Black) | ("0-1", Side::White) => 0.0,
            _ => return false,
        };
        let mut settled = session.clone();
        // Use the latest color rating if another tab settled a different game.
        settled.rating_before = self.rating(session.side);
        let expected = 1.0
            / (1.0 + 10.0_f64.powf((session.opponent_elo - settled.rating_before) as f64 / 400.0));
        settled.rating_after = Some(if self.engine.is_lc0() {
            (settled.rating_before + if score == 1.0 { 1 } else if score == 0.0 { -1 } else { 0 }).clamp(1, NODE_LEVELS.len() as i32)
        } else { settled.rating_before + (K_FACTOR * (score - expected)).round() as i32 });
        settled.completed_at = Some(completed_at);
        self.results.push(ResultEntry {
            session: settled,
            result: result.into(),
            score,
            timed_out,
            analysis: AnalysisStats::default(),
        });
        true
    }
    pub fn counts(&self, side: Option<Side>) -> [usize; 3] {
        let mut counts = [0; 3];
        for r in self
            .results
            .iter()
            .filter(|r| side.is_none_or(|s| r.session.side == s))
        {
            counts[if r.score == 1.0 {
                0
            } else if r.score == 0.5 {
                1
            } else {
                2
            }] += 1;
        }
        counts
    }
    pub fn recent_change(&self, side: Side) -> i32 {
        let games: Vec<_> = self
            .results
            .iter()
            .filter(|r| r.session.side == side)
            .rev()
            .take(10)
            .collect();
        self.rating(side)
            - games
                .last()
                .map(|r| r.session.rating_before)
                .unwrap_or(self.rating(side))
    }
    pub fn score_interval(&self) -> Option<(f64, f64, f64)> {
        // Hoeffding's bounded-score interval supports draws as 0.5. It is wide
        // for small samples and describes results, not a confidence band for Elo.
        let n = self.results.len();
        if n == 0 {
            return None;
        }
        let mean = self.results.iter().map(|r| r.score).sum::<f64>() / n as f64;
        let radius = (40.0_f64.ln() / (2.0 * n as f64)).sqrt();
        Some((mean, (mean - radius).max(0.0), (mean + radius).min(1.0)))
    }
    pub fn analysis_totals(&self) -> AnalysisStats {
        let mut total = AnalysisStats::default();
        for r in &self.results {
            total.total_player_moves += r.analysis.total_player_moves;
            total.missed_mates += r.analysis.missed_mates;
            total.allowed_mates += r.analysis.allowed_mates;
            for (phase, source) in total.phases.iter_mut().zip(&r.analysis.phases) {
                phase.merge(source);
            }
        }
        total
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Profiles {
    pub selected_id: String,
    pub profiles: Vec<Profile>,
}

impl Profiles {
    pub fn selected(&self) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == self.selected_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn lc0_profile() -> Profile {
        Profile {engine:TrainingEngine::Lc0 {identity:"http://localhost:8790/lc0".into(),name:"Lc0".into(),options:Default::default()},starting_level:3,..profile()}
    }
    #[test]
    fn legacy_profiles_stay_stockfish_and_target_elo_is_preserved() {
        let mut old: Profile = serde_json::from_str(r#"{"id":"old","name":"Player","results":[]}"#).unwrap();
        assert_eq!(old.engine, TrainingEngine::Stockfish);
        assert_eq!(old.session("old-game".into(),"now".into()).search_command(),"go movetime 1000");
        old.starting_elo = 2000;
        let session = old.session("elo".into(),"now".into());
        assert_eq!(session.opponent_elo,2000);
        assert!(old.settle(&session,"1-0","later".into(),false));
        assert_eq!(old.rating(Side::White),2016);
    }
    #[test]
    fn lc0_levels_are_color_specific_bounded_and_settled_once() {
        let mut p = lc0_profile();
        let white = p.session("w".into(),"now".into());
        assert_eq!(white.search_command(),"go nodes 16");
        assert_eq!(white.opponent_elo,0);
        assert!(p.settle(&white,"1-0","later".into(),false));
        assert!(!p.settle(&white,"1-0","later".into(),false));
        assert_eq!(p.rating(Side::White),4); assert_eq!(p.rating(Side::Black),3);
        let black = p.session("b".into(),"now".into());
        assert!(p.settle(&black,"1-0","later".into(),false));
        assert_eq!(p.rating(Side::Black),2);
        let draw = p.session("draw".into(),"now".into());
        assert!(p.settle(&draw,"1/2-1/2","later".into(),false));
        assert_eq!(p.rating(Side::White),4);
        let restored: Profile = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(restored.engine,p.engine); assert_eq!(restored.rating(Side::Black),2);
        for (initial,result,expected) in [(1,"0-1",1),(10,"1-0",10)] {
            let mut edge = lc0_profile(); edge.starting_level=initial;
            let game=edge.session("edge".into(),"now".into()); edge.settle(&game,result,"later".into(),false);
            assert_eq!(edge.rating(Side::White),expected);
        }
        let foreign=profile().session("foreign".into(),"now".into());
        assert!(!p.settle(&foreign,"1-0","later".into(),false));
    }
    fn profile() -> Profile {
        Profile {
            engine: TrainingEngine::default(), starting_level: 1, starting_elo: START_RATING,
            id: "p".into(),
            name: "Player".into(),
            results: vec![],
        }
    }
    #[test]
    fn alternates_and_requires_both_colors_to_progress() {
        let mut p = profile();
        let white = p.session("w".into(), "now".into());
        assert_eq!(white.opponent_elo, 1320);
        assert!(p.settle(&white, "1-0", "later".into(), false));
        assert_eq!(p.rating(Side::White), 1336);
        assert_eq!(p.overall(), 1320);
        let black = p.session("b".into(), "now".into());
        assert_eq!(black.side, Side::Black);
        assert_eq!(black.opponent_elo, 1320);
        p.settle(&black, "0-1", "later".into(), false);
        assert_eq!(p.overall(), 1336);
        assert_eq!(p.next_side(), Side::White);
        assert_eq!(p.target(Side::White), 1336);
    }
    #[test]
    fn losses_regress_draws_and_reload_do_not_double_count() {
        let mut p = profile();
        let session = p.session("loss".into(), "now".into());
        assert!(!p.settle(&session, "*", "later".into(), false));
        assert_eq!(p.next_side(), Side::White);
        p.settle(&session, "0-1", "later".into(), true);
        assert_eq!(p.rating(Side::White), 1304);
        assert_eq!(p.target(Side::White), 1320);
        let mut restored: Profile =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(!restored.settle(&session, "0-1", "later".into(), true));
        assert_eq!(restored.results.len(), 1);
        let black = restored.session("draw".into(), "now".into());
        restored.settle(&black, "1/2-1/2", "later".into(), false);
        assert_eq!(restored.rating(Side::Black), 1320);
        assert_eq!(restored.counts(None), [0, 1, 1]);
    }
    #[test]
    fn profiles_are_isolated_and_analysis_merges_without_rating_changes() {
        let mut p = profile();
        let session = p.session("game".into(), "now".into());
        let mut other = profile();
        other.id = "other".into();
        assert!(!other.settle(&session, "1-0", "later".into(), false));
        p.settle(&session, "1-0", "later".into(), false);
        p.results[0].analysis.phases[0].add(Some(210), 210);
        p.results[0].analysis.phases[0].add(Some(110), 110);
        let total = p.analysis_totals();
        assert_eq!(total.phases[0].blunders, 1);
        assert_eq!(total.phases[0].mistakes, 1);
        assert_eq!(total.phases[0].mean_loss(), 160.0);
        assert_eq!(p.rating(Side::White), 1336);
        assert_eq!(other.rating(Side::White), 1320);
        let (_, lower, upper) = p.score_interval().unwrap();
        assert_eq!((lower, upper), (0.0, 1.0));
    }
}

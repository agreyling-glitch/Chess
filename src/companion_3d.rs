//! The detached board proposes moves; only the main application changes games.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct MoveRequest {
    context: String,
    fen: String,
    uci: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> ChessApp {
        let ctx = egui::Context::default();
        let mut app = ChessApp::new(&eframe::CreationContext::_new_kittest(ctx));
        app.engine_enabled = false;
        app
    }

    fn snapshot(app: &ChessApp) -> serde_json::Value {
        serde_json::json!({"fen":app.board.to_string(),"context":app.external_3d_context(),"canMove":app.external_3d_can_move()})
    }

    fn request(app: &ChessApp, uci: &str) -> MoveRequest {
        MoveRequest { context: app.external_3d_context(), fen: app.board.to_string(), uci: uci.into() }
    }

    #[test]
    fn child_clicks_keep_selection_and_send_one_atomic_move_without_playing_locally() {
        let mut main = app();
        let mut child = app();
        // A detached app can have unrelated saved player/engine/online state.
        child.engine_enabled = true;
        child.player_side = PlayerSide::Black;
        child.fics_active = true;
        child.fics_playing = false;
        let value = snapshot(&main);
        assert!(child.sync_3d_companion_input(Some(&value)));
        child.select(Square::E2);
        assert_eq!(child.selected, Some(Square::E2));
        child.sync_3d_companion_input(Some(&value));
        assert_eq!(child.selected, Some(Square::E2), "Stable snapshots must preserve click selection");
        child.select(Square::E4);
        let proposed = child.take_3d_companion_move().unwrap();
        assert_eq!(proposed.uci, "e2e4");
        assert_eq!(child.board, Board::default());
        assert!(child.history.is_empty() && child.review_moves.is_empty());
        assert!(!child.fics_pending_move && !child.companion_3d_can_move());
        child.select(Square::D2);
        child.select(Square::D4);
        assert!(child.take_3d_companion_move().is_none(), "Wait for the main board before proposing another move");
        assert!(main.accept_external_3d_move(&proposed));
        assert_eq!(main.review_moves, ["e4"]);
        assert!(!main.accept_external_3d_move(&proposed), "A duplicated request must not apply twice");
        child.sync_3d_companion_input(Some(&snapshot(&main)));
        assert_eq!(child.board, main.board);
        assert!(child.companion_3d_can_move());
    }

    #[test]
    fn changed_context_permissions_and_disconnect_clear_selection_drag_and_promotion() {
        let main = app();
        for change in ["context", "fen", "permission", "disconnect", "missing"] {
            let mut child = app();
            let value = snapshot(&main);
            child.sync_3d_companion_input(Some(&value));
            child.select(Square::E2);
            child.piece_drag = Some((child.board, Square::E2, true));
            child.promotion = Some((Square::A7, Square::A8));
            let mut changed = value;
            match change {
                "context" => changed["context"] = "another-game".into(),
                "fen" => changed["fen"] = Board::default().make_move_new(ChessMove::new(Square::E2, Square::E4, None)).to_string().into(),
                "permission" => changed["canMove"] = false.into(),
                "disconnect" => changed["connected"] = false.into(),
                _ => {}
            }
            child.sync_3d_companion_input((change != "missing").then_some(&changed));
            assert!(child.selected.is_none() && child.legal_targets.is_empty() && child.piece_drag.is_none() && child.promotion.is_none(), "Failed to clear {change}");
            if matches!(change, "permission" | "disconnect" | "missing") {
                child.select(Square::D2);
                assert!(child.selected.is_none() && !child.companion_3d_can_move());
            }
        }
    }

    #[test]
    fn illegal_or_stale_requests_preserve_main_selection_and_game() {
        let mut main = app();
        main.select(Square::G1);
        let original_targets = main.legal_targets.clone();
        let valid = request(&main, "e2e4");
        for proposed in [
            MoveRequest { uci: "e2e5".into(), ..valid.clone() },
            MoveRequest { uci: "e2e4q".into(), ..valid.clone() },
            MoveRequest { uci: "e2e4 trailing".into(), ..valid.clone() },
            MoveRequest { fen: "stale position".into(), ..valid.clone() },
            MoveRequest { context: "stale context".into(), ..valid.clone() },
        ] {
            assert!(!main.accept_external_3d_move(&proposed));
            assert_eq!(main.board, Board::default());
            assert_eq!(main.selected, Some(Square::G1));
            assert_eq!(main.legal_targets, original_targets);
            assert!(main.history.is_empty() && main.review_moves.is_empty());
        }
        main.reset_for_side(PlayerSide::White);
        assert_eq!(main.board.to_string(), valid.fen);
        assert!(!main.accept_external_3d_move(&valid), "A new game at the same FEN must reject the old game's request");
    }

    #[test]
    fn main_rechecks_turn_review_prediction_finished_and_online_permissions() {
        for blocked in ["engine-turn", "review", "prediction", "promotion", "resigned", "draw", "spectator", "pending", "online-finished", "fics-disconnected", "lichess-disconnected"] {
            let mut main = app();
            main.fics_connected = true;
            let proposed = request(&main, "e2e4");
            match blocked {
                "engine-turn" => { main.engine_enabled = true; main.player_side = PlayerSide::Black; }
                "review" => main.review_index = Some(0),
                "prediction" => main.prediction_index = 1,
                "promotion" => main.promotion = Some((Square::A7, Square::A8)),
                "resigned" => main.local_resigned_white = Some(true),
                "draw" => main.local_agreed_draw = true,
                "spectator" => main.fics_active = true,
                "pending" => { main.fics_active = true; main.fics_playing = true; main.fics_pending_move = true; }
                "online-finished" => { main.fics_active = true; main.fics_playing = true; main.fics_game_finished = true; }
                "fics-disconnected" => { main.fics_active = true; main.fics_playing = true; main.fics_connected = false; }
                "lichess-disconnected" => { main.fics_active = true; main.fics_playing = true; main.lichess.active = true; }
                _ => unreachable!(),
            }
            assert!(!main.external_3d_can_move(), "Must block {blocked}");
            assert!(!main.accept_external_3d_move(&proposed), "Must reject a queued move after {blocked}");
            assert_eq!(main.board, Board::default());
        }
        let mut main = app();
        main.fics_game_finished = true;
        assert!(main.external_3d_can_move(), "Old online metadata must not block a local game");
        main.fics_active = true;
        main.fics_game_finished = false;
        main.fics_playing = true;
        main.fics_connected = true;
        assert!(main.accept_external_3d_move(&request(&main, "e2e4")));
        assert!(main.fics_pending_move);
        assert_eq!(main.board, Board::default(), "Online moves await server confirmation");
        assert!(!main.external_3d_can_move());
    }

    #[test]
    fn contexts_change_when_authority_returns_to_same_position_or_study_is_replaced() {
        let mut main = app();
        let first = main.external_3d_context();
        assert_eq!(first, main.external_3d_context());
        main.prediction_index = 1;
        assert_ne!(first, main.external_3d_context());
        main.prediction_index = 0;
        assert_ne!(first, main.external_3d_context());
        let study = "[Event \"Lesson\"]\n[StudyName \"Same title\"]\n[ChapterName \"Chapter\"]\n\n1. e4 e5 *";
        main.open_study(study).unwrap();
        let previous = request(&main, "e2e4");
        main.open_study(study).unwrap();
        assert!(!main.accept_external_3d_move(&previous));
        assert!(main.external_3d_context().len() < 256);
    }

    #[test]
    fn child_promotions_offer_all_pieces_and_only_main_applies_them() {
        for piece in [Piece::Queen, Piece::Rook, Piece::Bishop, Piece::Knight] {
            let mut main = app();
            main.board = Board::from_str("7k/P7/8/8/8/8/8/7K w - - 0 1").unwrap();
            let mut child = app();
            child.sync_3d_companion_input(Some(&snapshot(&main)));
            child.select(Square::A7);
            child.select(Square::A8);
            assert_eq!(child.promotion, Some((Square::A7, Square::A8)));
            assert!(child.take_3d_companion_move().is_none());
            child.queue_3d_companion_move(ChessMove::new(Square::A7, Square::A8, Some(piece)));
            let proposed = child.take_3d_companion_move().unwrap();
            assert_eq!(child.board, main.board);
            assert!(child.promotion.is_none());
            assert!(main.accept_external_3d_move(&proposed));
            assert_eq!(main.board.piece_on(Square::A8), Some(piece));
            assert_eq!(main.review_moves.len(), 1);
        }
    }

    #[test]
    fn atomic_external_moves_support_castling_en_passant_and_study_variations() {
        for (fen, uci) in [
            ("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", "e1g1"),
            ("7k/8/8/3pP3/8/8/8/7K w - d6 0 1", "e5d6"),
            ("k7/8/8/8/8/8/8/4K1R1 w G - 0 1", "e1g1"),
        ] {
            let mut main = app();
            main.board = Board::from_str(fen).unwrap();
            let expected = main.board.make_move_new(ChessMove::from_str(uci).unwrap());
            assert!(main.accept_external_3d_move(&request(&main, uci)), "Expected legal {uci} in {fen}");
            assert_eq!(main.board, expected);
        }
        let mut main = app();
        main.open_study("[StudyName \"Lesson\"]\n[ChapterName \"Chapter\"]\n\n1. e4 e5 *").unwrap();
        assert!(main.accept_external_3d_move(&request(&main, "d2d4")));
        assert_eq!(main.board.piece_on(Square::D4), Some(Piece::Pawn));
        assert!(main.study.as_ref().unwrap().node > 0);
    }

    #[test]
    fn wrong_puzzle_attempt_gets_a_fresh_context_even_when_position_is_unchanged() {
        let mut main = app();
        let puzzle = serde_json::from_value(serde_json::json!({
            "data":{"game":{"pgn":"1. e4 e5 2. Nf3 *"},"puzzle":{"id":"external-test","rating":1200,"initialPly":0,"solution":["e7e5","g1f3"]}},
            "played":[],"done":false,"recorded":false,
        })).unwrap();
        main.restore_saved_puzzle(Some(&puzzle));
        let before = main.board;
        let wrong = request(&main, "d7d5");
        assert!(main.accept_external_3d_move(&wrong));
        assert_eq!(main.board, before);
        assert_ne!(main.external_3d_context(), wrong.context);
        assert!(main.accept_external_3d_move(&request(&main, "e7e5")));
        assert!(!main.external_3d_can_move(), "A completed puzzle does not accept more moves");
    }

    #[test]
    fn external_best_move_attempt_uses_the_attempt_path_without_extending_game_history() {
        let mut main = app();
        main.review_index = Some(0);
        main.best_move_attempt = Some(BestMoveAttempt {
            target_index: 1, best_move: ChessMove::new(Square::E2, Square::E4, None),
            attempted_move: None, revealed: false,
        });
        assert!(main.accept_external_3d_move(&request(&main, "d2d4")));
        assert_eq!(main.best_move_attempt.unwrap().attempted_move, Some(ChessMove::new(Square::D2, Square::D4, None)));
        assert!(main.history.is_empty() && main.review_moves.is_empty());
        assert_eq!(main.review_index, Some(1));
    }

    #[test]
    fn detached_drag_uses_3d_picking_in_both_orientations_without_local_moves() {
        for flipped in [false, true] {
            let main = app();
            let mut child = app();
            child.sync_3d_companion_input(Some(&snapshot(&main)));
            child.board_3d_active = true;
            child.flipped = flipped;
            let ctx = child.puzzle_context.clone();
            let rect = egui::Rect::from_min_size(egui::pos2(20.0, 20.0), Vec2::splat(600.0));
            let from = crate::board3d::square_center(Square::E2, rect, flipped, child.board_3d_view()).unwrap();
            let to = crate::board3d::square_center(Square::E4, rect, flipped, child.board_3d_view()).unwrap();
            for (pos, button) in [(from, None), (from, Some(true)), (to, None), (to, Some(false))] {
                let mut events = vec![egui::Event::PointerMoved(pos)];
                if let Some(pressed) = button { events.push(egui::Event::PointerButton {
                    pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE,
                }); }
                run_test_ui(&ctx, egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(800.0))),
                    events, ..Default::default()
                }, |ui| { child.piece_drag_ui(ui, rect, 75.0, true); });
            }
            assert_eq!(child.take_3d_companion_move().map(|m| m.uci), Some("e2e4".into()));
            assert_eq!(child.board, Board::default());
            assert!(child.history.is_empty());
        }
    }
}

#[derive(Clone, Default)]
struct InputState {
    context: String,
    fen: String,
    can_move: bool,
    pending: bool,
    queued: Option<MoveRequest>,
}

fn input_id() -> egui::Id { egui::Id::new("companion_3d_input") }
fn generation_id() -> egui::Id { egui::Id::new("companion_3d_generation") }

impl ChessApp {
    pub(super) fn board_move_input_allowed(&self) -> bool {
        !(self.fics_active && self.lichess.active && !self.lichess.connected())
            && (!self.fics_active || (self.fics_playing && !self.fics_pending_move
                && self.board.side_to_move() == self.player_side.color()))
            && (self.review_index.is_none() || self.best_move_attempt.is_some() || self.study.is_some())
            && self.promotion.is_none() && self.board.status() == BoardStatus::Ongoing
            && (self.best_move_attempt.is_some() || !self.engine_enabled
                || self.board.side_to_move() == self.player_side.color())
    }

    pub(super) fn external_3d_can_move(&self) -> bool {
        self.board_move_input_allowed() && self.prediction_index == 0
            && !self.companion_training_blocked() && (!self.fics_active || !self.fics_game_finished)
            && (!self.fics_active || self.lichess.active || self.fics_connected)
            && (self.study.is_some() || self.best_move_attempt.is_some() || self.game_result() == "*")
            && self.study.as_ref().is_none_or(|study| !study.practice
                || (self.board.side_to_move() == Color::Black) == study.practice_black)
            && (!self.puzzle_mode() || self.puzzle_move_input_allowed())
    }

    pub(super) fn invalidate_external_3d_context(&self) {
        self.puzzle_context.data_mut(|data| {
            let generation = data.get_temp::<u64>(generation_id()).unwrap_or(0);
            data.insert_temp(generation_id(), generation.wrapping_add(1));
        });
    }

    pub(super) fn external_3d_context(&self) -> String {
        let identity = serde_json::json!({
            "generation": self.puzzle_context.data(|d| d.get_temp::<u64>(generation_id()).unwrap_or(0)),
            "started": self.move_started_at.to_bits(), "fen": self.board.to_string(),
            "history": self.history.len(), "review": self.review_index, "prediction": self.prediction_index,
            "analysis": self.analysis_workspace_active(&self.puzzle_context),
            "playerWhite": self.player_side.color() == Color::White, "engine": self.engine_enabled,
            "training": self.training.as_ref().map(|session| &session.id),
            "online": [self.fics_active, self.lichess.active, self.fics_playing, self.fics_pending_move, self.fics_game_finished],
            "game": self.fics_game_id, "lichess": self.lichess.game["id"],
            "study": self.study.as_ref().map(|s| (&s.name, s.chapter, s.node, s.practice, s.practice_black, s.attempts)),
            "puzzle": self.puzzle_mode().then(|| self.puzzle_move_context()),
            "attempt": self.best_move_attempt.map(|a| (a.target_index, a.best_move.to_string(), a.attempted_move.map(|m| m.to_string()), a.revealed)),
            "canMove": self.external_3d_can_move(),
        });
        let fingerprint = egui::Id::new(identity.to_string()).value();
        let epoch = self.puzzle_context.data_mut(|data| {
            let id = egui::Id::new("companion_3d_context_identity");
            let (previous, epoch) = data.get_temp::<(u64, u64)>(id).unwrap_or((fingerprint, 0));
            let epoch = if previous == fingerprint { epoch } else { epoch.wrapping_add(1) };
            data.insert_temp(id, (fingerprint, epoch));
            epoch
        });
        format!("{fingerprint:016x}-{epoch:016x}")
    }

    pub(super) fn accept_external_3d_move(&mut self, request: &MoveRequest) -> bool {
        if !self.external_3d_can_move() || request.context != self.external_3d_context()
            || request.fen != self.board.to_string() || !matches!(request.uci.len(), 4 | 5) {
            return false;
        }
        let Some(mv) = Self::parse_uci_value(&request.uci)
            .filter(|mv| mv.to_string() == request.uci)
            .filter(|mv| MoveGen::new_legal(&self.board).any(|legal| legal == *mv)) else { return false; };
        // Invalid requests never disturb selection, promotion, or a game in the main window.
        if self.best_move_attempt.is_some() { self.submit_best_move_attempt(mv); }
        else { self.play(mv); }
        // A legal but incorrect puzzle/best-move attempt can leave the FEN unchanged.
        self.invalidate_external_3d_context();
        true
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn poll_external_3d_moves(&mut self, ctx: &egui::Context) {
        if let Ok(requests) = serde_json::from_str::<Vec<MoveRequest>>(&take_3d_moves()) {
            for request in requests.into_iter().take(32) {
                self.accept_external_3d_move(&request);
            }
        }
        if companion_3d_open() { ctx.request_repaint_after(std::time::Duration::from_millis(100)); }
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn publish_external_3d_position(&self) {
        publish_3d_position(&serde_json::json!({
            "fen": self.board.to_string(), "flipped": self.flipped,
            "canMove": self.external_3d_can_move(), "context": self.external_3d_context(),
            "theme": self.board_3d_theme,
            "appearance": if self.board_3d_active { self.stored_piece_appearance() } else {
                self.theme_adjustments.get(&theme_adjustment_key(true, self.piece_set, self.board_3d_theme)).copied().unwrap_or_default()
            },
            "coordinates": self.show_coordinates, "highlight": self.show_highlighted_move,
            "lastMove": self.last_move.map(|mv| [mv.get_source().to_string(), mv.get_dest().to_string()]),
            "light": self.show_radial_light,
        }).to_string());
    }

    pub(super) fn has_3d_companion_input(&self) -> bool {
        self.puzzle_context.data(|data| data.get_temp::<InputState>(input_id()).is_some())
    }

    pub(super) fn sync_3d_companion_input(&mut self, snapshot: Option<&serde_json::Value>) -> bool {
        let mut input = self.puzzle_context.data(|d| d.get_temp::<InputState>(input_id())).unwrap_or_default();
        let position = snapshot.and_then(|value| {
            if value["connected"].as_bool() == Some(false) { return None; }
            let fen = value["fen"].as_str()?;
            let context = value["context"].as_str().unwrap_or("");
            let board = Board::from_str(fen).ok()?;
            Some((board, fen.to_owned(), context.to_owned(), !context.is_empty() && value["canMove"].as_bool() == Some(true)))
        });
        let connected = position.is_some();
        let (fen, context, can_move) = if let Some((board, fen, context, can_move)) = position {
            self.board = board;
            (fen, context, can_move)
        } else { (String::new(), String::new(), false) };
        if input.fen != fen || input.context != context || input.can_move != can_move || !connected {
            self.clear_3d_companion_selection();
            input.pending = false;
            input.queued = None;
        }
        input.fen = fen;
        input.context = context;
        input.can_move = can_move;
        self.puzzle_context.data_mut(|data| data.insert_temp(input_id(), input));
        connected
    }

    fn clear_3d_companion_selection(&mut self) {
        self.selected = None;
        self.legal_targets.clear();
        self.promotion = None;
        self.piece_drag = None;
    }

    pub(super) fn companion_3d_can_move(&self) -> bool {
        self.puzzle_context.data(|d| d.get_temp::<InputState>(input_id()))
            .is_some_and(|input| input.can_move && !input.pending)
    }

    pub(super) fn select_3d_companion(&mut self, square: Square) {
        if !self.companion_3d_can_move() || self.promotion.is_some() { return; }
        if let Some(from) = self.selected.filter(|_| self.legal_targets.contains(&square)) {
            if self.board.piece_on(from) == Some(Piece::Pawn) && matches!(square.get_rank(), Rank::First | Rank::Eighth) {
                self.promotion = Some((from, square));
            } else { self.queue_3d_companion_move(ChessMove::new(from, square, None)); }
        } else if self.board.color_on(square) == Some(self.board.side_to_move()) {
            self.selected = Some(square);
            self.legal_targets = MoveGen::new_legal(&self.board).filter(|mv| mv.get_source() == square)
                .map(|mv| mv.get_dest()).collect();
        } else { self.clear_3d_companion_selection(); }
    }

    pub(super) fn queue_3d_companion_move(&mut self, mv: ChessMove) {
        if !self.companion_3d_can_move() || !MoveGen::new_legal(&self.board).any(|legal| legal == mv) { return; }
        self.puzzle_context.data_mut(|data| {
            let mut input = data.get_temp::<InputState>(input_id()).unwrap();
            input.queued = Some(MoveRequest { context: input.context.clone(), fen: input.fen.clone(), uci: mv.to_string() });
            input.pending = true;
            data.insert_temp(input_id(), input);
        });
        self.clear_3d_companion_selection();
    }

    fn take_3d_companion_move(&self) -> Option<MoveRequest> {
        self.puzzle_context.data_mut(|data| {
            let mut input = data.get_temp::<InputState>(input_id())?;
            let request = input.queued.take();
            data.insert_temp(input_id(), input);
            request
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn companion_3d_promotion_ui(&mut self, ctx: &egui::Context) {
        if let Some((from, to)) = self.promotion {
            let mut choice = None;
            let response = egui::Modal::new(egui::Id::new("companion_3d_promotion"))
                .frame(Self::dialog_frame()).show(ctx, |ui| {
                    ui.label("Promote pawn to");
                    ui.horizontal(|ui| {
                        for (piece, name) in [(Piece::Queen, "Queen"), (Piece::Rook, "Rook"), (Piece::Bishop, "Bishop"), (Piece::Knight, "Knight")] {
                            if ui.button(name).clicked() { choice = Some(piece); }
                        }
                    });
                });
            if let Some(piece) = choice { self.queue_3d_companion_move(ChessMove::new(from, to, Some(piece))); }
            else if response.should_close() { self.clear_3d_companion_selection(); }
        }
        if let Some(request) = self.take_3d_companion_move() {
            if !submit_3d_move(&serde_json::to_string(&request).unwrap()) {
                self.puzzle_context.data_mut(|data| {
                    let mut input = data.get_temp::<InputState>(input_id()).unwrap();
                    input.pending = false;
                    data.insert_temp(input_id(), input);
                });
            }
        }
    }
}

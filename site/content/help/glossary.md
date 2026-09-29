---
title: "Analysis glossary"
description: "Quick, plain-language definitions of the short labels and chess terms you may see in Ironwood."
category: "terms"
keywords: [glossary, CPL, PV, MultiPV, NPS, PGN, FEN, SAN, UCI, NNUE, FICS, Elo, symbols]
weight: 45
---

**Accuracy** — A 0–100% summary of how well a player's moves preserved the chances Stockfish saw. See [Accuracy and CPL](/help/accuracy-and-cpl/).

**CPL (centipawn loss)** — The estimated amount of evaluation given up on one move. One centipawn is one hundredth of a pawn in the engine's scoring system. Lower is usually better. See [Accuracy and CPL](/help/accuracy-and-cpl/).

**Depth** — How far ahead Stockfish searched along a line, counted in half-moves. More depth can reveal ideas that a shorter search misses.

**Evaluation (eval)** — Stockfish's estimate of who stands better. Positive favors White; negative favors Black. See [Evaluation and the graph](/help/evaluation/).

**Elo** — A rating system based on results against other rated players. See [What is Elo?](/help/elo-rating/).

**FEN** — A compact text description of one chess position, including whose turn it is. It records a position, not the history of a whole game. See [What is FEN?](/help/fen/).

**FICS** — The Free Internet Chess Server, which Ironwood can connect to for online games.

**Hash** — Engine memory used to remember positions it has already searched.

**MultiPV** — Multiple principal variations: several candidate engine lines shown together. See [Best move and engine line](/help/engine-line/).

**NNUE** — A type of neural-network evaluation used by Stockfish to judge positions. The engine still searches possible moves; NNUE helps it estimate how promising positions are.

**Nodes** — Chess positions examined by the engine during a search.

**NPS (nodes per second)** — How quickly the engine is examining positions on your device.

**PGN** — Portable Game Notation, a text format for a game's moves and optional annotations. See [What is PGN?](/help/pgn/).

**PV (principal variation)** — The engine's current sample line of strong moves for both sides. See [Best move and engine line](/help/engine-line/).

**SAN** — Standard Algebraic Notation, the familiar short chess move format such as `Nf3` for a knight move to f3. See [How to read chess moves](/help/chess-notation/).

**Move symbols** — Ironwood uses `★` for Best, `+` for Good, `?!` for Inaccuracy, `?` for Mistake, and `??` for Blunder. See [What do the move symbols mean?](/help/move-symbols/).

**UCI move** — A coordinate form of a move used by chess software, such as `g1f3` for a move from g1 to f3.

**Verification** — Ironwood's deeper second look at positions around moves that may have changed the game. See [Game phases and critical moves](/help/game-phases-and-critical-moves/).

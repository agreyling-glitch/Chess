---
title: "Evaluation and the graph"
description: "What a positive or negative score means, and how to read a swing in the graph."
category: "analysis"
keywords: [eval, score, graph, centipawn, white, black, mate]
weight: 20
---

## The score is from White's point of view

An evaluation is Stockfish's estimate of a position. A positive number favors White; a negative number favors Black. Around **0.00** means Stockfish sees a roughly balanced position. **+1.00** is an advantage for White worth about one pawn in the engine's scoring system; **−1.00** favors Black by about the same amount. It does **not** mean a pawn has necessarily been won or that the game is decided.

A **centipawn** is one hundredth of a pawn in that scoring system. A score of +0.35 is 35 centipawns in White's favor. The number expresses an estimated advantage, not a guaranteed result.

## Reading the graph

The graph plots the score after each move. A rise means the position improved for White; a fall means it improved for Black. A large change after a move is a good place to investigate. Click or tap that point, then compare the played move with Stockfish's suggestion.

The graph can move when the engine searches deeper. A flat graph also does not mean the moves were easy: both players may have found the only safe moves.

## Mate scores

**Mate in 3** means Stockfish sees a forced checkmate in three moves with best play. Mate scores are separate from pawn scores. A mate score is more decisive than a large numerical evaluation, and Ironwood treats mate-related moves separately when judging move quality.

## Analysis controls and positional tabs

Use **Expand** above the graph to open the full-screen evaluation view. **Print analysis**, the gold button beside it, opens the printable report prompt when engine results are available and analysis is paused or stopped.

Below the graph, **Summary** contains the game-review details. **Pawn Structure**, **Piece Mobility**, and **King Safety** explore the position and how its patterns change through the game. They work without a completed engine analysis; see [Positional insights](/help/positional-insights/) for controls and how to read each view.

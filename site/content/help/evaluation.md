---
title: "Evaluation and the graph"
description: "What a positive or negative score means, and how to read a swing in the graph."
category: "analysis"
keywords: [eval, score, graph, centipawn, white, black, mate]
weight: 20
lastmod: 2026-10-21T00:00:00-05:00
---

## The score is from White's point of view

An evaluation is Stockfish's estimate of a position. A positive number favors White; a negative number favors Black. Around **0.00** means Stockfish sees a roughly balanced position. **+1.00** is an advantage for White worth about one pawn in the engine's scoring system; **−1.00** favors Black by about the same amount. It does **not** mean a pawn has necessarily been won or that the game is decided.

A **centipawn** is one hundredth of a pawn in that scoring system. A score of +0.35 is 35 centipawns in White's favor. The number expresses an estimated advantage, not a guaranteed result.

## Reading the graph

The graph plots the score after each move. A rise means the position improved for White; a fall means it improved for Black. A large change after a move is a good place to investigate. Click or tap that point, then compare the played move with Stockfish's suggestion.

In the expanded graph, a light ivory area between the line and zero shows White's advantage; a dark charcoal area shows Black's advantage. The slate background distinguishes both fills. Unanalyzed positions leave gaps, and the gold line shows the evaluation itself.

The graph can move when the engine searches deeper. A flat graph also does not mean the moves were easy: both players may have found the only safe moves.

## Mate scores

**Mate in 3** means Stockfish sees a forced checkmate in three moves with best play. Mate scores are separate from pawn scores. A mate score is more decisive than a large numerical evaluation, and Ironwood treats mate-related moves separately when judging move quality.

## Analysis controls and positional tabs

Open the **Analysis** tab. Use **Expand** inside the graph’s upper-right corner to open the full-screen evaluation view. **Print analysis**, beside the **Saved Analysis** heading, opens the printable report prompt when engine results are available and analysis is paused or stopped.

Below the graph, **Summary** contains the game-review details. **Pawn Structure**, **Piece Mobility**, and **King Safety** explore the position and how its patterns change through the game. They work without a completed engine analysis; see [Positional insights](/help/positional-insights/) for controls and how to read each view.

In **Move Analysis**, the mini player below **Engine continuation** steps through the saved best line on the live board. Use First, Previous, Play/Pause, Next and Last without leaving Analysis. Playback defaults to 2 seconds per move and stops at the end. Right-click Play/Pause to choose Very Fast (0.5 seconds), Fast (1 second), Normal (2 seconds), Slow (3 seconds), or Analysis (5 seconds). The speed is shared with the Moves, Prediction and expanded graph players. Leaving the preview restores the played-game position; the game and saved analysis stay unchanged.

## Expanded graph and position preview

The expanded view includes move navigation, per-position details and notes, capture markers, and a framed board preview. Scroll down to advance a move or up to go back. The preview shows saved drawings and best-move arrows using the main board’s rules. Captured-piece markers always use Merida. See [Expanded game analysis graph](/help/expanded-analysis-graph/) for the full guide.

## Prediction versus Analysis

In **Prediction**, **Analyze position** searches the displayed position and fills **Engine prediction** with candidate continuations. In **Analysis**, **Analyze full game with Stockfish 19** examines the played game and builds its evaluation graph and summary. The live score is labeled **White perspective** regardless of which side is at the bottom. Full-game analysis temporarily holds live position analysis so they do not compete. Search limits are configured in [Engine settings](/help/search-depth-and-settings/).

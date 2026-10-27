---
title: "Expanded game analysis graph"
description: "Navigate the full graph, read capture markers, and inspect positions with drawings and engine arrows."
category: "analysis"
keywords: [expand, graph, captures, Merida, wheel, arrows, drawings, notes, preview]
weight: 21
lastmod: 2026-10-27T00:00:00-05:00
---

## Open the full graph

Open the **Analysis** tab and run **Analyze full game with Stockfish 19**, then choose **Expand** inside the graph’s upper-right corner. The expanded view shows the game title, analyzed-position count, evaluation graph, move details, notes, and a board preview. Use the close button to return to the workspace.

This **Expand** opens the played game's evaluation graph. The separate Expand beneath **Engine prediction** opens [predicted continuations as mini boards](/help/engine-line/#expand-the-prediction-into-boards).

## Read evaluations and phases

The gold line shows the evaluation from White's perspective. Above zero favors White; below zero favors Black. The ivory and dark areas show the corresponding advantage. The vertical scale is nonlinear: extreme scores are compressed, and mate scores reach the edges. Gaps indicate positions without analysis, rather than an equal evaluation.

Opening, middlegame, and endgame bands use the same approximate phase definitions as the summary. Colored markers identify inaccuracies, mistakes, and blunders. Note markers identify positions with written notes. See [Evaluation](/help/evaluation/), [Move quality](/help/move-quality/), and [Game phases](/help/game-phases-and-critical-moves/) for their meaning and limits.

## Navigate positions

- **Hover the graph** to inspect a position and its board temporarily.
- **Click or drag on the graph** to select a position in the game.
- Use **Previous**, **Next**, or the **Position** slider to select a move.
- **Scroll down** within the expanded view to advance one move; **scroll up** goes back. Wheel navigation keeps the selected position visible until you move the pointer and resume graph hovering.

Move details show the selected move, phase, evaluation, available search information, move-quality label, mate consequences, and best continuation. Results can change while analysis continues.

## Captured pieces

Two rows beneath the move labels show captures at the moves where they occurred. **White** contains captured white pieces; **Black** contains captured black pieces. These identify the lost pieces, rather than the capturing player's color. Hover an icon for the move and captured piece.

Capture icons always use **Merida**, independently of your board theme. They mark individual capture events, rather than a cumulative material total. En passant is included; promotion is not counted as a capture of the promoted pawn.

## Board preview, engine arrows, and drawings

The preview follows the position being inspected. It has a sunken frame and coordinates on all four sides, follows your board orientation, and uses your selected piece set. It displays the personal drawings saved for that position, including arrows, shapes, and ghost pieces.

Best-move arrows follow the main board's rules and the **Show best-move arrows** preference: for an inaccuracy, mistake, or blunder, the arrow indicates the engine's preferred alternative to the played move when available. It illustrates the earlier decision on the displayed board; it is not necessarily a legal next move from the position after that decision.

The preview displays annotations; use [Draw on board](/help/drawing-on-the-board/) on the main board to create or edit them. **Your note** below the details shows the selected position's writing. Choose **Add note…** or **Edit note…** to change it.

## Switch saved analyses without closing the graph

When the game has multiple saved analysis runs, numbered gray buttons appear beneath the engine and analyzed-position heading. Click a number or press **1–9** to select the corresponding run. The graph, highlighted button, and engine heading follow that saved run, and the main Saved Analysis dropdown stays synchronized. Hover a number to see that run's recorded engine, analysis budget, verification status, and engine options. Buttons beyond 9 remain clickable.

Switching is disabled during full-game analysis or while a run is loading. Number shortcuts do not select runs while you are typing. Choosing a saved run does not change the engine configured for the next analysis. [Saved analysis guide](/help/saved-analysis/).

---
title: "Accuracy and CPL"
description: "Read the side-by-side accuracy and average centipawn loss charts for both players and each game phase."
lastmod: 2026-10-28T00:00:00-05:00
category: "terms"
keywords: [CPL, ACPL, average centipawn loss, accuracy, percentage, pawn, loss, comparison, charts, gold, sage]
weight: 40
---

## Compare the players

After full-game analysis finishes, open **Analysis → Analysis Summary**. Two
comparison charts replace the separate player summary cards:

- **Accuracy (higher is better):** compare the players' percentages on a 0–100 scale.
- **Average centipawn loss (lower is better):** compare how much evaluation each
  player gave up per move. This chart adjusts its scale to fit the game's results.

Each chart groups the results into **Overall**, **Opening**, **Middlegame**, and
**Endgame**, with the players' bars side by side. **Gold represents White** and
**sage green represents Black**. The legend includes both names, and numerical
values appear above the bars. Both players share the same scale within each
chart; the accuracy and CPL charts use different units and scales.

Phase labels show moves per player: **10 moves each** means ten by White and ten
by Black. When the counts differ, **W** and **B** identify each player's count.
Hover over a bar for the player, phase, value, and move count. Click a phase bar
to jump to that player's first move in the phase. The current phase has a faint
gold background; the Overall group covers the whole analyzed game.

A dash means no result is available, rather than zero. **Not reached** means the
game never entered that phase. Mate-score moves do not contribute to average
CPL, so its evaluated-move coverage can differ from accuracy. Selecting another
[saved analysis run](/help/saved-analysis/) updates the comparison to that run.

Use the charts to choose positions to study, then inspect the moves themselves.
The [phase guide](/help/game-phases-and-critical-moves/) explains how the groups
are defined. Wait for any critical-move verification to finish before treating
the results as final.

## What does CPL mean?

**CPL** means **centipawn loss**. It estimates how much evaluation a player gave up with one move. A centipawn is one hundredth of a pawn in Stockfish's scoring system. For example, 25 CPL is an estimated loss of 0.25 pawns of advantage. This is a way to compare moves; it does not mean a quarter of a pawn disappeared from the board.

Ironwood compares the engine's score before and after the move from the moving player's point of view. Loss cannot go below zero. If the played move matches the engine's recommended move, Ironwood reports **0 CPL**, even when a later search changes the position's score. CPL is unavailable when either position has a mate score.

## What is average CPL?

**Average CPL** is the mean of a player's moves that have a CPL result. Lower generally means the player's moves stayed closer to the engine's evaluations. It is not a rating or a measure of playing strength. A forced or already lost game can make the average misleading, and mate-score moves are excluded from this average.

## What does accuracy mean?

**Accuracy** is Ironwood's 0–100% summary of move quality. For each move, it estimates the loss in winning chances from the evaluations, then averages those losses. Ironwood converts the average to a percentage: `100 × e^(−average loss ÷ 225)`. A higher number means the moves preserved more of the chances Stockfish saw.

Accuracy and average CPL use **different inputs**, so they need not move together. Accuracy uses Ironwood's winning-chance-based quality loss and includes its special handling of mate situations. Average CPL uses raw centipawn score changes and leaves out mate-score moves. Neither percentage nor CPL tells you *why* a move was good; inspect the position for that.

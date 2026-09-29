---
title: "Accuracy and CPL"
description: "Two different ways Ironwood summarizes how closely a player followed strong moves."
category: "terms"
keywords: [CPL, ACPL, average centipawn loss, accuracy, percentage, pawn, loss]
weight: 40
---

## What does CPL mean?

**CPL** means **centipawn loss**. It estimates how much evaluation a player gave up with one move. A centipawn is one hundredth of a pawn in Stockfish's scoring system. For example, 25 CPL is an estimated loss of 0.25 pawns of advantage. This is a way to compare moves; it does not mean a quarter of a pawn disappeared from the board.

Ironwood compares the engine's score before and after the move from the moving player's point of view. Loss cannot go below zero. If the played move matches the engine's recommended move, Ironwood reports **0 CPL**, even when a later search changes the position's score. CPL is unavailable when either position has a mate score.

## What is average CPL?

**Average CPL** is the mean of a player's moves that have a CPL result. Lower generally means the player's moves stayed closer to the engine's evaluations. It is not a rating or a measure of playing strength. A forced or already lost game can make the average misleading, and mate-score moves are excluded from this average.

## What does accuracy mean?

**Accuracy** is Ironwood's 0–100% summary of move quality. For each move, it estimates the loss in winning chances from the evaluations, then averages those losses. Ironwood converts the average to a percentage: `100 × e^(−average loss ÷ 225)`. A higher number means the moves preserved more of the chances Stockfish saw.

Accuracy and average CPL use **different inputs**, so they need not move together. Accuracy uses Ironwood's winning-chance-based quality loss and includes its special handling of mate situations. Average CPL uses raw centipawn score changes and leaves out mate-score moves. Neither percentage nor CPL tells you *why* a move was good; inspect the position for that.

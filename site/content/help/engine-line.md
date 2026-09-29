---
title: "Best move and engine line"
description: "Read Stockfish's suggestion and the sample continuation without treating it as a promise."
category: "analysis"
keywords: [best move, engine line, PV, principal variation, variation, multipv]
weight: 50
---

## Best move

The **best move** is Stockfish's current top choice in a position. It is the move the engine prefers at the search depth and settings used. A longer search can change the choice, especially in complicated positions.

## Engine line or PV

An **engine line** is a sequence of moves Stockfish expects if both sides continue with strong play. Engine programs call this the **principal variation**, or **PV**. Read it as “one likely strong continuation,” not as a prediction that a human opponent will play those exact moves.

For example, `1. e4 e5 2. Nf3` means White moves a pawn to e4, Black responds by moving a pawn to e5, and White moves a knight to f3. The letters name pieces and board squares. Clicking through the line on the board is often easier than reading the notation alone.

## Multiple lines

**MultiPV** asks Stockfish to show several leading candidate moves instead of only its top choice. This helps compare plans. The second line is not necessarily bad; it is simply ranked below the first by that search. Showing more lines may reduce how deeply each one can be examined with the same time or computing budget.

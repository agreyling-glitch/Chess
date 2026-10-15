---
title: "Best move and engine line"
description: "Read Stockfish's suggestion and the sample continuation without treating it as a promise."
category: "analysis"
keywords: [best move, engine line, PV, principal variation, variation, multipv]
weight: 50
lastmod: 2026-10-15T00:00:00-05:00
---

## Best move

The **best move** is Stockfish's current top choice in a position. It is the move the engine prefers at the search depth and settings used. A longer search can change the choice, especially in complicated positions.

## Engine line or PV

An **engine line** is a sequence of moves Stockfish expects if both sides continue with strong play. Engine programs call this the **principal variation**, or **PV**. Read it as “one likely strong continuation,” not as a prediction that a human opponent will play those exact moves.

For example, `1. e4 e5 2. Nf3` means White moves a pawn to e4, Black responds by moving a pawn to e5, and White moves a knight to f3. The letters name pieces and board squares. Clicking through the line on the board is often easier than reading the notation alone.

## Multiple lines

**MultiPV** asks Stockfish to show several leading candidate moves instead of only its top choice. This helps compare plans. The second line is not necessarily bad; it is simply ranked below the first by that search. Showing more lines may reduce how deeply each one can be examined with the same time or computing budget.

## Expand the prediction into boards

Click the gold **Expand** button beneath the **Engine prediction** table in Real-Time Analysis. A separate window shows one mini board after each predicted move, with its move label and highlighted source and destination squares. Click a mini board to inspect that position on the main board when active play or training does not prevent navigation.

The title includes the variation number, evaluation, first move, and search depth. Each **Expand** click captures a separate prediction: change the selected variation or analyze another position, then expand again to compare lines side by side. Existing windows keep their captured moves rather than following later engine updates.

Resize a window to scale its boards and change the grid layout, or use the title's **Maximize** and **Restore** controls. Small windows keep pieces readable and scroll additional rows. Each window has a gold border and one shared sunken panel with a visible paper-grain background. Closing one leaves the others open.

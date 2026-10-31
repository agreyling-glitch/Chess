---
title: "Move labels"
description: "Understand Best, Good, Inaccuracy, Mistake, and Blunder in Ironwood."
lastmod: 2026-10-31T00:00:00-05:00
category: "analysis"
keywords: [best, good, inaccuracy, mistake, blunder, quality, threshold, classification]
weight: 30
---

Ironwood compares a played move with Stockfish's recommendation and with the position before and after the move. A label answers **how much the move seems to have hurt the player's chances**, not whether the move looked attractive.

| Label | Plain meaning |
| --- | --- |
| **Best** | The engine's recommended move, or a move with almost no estimated loss. |
| **Good** | A sound move that gives away little. |
| **Inaccuracy** | A small missed opportunity. |
| **Mistake** | A meaningful loss of chances. |
| **Blunder** | A severe loss of chances. |

Ironwood converts evaluations into an estimate of winning chances before setting these labels. Its cutoffs on that *quality-loss scale* are 0–10 for Best, 11–50 for Good, 51–100 for Inaccuracy, 101–200 for Mistake, and above 200 for Blunder. **These are not CPL cutoffs.** The same centipawn change can matter much more in an equal position than in a position that is already won.

If the played move matches Stockfish's recommended move, Ironwood marks its quality loss as zero. Forced mate situations receive special treatment: finding or escaping mate is rewarded, while allowing or missing a forced mate is heavily penalized. Results may change after deeper verification.

Use a label as a pointer to a position worth studying. Look at the board and the [engine line](/help/engine-line/) before deciding why the move mattered.

## Compare move quality by player

Open **Analysis → Critical Analysis** after full-game analysis. **Move quality**
is expanded by default and can be collapsed. Each
player has a stacked horizontal bar showing the share of their classified moves
in each category. The legend uses Ironwood's category colors: gold for Best,
green for Good, yellow for Inaccuracy, orange for Mistake, and red for Blunder.

The total beside the player's name counts classified moves. Segment widths show
proportions, so each player's bar fills the same width even if their move totals
differ. Counts appear inside segments when there is room; the breakdown below
always lists every category, including zeros. Hover over a segment for its count
and percentage. Click a segment or a nonzero count below it to jump to that
player's next matching move.

**Forced mate missed** and **Forced mate allowed** cards show totals across both
players. Finding or escaping a forced mate also gets a card when present. Click
a nonzero card to jump to the next matching move. These events are already
included in the move-quality classifications; do not add the cards to the bar
totals as extra moves.

---
title: "Move labels"
description: "Understand Best, Good, Inaccuracy, Mistake, and Blunder in Ironwood."
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

---
title: "Review time use and move quality"
description: "Read time-per-move bars, remaining-clock lines, and the time-versus-quality quadrant."
category: "analysis"
keywords: [time, clocks, increment, time trouble, quadrant, CPL, rushed errors, budget]
weight: 23
lastmod: 2026-10-31T00:00:00-05:00
---

Open **Analysis** and choose a timing tab below the evaluation graph. The views use recorded move timings and the selected analysis run. Hover chart marks for move details; click to review that position. Missing data is left unavailable rather than invented.

## Time per move

Separate charts show White and Black's seconds spent on each move. Colors suggest positions to review:

- **Gray — Routine / unclassified:** other recorded moves, including moves without a quality classification.
- **Blue — Deep think, good move:** more than 20 seconds on an analyzed Best or Good move.
- **Orange — Long think, likely easy move:** more than 20 seconds on a Best or Good move that was the only legal move, or an engine-approved recapture with neither the starting nor resulting position in check.
- **Red — Rushed, then erred:** 5 seconds or less on an analyzed Mistake or Blunder.

These are review prompts, not proof of a position's difficulty. Before spending more than about 20 seconds, ask whether this is a critical moment: a pawn break, a forced sequence, or a tactic that could swing the game. Repeated long thinks on easy recaptures alongside quick tactical mistakes can suggest a need to reallocate time.

## Clock burn-down

White's remaining clock is solid blue; Black's is dotted orange. The dashed even-pace budget spreads the starting clock to retain 60 seconds at move 40, or the starting amount if it was below 60 seconds. The spending allowance includes the increment. The red line marks 60 seconds.

Falling below the budget after the opening can suggest early overspending. Steep drops identify expensive thinks. Aim to reach around move 30 with a cushion above the reference line, while adapting to your time control. Sixty seconds is a review guide, not a universal danger threshold.

Actual clocks include increments and can rise when a player spends less than the increment. The budget is omitted when the starting time and increment are unavailable. Missing clock records break the plotted line rather than implying a known clock history.

## Time spent vs. move quality quadrant

Each player has a scatter chart: seconds spent on the horizontal axis and estimated centipawn loss on the vertical axis. Lower loss is better. Dashed guides divide the chart at **20 seconds** and **80 estimated CPL**; these fixed guides are separate from Ironwood's move-quality classification thresholds.

- **Top left — Rushed errors:** add a blunder check, such as “What does their last move threaten?”
- **Top right — Thought hard, still wrong:** work on a candidate-move routine and avoid replaying the same line repeatedly.
- **Bottom right — Time well spent:** longer thinks that preserved the position well.
- **Bottom left — Routine:** quick moves with lower estimated loss.

Points are gray for quick and fine, red for quick and costly, blue for slow and accurate, and gold for slow and still costly. “Slow” means more than 20 seconds; “costly” means at least 80 estimated CPL. Only moves with both recorded elapsed time and numerical CPL are plotted. Mate transitions and missing estimates are omitted. Deeper analysis or a different saved run can change the plotted losses.

See [PGN timings](/help/pgn/#time-spent-and-time-remaining), [Move quality](/help/move-quality/), and [Compare saved analyses](/help/saved-analysis/).

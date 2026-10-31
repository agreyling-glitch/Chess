---
title: "Where the thinking time goes"
date: 2026-10-31T00:00:00-05:00
description: "Three new analysis views connect thinking time, clock budgets, and move quality."
slug: "where-the-thinking-time-goes"
---

A game review can show where a mistake happened without explaining how you arrived there. Did you move before checking the opponent's threat? Or spend a minute calculating, then choose the wrong candidate anyway? Those are different habits to practice.

Ironwood now has three timing views beside Summary and Critical Analysis. Each chart connects back to the board: hover for details, then click a move to study it. The sunken sections and faint grain keep the charts consistent with the rest of the workspace.

## Start with the individual thinks

**Time per move** shows each player's recorded thinking time. Orange flags long thinks on likely easy moves; red flags quick moves that became mistakes. These are conservative prompts. A recapture can still deserve thought, and a quick mistake can happen even when the clock leaves little choice.

Before another long think, ask: “Is this a critical moment?” A pawn break, a forcing sequence, or a tactical threat can justify spending time. A pattern of spending on easy replies and rushing difficult choices is worth reviewing across several games.

## Watch the clock as a budget

**Clock burn-down** puts both clocks beside an even-pace guide and a 60-second reference. A steep drop makes an expensive decision visible. Falling below the budget early can help explain why later moves felt rushed.

In increment games, the clock can rise. Ironwood keeps those actual values: the chart is a record of time remaining after moves, not a forced downward curve. The budget explains its assumptions, and it is omitted when the time control is unavailable.

## Separate speed from the thinking process

The **Time spent vs. move quality quadrant** compares seconds spent with estimated centipawn loss. The top-left area suggests slowing down for a blunder check. The top-right area suggests improving the process: name candidate moves, check the opponent's replies, and avoid circling through the same line. The bottom-right area highlights longer thinks that preserved the position well.

The guides at 20 seconds and 80 CPL are fixed review aids, not universal definitions of good time use. Your time control and the position still matter. Missing timings and mate transitions without numerical CPL do not become invented points.

## Keep each review with its game

The details panel now separates the overall comparison charts in **Summary** from the turning point, expanded Move quality breakdown, and move-by-move review in **Critical Analysis**.

This update also fixes a storage boundary: explicitly saving a temporary profile review now creates its own game entry. Previously, that action could retain older analyses from the game opened before it. Position checks prevent applying those results to the wrong game, and failed selections now explain the mismatch. Existing saved payloads remain intact; older mixed records are not automatically repaired.

The [timing guide](/help/time-management/) explains each color and threshold. The [changelog](/changelog/2026-10-31-analysis-tabs-and-time-review/) lists the accompanying interface and saved-analysis fixes.

---
title: "Analysis workspace and portable reports"
date: 2026-09-22T00:00:00-05:00
description: "Full-game review, lossless analysis files, annotated PGN, and printable critical-position reports."
slug: "2026-09-22-analysis-workspace"
version: "2026.09.22"
---

## Added

- Full-game Stockfish analysis with configurable node budgets, pause, resume, stop, and progress controls.
- Accuracy, average centipawn loss, move-quality classifications, an evaluation graph, and best-move guidance.
- Interactive game and engine-prediction move lists with keyboard navigation and board highlighting.
- Versioned Ironwood analysis JSON for lossless export and restoration of engine results.
- Annotated PGN export and re-import of Ironwood evaluation, depth, node, best-move, and principal-variation annotations.
- A printable analysis report with a complete annotated move list, quality-colored markers, evaluations, critical-position diagrams, played-move highlights, Stockfish best-move arrows, centipawn loss, and engine lines. Use the browser's **Save as PDF** destination to create the PDF.

## Improved

- Imported games and their analysis survive refreshes and installed-app restarts.
- Export commands distinguish between unavailable, running, paused, partial, and completed analysis.
- The installed app now offers controlled updates without discarding games or unnecessarily downloading the large offline engine again.

All analysis still runs locally in the Stockfish Worker. Exporting a report does not upload the game or send positions to a report service.

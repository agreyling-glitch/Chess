---
title: "Move timings in PGN"
date: 2026-10-29T00:00:00-05:00
slug: "2026-10-29-pgn-move-timings"
version: "2026.10.29"
description: "Record time spent and time remaining per move, and preserve imported PGN timings."
---

- FICS history Review preserves per-move elapsed times and reconstructs remaining clocks from the initial time and increment when available.
- Lichess profile Review now requests clock history and calculates missing elapsed times using the increment, including recent and notable games.
- Added a Time column to Game Moves, with White and Black elapsed times stacked and remaining clocks in hover details.
- Added per-move time spent (`%emt`) and time remaining (`%clk`) to plain PGN, annotated PGN, and copied moves.
- Record local clock time after increment; untimed games include time spent only.
- Preserve imported mainline timing values and fractional precision, plus the imported TimeControl header.
- Keep timing with saved games, backups, and analysis JSON. Undo and branching discard timings for discarded moves.
- Record server-provided clocks for FICS and Lichess without inventing elapsed times for missing move history.

See [PGN move timings](/help/pgn/#time-spent-and-time-remaining) for examples and availability.

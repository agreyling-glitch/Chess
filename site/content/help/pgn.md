---
title: "What is PGN?"
description: "Understand the portable game file that records moves, results, and optional notes."
category: "terms"
keywords: [PGN, Portable Game Notation, import, export, moves, result, annotations]
weight: 86
---

**PGN** stands for **Portable Game Notation**. It is a text format for a *whole chess game*. It can include player names, date, result, the moves, and comments. Most chess software can read it.

Here is a short example:

```text
[White "Alex"]
[Black "Sam"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 3. Bb5 a6 1-0
```

The lines in brackets are **tags** with game details. `1.` starts White's first move; Black's reply follows. `1-0` means White won, `0-1` means Black won, `1/2-1/2` means a draw, and `*` means no result is recorded. The moves use [standard algebraic notation](/help/chess-notation/).

In Ironwood, you can import PGN to replay and analyze a game. Use the game's copy or export controls to share the PGN, either through the selected move or for the full game. Ironwood can include supported analysis comments in exported PGN, including evaluation, depth, nodes, CPL, and a suggested line. A PGN with those comments can restore some analysis when imported; a plain PGN supplies the moves and can be analyzed again.

For a game that begins from a custom position, PGN can include `SetUp` and `FEN` tags. The FEN says where the board started; the PGN then records what happened next. For Ironwood's richer saved analysis, use its [analysis JSON](/help/saving-and-sharing/) export.

For completed Lichess games, Ironwood can fetch the PGN for you. See [Import games from Lichess](/help/import-from-lichess/).

Open **Game History → Import…** to paste a game or collection, fetch games from Lichess or Chess.com, or choose **Scoresheet → Enter scoresheet** to enter and validate moves beside a reference photo. See [Import games](/help/import-games/) for all four tabs and [Import a game from a scoresheet](/help/import-a-scoresheet/) for manual entry.

## Time spent and time remaining

Ironwood records move times for new games and includes them in both plain and annotated PGN exports, including copied moves. Each move can carry two comments:

```text
1. e4 { [%emt 0:00:08.250] [%clk 0:04:53.750] }
```

The **Game Moves** table has a **Time** column after the White and Black moves. It shows time spent in seconds, with White above Black and a matching color marker beside each value. Hover over either time to see its full time spent and remaining clock. A dash means the time was not recorded; a missing move leaves its line blank. The column is available in both the main window and the detached Study panel.

**`%emt`** is the time spent on that move. **`%clk`** is that player's time remaining after the move, including any increment. Values use hours, minutes, and seconds; newly recorded local timings include milliseconds. Untimed games record time spent without a remaining clock.

Imported mainline `%emt` and `%clk` comments retain their values and precision when you save, copy, or export the game. Timing also travels with saved games, backups, and analysis JSON. Missing elapsed times in Lichess PGNs are calculated from consecutive clocks for the same player and the recorded increment. Explicit `%emt` values are preserved. Missing clocks, an unknown increment, or an unexplained clock increase leave the time unknown.

For online games, Ironwood records timings supplied by the server. FICS supplies the previous move's elapsed time and remaining clock. Lichess elapsed time is derived only when consecutive clock updates and the increment are available. Earlier moves missed during a late join or reconnect keep unknown timings unless they are supplied by an imported PGN. Games saved before timing recording was added cannot recover unrecorded move times.

FICS **history Review** also preserves the elapsed time shown beside each move in the server’s move list. When the history header provides the initial time and increment, Ironwood reconstructs each player’s remaining clock as previous time minus time spent plus increment. Missing move times break that player’s remaining-clock calculation; elapsed times supplied later are still shown. If the header is missing or the game is untimed, only the supplied elapsed times are displayed. Reopen a history Review after updating to retrieve these timings.

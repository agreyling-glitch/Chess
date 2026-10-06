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

Open **Game → Import…** to paste a game or collection, fetch games from Lichess or Chess.com, or choose **Scoresheet → Enter scoresheet** to enter and validate moves beside a reference photo. See [Import games](/help/import-games/) for all four tabs and [Import a game from a scoresheet](/help/import-a-scoresheet/) for manual entry.

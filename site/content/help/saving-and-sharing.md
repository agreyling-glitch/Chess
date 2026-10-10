---
lastmod: 2026-10-10T00:00:00-05:00
title: "Saving and sharing analysis"
description: "Understand saved games, PGN, analysis JSON, and printable reports."
category: "using-ironwood"
keywords: [save, export, import, PGN, JSON, PDF, print, report, local]
weight: 80
---

Ironwood keeps saved games in your browser's local storage. A **PGN** file is a portable text record of chess moves. You can import a PGN to review a game, and export one to share its moves and supported annotations. See [What is PGN?](/help/pgn/) for an example.

An **Ironwood analysis JSON** file stores Ironwood's richer analysis data so you can restore results without rerunning Stockfish. JSON is a structured text file for software; you do not need to edit it by hand. If you share it, remember that game details and any notes in it may go with the file.

Choose **Print analysis** at the right of Game Analysis to open a **printable analysis report** with scores, move quality, phases, and critical positions. Its **Your notes and drawings** section includes written notes, manual symbols, and diagrams for positions with personal drawings or ghost pieces. Drawing-only positions are included even if they are not critical moves. Pause or stop analysis before printing. Use your browser's Print dialog to save the report as a PDF if you want a document to keep or share.

Analysis runs on your device. Online games and public help pages need an internet connection; saved games and already cached game resources may be available offline.

For instructions on Starting notes, move notes, and manual symbols, see [Taking notes and adding annotations](/help/taking-notes/). Annotated PGN and Analysis JSON preserve them; plain PGN omits them.

[Board drawings](/help/drawing-on-the-board/) also travel with annotated PGN and Analysis JSON. Saved games and Analysis JSON preserve colors, endpoints, solid/dotted squares and circles, translucent filled squares, crosses, ghost pieces, and solid/dashed/curved/L-shaped knight arrows. Annotated PGN uses arrow and square comments (`[%cal ...]` and `[%csl ...]`) plus Ironwood’s `[%iw_arrow_styles ...]` extension. Other chess programs may ignore the style extension and display solid arrows and square outlines instead. Ghost pieces and their colors use the `[%iw_ghosts ...]` extension, which other apps may ignore. Plain PGN omits personal drawings.

[Tactical Map](/help/tactical-map/) findings are generated from the position rather than saved as personal annotations. Drawing undo history is temporary and is not included in exports.

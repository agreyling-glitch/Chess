---
title: "Private analysis, portable results"
date: 2026-09-22T00:00:00-05:00
description: "How Ironwood turns local Stockfish analysis into JSON, annotated PGN, and readable study reports."
slug: "private-analysis-portable-results"
---

Ironwood's analysis workspace now follows a game from import to review without moving the game to a server. Stockfish 19 evaluates each position in its dedicated browser Worker, the board remains available for navigation while analysis runs, and the resulting timeline can be saved in formats intended for different jobs.

## One analysis, three forms

The versioned Ironwood JSON format is the lossless archive. It records the position timeline, evaluation, mate score, depth, node count, best move, principal variation, engine configuration, and summary measurements. Re-importing it restores the analysis immediately rather than asking Stockfish to calculate the game again.

Annotated PGN is the interoperable chess record. Ironwood writes standard PGN moves with structured evaluation directives and readable comments. It can now restore its own evaluation, depth, node, best-move, and principal-variation annotations when that PGN is imported again. Ordinary PGN remains useful too: it loads the game and leaves analysis ready to run.

The printable report is for reading and teaching. Its first page includes a complete score sheet with every move, quality symbol, evaluation, and critical centipawn loss. A color-and-symbol legend keeps the notation understandable in color or grayscale.

The following pages focus on critical positions instead of producing a full diagram for every half-move. Each inaccuracy, mistake, or blunder receives a diagram of the decision position. The played move is highlighted with its quality color and a muted green arrow shows Stockfish's preferred alternative. The accompanying notes record the resulting evaluation, centipawn loss, search depth, nodes, and principal variation. A summary places both players' accuracy and average centipawn loss at the front.

## Why the browser prints the PDF

Ironwood builds a self-contained, print-optimized report locally and opens the browser's print dialog. Choosing **Save as PDF** uses the platform's mature PDF renderer, keeps board diagrams sharp, and avoids adding a second large document engine to the game download. It also leaves paper size, margins, page selection, and physical printing under the player's control.

No report server is involved. The same local-first boundary used for analysis applies to presentation: the imported game, engine results, and finished document remain on the player's device unless the player chooses to share them.

## A foundation for deeper review

Critical-position reports are the first report style. The same structured analysis can later support every-move workbooks, selected-position collections, opening summaries, and human-readable explanations. JSON remains the durable source, PGN remains the chess interchange format, and PDF becomes the portable reading format.

[Analyze a game in Ironwood →](/play/)

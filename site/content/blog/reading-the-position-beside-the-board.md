---
title: "Reading the position beside the board"
date: 2026-10-06T00:00:00-05:00
description: "A wider analysis workspace connects move history, engine lines, positional patterns, opening statistics, and easier game imports."
slug: "reading-the-position-beside-the-board"
---

A score tells you that a position changed, but understanding the change takes more than a number. A pawn move can break a chain, a piece can lose its useful squares, or a king can become exposed before the engine's preferred continuation makes the reason clear. This update puts those questions beside the board and gives each one a view of its own.

## Follow the game from left to right

The workspace now follows a simple order: **Game Moves**, the **live board and balance bar**, **Real-Time Analysis**, and **Game Analysis**. The compact engine panel has a fixed width, leaving the remaining space for the game summary and study tools.

Game Moves starts higher without the result banner, and its navigation controls follow the board's bottom coordinates. The balance bar follows the board edge, with consistent spacing whether the border is enabled or hidden. Moving between positions holds the previous evaluation while Stockfish works, then animates the bar to its new balance over 350 milliseconds. A refreshed review also starts live analysis once the engine is ready.

Real-Time Analysis gives the score a larger, centered gold-accented card. Its centered **Analyze position** button sits above a White/Black prediction table. Reading an engine line vertically now feels closer to reading the game itself, and the prediction table displays the full continuation without its own scrolling area.

## Ask what changed in the position

Below the evaluation graph, **Summary**, **Pawn Structure**, **Piece Mobility**, **King Safety**, and **Opening Explorer** offer different ways to examine the selected position.

**Pawn Structure** shows chains, isolated pawns, and passed pawns on a miniature board beside compact summaries for both colors. Its evolution table records pawn changes rather than repeating every position. Ten entries stay visible, with the remainder scrollable; select a row to see the structure at that point in the game.

**Piece Mobility** follows a piece's available destinations over time. Click a piece on the miniature board or main board to select it, and **Shift-click** another to compare them. The graph can help reveal when a piece becomes restricted or gains room. Its counts respect pins and king safety, exclude castling and en passant, and stop when a piece is captured. A high count alone does not establish that the piece is well placed.

**King Safety** highlights shelter pawns, attacked squares, and nearby pawn-open files. Its evolution table lets you revisit changes in either king's shelter or pressure. These are structural observations: pressure includes pinned attackers, and a pawn-open file may still contain other pieces. Use the engine to investigate whether a weakness leads to a concrete attack.

The [positional insights guide](/help/positional-insights/) explains the tools and their limits.

## Explore an opening without rewriting the game

**Opening Explorer** shows common continuations from Lichess's Masters and player-game databases. Each row pairs a move and game count with a White/draw/Black results bar. The Lichess games source also offers speed and rating filters. Those outcomes describe the database's games; they are not an engine recommendation.

Click a continuation to preview it on the main board, then explore further. **Back** retraces the preview and **Back to game** returns to the selected position. Your game history stays intact. When Lichess supplies an ECO code, the **ChessCodex** link opens the corresponding opening category in a new tab.

Opening Explorer requires a Lichess token with no permissions selected. The masked connection field keeps it in memory for the app session; refreshing clears it. This token is separate from game imports, which need no login. The [Opening Explorer guide](/help/opening-explorer/) covers connection setup and previews.

## Bring in the games you want to study

The **Import games** window now keeps its size as you switch among **Paste a game or collection**, **Lichess**, **Chess.com**, and **Scoresheet**. Both online sources offer a game count, time-control filter, and optional UTC date range. Choose up to 20, 50, 100, or 200 matching games, or reset the filters to start again. A completed Lichess game link still imports that individual game.

Chess.com imports use public player archives, so a recently finished game may take time to appear. After fetching from either source, review the returned collection and select the games to keep. Importing and full-game analysis are separate steps: open an imported game and start its analysis when ready. Analysis JSON can restore results you already exported.

The Scoresheet tab explains how to add an optional reference photo, enter and validate moves, and import the game. The photo remains a manual-entry reference. See the [import guide](/help/import-games/) for the complete workflow.

## Choose a board and keep the workspace readable

**Board → Theme** gathers 2D and 3D previews in one full-size chooser. Theme names appear above the boards, and gold guidance marks the selection action. Double-click a preview to apply it and close the chooser, or right-click the live board's 2D/3D control to open it. Other 3D settings remain in the Board menu.

Captured pieces use the Cburnett set regardless of theme, with larger icons beneath the player names. The analysis headings, selectors, table columns, and navigation controls have also received spacing and alignment updates. Mobile visitors now see a desktop-only message before the chess app or engine starts downloading.

The [Gallery](/gallery/) separates Game and Analysis and includes the new positional views and full workspace. The [October 6 changelog](/changelog/2026-10-06-positional-insights-and-analysis-layout/) lists the additions and fixes, while [Features](/features/) and Help provide the current reference.

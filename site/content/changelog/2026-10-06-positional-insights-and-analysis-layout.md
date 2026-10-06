---
title: "Positional insights and a new analysis workspace"
date: 2026-10-06T00:00:00-05:00
description: "Pawn Structure, Piece Mobility, King Safety, Opening Explorer, Chess.com imports, improved filters, and a redesigned analysis layout."
slug: "2026-10-06-positional-insights-and-analysis-layout"
version: "2026.10.06"
---

## Added

- **Pawn Structure** below the game evaluation graph. Inspect pawn chains, isolated pawns, and passed pawns on a miniature board beside aligned White and Black summaries. The structure evolution table lists pawn changes, displays ten entries at a time, and scrolls through the rest. Select an entry to inspect its position.
- **Piece Mobility** with a graph of available destinations across the game, a miniature board, and two piece selectors. Click a piece on either board to select it; **Shift-click** to compare a second piece. Click or drag the graph to inspect a position. Mobility respects pins and king safety, excludes castling and en passant, and ends a captured piece's line.
- **King Safety** with White and Black king selection, highlighted shelter and pressure squares, structural summaries, and a safety evolution table. The assessment describes pawn shelter, nearby pawn-open files, and enemy pressure independently of engine evaluation.
- **Opening Explorer**, with **Masters** and **Lichess games** sources, speed and rating filters for Lichess games, game counts, and White/draw/Black result bars. Explore continuations on the main board without editing the game; use **Back** or **Back to game** to leave a preview.
- A masked, session-only Lichess token connection for Opening Explorer. Create a token with no permissions selected. Reloading clears it; Disconnect also clears cached results. Authentication, connection, and rate-limit errors have distinct guidance.
- A **ChessCodex** opening reference link based on the ECO code returned by Lichess. It opens in a new browser tab.
- **Chess.com game import** from public player archives, alongside Lichess import. Neither game-import source requires a login or token.
- Shared online import filters for **20, 50, 100, or 200 games**, time control, and optional UTC date ranges, with **Reset filters**. Filters apply to usernames; individual Lichess game links still fetch one game.
- A desktop-only entry screen for mobile devices, preventing the chess app and engine from starting or downloading there.

## Improved

- The workspace now places **Game Moves → live board and balance bar → Real-Time Analysis → Game Analysis** from left to right. Real-Time Analysis has a fixed compact width; Game Analysis fills the remaining horizontal space.
- The balance bar follows the board's height, attaches beside its border when enabled, and keeps consistent gaps around it. It retains its previous score while a new evaluation loads and animates to the new balance over 350 milliseconds.
- Game Moves no longer shows the result banner above the table. Its table follows the board dimensions, with navigation controls aligned to the bottom coordinates.
- Real-Time Analysis has a fixed heading matching Game Analysis, a larger centered score in a gold-accented card, and a centered gold **Analyze position** button. The redundant local-engine caption was removed.
- Engine predictions use an aligned White/Black move table instead of a horizontal move list. The entire prediction line is displayed without an inner scrolling area; section spacing, table gutters, and centered navigation controls improve readability.
- Game Analysis has **Summary**, **Pawn Structure**, **Piece Mobility**, **King Safety**, and **Opening Explorer** tabs beneath the graph. **Expand** and the gold **Print analysis** button sit beside **Analyze again**. Pawn Structure is excluded from the expanded graph window.
- **Board → Theme** opens a full-size chooser with 2D and 3D previews. Theme names sit above each preview, with gold selection guidance on the board. Double-click a preview to select it and close the chooser; right-click the live board's 2D/3D control to open it. Other 3D settings remain under Board.
- Captured pieces always use the Cburnett set, with larger icons and spacing that accommodates them beneath player names.
- The fixed-size **Import games** window has **Paste a game or collection**, **Lichess**, **Chess.com**, and **Scoresheet** tabs styled like Engine Settings. Online import panels have clearer inputs, filters, fetch controls, and status messages. Scoresheet import has a three-step guide.
- Removed the full-game-analysis checkbox from import. Import games first, then start full-game analysis from Game Analysis when ready.
- The Gallery now separates **Game** and **Analysis**, includes screenshots of all three positional tools, and shows the redesigned full-analysis workspace.
- Updated Features and Help for positional insights, Opening Explorer, theme selection, imports, filters, and related game-review workflows.

## Fixed

- Restored game reviews schedule real-time analysis after Stockfish becomes ready following a browser refresh.
- Main-board click and Shift-click selection in Piece Mobility no longer depends on the former combined Moves/Analysis panel.
- Replaced unsupported player and mobility legend glyphs with drawn markers.
- Corrected clipping and spacing around captured pieces, prediction tables, and analysis headings.
- Opening Explorer distinguishes rejected tokens from general connection failures and handles request caching and rate limits.

See the [positional insights guide](/help/positional-insights/), [Opening Explorer guide](/help/opening-explorer/), [import guide](/help/import-games/), and updated [Gallery](/gallery/).

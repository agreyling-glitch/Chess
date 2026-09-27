---
title: "Game review and saved-library improvements"
date: 2026-09-27T12:00:00-05:00
description: "Move notes, a full-screen analysis graph, board-image copying, and multi-game library actions."
slug: "2026-09-27-game-review-and-library"
version: "2026.09.27"
---

## Added

- Personal text notes on moves and the starting position, with markers in Game Moves and the evaluation graph. Notes appear in annotated PGN, Analysis JSON, and printed reports; they remain separate from engine analysis.
- A full-screen analysis graph with evaluation gridlines, move labels, mistake markers, position navigation, and detailed information for the selected position.
- **Copy Board to Clipboard** in the Game Moves right-click menu, creating a PNG of the selected position with the current board appearance. The menu also offers selected-position FEN and partial or full PGN copying.
- Multi-select in Saved Games: choose individual games, a page, or all matches, then delete the selection or export it as PGN, annotated PGN, or Analysis JSON. Multiple JSON games download as a ZIP of individual files.
- A maximized Saved Games view and a larger modal for each game's final-board preview.

## Improved

- A dark board frame with gold coordinates and piece shadows are enabled by default.
- The fixed-size New Game window separates **You vs Engine** from **Play Both Sides**. Engine settings use larger traditional tabs for play, realtime analysis, and full-game analysis controls.
- **Print analysis report** is emphasized at the bottom of Game Analysis; its preview prompt has larger text and a single prominent action. The Game menu now says **Import…**.
- Notes and partial analysis remain attached to their games while switching among observed games.

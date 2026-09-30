---
title: "Lichess imports, board rotation, and graph navigation"
date: 2026-09-30T00:00:00-05:00
description: "Import completed Lichess games, rotate the 3D board, and inspect move details beside the pointer in the expanded analysis graph."
slug: "2026-09-30-lichess-import-and-board-rotation"
version: "2026.09.30"
---

## Added

- Import completed games from Lichess through **Game → Import…**. Enter a username to fetch the latest 20 completed games, or paste an individual game link. No Lichess login or API token is required.
- Choose which fetched games to save in Imported Games, with duplicate checking. Open a saved game for review and analysis, or enable immediate full-game analysis when importing a single game.
- Rotate the 3D board by holding the right mouse button and dragging. Double right-click the board or use the reset-view button to return to the default perspective.
- Player-name links to FIDE profiles in Saved Games and around the board when the imported PGN provides a valid `WhiteFideId` or `BlackFideId`. Profiles open in a new tab.
- A [Lichess import help guide](/help/import-from-lichess/), with links from Getting Started, PGN, and Saved Games, and a Lichess import section on the Features page.

## Improved

- Redesigned Import Games with a gold border, separate Lichess and paste sections, a compact game-text editor, and a gold Continue button.
- Replaced the Import Games Cancel button with a close **×** in the top-right corner.

## Fixed

- Fetched Lichess game text now appears in the import window and enables Continue without requiring a board move.
- Move details in the expanded analysis graph now appear beside the pointer while hovering or dragging through positions, instead of at the graph's lower-left edge.

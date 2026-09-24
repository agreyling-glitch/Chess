---
title: "Saved games, storage tools, and clearer play"
date: 2026-09-24T10:00:00-05:00
description: "A persistent game library, portable backups, global display preferences, and safer installed-app updates."
slug: "2026-09-24-saved-games-and-storage"
version: "2026.09.24"
---

## Added

- A Saved Games library for games against Stockfish and imported games, backed by IndexedDB.
- Multi-game PGN import with player search and filtering, per-game selection, import progress, and duplicate detection. Imported games can be analyzed after opening.
- Player and analysis filters for large imported collections.
- Favorites that remain attached to games across personal and imported categories.
- New games can start with the player as White or Black, or with the side chosen at random. Stockfish opens when the player chooses Black.
- Five board piece sets: System (Unicode), Cburnett, Merida, Royal Rascals, and Undead Court.
- Optional move animation and move sounds, including distinct feedback for captures and checks.
- A **Storage** menu with versioned JSON backup, validated Merge or Replace restore, storage information, saved-game clearing, and a separate full local-data reset.
- Global display preferences for board pieces, move notation, coordinates, best-move arrows, animation, and sound.
- Optional figurine algebraic notation in move lists.
- Win, loss, and draw badges beside both players, plus a result summary above the move list.
- Direct board controls for undo and board flipping.
- Version diagnostics in the About window, including loaded, server, service-worker, WebAssembly, and engine versions with a copyable privacy-safe report.

## Improved

- Existing single-game production saves migrate into the new Saved Games library on first launch.
- Save and Load commands are easier to find directly beneath **New game**.
- Game results distinguish recorded PGN outcomes from the final board position, including resignation and other non-board endings.
- Piece-set and notation choices now belong to the player rather than to an individual saved game.
- The engine-start sequence is more resilient to page refreshes and overlapping worker messages.
- Undo and flip controls use vector drawing so they render consistently without icon-font dependencies.
- Print analysis again warns that the browser print window may temporarily pause Ironwood.
- Local development bypasses stale application, stylesheet, and WebAssembly caches so a normal refresh loads the current build.

## Update safety

- Installed-app updates compare independently versioned application-shell, WebAssembly, and Stockfish assets.
- Offline engine data is preserved during ordinary application updates and game-library clearing.
- Destructive storage operations describe exactly what will be removed and require confirmation.

Games, analysis, preferences, and backups remain on the device unless the player explicitly downloads or moves a backup file.

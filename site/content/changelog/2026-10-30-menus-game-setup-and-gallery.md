---
title: "Direct play menus, clearer game setup, and an updated gallery"
date: 2026-10-30T00:00:00-05:00
slug: "2026-10-30-menus-game-setup-and-gallery"
version: "2026.10.30"
description: "Simplified navigation, installed-engine selection, stable move navigation, unified storage tools, and new screenshots."
---

- Redesigned the home page with a warmer gold-and-green palette, a larger 3D board preview, screenshot-led study cards, clearer online and Companion sections, and responsive layouts. Install and offline-engine controls remain available.

- Reorganized the main menus into Play, Training, Game History, Settings, Tools, and Help. Training lists Train against AI before Lichess Puzzles.
- Replaced New Game mode tabs with Play menu choices. Online via Lichess and Online via FICS open their lobby directly. Signed-out Lichess users choose Sign in or Reconnect saved session inside the lobby; opening it no longer starts OAuth automatically.
- Added an installed-engine dropdown and refresh icon to vs AI Engine. The local setup window uses a 3D gold border and a wider sunken Game Settings section with room for engine descriptions.
- Moved Storage information, Clear saved games, and Reset all local data to the bottom of Game History. Removed the Storage menu and duplicate backup/restore entries.
- Retired the old Your game archive window. Saved Games handles Browser and Companion libraries, collections, metadata, copies, moves, exports, and deletion; Backup and Restore handles the selected storage location.
- Restyled Backup and Restore with a 3D gold border, heavy outer grain, and a fine-grain sunken section, matching the metadata editor.
- Kept failed Saved Games deletion messages visible after controls are re-enabled; failed deletions retain their games.
- Prevented duplicate live-engine searches that could stop Companion engines with “Previous search has not drained.”
- Kept Game Moves stationary while the selected row is visible and removed its scroll animation. The Time column shows elapsed times; hover details include available remaining clocks.
- Refreshed 15 gallery screenshots, added eight views for training, puzzle progress, imports, online players, FICS, Tactical Map, and saved-analysis shortcuts, and updated the example PDF. Captions now cover the comparison charts and move times.

See [Features](/features/), [Playing a game](/help/playing-a-game/), [Saved Games and backups](/help/saved-games-and-backups/), and the [Gallery](/gallery/). Companion server backups and collection filtering are included in the [Companion 0.1.7 public release](/changelog/2026-10-30-companion-0-1-7/).

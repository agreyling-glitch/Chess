---
title: "Resizable workspace, direct imports, and faster Lichess profiles"
date: 2026-11-05T00:00:00-06:00
slug: "2026-11-05-workspace-and-lichess-loading"
version: "2026.11.05"
description: "Resize the board divider, import from Saved Games, and reuse Lichess data with sections that load when opened."
---

- Made the divider between the live board and right panel draggable. Move it left to widen the panel and shrink the board, or right to give the board more room. Game, study, and puzzle panels share the divider position. Fixed the divider drifting right instead of retaining the selected width.
- Removed the top **IRONWOOD CHESS** branding panel and slightly tightened the prediction table's rows and scroll area.
- Added **Manage library → Import games…** inside Saved Games. A separate library window opens Import in the main app, or opens the app's import page if the main window is unavailable.
- Removed the PGN text-box detour after successful Lichess and Chess.com fetches. Collections open game selection directly; single games proceed to review or destination selection, and Lichess studies open in Study mode. Manual PGN entry and browsed single files retain their Continue step.
- Shared Lichess profile caches across windows for the same player and combined duplicate in-flight reads. Reopening a profile can reuse its summary, recent games, history, activity, and performance data. Completed-game previews reuse downloaded games and fetch only missing ones; completed single-game PGN imports also use a short-lived memory cache.
- Unified ordinary Lichess online-play, import, and study-refresh requests in one queue per app window. They share a pause of at least one minute after a rate-limit response and respect longer retry intervals. Live account, game, and seek streams remain independent; disconnecting does not reset the pause.
- Made profile loading independent of rating history. **Rating history** starts collapsed and loads when expanded; **Recent games** loads when opened, while activity and performance remain on demand. Added section refresh controls and kept cached profiles, charts, statistics, and game lists visible during updates, including in detached profile windows.
- Updated Help, Features, and the project README with the new import, workspace, profile, caching, and refresh behavior.

See [board and view settings](/help/board-and-view/), [import games](/help/import-games/), [Saved Games](/help/saved-games-and-backups/), and [Lichess player profiles](/help/lichess-player-profiles/).

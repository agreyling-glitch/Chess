---
title: "Saved Games search, playback, and engine delivery security"
date: 2026-10-23T00:00:00-05:00
slug: "2026-10-23-library-playback-and-engine-security"
version: "2026.10.23"
description: "Search openings and library filters, replay moves with adjustable autoplay, open a responsive library window, and secure engine asset delivery."
---

## Engine delivery security

- Restricted the engine proxy to known Stockfish assets on its configured host. Requests cannot select another upstream host, redirects are rejected, and upstream cookies and service-worker permission headers are not passed through.
- Added regression tests covering host escapes, redirects, unsupported paths, safe headers, conditional requests, and local engine delivery.

## Find and manage saved games

- Replaced category buttons with a raised **Category** dropdown and a collapsible **Filters** section that remembers its expanded state.
- Added dedicated searchable **Player**, **Source**, **Date** and **Opening** filters. Opening entries show the ECO code and opening name together. Games without opening information are classified from their moves using a bundled CC0 opening dataset, refreshed and validated during release preparation.
- Added a separate Saved Games window with responsive cards and remembered size and position. Window controls use gray beveled buttons.
- Made game titles clickable with an **Open game** tooltip, and moved the remaining card actions into a hamburger menu.
- Added **Clear selection** across pages and a progress bar for large exports that clears after completion.
- Added a **Browse** button to the paste/import panel for choosing a PGN or JSON file.

## Playback and analysis review

- Added automatic playback to the main Game Moves controls and consistent speed choices on players: 0.5, 1, 2, 3 or 5 seconds per move, with 2 seconds as the default. Right-click Play to choose a speed.
- Added a side-by-side move table to the larger Saved Games player, with a rounded outline around the current move and automatic scrolling. Its autoplay speed is remembered.
- Added a mini player directly beneath the engine continuation in Move Analysis, with spaced move labels and an outline tracking the current move.
- Moved **Start critical review** beneath the Move Analysis heading and hid **Try the best move** when the played move matches the engine recommendation.
- Preserved separate saved analysis runs for Browser games.
- Made Board Theme and Training Profiles pop-outs fill and adapt to their browser window.

See [Features](/features/) and [Saved Games help](/help/saved-games-and-backups/).

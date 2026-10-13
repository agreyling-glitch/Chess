---
title: "Faster startup and 3D updates"
date: 2026-10-12T00:00:00-05:00
slug: "2026-10-12-faster-startup-and-3d-updates"
version: "2026.10.12"
description: "On-demand theme artwork, a 2D-first chooser, deferred startup work, and smaller 3D updates for selection, moves, and game review."
---

## Startup and downloads

- Moved the web app's existing CC0 Wood and shared Marble/Glass 3D models and textures out of WebAssembly into on-demand packs. Art Deco and Egyptian use the shared Marble board assets. Native builds retain embedded artwork.
- Moved Marble, Wood, and Glass images for Hybrid Marble into on-demand piece packs. Downloaded theme artwork uses separate caches for offline reuse across app updates, subject to browser cache eviction.
- Overlapped WebAssembly compilation with saved-game restoration. Optional tools load when opened; saved Lichess sessions and background images restore after the first board frame. Stockfish starts after that frame when needed.
- Removed the optional built-in background image and standalone 3D viewer from installation precaching. They load when used and can be reused offline while runtime-cached.

## Themes and board interaction

- Split **Board Theme** into **2D** and **3D** tabs. The chooser always opens on 2D; 3D previews load when that tab is selected. Switching tabs leaves the current board unchanged.
- Kept boards usable while optional artwork loads: unavailable hybrid piece images use Merida, and unavailable 3D artwork uses a 2D board until ready. Failed downloads offer **Retry**.
- Separated 3D highlights from board and piece geometry, so selecting a piece uploads only the changed highlights.
- Cached 3D piece and shadow geometry per square. Moves and history navigation rebuild and upload only changed squares, including captures, promotion, castling, and en passant. Zoom reuses retained geometry; camera rotation, theme, size, and appearance changes still rebuild it.
- Removed unused CPU lighting calculations from GPU geometry construction. GPU shading, CPU previews, fallback rendering, and PNG export retain their existing appearance.

## Other performance improvements

- Render only visible Game Moves rows while preserving selection, navigation, and context actions.
- Schedule local-clock redraws around displayed time changes and avoid unnecessary idle redraws.
- Skip repeated, unchanged Stockfish thread and hash settings to avoid redundant resource reconfiguration. Changed settings and search commands retain their ordering.

## Help and journal

- Expanded [Board themes and appearance](/help/board-themes/) and [Board and view settings](/help/board-and-view/) with tab behavior, on-demand downloads, fallback boards, Retry, and offline limits.
- Added [A lighter start and faster 3D updates](/blog/a-lighter-start-and-faster-3d-updates/) with measured vertex-upload reductions and CPU construction timings, including their scope and limitations.

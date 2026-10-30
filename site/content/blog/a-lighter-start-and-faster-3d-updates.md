---
title: "A lighter start and faster 3D updates"
date: 2026-10-12T00:00:00-05:00
description: "Artwork loads when needed, 2D themes have their own tab, and the 3D board reuses the pieces and surfaces that have not changed."
slug: "a-lighter-start-and-faster-3d-updates"
---

This performance pass reduces the work Ironwood does before the board appears and between moves. Optional artwork loads when it is needed, the theme chooser opens on 2D, and the 3D renderer keeps unchanged geometry ready for the next frame.

## Load artwork when it is needed

The web app's Wood and shared Marble/Glass models and textures now live outside its WebAssembly package. The existing CC0 files total 5.79 MB. Another 1.67 MB of Marble, Wood, and Glass piece images for Hybrid Marble also loads separately. These are source asset sizes, not measured savings in compressed downloads.

Open **Settings → Board → Theme…** and the chooser starts on **2D**. Browsing flat-board themes does not load 3D previews; selecting **3D** loads those previews. Switching tabs leaves the current board unchanged until you choose a theme.

Downloads have a usable fallback. Hybrid piece images temporarily use Merida, while a 3D board keeps the selected 2D board available until its artwork is ready. Failed downloads offer **Retry**. Complete theme downloads are cached separately from app updates and can be reused offline while the browser retains them. Installing the app alone does not download every theme.

The optional built-in background and standalone 3D viewer are also outside the installation precache. They download when used. [Board themes and appearance](/help/board-themes/#loading-artwork-and-offline-use) explains theme loading and offline use; [Board and view settings](/help/board-and-view/#app-background) covers backgrounds.

## Give the first board less to wait for

WebAssembly compilation now overlaps saved-game restoration. Optional tools such as FICS, the Position Editor, scoresheet entry, and tablebase lookups load when opened. Saved Lichess sessions and background images restore after the first board frame, and Stockfish starts after that frame when play or analysis needs it.

During play, unchanged engine thread and hash settings no longer trigger repeated resource reconfiguration. Long Game Moves lists render the visible rows, and local clocks schedule redraws around the next displayed second. These changes remove repeated work without changing chess rules or engine strength.

## Keep the unchanged parts of a 3D position

Selecting a piece changes its highlights, so the renderer now updates a small highlight buffer while retaining the board and pieces. Moving a piece or stepping through game history rebuilds only the squares whose occupants changed, including their shadows. Castling, en passant, captures, and promotions follow the same rule.

The renderer also skips CPU lighting calculations when preparing geometry for the GPU, which already supplies its own lighting. CPU-rendered previews, fallback rendering, and copied PNG images retain their lighting calculations.

Measurements from this session show the effect at three different points:

| Operation | Before | After |
| --- | --- | --- |
| Select e2 in the starting Marble position: vertex data uploaded in the browser | 24.02 MB | 1.87 KB |
| Play or replay e2–e4 in Marble: vertex data uploaded in the browser | 24.02 MB | 0.51 MB |
| Build a full Marble scene: median native CPU time, before and after removing unused lighting | 18.24 ms | 10.46 ms |

The upload figures count vertex data, including highlights. The CPU figure is the median of 31 warm release-build samples and excludes asset loading and GPU work. Across all five 3D themes, full-scene CPU construction took roughly 37–43% less time after the lighting change. These measurements do not establish browser frame rates or end-to-end startup times.

Camera rotation, theme, size, and appearance changes still rebuild the scene. Zoom reuses the cached geometry through the shader. Regression checks compare the resulting GPU data with a full rebuild, including special moves and history navigation.

The [changelog](/changelog/2026-10-12-faster-startup-and-3d-updates/) lists the changes, and the updated help explains what to expect when choosing a theme for the first time.

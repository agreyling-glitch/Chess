---
title: "Focus mode, theme adjustments, and player statistics"
date: 2026-10-09T00:00:00-05:00
slug: "2026-10-09-focus-themes-and-player-statistics"
version: "2026.10.09"
description: "Board focus mode, personal opening history, online account statistics, and individually customizable 2D and 3D themes."
---

## Added

- Focus mode expands the live board and keeps player information, clocks, captures, coordinates, and essential board controls visible. Toggle it with the toolbar icon or **Shift+Enter**, and leave with **Escape**. Real-time position analysis pauses while focused; engine play and full-game analysis continue.
- Lichess profile statistics after Connected in the status bar: account record and time-control ratings, with further statistics on hover.
- FICS account statistics beside Connected: Lightning, Blitz, and Standard ratings, with records and rating details on hover. Guest accounts are identified separately.
- **My games** and **Player** sources in Opening Explorer, with player/color, speed, and month filters, player results, recent matching games, and import for analysis.
- **Theme adjustments…** and gear buttons on theme cards. Adjustments are saved separately for each theme on this device, with a reset for the selected theme.
- Three complete 2D piece themes: Classic Staunton, Neon Geometric, and Art Deco Faceted. Each has coordinated board squares, frame, and coordinate colors; previews and copied board images follow its palette.
- 2D gradient colors with separate start/end colors for each side, four directions, and presets. Illustrated sets also support adjustable inner shading, custom colors, smoothed contrasting outlines, size, and shadow controls.
- 3D controls for piece size and height, side colors, material finish, shadow strength/softness, overall appearance, and radial lighting.

## Changed and fixed

- Grouped 2D and 3D adjustments into consistently spaced sections with aligned controls. Each view shows applicable settings; contrasting outlines are now limited to 2D.
- Moved 3D appearance and radial-light controls into per-theme adjustments.
- Removed the separate 2D adjustments preview. Fixed the texture-cache size conflict and replaced expensive neighborhood outline processing with a linear-time pass. Blur and recoloring reuse cached fixed-resolution sprites on ordinary redraws.
- Personal opening statistics show indexing progress rather than treating an initial empty response as a final zero result, and can be refreshed after indexing.

## Help and site

- Added [Board themes and appearance](/help/board-themes/), including right-button drag rotation, right-button double-click reset, and scroll-wheel zoom.
- Updated focus mode, keyboard input, Lichess play, FICS play, Opening Explorer, and features documentation.
- Added the Twelve Tools badge below the website footer.

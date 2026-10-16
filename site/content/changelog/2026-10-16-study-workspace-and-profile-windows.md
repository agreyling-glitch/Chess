---
title: "Expanded analysis, profile pop-outs, and refreshed guides"
date: 2026-10-16T00:00:00-05:00
slug: "2026-10-16-study-workspace-and-profile-windows"
version: "2026.10.16"
description: "Review captures and annotations on the expanded graph, move player profiles to another monitor, and use the refreshed Help center."
---

## Expanded game analysis

- Added move-aligned capture rows for white and black pieces, with larger Merida icons and hover details. Capture icons use Merida independently of the selected board theme.
- Added a sunken board frame and coordinates to the position preview. The preview follows board orientation and shows saved drawings and best-move arrows using the main board’s rules.
- Added mouse-wheel move navigation and preserved the selected move until graph hovering resumes.
- Increased spacing between the graph, move details, notes, and board preview.
- Fixed a UI-state lock deadlock that could freeze the app when opening the expanded graph. Added a rendering regression test.
- Kept Analyze again, Expand, and Print analysis separated in narrow panes without displacing the analysis content.

## Lichess player profiles

- Added on-demand daily Activity with game and puzzle results and rating changes. Replaced an unsupported rating-change glyph with readable text.
- Made Recent games collapsible and kept cards stable while resizing.
- Added a raised **↗** title-bar button, matching the FICS console, to move a profile into a separate browser window. Multiple pop-outs share the main window’s Lichess connection and preserve player and graph selections.
- Made pop-out profiles fill the browser area with consistent margins and a fixed header. Corrected the browser title and restored dragging from in-app profile headers.
- Routed **Review on Ironwood** from a separate profile window back to the main board, retaining active-play restrictions and explicit saving.
- Reworked notable-game loading to use a batch export request, avoiding browser fetch failures from individual game exports. Performance details retain usable statistics when previews fail.

## Engine predictions and workspace

- Added frames, coordinates, and recessed edges to expanded prediction boards.
- Refined the docked prediction table with plain headers and move numbers, a narrower number column, wider move columns, a centered variation selector with room for depth, and a centered gray raised Expand button.
- Removed the depth/node-count row above Analyze position and reduced the extra gap below the real-time score.
- Added fine linen and deeper sunken surfaces to the About technology section, Saved Games controls, main move list, real-time score, and prediction panel. Technology badges have individual raised styling and shadows.
- Applied paper grain to study-window backgrounds, including training profiles, engine settings, Notes, Draw on board, Tactical Map, Saved Games, and enlarged playback. Analysis details retain solid recessed sections where appropriate for readability.
- Redesigned New Game around grouped controls, larger section labels, compact engine settings, smaller centered start/connect buttons, and vertical resizing. Fixed horizontal growth when choosing Training vs AI.
- Refreshed Storage information with recessed panels and a capacity chart, and refined the Position Editor’s inset layout.

## Help and Features

- Added dedicated guides for the expanded analysis graph, Lichess player profiles, and troubleshooting.
- Added Help quick links, article contents navigation, missing topic filters, and multi-term search across article text.
- Reorganized Features with group navigation and current descriptions of analysis, online profiles, training, imports, board controls, and local storage.
- Corrected outdated puzzle-board descriptions: puzzles use the main board, and played attempts appear under Saved Games → Puzzles. Updated the related blog article and README.

See [Features](/features/) for the current overview and [Help](/help/) for instructions. Profile pop-outs require the main Ironwood window to remain open. Games, notes, and settings remain local unless you explicitly use an online or sharing feature.

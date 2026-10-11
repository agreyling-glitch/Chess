---
title: "Syzygy endgames and Hybrid Marble"
date: 2026-10-11T00:00:00-05:00
slug: "2026-10-11-syzygy-and-hybrid-marble"
version: "2026.10.11"
description: "Explore tablebase continuations with hover arrows, DTZ/DTM badges, and a branching tree; customize a flat board with 3D pieces and textured squares."
---

## Added

- **Syzygy tablebases** in Game Analysis for standard positions with seven pieces or fewer, including kings, and no castling rights. Lookups use Lichess's service, require internet access, and need no sign-in or tablebase download.
- **Win, draw, and loss outcomes**, with explicit 50-move-rule draw labels for cursed wins and blessed losses. The position card reports the side-to-move outcome; continuation rows report the outcome for the player making the move.
- **Hover arrows** on the main board for tablebase continuations, using the existing flat and full 3D board arrow rendering. Clicking explores the move without changing recorded game moves.
- **DTZ and DTM badges** beside continuations when supplied. Pawn moves and captures receive zeroing labels. Tooltips explain half-move counts, rounding, and the resulting-position viewpoint.
- An **exploration tree** that retains sibling branches, supports revisiting and collapsing positions, and provides Back, Reset preview, and Clear controls. The tree holds up to 256 positions.
- **Position settings & API details** with the root's 50-move clock, a refresh action that bypasses Ironwood's cache, and a link to the selected position on Lichess. Lookup status distinguishes fetched and cached results.
- **Hybrid Marble**, a flat board with 3D piece sprites rendered from the bundled models. Its piece selector offers Marble, Wood, Glass, Art Deco, and Egyptian.
- **Complement / Custom board colors** for Hybrid Marble. Complement follows the piece style; Custom exposes separate light- and dark-square color pickers.
- **Board textures:** Smooth, Marble veins, Wood grain, Linen, Slate, Glass, and Sand. All except Smooth add beveled edges and soft lighting for depth; Glass includes reflections.

## Changed and refined

- Tablebase continuation labels, hover tooltips, and exploration-tree moves respect the **Letters / Figurines** preference.
- Tablebase rows use alternating shading and separate distance badges aligned on the right.
- Refined the hybrid piece camera to a more top-down angle, increased piece size, and adjusted vertical placement and shadows. Higher-resolution source renders are downsampled for smoother edges.
- Hybrid piece styles share saved appearance settings. Board copies include the selected piece style, square colors, and texture.
- Hybrid artwork is rendered from existing licensed assets; board textures are original procedural overlays.

## Help and site

- Added a detailed [Syzygy endgame tablebase guide](/help/syzygy-tablebases/) covering outcomes, DTZ/DTM, the 50-move clock, arrows, branching exploration, caching, and troubleshooting.
- Expanded [Board themes and appearance](/help/board-themes/) with Hybrid Marble's piece styles, colors, and textures, and added tablebase terms to the glossary.
- Updated [Features](/features/) and added the journal post [Exploring endings and shaping the board](/blog/exploring-endings-and-shaping-the-board/).

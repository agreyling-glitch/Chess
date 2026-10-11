---
title: "Exploring endings and shaping the board"
date: 2026-10-11T00:00:00-05:00
description: "Syzygy continuations turn endgame results into a study path, while Hybrid Marble combines a flat board with shaded 3D pieces."
slug: "exploring-endings-and-shaping-the-board"
---

A winning endgame result raises the next question: which moves keep the win, and what happens after the opponent's reply? Ironwood's new Syzygy panel lets you follow those questions on the board. Alongside it, Hybrid Marble offers another way to view the position: a flat board with shaded 3D pieces and a choice of square colors and textures.

## Follow the result onto the board

In a completed game's **Game Analysis → Tablebases** tab, look up a standard position with seven pieces or fewer and no castling rights. Both kings count toward that limit. Results come from Lichess's tablebase service over the internet, with no sign-in or local tablebase files required.

The position card names the side to move and its outcome. Each continuation shows the outcome for the player making that move. Hover a row to draw the move's arrow on the main board; click it to see the resulting position and available replies. Letters and Figurines preferences carry through the move list, tooltips, and tree.

The exploration tree keeps alternatives together. Return to a position, try a different reply, and both branches remain available. **Back** steps toward the root, **Reset preview** returns to the game position while preserving branches, and **Clear** starts the exploration over. Recorded game moves stay intact.

## Give the distance badges context

Compact **DTZ** and **DTM** badges make continuations easier to compare. DTZ describes progress toward a pawn move, capture, or mate; DTM describes distance to mate when the service supplies it. The counts use half-moves, and a continuation's badges describe the position after that move. Pawn moves and captures receive labels of their own.

These distances need the outcome beside them. DTZ is not generally a mate count, and a short mate distance alone does not settle the 50-move-rule question. Ironwood labels cursed wins and blessed losses as draws under that rule and lets you inspect the root's half-move clock for imported setup positions.

The new [Syzygy guide](/help/syzygy-tablebases/) explains these distinctions with examples. It also covers cached lookups, refresh, missing values, and opening the selected position on Lichess.

## Keep the board flat and give the pieces depth

**Hybrid Marble** sits among the flat-board themes. Its pieces use a fixed, more top-down 3D view, while square selection, arrows, and normal board controls retain the flat layout. Full 3D themes remain available when you want to rotate the camera.

The hybrid view grew through refinements to piece size, base placement, camera angle, and edge quality. Rendering at higher resolution before downsampling gives the sprites smoother contours at normal board sizes. Pieces and their shadows move together as their placement changes.

Open the theme's **gear** to choose **Marble, Wood, Glass, Art Deco, or Egyptian** pieces. These sprites are rendered from Ironwood's bundled models and textures. Changing the piece style keeps the board flat and retains the hybrid appearance controls.

## Choose the surface around the pieces

**Complement** provides a square palette suited to the selected piece style. **Custom** lets you choose light and dark squares directly. Textures work with either palette: Smooth, Marble veins, Wood grain, Linen, Slate, Glass, and Sand.

All except Smooth use bevels and soft lighting to suggest depth. Glass adds reflections; Sand adds fine grain. The board's Glass texture is independent of Glass pieces, so a Wood or Egyptian set can use the same surface. The texture patterns are original procedural artwork.

The chosen style, colors, and texture save on this device and carry into copied board images. [Board themes and appearance](/help/board-themes/) covers the controls, [Features](/features/) gives the broader overview, and the [changelog](/changelog/2026-10-11-syzygy-and-hybrid-marble/) lists this update's additions.

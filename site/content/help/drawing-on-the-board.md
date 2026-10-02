---
title: "Drawing arrows and highlighting squares"
description: "Illustrate positions with colored arrows and square outlines, using mouse shortcuts or touch-friendly drawing controls."
category: "using-ironwood"
keywords: [arrows, highlights, highlighted squares, drawing, draw on board, colors, green, red, yellow, Alt, touch, cal, csl, board annotations]
weight: 17
---

Use board drawings to show plans, threats, routes, and important squares without moving any pieces. For example, draw an arrow from `e2` to `e4` to illustrate a pawn push, or outline `f6` to identify a target square. You can combine drawings with [written notes and annotation symbols](/help/taking-notes/).

## Use the drawing controls

Select the position you want to illustrate, then use **Draw on board** above the note panel. Choose **Green**, **Red**, or **Yellow**. Click or tap a square to outline it; drag from one square to another to draw an arrow. This works on both the 2D and 3D boards.

## Keyboard shortcuts

With a keyboard, you can draw temporarily without turning on drawing mode:

| Shortcut | Color |
| --- | --- |
| **Alt-click / Alt-drag** | Green |
| **Alt+Shift-click / drag** | Red |
| **Alt+Ctrl-click / drag** | Yellow |

## Change or remove drawings

Repeat the same drawing in the same color to remove it. Drawing over the same square or arrow in another color replaces its color. **Clear drawings** removes all your drawings for the displayed position. Turn **Draw on board** off when you want to play moves again. Releasing a drag outside the board cancels it.

## Review, save, and share

Drawings belong to the displayed position, including the starting position. They reappear when you return to that position, stay separate from Stockfish's best-move arrows, and are hidden while stepping through a predicted engine line. Saved games and Analysis JSON preserve them. Annotated PGN carries them in `[%cal ...]` arrow and `[%csl ...]` square comments; other software's support for these extensions varies. Plain PGN leaves them out.

Drawings on retained positions remain when you choose **Play from this position**; drawings on discarded later moves are removed.

## If drawing does not work

- Use **Draw on board** if your browser or operating system intercepts an Alt shortcut.
- Turn drawing mode off to select pieces and play moves normally.
- Return to a game position before drawing if you are stepping through a predicted engine line.
- Select the correct move or **Starting position** before drawing: each position has its own markings.

See [Board and view settings](/help/board-and-view/) for changing the board display, or [Saving and sharing analysis](/help/saving-and-sharing/) for export formats.

## Copy an annotated board image

Right-click the desired move in **Game Moves** and choose **Copy Annotated Board to Clipboard**. The PNG includes that position's personal arrows and square outlines, plus any displayed Stockfish recommendation arrow, in the active 2D or 3D view and current orientation. Paste it into an application that accepts images. Written notes and annotation symbols are not added to the image. **Copy Board to Clipboard** remains available for a board image without personal drawings.

Annotated copy is available when the move you right-click has personal drawings or a displayed Stockfish recommendation arrow. Drawings attached to another move are not included; select that move and open its menu instead. Hover over the command to see its target position and drawing count.

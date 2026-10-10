---
lastmod: 2026-10-10T00:00:00-05:00
title: "Board and view settings"
description: "Switch between 2D and 3D, change notation, and adjust what the board shows."
category: "using-ironwood"
keywords: [2D, 3D, board, view, theme, piece set, notation, arrows, coordinates, sounds]
weight: 13
---

Use **View → Show 2D board** or **Show 3D board** to switch the board display. The position and moves stay the same. **Game → Flip board** puts the other color at the bottom without changing the position.

The Compact workspace uses tabs for Game Moves and Game Analysis. **View → Move notation** chooses letters such as `Qf3` or piece pictures such as `♕f3`. Both show the same move; see [How to read chess moves](/help/chess-notation/).

Open **View → Board → Theme…** for a full-size chooser with 2D and 3D board previews side by side. Click a preview to apply that piece set or theme and switch to its board view. The selected card has a gold border. Double-click a preview to apply it and close the chooser, or use the close button or Escape. You can also right-click the **2D/3D** icon above the live board to open the chooser.

The **3D appearance** slider and **Show radial light below board** are in **View → Board → Theme adjustments…** when a 3D board is active. Click a theme card’s gear to open its own adjustments. See [Board themes and appearance](/help/board-themes/) for gradients, shading, and per-theme settings. The same menu also controls move highlights, piece shadows, the board frame, square coordinates, and best-move arrows. A best-move arrow points to Stockfish's suggested move when one is available. Under **View → Move feedback**, you can control move animation and sounds.

These are display preferences. They do not change the legality of a move or the engine's evaluation.

## App background

Open **View → Background…** to customize the app background in a modal window. Choose one of the three buttons on its top row; the active option is highlighted in gold:

- **Gradient Only:** use the app gradient without an image. This is the default on first launch.
- **Default:** use the built-in Ironwood gold wooden knight artwork without a circular emblem.
- **Uploaded Image:** choose from your uploaded images in the list below. If there are no uploaded images, clicking this option opens the file picker.

**Upload image…** and **Remove image** appear when an uploaded image is selected. Upload PNG, JPEG, or WebP files, switch between the saved images, or remove the selected upload. The built-in Default image cannot be deleted. Uploaded images are saved on this device only.

Under **Appearance**, adjust **Image opacity**, **Background gradient**, and **3D background opacity**. Image opacity runs from 0% (hidden) to 100% (full strength). A 0% 3D backdrop reveals the app background while keeping the board and real pieces opaque. The initial gradient strength is 17%, and the initial 3D background opacity is 0%.

Changes apply immediately; choose **Done** to close the window. Returning users keep their saved background choices.

## Floating board tools

Above the board, the tools appear in this order: **Draw on board** (pencil), **Notes** (document), **Tactical Map** (bullseye), then **2D/3D**. Click a tool icon to open or close its translucent floating window. Drag a window to reposition it; its controls no longer take up space in Game Moves.

- **Draw on board:** add colored square outlines, circles, filled squares, crosses, ghost pieces, and solid, dashed, curved, or L-shaped knight arrows. Edit or delete individual drawings in its collapsible list; use Undo or Ctrl+Z to restore drawing changes. This window resizes vertically and keeps a fixed width. Opening it enables drawing; its slide control or closing the window returns to playing moves. See [Drawing arrows and highlighting squares](/help/drawing-on-the-board/).
- **Notes:** read or edit the current position’s note in the Note tab, or the starting note in Start. Double-click note text or a move in Game Moves to edit; expand All notes to browse the game’s notes. See [Taking notes and adding annotations](/help/taking-notes/).
- **Tactical Map:** filter automatic Danger, Pins, and Forks findings and select one to trace its squares and attack lines. Enable **Attack vs Defense** to show red attacker and green defender counts in the top corners of every occupied square in 2D. See [Tactical Map](/help/tactical-map/) for what the findings mean and when the map is available.

Notes and Tactical Map windows can be resized. Personal drawings, written notes, and automatic tactical findings have separate controls and work with either board view.

## Type a move

Start typing during your turn to open the translucent move-entry overlay. Enter plays the move, Backspace edits, and Escape cancels. Lowercase piece letters work, including `nc3`. See [Keyboard move input](/help/keyboard-input/) for notation examples and navigation shortcuts.

## Resize the analysis panel

Drag the left edge of the right panel to widen it; the panel can use roughly three quarters of the window width on a large screen. Scroll down to inspect lower content. Under **Game Analysis**, use **Summary**, **Pawn Structure**, **Piece Mobility**, and **King Safety** to switch between review and positional insights. See [Positional insights](/help/positional-insights/) for details.

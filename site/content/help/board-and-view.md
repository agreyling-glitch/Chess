---
lastmod: 2026-10-17T00:00:00-05:00
title: "Board and view settings"
description: "Switch between 2D and 3D, change notation, and adjust what the board shows."
category: "using-ironwood"
keywords: [2D, 3D, board, view, theme, piece set, notation, arrows, coordinates, sounds, background, loading, download, offline, fallback, retry]
weight: 13
---

Use **View → Show 2D board** or **Show 3D board** to switch the board display. The position and moves stay the same. **Game → Flip board** puts the other color at the bottom without changing the position.

Click **3D ↗** beside the board's **2D/3D** button to open a companion 3D window. Drag it to a second monitor while continuing to use the main board. The companion follows moves, game history, and board orientation. Use its theme dropdown to choose Marble, Wood, Glass, Art Deco, or Egyptian independently of the main board. Click **Flip board** to flip only the companion board; its orientation then stays independent of the main board while moves continue to synchronize. **Reset view** resets the camera without changing this orientation. Click **?** for camera controls. Click outside the help pop-up or press Escape to dismiss it. It is view-only, with independent rotation and zoom: right-drag to rotate, hold the right mouse button and scroll to zoom, or click **Reset view**. Click **3D ↗** again to focus the existing window. Closing it leaves your main workspace running. If your browser blocks the window, allow pop-ups for Ironwood. Keep the main window open for synchronization.

On either a **2D or 3D board**, scroll the mouse wheel over the board to step through moves: **scroll down** goes forward and **scroll up** goes backward. After selecting a move in **Engine prediction**, the wheel steps through that prediction one move at a time, over either the board or the prediction table. Select a move in Game Moves to return to game navigation. On a **3D board**, hold the **right mouse button** while scrolling over the board to zoom instead. Wheel navigation is disabled during active training and online play.

**View → Move feedback → Capture dissolve** turns the capture effect on or off. Captured pieces briefly shrink and fade with a soft gold pulse; on 3D boards the matching model also sinks slightly. This preference is saved on your device. Capture effects and live-move animations are suppressed when your system requests reduced motion.

Capture dissolve is used for live captures only. Stepping through or replaying a game does not trigger it.

**View → Move feedback → Highlight king in check** colors the checked king's square red in both 2D and 3D. It follows the position displayed during play, review, and engine predictions, and is enabled by default. Your choice is saved on this device.

For 3D camera controls, **right-drag** rotates and tilts the board. Add **Shift** to lock rotation and change only the viewing height/tilt; **Shift + right mouse button + wheel** still zooms. See [Rotate, zoom, and reset the 3D board](/help/board-themes/#rotate-zoom-and-reset-the-3d-board).

The Compact workspace uses tabs for Game Moves and Game Analysis. **View → Move notation** chooses letters such as `Qf3` or piece pictures such as `♕f3`. Both show the same move; see [How to read chess moves](/help/chess-notation/).

Open **View → Board → Theme…** for a movable, resizable chooser with **2D** and **3D** tabs. It always opens on **2D**; select **3D** to load and browse the 3D previews. Click a preview to apply that piece set or theme and switch to its board view. The selected card has a gold border. Double-click a preview to apply it and close the chooser, or use the close button or Escape. You can also right-click the **2D/3D** icon above the live board to open the chooser.

The first use of a 3D theme may download its artwork. You can continue using the 2D board while it loads; the selected 3D view appears automatically when ready. If the download fails, reconnect and choose **Retry** above the board. See [Loading artwork and offline use](/help/board-themes/#loading-artwork-and-offline-use) for theme downloads and cached offline use.

The **3D appearance** slider and **Show radial light below board** are in **View → Board → Theme adjustments…** when a 3D board is active. Click a theme card’s Adjust button to open its own adjustments. See [Board themes and appearance](/help/board-themes/) for gradients, shading, and per-theme settings. The same menu also controls move highlights, piece shadows, the board frame, square coordinates, and best-move arrows. A best-move arrow points to Stockfish's suggested move when one is available. Under **View → Move feedback**, you can control move animation and sounds.

These are display preferences. They do not change the legality of a move or the engine's evaluation.

## App background

Open **View → Background…** to customize the app background. Drag the title or outer background to move the window, and drag its edges or corners to resize it. The window has a gold 3D border, heavy outer paper grain, linen behind the image controls, and subtle grain behind Appearance. Choose one of the three image buttons; the active option is highlighted in gold:

Use the raised **↗** button to open Background in a separate browser window. Image selection, uploads, removal, and appearance sliders control the main window's background. **×** or Done closes the settings window. Click **↗** again from the main window's Background settings to focus an existing pop-out. Allow pop-ups for Ironwood and keep the main window open.

- **Gradient Only:** use the app gradient without an image. This is the default on first launch.
- **Default:** use the built-in Ironwood gold wooden knight artwork without a circular emblem. It downloads when selected online and can then be reused offline while cached.
- **Uploaded Image:** choose from your uploaded images in the list below. If there are no uploaded images, clicking this option opens the file picker.

**Upload image…** and **Remove image** appear when an uploaded image is selected. Upload PNG, JPEG, or WebP files, switch between the saved images, or remove the selected upload. The built-in Default image cannot be deleted. Uploaded images are saved on this device only.

Under **Appearance**, adjust **Image opacity**, **Background gradient**, and **3D background opacity**. Image opacity runs from 0% (hidden) to 100% (full strength). A 0% 3D backdrop reveals the app background while keeping the board and real pieces opaque. The initial gradient strength is 17%, and the initial 3D background opacity is 0%.

Changes apply immediately; choose **Done** to close the window. Returning users keep their saved background choices.

## Floating board tools

Above the board, the tools appear in this order: **Draw on board** (pencil), **Notes** (document), **Tactical Map** (bullseye), then **2D/3D**. Click a tool icon to open or close its translucent floating window. Drag a window to reposition it; its controls no longer take up space in Game Moves.

- **Draw on board:** add colored square outlines, circles, filled squares, crosses, ghost pieces, and solid, dashed, curved, or L-shaped knight arrows. Edit or delete individual drawings in its collapsible list; use Undo or Ctrl+Z to restore drawing changes. This window resizes vertically and keeps a fixed width. Opening it enables drawing; its slide control or closing the window returns to playing moves. See [Drawing arrows and highlighting squares](/help/drawing-on-the-board/).
- **Notes:** read or edit the current position’s note in the Note tab, or the starting note in Start. Double-click note text or a move in Game Moves to edit; expand All notes to browse the game’s notes. See [Taking notes and adding annotations](/help/taking-notes/).
- **Tactical Map:** explore **Opponent threats** and **What changed?**, or click any piece on the main board for its **Positional activity** rating and reasons. In 2D, **Knight reach · two hops** previews either side's knights, supports click-to-lock routes, and shows pin-aware **Destination safety**. **Attack vs Defense** shows red geometric attacker counts and green **geometric defenders / legal recapturers** bubbles; a dash means no legal enemy capture to recapture. The **Danger**, **Pins**, and **Forks** filters sit directly above the cards in **Patterns**. See [Tactical Map](/help/tactical-map/) for instructions, color legends, and analysis limits.

Notes and Tactical Map windows can be resized. Personal drawings, written notes, and automatic tactical findings have separate controls and work with either board view.

## Type a move

Start typing during your turn to open the translucent move-entry overlay. Enter plays the move, Backspace edits, and Escape cancels. Lowercase piece letters work, including `nc3`. See [Keyboard move input](/help/keyboard-input/) for notation examples and navigation shortcuts.

## Resize the analysis panel

Drag the left edge of the right panel to widen it; the panel can use roughly three quarters of the window width on a large screen. Scroll down to inspect lower content. Under **Game Analysis**, use **Summary**, **Pawn Structure**, **Piece Mobility**, and **King Safety** to switch between review and positional insights. See [Positional insights](/help/positional-insights/) for details.

## Window styling

The main menu is a compact sunken strip that ends after Help. Its gold-accented buttons keep a stable size on hover. Background controls, Board Theme, and About use matching recessed sections and gold borders; Board Theme cards expose an **Adjust** button for appearance settings.

## Expanded analysis windows

The [expanded game graph](/help/expanded-analysis-graph/) shows a framed position preview with saved drawings and engine arrows, plus move-aligned capture icons using Merida. Its mouse wheel steps through the game. [Expanded prediction windows](/help/engine-line/) show independent snapshots of candidate lines, with framed coordinate boards that scale to available space.

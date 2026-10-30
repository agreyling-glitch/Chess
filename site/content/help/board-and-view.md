---
lastmod: 2026-10-27T00:00:00-05:00
title: "Board and view settings"
description: "Switch between 2D and 3D, change notation, and adjust what the board shows."
category: "using-ironwood"
keywords: [2D, 3D, board, view, theme, piece set, notation, arrows, coordinates, sounds, background, loading, download, offline, fallback, retry]
weight: 13
---

Use the **2D/3D** button above the board to switch the board display. The position and moves stay the same. **Flip board** above the board puts the other color at the bottom without changing the position.

Click **3D ↗** beside the board's **2D/3D** button to open a companion 3D window. Drag it to a second monitor while continuing to use the main board. The companion follows moves, game history, and board orientation. Use its theme dropdown to choose Marble, Wood, Glass, Art Deco, or Egyptian independently of the main board. Click **Flip board** to flip only the companion board; its orientation then stays independent of the main board while moves continue to synchronize. **Reset view** resets the camera without changing this orientation. Click **?** for camera controls. Click outside the help pop-up or press Escape to dismiss it. It is view-only, with independent rotation and zoom: right-drag to rotate, hold the right mouse button and scroll to zoom, or click **Reset view**. Click **3D ↗** again to focus the existing window. Closing it leaves your main workspace running. If your browser blocks the window, allow pop-ups for Ironwood. Keep the main window open for synchronization.

On either a **2D or 3D board**, scroll the mouse wheel over the board to step through moves: **scroll down** goes forward and **scroll up** goes backward. After selecting a move in **Engine prediction**, the wheel steps through that prediction one move at a time, over either the board or the prediction table. Open **Moves** and select a move in Game Moves to return to game navigation. On a **3D board**, hold the **right mouse button** while scrolling over the board to zoom instead. Wheel navigation is disabled during active training and online play.

**Settings → Move feedback → Capture dissolve** turns the capture effect on or off. Captured pieces briefly shrink and fade with a soft gold pulse; on 3D boards the matching model also sinks slightly. This preference is saved on your device. Capture effects and live-move animations are suppressed when your system requests reduced motion.

Capture dissolve is used for live captures only. Stepping through or replaying a game does not trigger it.

**Settings → Move feedback → Highlight king in check** colors the checked king's square red in both 2D and 3D. It follows the position displayed during play, review, and engine predictions, and is enabled by default. Your choice is saved on this device.

For 3D camera controls, **right-drag** rotates and tilts the board. Add **Shift** to lock rotation and change only the viewing height/tilt; **Shift + right mouse button + wheel** still zooms. See [Rotate, zoom, and reset the 3D board](/help/board-themes/#rotate-zoom-and-reset-the-3d-board).

The workspace has the live board on the left and **Moves**, **Prediction**, and **Analysis** tabs on the right. **Moves** contains Game Moves; **Prediction** contains Engine Prediction; **Analysis** contains the full-game graph and review tools. **Settings → Move notation** chooses letters such as `Qf3` or piece pictures such as `♕f3`. Both show the same move; see [How to read chess moves](/help/chess-notation/).

Open **Settings → Board → Theme…** for a movable, resizable chooser with **2D Pieces** and **3D Boards** tabs. It always opens on **2D Pieces**; select **3D Boards** to load and browse the 3D previews. Click a preview to apply that piece set or theme and switch to its board view. The selected card has a gold border. Double-click a preview to apply it and close the chooser, or use the close button or Escape. You can also right-click the **2D/3D** icon above the live board to open the chooser.

The first use of a 3D theme may download its artwork. You can continue using the 2D board while it loads; the selected 3D view appears automatically when ready. If the download fails, reconnect and choose **Retry** above the board. See [Loading artwork and offline use](/help/board-themes/#loading-artwork-and-offline-use) for theme downloads and cached offline use.

The **3D appearance** slider and **Show radial light below board** are in **Settings → Board → Theme adjustments…** when a 3D board is active. Click a theme card’s Adjust button to open its own adjustments. See [Board themes and appearance](/help/board-themes/) for gradients, shading, and per-theme settings. The same menu also controls move highlights, piece shadows, the board frame, square coordinates, and best-move arrows. A best-move arrow points to Stockfish's suggested move when one is available. Under **Settings → Move feedback**, you can control move animation and sounds.

These are display preferences. They do not change the legality of a move or the engine's evaluation.

## App background

Open **Settings → Background…** to customize the app background. Drag the title or outer background to move the window, and drag its edges or corners to resize it. The window has a gold 3D border, heavy outer paper grain, linen behind the image controls, and subtle grain behind Appearance. Choose one of the three image buttons; the active option is highlighted in gold:

Use the raised **↗** button to open Background in a separate browser window. Image selection, uploads, removal, and appearance sliders control the main window's background. **×** or Done closes the settings window. Click **↗** again from the main window's Background settings to focus an existing pop-out. Allow pop-ups for Ironwood and keep the main window open.

- **Gradient Only:** use the app gradient without an image. This is the default on first launch.
- **Default:** use the built-in Ironwood gold wooden knight artwork without a circular emblem. It downloads when selected online and can then be reused offline while cached.
- **Uploaded Image:** choose from your uploaded images in the list below. If there are no uploaded images, clicking this option opens the file picker.

**Upload image…** and **Remove image** appear when an uploaded image is selected. Upload PNG, JPEG, or WebP files, switch between the saved images, or remove the selected upload. The built-in Default image cannot be deleted. Uploaded images are saved on this device only.

Under **Appearance**, adjust **Image opacity**, **Background gradient**, and **3D background opacity**. Image opacity runs from 0% (hidden) to 100% (full strength). A 0% 3D backdrop reveals the app background while keeping the board and real pieces opaque. The initial gradient strength is 17%, and the initial 3D background opacity is 0%.

Changes apply immediately; choose **Done** to close the window. Returning users keep their saved background choices.

## Study and annotation tools

Choose **Tactical** for Tactical Map, or **Annotations** for **Drawings**, **Notes**, and **Chess Symbols**. These tools now live beside the board in sunken panels with gray subtabs. Their old toolbar icons and floating windows have been removed.

- **Drawings:** colors, shapes, arrows, and ghost pieces. Entering Drawings enables drawing; use its switch to turn drawing off, or leave the subtab. Undo and Ctrl+Z restore drawing changes. [Drawing guide](/help/drawing-on-the-board/).
- **Notes:** read and edit the selected position's writing or starting note. The editor stays in the panel; All notes browses the game's notes. [Notes guide](/help/taking-notes/).
- **Chess Symbols:** apply common annotations or search More symbols by description or numeric code. The right-click menu still offers common choices. [Symbol guide](/help/move-symbols/).
- **Tactical:** inspect temporary attacker/defender counts, threats, position changes, piece activity, Knight reach, and tactical patterns in either board view. Switching away hides these overlays. [Tactical Map guide](/help/tactical-map/).

## Type a move

Start typing during your turn to open the translucent move-entry overlay. Enter plays the move, Backspace edits, and Escape cancels. Lowercase piece letters work, including `nc3`. See [Keyboard move input](/help/keyboard-input/) for notation examples and navigation shortcuts.

## Board and panel sizing

The live board fits the available space as you resize the browser. Switching between 2D and 3D preserves its dimensions. The Moves panel fits its contents. Use **Position** for Pawn Structure, Piece Mobility, and King Safety; **Explore** for Openings and Endgames; **Analysis** for engine results. Each study tab identifies the selected position and move. Long panels scroll. [Workspace guide](/help/getting-started/#find-your-way-around).

## Window styling

The main menu is a compact sunken strip that ends after Help. Its gold-accented buttons keep a stable size on hover. Background controls, Board Theme, and About use matching recessed sections and gold borders; Board Theme cards expose an **Adjust** button for appearance settings.

## Expanded analysis windows

The [expanded game graph](/help/expanded-analysis-graph/) shows a framed position preview with saved drawings and engine arrows, plus move-aligned capture icons using Merida. Its mouse wheel steps through the game. [Expanded prediction windows](/help/engine-line/) show independent snapshots of candidate lines, with framed coordinate boards that scale to available space.

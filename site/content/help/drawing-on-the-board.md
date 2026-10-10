---
title: "Drawing arrows and highlighting squares"
description: "Draw arrows, square highlights, crosses, and ghost pieces; edit drawings, switch styles, and undo changes."
category: "using-ironwood"
lastmod: 2026-10-10T00:00:00-05:00
keywords: [ghost pieces, knight arrows, filled square, cross, black, white, arrows, highlights, squares, circles, drawing, draw on board, colors, dashed, dotted, curved, undo, Ctrl+Z, Alt, cal, csl, board annotations]
weight: 17
---

Use board drawings to show plans, threats, routes, and important squares without moving pieces. Drawings belong to the displayed position and work on both the 2D and 3D boards. You can combine them with [written notes and annotation symbols](/help/taking-notes/).

## Open the drawing window

Click the **pencil icon** above the board, immediately to the left of the Notes icon, to open the translucent **Draw on board** window. Click it again, or the window’s close button, to close it. Drawing tools live in this window rather than in Game Moves or Notes.

Opening the window enables drawing. The slide control beside its title turns drawing on or off; hover it to see **Drawing Enabled** or **Drawing disabled**. Turn it off to play moves while keeping the tools visible. Closing the window also disables drawing and cancels an unfinished drag.

Drag the window to reposition it. Its width is fixed to fit the controls; resize it vertically to give the drawing list more room. The window opens tall enough to show the drawing controls on a typical desktop display. Colors, shapes, arrows, and ghost-piece buttons use matching sizes and aligned columns. Smaller windows and long drawing lists may still scroll. Hover a choice to see its name.

## Choose a color and shape

![Draw on board window with six colors, square and arrow styles, and ghost-piece selectors.](/gallery/draw-on-board.png)

Select a **Green**, **Red**, **Yellow**, **Blue**, **Black**, or **White** color swatch. Then choose a shape or arrow:

| Picker row | Choices | How to draw |
| --- | --- | --- |
| Shapes | Solid square, dotted square, solid circle, dotted circle, translucent filled square, cross (×) | Click the square to mark. |
| Arrows | Solid, dashed, curve left, curve right, knight move, dashed knight move | Hold the left mouse button on the starting square, drag to the target, and release. |

A **filled square** highlights an area while leaving pieces visible; a **cross** marks a target or square to avoid. **Knight move** arrows follow an L-shaped route, with the longer leg first. Select the dashed version for an alternative route.

Selecting a shape or arrow enables drawing. Squares and circles mark one square; arrows need different starting and ending squares. Releasing outside the board or pressing **Escape** cancels an unfinished drawing.

## Switch arrow style while dragging

Keep holding the **left mouse button** and tap the **right mouse button** to change the arrow before finishing it:

- **Solid ↔ dashed:** switches a straight arrow’s line style.
- **Knight move ↔ dashed knight move:** switches an L-shaped arrow’s line style.
- **Curve left ↔ curve right:** reverses a curved arrow’s bend.

Each right-button press toggles the style again. The preview updates as you drag; release the left button to save the arrow. The selected arrow picker updates too. On the 3D board, right-click during a drawing drag changes the arrow instead of rotating the board.

## Place ghost pieces

Choose a color, then select a piece in either row under **Ghost pieces**. The rows offer White and Black piece symbols. Click a board square to place a translucent ghost in the selected color. A ghost illustrates a possible placement without changing the real position, legal moves, or engine evaluation.

Click the same square with the same piece and color selected to remove it. Choose another ghost piece or color and click that square to replace its ghost. A ghost can share a square with a square marking. Ghosts also support Delete, Clear, and Undo, and their color and square can be edited in the drawing list.

Ghosts work on both board views. In the 3D view they remain flat, upright symbols over their squares rather than 3D piece models.

## Edit individual drawings

Expand **Drawings (count)** below the pickers to see the drawings for the displayed position. The list uses the available vertical space and scrolls when needed.

Each entry shows its shape or arrow type. Use its color picker to change the color. Arrows have **Start** and **End** square selectors; single-square shapes and ghost pieces have one **Square** selector. Changes update the board and save immediately. An arrow needs two different endpoints, and another drawing cannot already occupy the same square or directed route.

Use **Delete** on an entry to remove only that drawing. Repeat an identical drawing to remove it from the board, or draw over the same square or arrow route with a different color or style to replace it. **Clear** in the window header removes every drawing for the displayed position.

## Undo drawing changes

Click the **undo icon** beside Clear, or press **Ctrl+Z** while the drawing window is open, to undo the last drawing change for the displayed position. Undo covers additions, replacements, color and square edits, individual deletions, and Clear. The restored drawings save immediately.

Each position has its own undo steps. Undo history is temporary, retains up to 128 changes across the game, and resets when a game is loaded or its move history is replaced or shortened. It is not included in saved files. When a text field is active, Ctrl+Z keeps its normal text-editing behavior.

## Alt shortcuts

You can draw temporarily without opening the window or enabling drawing mode:

| Shortcut | Color |
| --- | --- |
| **Alt-click / Alt-drag** | Green |
| **Alt+Shift-click / drag** | Red |
| **Alt+Ctrl-click / drag** | Yellow |

Alt-click draws a solid square outline; Alt-drag draws a solid straight arrow. These shortcuts do not use the selected shape or arrow style, and right-click style switching does not apply while Alt is held.

## Review, save, and share

Moves with drawings show a **blue drawing icon** in Game Moves, even without a written note. Hover the move to see its drawing count. If a move has both a note and drawings, both indicators appear.

Drawings belong to the displayed position, including the starting position. They reappear when you return to that position and are hidden while stepping through a predicted engine line. Drawings on retained positions remain when you choose **Play from this position**; drawings on discarded later moves are removed.

Saved games and **Analysis JSON** preserve drawings and their styles. **Annotated PGN** carries arrows and square markings in `[%cal ...]` and `[%csl ...]` comments, with `[%iw_arrow_styles ...]` for Ironwood’s shape and arrow styles. Other chess apps may ignore the style extension and show straight solid arrows or square outlines instead of curves, circles, or dotted outlines. Ghost pieces use the `[%iw_ghosts ...]` extension, including their selected colors; other apps may ignore it. **Plain PGN** omits personal drawings.

**Print analysis** includes personal drawings, ghost pieces, written notes, and manual annotation symbols in **Your notes and drawings**. Positions with drawings receive board diagrams, including positions outside the engine’s critical list.

Personal drawings remain separate from Stockfish’s best-move arrows and [Tactical Map](/help/tactical-map/) overlays.

## Copy an annotated board image

Right-click the desired move in **Game Moves** and choose **Copy Annotated Board to Clipboard**. The PNG includes that position’s personal drawings, preserving colors and styles, plus any displayed Stockfish recommendation arrow. It uses the active 2D or 3D view and current orientation. Paste it into an application that accepts images. Written notes and annotation symbols are not added to the image.

**Copy Board to Clipboard** remains available for an image without personal drawings. Annotated copy is available when the target move has personal drawings or a displayed Stockfish recommendation arrow. Hover the command to see its target position and drawing count.

## If drawing does not work

- Check the header’s slide control: drawing must be enabled to use the graphical tools.
- Use the pencil window if your browser or operating system intercepts an Alt shortcut.
- Return to a game position if you are stepping through a predicted engine line.
- Select the intended move before drawing: each position has its own markings.
- Keep holding left-click when tapping right-click to switch arrow style.
- Turn drawing off or close the window when you want to play moves.

See [Board and view settings](/help/board-and-view/) for display controls, or [Saving and sharing analysis](/help/saving-and-sharing/) for export formats.

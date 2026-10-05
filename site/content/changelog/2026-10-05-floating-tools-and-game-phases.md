---
title: "Floating study tools and game-phase highlights"
date: 2026-10-05T00:00:00-05:00
description: "Tactical Map, floating Notes and drawing tools, editable drawings with undo, and phase highlights on the expanded graph."
slug: "2026-10-05-floating-tools-and-game-phases"
version: "2026.10.05"
---

## Added

- **Tactical Map** in a translucent floating window, opened with the bullseye beside **2D/3D**. Color-coded **Danger**, **Pins**, and **Forks** filters show counts; selecting a finding highlights relevant squares and attack lines on either board view. Findings are geometric clues rather than engine-confirmed wins. The map is hidden during rated training games, active online play, best-move exercises, and prediction previews.
- A floating **Notes** window with **Note** and **Start** tabs, hover descriptions, and a collapsible **All notes** list. Read the starting note without leaving the current board position; click list entries to view notes or double-click to edit.
- Double-click editing from note text, the empty-note prompt, and moves in **Game Moves**. Note window and editor titles identify the move and its plain-language description.
- **Draw on board** in its own translucent window, opened with the pencil icon to the left of Notes. Graphical pickers offer green, red, yellow, and blue; solid or dotted squares and circles; and solid, dashed, left-curved, and right-curved arrows.
- Right-click style switching during an arrow drag: hold left-click and tap right-click to toggle solid/dashed or reverse a curve. The preview and selected picker update before the drawing is completed.
- A collapsible **Drawings** list for changing individual colors and squares or arrow endpoints, with a Delete control for each drawing.
- Drawing **Undo**, beside Clear, and **Ctrl+Z** while the drawing window is open. Undo restores additions, replacements, edits, deletions, and Clear for the displayed position.
- Opening, middlegame, and endgame highlights on the **expanded analysis graph**, with colored bands, boundary markers, labels, a legend, and phase details on hover and selection. Classification matches the phase summaries and reports.

## Improved

- Notes, Tactical Map, and drawing controls no longer crowd Game Moves. Their non-selectable titles allow click-and-drag repositioning.
- The drawing window uses a compact fixed width, centered pickers, and vertical resizing. Its scrolling drawing list grows with available height. A header slide control enables or disables drawing, with the title, Clear, Undo, and close control on one line.
- Shape and arrow styles are preserved in saved games, Analysis JSON, and annotated board images. Annotated PGN includes an Ironwood style extension; other apps may show simpler arrows or square outlines.
- Updated [drawing help](/help/drawing-on-the-board/), [Notes help](/help/taking-notes/), [Tactical Map help](/help/tactical-map/), related keyboard and export guides, Features, and the Gallery’s expanded-analysis screenshot.
- [Game phases help](/help/game-phases-and-critical-moves/) now explains the opening’s first 20 half-moves, the 26-point non-pawn material threshold, and classification using the board before each move.

## Fixed

- Personal drawings stay aligned with the 3D board when zooming.
- Browser mouse-button handling recognizes right-click while left-click is held, enabling style changes during a drawing drag. 3D rotation is suppressed during that drag.
- Tactical finding cards use drawn markers and readable move text instead of unsupported glyphs.

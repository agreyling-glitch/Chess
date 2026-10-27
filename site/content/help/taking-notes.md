---
lastmod: 2026-10-27T00:00:00-05:00
title: "Taking notes and adding annotations"
description: "Use Annotations for drawings, written notes, and searchable chess symbols beside the board."
category: "using-ironwood"
keywords: [Annotations, Drawings, Notes, Chess Symbols, starting note, move note, comments, NAG, More symbols]
weight: 16
---

## Open Annotations

Choose **Annotations**, to the right of **Tactical**, then **Drawings**, **Notes**, or **Chess Symbols**. Gray subtabs and sunken sections keep these tools beside the board. Notes and drawing tools no longer open from toolbar icons or floating windows. The position label above the panel identifies the selected move.

Your writing, symbols, and drawings are independent of engine results. You can use them without running analysis.

## Add or edit a move note

1. Select the move in **Moves**.
2. Choose **Annotations → Notes → Position note**.
3. Choose **Add note** or **Edit note**, or double-click the note text.
4. Write in the editor inside Notes and choose **Save note**. **Cancel** leaves the saved note unchanged; **Delete note** removes it.

Double-clicking a move in Game Moves, or right-clicking it and choosing **Add note…** or **Edit note…**, opens the same editor under Annotations. Its heading identifies the move and its plain-language description. Long text scrolls within the panel.

For example, after `Nf3`: “Develop the knight before moving the queen. Next, prepare castling.” The note belongs to that displayed position.

## Starting notes and all notes

Choose **Starting note** to read or edit the introduction while the board stays on its current move. A starting note can describe an opening plan, puzzle instructions, or a custom starting position imported from FEN. The right-click **Starting position note…** command opens its editor too.

Expand **All notes (count)** to browse every nonempty note, including the starting note. Click a move-note entry to display that move; double-click to edit. Selecting the starting-note entry keeps the board on its current move. Moves with writing have a gold note marker; drawings have a separate blue marker. Hover a move to see its note and drawing count.

## Add Chess Symbols

Choose **Annotations → Chess Symbols** to apply move-quality symbols such as `!`, `!!`, `!?`, `?!`, `?`, and `??`, or position assessments such as `=`, `∞`, `+=`, and `-/+`. The right-click **Add annotation…** menu keeps these common choices available too.

A new move-quality choice replaces the previous move-quality choice. A new overall position assessment replaces the previous overall assessment. Selecting the same choice again removes it. Advanced positional and unfamiliar imported annotations are preserved when you change a common symbol. The initial position supports position annotations; move-quality choices require a played move.

**Applied to this position** lists the current annotations with descriptions. Click an entry to remove that annotation, or use **Clear annotation** to remove all manual symbols from that position.

Expand **More symbols** and search by description, side, or code: try `initiative`, `Black`, or `$37`. Ironwood preserves every valid NAG code from `$0` to `$255`. Descriptions cover the original standard set through `$139`; other codes keep a numeric fallback because their meanings can vary between programs. `$0` is a placeholder. Unfamiliar codes remain visible rather than disappearing.

Compact symbols or `$number` codes appear beneath move notation; hover for descriptions. Notes and printed reports also show descriptions. These are your assessments, separate from [engine move-quality marks](/help/move-symbols/).

## Draw on the board

Choose **Annotations → Drawings** for colors, square outlines, circles, filled squares, crosses, arrows, and ghost pieces. Entering Drawings enables drawing; the slide switch turns drawing input on or off. Leaving Drawings disables drawing and cancels an unfinished drag. Alt-click and Alt-drag remain available for quick annotations. [Drawing guide](/help/drawing-on-the-board/).

Tactical Map overlays are temporary inspection aids, not saved personal drawings. Open **Tactical** to use them.

## Save and share

Notes, symbols, and drawings stay attached to their positions in saved games. Rerunning analysis does not replace them. **Play from this position** retains annotations on the part of the game you keep and removes those on discarded later moves.

- **Annotated PGN** preserves notes and NAGs, with comment extensions for drawings. Other programs may simplify unsupported drawing styles or ignore ghost pieces.
- **Analysis JSON** preserves personal annotations alongside Ironwood's analysis data.
- **Plain PGN** omits personal annotations.
- **Copy Annotated Board to Clipboard** includes the selected position's drawings and any displayed engine recommendation arrow.

Notes also appear in the expanded graph and printed analysis reports. Saved Games previews show the starting note; open the game to inspect move notes. [Saving and sharing](/help/saving-and-sharing/).

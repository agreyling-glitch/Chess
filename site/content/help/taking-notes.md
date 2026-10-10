---
lastmod: 2026-10-10T00:00:00-05:00
title: "Taking notes and adding annotations"
description: "Add Starting notes, record thoughts on individual moves, and use annotation symbols to review and share your games."
category: "using-ironwood"
keywords: [notes, notetaking, note-taking, starting note, move note, comments, annotation, symbols, NAG, study]
weight: 16
---

Notes help you remember what you were thinking, explain a plan, and record lessons for your next game. You can attach a **Starting position note** to the initial board or a **move note** to the position after a particular move. You do not need to run Stockfish to write either kind.

## Add a note to a move

Double-click a move in **Game Moves** to open its note editor directly, whether or not it already has a note. To use the floating Notes window instead:

1. Select the move in **Game Moves** to display its position on the board.
2. Click the **Notes icon** to the left of the Tactical Map bullseye above the board, select the **Note** tab, then double-click **No note for this position.** in the floating **Notes** window. You can also right-click the move and choose **Add note…**.
3. Write your thoughts, then choose **Save note**.

For example, a note after `Nf3` could say: “Develop the knight before moving the queen. Next, prepare castling.” The note belongs to that displayed position, so include the move or plan you mean when describing an earlier decision.

The translucent Notes window follows the board as you step through the game. Drag it to reposition it, resize it, or close it with its close button or the Notes icon. Long notes scroll within the panel. Moves with notes have a small gold note marker; drawings have a separate blue drawing icon, and moves with both show both indicators; selecting the move displays its note. Double-click the note text to edit it. Hover **Note** for “Position Note” or **Start** for “Starting Note.”

The title identifies the note’s position with the move and a plain-language description, for example **Notes: Nxb5 (Knight to B5)**. The note editor uses the same title style. The starting note is titled **Notes: Starting position**.

## Browse all notes

Expand **All notes (count)** below the note text to browse every nonempty note in the game, including the Starting note. Each entry shows its position and note text. Click a move-note entry to display that move on the board and read its note; double-click an entry to edit it.

Clicking the **Starting note** entry selects the Start tab without moving the board away from the current move. The list scrolls when there are many notes. Empty positions do not appear in this list; double-click their moves in Game Moves or the empty-note prompt to add a note.

## Add a Starting note

Open the **Notes** window and select the **Start** tab. Double-click the note text, or **No note for this position.** if it is empty, write the introduction, and choose **Save note**. The board stays on the selected move. You can also right-click a move and choose **Starting position note…**.

A Starting note is useful for an opening plan, a puzzle's instructions, or context about a custom starting board. It describes the game's initial position, which may be a position imported from FEN rather than the usual chess setup.

Use the first-position navigation button to return to that board and read the full note in **Notes**. The **Start** tab lets you read the starting note while the board stays on a later move.

## Edit or remove a note

Select the **Note** or **Start** tab and double-click the note text. Choose **Save note** to keep your changes or **Delete note** to remove the note. Closing the editor without saving leaves the saved note unchanged.

Notes stay separate from engine analysis. Running or rerunning Stockfish does not replace your writing. If you use **Play from this position**, notes and annotations on the retained part of the game remain; those on discarded later moves are removed.

## Find your notes later

- **Main board:** the floating Notes window shows the displayed position's note in either workspace layout.
- **Expanded analysis graph:** select a position to read its note alongside the analysis details.
- **Saved Games:** games with a Starting note show a two-line preview. Click the note preview to open the game at its starting position.
- **Larger saved-game board preview:** click the small board to enlarge it and read the full Starting note beneath it. The preview initially shows the final position; its move controls and autoplay let you browse the game. The Starting note still describes the initial position.

Move notes become visible when you open the game and select their moves. The Saved Games note preview shows the Starting note.

## Add annotation symbols

Right-click a move and choose **Add annotation…**. Select a symbol from either group:

| Move quality | Meaning |
| --- | --- |
| `!` | Good move |
| `!!` | Brilliant move |
| `!?` | Interesting move |
| `?!` | Dubious move |
| `?` | Mistake |
| `??` | Blunder |

| Position assessment | Meaning |
| --- | --- |
| `=` | Equal position |
| `∞` | Unclear position |
| `+=` / `=+` | Slight advantage for White / Black |
| `+/-` / `-/+` | Clear advantage for White / Black |
| `+−` / `−+` | Decisive advantage for White / Black |

You can choose **one symbol from each group** for a move. Selecting another symbol in the same group replaces the previous selection; selecting the current symbol again removes it. **Clear annotation** removes the position's manual annotations. The starting position supports position assessments, since no move has been played there yet.

Your symbols appear in gold beneath the move text and in the Notes window’s Note tab. They express **your assessment**, independently of [Stockfish's move-quality marks](/help/move-symbols/). A note can explain why you chose a symbol.

The text forms above keep the symbols readable with Ironwood's fonts. Annotated PGN stores their standard numeric annotation glyphs (**NAGs**), so the receiving program may display a different-looking equivalent symbol.

## Draw arrows and highlight squares

Click the **pencil icon** to the left of Notes above the board to open the separate translucent **Draw on board** window. Choose colored square outlines, circles, filled squares, crosses, ghost pieces, or arrows to illustrate the displayed position. Keyboard users can also use Alt-click or Alt-drag. Tactical Map’s Attack vs Defense bubbles are automatic, temporary counts rather than personal drawings. Drawings stay attached to their positions and are preserved in saved games, annotated PGN, and Analysis JSON.

See [Drawing arrows and highlighting squares](/help/drawing-on-the-board/) for shape and arrow styles, right-click switching while dragging, editing individual drawings, undo, and shortcuts.

## Save and share

Notes and manual annotations stay with saved games on this device. To share or back them up:

- **Annotated PGN** includes notes, manual annotations, and supported engine analysis. Ironwood can restore its own exported notes and standard main-line NAGs when importing the file.
- **Analysis JSON** preserves notes and manual annotations alongside Ironwood's analysis data.
- **Plain PGN** contains the game record without your notes, manual symbols, or engine analysis.
- **Printed analysis reports** include your written notes, manual annotation symbols, and board drawings in **Your notes and drawings**. Positions with drawings include board diagrams, even when the engine has not marked them as critical.

Check your notes before sharing an annotated file or report, since your writing is included. Notes in another program's ordinary PGN comments are not automatically converted into Ironwood move notes.

See [Saved games and backups](/help/saved-games-and-backups/) for local storage and full backups, or [Saving and sharing analysis](/help/saving-and-sharing/) for export formats.

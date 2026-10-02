---
title: "Taking notes and adding annotations"
description: "Add Starting notes, record thoughts on individual moves, and use annotation symbols to review and share your games."
category: "using-ironwood"
keywords: [notes, notetaking, note-taking, starting note, move note, comments, annotation, symbols, NAG, study]
weight: 16
---

Notes help you remember what you were thinking, explain a plan, and record lessons for your next game. You can attach a **Starting position note** to the initial board or a **move note** to the position after a particular move. You do not need to run Stockfish to write either kind.

## Add a note to a move

1. Select the move in **Game Moves** to display its position on the board.
2. Choose **Add note…** in the **Your note** panel, or right-click the move and choose **Add note…**.
3. Write your thoughts, then choose **Save note**.

For example, a note after `Nf3` could say: “Develop the knight before moving the queen. Next, prepare castling.” The note belongs to that displayed position, so include the move or plan you mean when describing an earlier decision.

The note panel follows the board as you step through the game. Long notes scroll within the panel. Moves with notes have a small note marker; selecting the move displays its note. On devices without a right mouse button, use the panel's Add/Edit controls.

## Add a Starting note

Use the **Starting position** row above the moves in Game Moves. Select **Add note…**, write the introduction, and choose **Save note**. You can also right-click a move and choose **Starting position note…**.

A Starting note is useful for an opening plan, a puzzle's instructions, or context about a custom starting board. It describes the game's initial position, which may be a position imported from FEN rather than the usual chess setup.

Select **Starting position** to return to that board and read the full note. If you are viewing a later move, **View starting note** in the note panel takes you there too.

## Edit or remove a note

Select the position and choose **Edit note…**. Choose **Save note** to keep your changes or **Delete note** to remove the note. Closing the editor without saving leaves the saved note unchanged.

Notes stay separate from engine analysis. Running or rerunning Stockfish does not replace your writing. If you use **Play from this position**, notes and annotations on the retained part of the game remain; those on discarded later moves are removed.

## Find your notes later

- **Main board:** the note panel shows the displayed position's note in either workspace layout.
- **Expanded analysis graph:** select a position to read its note alongside the analysis details.
- **Saved Games:** games with a Starting note show a two-line preview. Click the note preview to open the game at its starting position.
- **Larger saved-game board preview:** click the small board to enlarge it and read the full Starting note beneath it. The board is labeled **Final position**; the Starting note still describes the initial position.

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

Your symbols appear in gold beneath the move text and in the note panel. They express **your assessment**, independently of [Stockfish's move-quality marks](/help/move-symbols/). A note can explain why you chose a symbol.

The text forms above keep the symbols readable with Ironwood's fonts. Annotated PGN stores their standard numeric annotation glyphs (**NAGs**), so the receiving program may display a different-looking equivalent symbol.

## Draw arrows and highlight squares

Use **Draw on board** above the note panel to outline squares or draw colored arrows on the displayed position. Keyboard users can also use Alt-click or Alt-drag. Drawings stay attached to their positions and are preserved in saved games, annotated PGN, and Analysis JSON.

See [Drawing arrows and highlighting squares](/help/drawing-on-the-board/) for colors, shortcuts, touch controls, removing drawings, and troubleshooting.

## Save and share

Notes and manual annotations stay with saved games on this device. To share or back them up:

- **Annotated PGN** includes notes, manual annotations, and supported engine analysis. Ironwood can restore its own exported notes and standard main-line NAGs when importing the file.
- **Analysis JSON** preserves notes and manual annotations alongside Ironwood's analysis data.
- **Plain PGN** contains the game record without your notes, manual symbols, or engine analysis.
- **Printed analysis reports** include your written notes in the **Your notes** section.

Check your notes before sharing an annotated file or report, since your writing is included. Notes in another program's ordinary PGN comments are not automatically converted into Ironwood move notes.

See [Saved games and backups](/help/saved-games-and-backups/) for local storage and full backups, or [Saving and sharing analysis](/help/saving-and-sharing/) for export formats.

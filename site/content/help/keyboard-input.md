---
lastmod: 2026-10-09T00:00:00-05:00
title: "Keyboard move input"
description: "Type chess moves with a translucent input overlay, and navigate game positions with arrow keys."
category: "using-ironwood"
keywords: [keyboard, typing, input, moves, lowercase, shortcuts, Enter, Escape, arrows, nc3]
weight: 12
---

During your turn, start typing a move. The first keystroke opens a translucent overlay showing your input; you do not need to click a move-entry field first.

| Key | Action |
| --- | --- |
| Enter | Play the typed move if it is legal. |
| Backspace | Remove the last character. |
| Escape | Clear the input and dismiss the overlay. |

## Accepted move notation

| Example | Meaning |
| --- | --- |
| `e4` | Move a pawn to e4. |
| `Nc3` or `nc3` | Move a knight to c3. |
| `exd5` | Capture on d5 with the e-file pawn. |
| `O-O`, `o-o`, or `0-0` | Castle on the king side. |
| `O-O-O` | Castle on the queen side. |
| `a8=Q` or `a8=q` | Promote a pawn to a queen on a8. |
| `e2e4` | Move from e2 to e4 using coordinate notation. |

Piece letters can be lowercase. Pawn notation is checked first, so `b4` still means the b-pawn move. If two pieces of the same type can reach a square, include the required file or rank, such as `Nbd2`. You can omit a trailing check or checkmate marker.

An illegal or unrecognized move stays in the overlay with an error message. Correct it with Backspace and press Enter again, or cancel with Escape. Typing does not pause the game clock.

Move entry works in local games, [Training vs AI](/help/training-vs-ai/), and online games when it is your turn. It pauses while an app dialog or text field is active, while viewing past positions, and during your opponent's turn. In Play Both Sides, you can type moves for either color.

## Navigate positions

Use **Left Arrow** and **Right Arrow** to move backward and forward through game positions. In a [larger Saved Games preview](/help/saved-games-and-backups/), these keys select the previous or next move and pause autoplay.

## Drawing shortcuts

With the **Draw on board** window open, **Ctrl+Z** undoes the last drawing change for the displayed position. It covers additions, edits, deletions, and Clear. Active text fields keep their own text undo behavior.

**Alt-click / Alt-drag** draws a green square or arrow without enabling drawing mode. Add **Shift** for red or **Ctrl** for yellow. While dragging an arrow with the graphical tools, hold left-click and tap right-click to toggle solid/dashed or left/right curve styles. See [Drawing arrows and highlighting squares](/help/drawing-on-the-board/) for the full controls.

Voice move entry is not currently built into Ironwood.


## Focus mode

**Shift+Enter** toggles focus mode when you are not editing a text field. **Escape** exits focus mode. The board toolbar’s focus icon provides the same toggle. See [Focus on the board](/help/playing-a-game/#focus-on-the-board).

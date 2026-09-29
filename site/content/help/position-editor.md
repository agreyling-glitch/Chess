---
title: "Using the Position Editor"
description: "Set up a board, import a screenshot, check position details, and start from your custom position."
category: "using-ironwood"
keywords: [Position Editor, screenshot, image, FEN, custom position, set up board, crop]
weight: 90
---

Use the Position Editor to build a puzzle or a custom starting position without typing a [FEN](/help/fen/) yourself.

To bring in a puzzle from a different website, follow [Import a chess puzzle from another site](/help/import-a-puzzle/) for separate FEN and screenshot instructions.

## Open it

Choose **Tools → Position Editor** to begin with the current board. If you are creating a new local game, choose **Game → New Game → Local → Starting position: Custom FEN → Edit board / Import screenshot**. This route returns the edited FEN to the New Game dialog.

## Arrange pieces

Select a piece from the palette, then click a square to place it. Drag an existing piece to another square to move it. Right-click a square, select the erase tool and click, or drag a piece into the trash to remove it. **Starting position** resets the board; **Clear** empties it. **Flip view** changes only which side you see at the bottom. **Rotate position** turns the actual arrangement around, so use it only when the pieces are facing the wrong way.

The editor requires exactly one king of each color and no pawn on the first or eighth rank. Ironwood also checks the completed FEN before starting a game.

## Import a board image

Choose an image file, paste an image into the editor, or drop one onto it. Crop closely around the board for the best result. Recognition runs on your device, but its model may need to download when first opened. Review **every piece** after import; orange outlines flag squares the recognizer is less sure about. The Cburnett and Merida piece sets are the tested examples, so other artwork may be less reliable.

An image cannot reveal game history. After importing, set **Side to move**, **Castling rights**, **En passant target**, **Halfmove clock**, and **Fullmove number** yourself. Use `-` for no castling right or en passant target. Standard castling rights use `KQkq`; Chess960 can use rook-file letters. If you do not know a right still exists, do not guess from the picture.

## Use the position

Click **Use in Ironwood** to send the FEN back to the game. For a new game, check that the New Game dialog says **Valid position**, then start it. **Copy FEN** copies the position text to share or save. **Load FEN** accepts an existing complete six-field FEN and fills the editor from it.

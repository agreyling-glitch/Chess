---
title: "What is FEN?"
description: "Learn how a FEN records one chess position and how to use it in Ironwood."
category: "terms"
keywords: [FEN, Forsyth Edwards, position, board, castling, en passant, custom]
weight: 85
---

**FEN** stands for **Forsyth–Edwards Notation**. It is a short line of text describing *one position*: which pieces are on the board, whose turn it is, and a few rules that depend on what happened earlier. It is useful for sharing a puzzle or starting a new game from a particular setup. It does not contain the moves that led there; use [PGN](/help/pgn/) for a whole game.

The normal starting position is:

```text
rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1
```

The six space-separated parts mean:

1. **Piece placement.** The eight slash-separated rows run from Black's back rank to White's back rank. Letters name pieces; capital letters are White, lowercase letters are Black. Numbers count empty squares.
2. **Side to move.** `w` means White moves next; `b` means Black.
3. **Castling rights.** `KQkq` says both sides may still castle on either side, if the other conditions are met. `-` means no castling rights. Chess960 positions can use rook-file letters.
4. **En passant target.** A square where an en passant capture is available, or `-` if none. En passant is a special pawn capture immediately after an opposing pawn moves two squares.
5. **Halfmove clock.** The number of half-moves since the last pawn move or capture; it matters for the fifty-move draw rule.
6. **Fullmove number.** The current move number, beginning at 1 and increasing after Black moves.

You do **not** need to write FEN by hand. In Ironwood, use **Tools → Position Editor** to arrange the board and copy its FEN. To play from it, choose **Game → New Game → Local → Starting position: Custom FEN**, paste the complete FEN, and check that Ironwood says the position is valid. You can also use **Edit board / Import screenshot** from that dialog. See [Using the Position Editor](/help/position-editor/) for a walkthrough.

Be careful with the non-board fields: a picture alone cannot tell whose turn it is or whether castling and en passant are still legal.

If you copied a FEN from a puzzle website, follow [Import a chess puzzle from another site](/help/import-a-puzzle/) to set it up and explore the moves in Ironwood.

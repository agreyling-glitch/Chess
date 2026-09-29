---
title: "How to read chess moves"
description: "A beginner's guide to board squares, piece letters, captures, check, and castling."
category: "terms"
keywords: [SAN, notation, moves, squares, check, checkmate, capture, castling]
weight: 87
---

Board squares have a letter **a–h** for the file (column) and a number **1–8** for the rank (row). `e4` is one square. When reading from White's side, `a1` is the bottom-left corner.

The usual move list uses **SAN**, or **Standard Algebraic Notation**. A capital letter names a piece: `K` king, `Q` queen, `R` rook, `B` bishop, and `N` knight. Pawns use no piece letter. For example, `Nf3` means a knight moves to f3, while `e4` means a pawn moves to e4.

| Mark | Meaning | Example |
| --- | --- | --- |
| `x` | Capture a piece | `Bxe6` means a bishop captures on e6. |
| `+` | Give check | `Qh5+` attacks the king. |
| `#` | Give checkmate | `Qxf7#` ends the game by checkmate. |
| `O-O` | Castle kingside | The king and rook move together toward the h-file side. |
| `O-O-O` | Castle queenside | The king and rook move together toward the a-file side. |
| `=Q` | Promote a pawn to a queen | `e8=Q` means the pawn reaches e8 and becomes a queen. |

Sometimes a file or rank is added to distinguish two pieces that could reach the same square, such as `Nbd2`. `1.` is White's first turn; `1...` marks Black's first turn when shown separately.

Engine data may use **UCI coordinates** instead: `g1f3` means a piece moves from g1 to f3. The letters in a UCI move are square names, not a sentence. See [Move symbols](/help/move-symbols/) for Ironwood's separate quality marks such as `?` and `??`.

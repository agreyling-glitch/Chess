---
title: "What do the move symbols mean?"
description: "A quick guide to the star, plus, question marks, and move notation shown during review."
category: "analysis"
keywords: ["?", "??", "?!", star, plus, symbols, annotations, move list]
weight: 35
---

After analysis, Ironwood puts a symbol next to a move to summarize its [move-quality label](/help/move-quality/):

| Symbol | Ironwood label | Meaning |
| --- | --- | --- |
| **★** | Best | Stockfish's choice, or virtually no estimated loss. |
| **+** | Good | A sound move with only a small estimated loss. |
| **?!** | Inaccuracy | A small missed opportunity. |
| **?** | Mistake | A meaningful loss of chances. |
| **??** | Blunder | A severe loss of chances. |

These are Ironwood's analysis marks. They are not move commands or a judgment of the player. A mark may change after the deeper critical-move check finishes. The thresholds use estimated loss of *winning chances*, so `?` does not mean a fixed number of [centipawns](/help/accuracy-and-cpl/).

**Do not confuse the `+` quality mark with `+` in chess move notation.** In a move such as `Qh5+`, the trailing `+` means *check*: the move attacks the king. A trailing `#`, as in `Qxf7#`, means *checkmate*. `O-O` means castling toward the kingside and `O-O-O` means castling toward the queenside. See [How to read chess moves](/help/chess-notation/) for the rest of the notation.

## Your own annotations

You can also right-click a move and choose **Add annotation…** to add your own quality or position-assessment symbol. Manual annotations appear in gold beneath the move text and stay separate from Stockfish's marks. See [Taking notes and adding annotations](/help/taking-notes/) for the symbols, editing steps, and export behavior.

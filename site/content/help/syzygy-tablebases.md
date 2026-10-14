---
title: "Syzygy endgame tablebases"
description: "Read exact endgame outcomes, understand DTZ and DTM, and explore branching continuations on the board."
category: "analysis"
keywords: [Syzygy, tablebases, endgame, seven pieces, DTZ, DTM, half-moves, cursed win, blessed loss, 50-move rule, continuation, hover, arrow, exploration tree]
weight: 76
lastmod: 2026-10-14T00:00:00-05:00
---

## Look up an endgame

Open a completed game and select an endgame position in **Game Moves**. In **Game Analysis**, below the evaluation graph, select **Tablebases**, then **Look up this position**. You can also import a setup position for study. The Tablebases tab is unavailable during an active online game.

Ironwood supports standard positions with **seven pieces or fewer, counting both kings**, and **no castling rights**. A king, rook, and pawn against a king and rook is a five-piece position. Pawns count as pieces for coverage. Positions outside this range show **Outside tablebase coverage**.

Lookup requires an internet connection but does not require a Lichess account or sign-in. Ironwood sends the selected position's FEN to Lichess's tablebase service. You do not need to download tablebase files or finish a Stockfish analysis first.

## Read the position result and the move rows

The result card names the side to move and shows that side's outcome. A continuation row instead shows the outcome **for the player making that move**. For example, if Black is to move, **Win** beside a Black move means that move preserves a win for Black. After you click it, the new position has White to move, so its card may say **Loss**. Those labels agree: the viewpoint has changed.

| Outcome | How to read it |
| --- | --- |
| **Win** | A winning tablebase outcome with best play. |
| **Draw** | A drawn outcome with best play. A mistake can still change the result. |
| **Loss** | A losing outcome against best play. Explore alternatives to see how the defender can resist. |
| **Draw under 50-move rule (cursed win)** | Winning without the rule, but the rule prevents forcing a win. |
| **Draw under 50-move rule (blessed loss)** | Losing without the rule, but the rule permits a draw. |
| **Uncertain — see Lichess** | The service returned a category Ironwood does not present as a definite win, draw, or loss. Use the Lichess link for more context. |

In the move list, the longer rule-related labels are shortened to **Draw · 50-move rule**. Hover the row for its full outcome. Ironwood preserves Lichess's move ranking; the first row has a gold marker. Consider the outcome before comparing distances.

## DTZ: progress toward resetting the clock

**DTZ** is distance to zeroing: progress toward a pawn move or capture that resets the 50-move counter, or toward mate. It is not generally a mate distance. The service supplies a rule-aware DTZ value that may be rounded.

## DTM: distance to mate

**DTM** is distance to mate. It is displayed only when supplied by the service. A missing badge means that distance is unavailable, not zero. DTM alone does not establish whether the 50-move rule permits a win; read the outcome too.

Both distances count **half-moves (plies)**: one move by one player. Two half-moves are one White move and one Black move. Ironwood displays distance magnitudes without the API's sign; the outcome label supplies the winning or losing viewpoint.

The badges beside a continuation describe the position **after that move**. The distances in the main card describe the currently displayed position. This distinction matters when counting a line from the board before the listed move.

| Row badge | Meaning |
| --- | --- |
| **DTZ 2** | The resulting position has a reported zeroing distance of two half-moves. It does not necessarily mean mate in two. |
| **DTM 6** | The resulting position has a reported mate distance of six half-moves. |
| **Pawn move** | This continuation itself resets the clock with a pawn move. This label replaces its DTZ badge. |
| **Capture** | This continuation itself resets the clock with a capture. This label replaces its DTZ badge. |
| **DTM 0** | The resulting position is already mate when the API supplies this value. |

Two winning continuations can have the same DTZ but different DTM values. One can make similar progress toward resetting the clock while taking longer to mate. The ranked list favors tablebase progress and does not promise that every first-row move minimizes DTM.

For the supplied distance fields and ranking convention, see the [Lichess tablebase API documentation](https://github.com/lichess-org/lila-tablebase#http-api).

## Preview arrows and explore a line

**Hover a continuation** to show its move arrow on the main board. The pieces stay in place until you click. Leaving the row clears the arrow. Hover works on the flat and full 3D boards; touch users can click or tap a continuation to explore it.

**Click a continuation** to display the resulting position and look up its replies. The **Preview · … plies** label shows how far you have explored from the selected game position. This exploration does not add moves to your game or replace its recorded moves.

The continuation labels, tooltips, and exploration-tree moves follow your **Letters / Figurines** notation preference.

## Keep and compare branches

After your first explored move, the **Exploration tree** records the branch. Click any tree position to return to it and examine another reply. Existing sibling branches stay available, so you can compare defenses without rebuilding the first line.

- **Back** returns to the parent position, one preview half-move earlier.
- **Reset preview** returns to the game position while keeping explored branches.
- **Game position** in the tree also returns to the root.
- **Disclosure triangles** collapse or expand branches. Selecting a descendant opens its ancestors.
- **Clear** deletes the explored branches and returns to the game position.

The tree holds up to 256 explored positions. Clear it to start another exploration when that limit is reached. Selecting another game position starts a new root and clears the previous position's exploration.

## Check the 50-move clock

Open **Position settings & API details** to inspect **50-move clock at root (plies)**. The clock counts half-moves since the most recent pawn move or capture. A clock of 80 represents 40 full moves without either event. At 100 half-moves, the usual 50-move draw-claim threshold is reached.

Recorded moves determine the default. For imported setup positions, check that the clock in the supplied FEN matches the position you intend to study. You can edit the root clock when you are at the game position; changing it clears explored branches so subsequent lookups use the new starting value. During exploration, pawn moves and captures reset the clock, and other moves increase it by one.

A FEN describes a position and its counters, not the full game history. Tablebase lookup is an endgame study tool; it does not reconstruct repetition claims or adjudicate every game-history condition.

## Cached results, refresh, and connection problems

**Lichess API** means this lookup was fetched from the service. **Cached result** means Ironwood reused a result from its in-memory cache. Normal redraws do not repeatedly fetch the position. Returning to an explored position can reuse the cache.

Under **Position settings & API details**, **Refresh tablebase result** bypasses that cache and requests the displayed position again. **Open this position on Lichess** opens the selected preview position, including its move counters, in a new tab.

If a request fails, check your connection and use Refresh. A rate-limit message asks you to wait one minute before retrying. An unavailable position or missing distance value should not be read as a draw. Use the Lichess link for a second view of the position.

## Compare with Stockfish

Stockfish's evaluation and prediction line come from its engine search. Tablebases give endgame outcomes for covered positions. The engine panel and tablebase preview can refer to different positions while you explore; return to **Game position** before comparing the original game position with its engine analysis.

For a useful study exercise, identify which moves preserve the result, compare their DTZ/DTM badges, then explore the opponent's replies. A drawn endgame may still demand a precise defense, and a winning one may require careful clock management.

## Panel layout

Sunken sections group exploration controls, position settings, outcomes, and continuations. Tablebase access requires internet but no Lichess sign-in, so it does not need to replace a FICS connection. Tablebases remain unavailable during live online play and training.

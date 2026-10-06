---
title: "Pawn structure, piece mobility, and king safety"
description: "Explore the positional insight tabs, compare pieces, and follow changes through a game."
category: "analysis"
keywords: [pawn, structure, chains, isolated, passed, mobility, piece, compare, shift, king, safety, shelter, pressure, timeline]
weight: 75
lastmod: 2026-10-05T00:00:00-05:00
---

## Open the positional insight tabs

Open a played or imported game and choose **Game Analysis**. Below the evaluation graph, select **Summary**, **Pawn Structure**, **Piece Mobility**, or **King Safety**. The three positional views work directly from the game positions; they do not require a completed Stockfish analysis. They follow the selected move as you navigate the game.

Drag the left edge of the right panel to give these views more horizontal room. Scroll the panel to reach the lower content. The structure and safety evolution tables show ten rows at a time, with additional entries scrollable.

## Pawn Structure

Choose **Chains**, **Isolated**, or **Passed** to highlight that pattern in gold on the pawn diagram. White is at the bottom. The White and Black cards list counts and squares beside the board.

| Pattern | What Ironwood counts |
| --- | --- |
| **Chains** | Connected groups of friendly pawns linked by diagonal protection. A group must contain at least two pawns. |
| **Isolated** | Pawns with no friendly pawn on either adjacent file. |
| **Passed** | Pawns with no enemy pawn ahead on the same or adjacent files. |

A chain count is a count of groups; isolated and passed counts are counts of individual pawns. The same pawn can belong to more than one pattern.

The **Structure evolution** table includes the starting position and positions where pawn locations change. Its grouped White and Black columns use **C** for chains, **I** for isolated, and **P** for passed. Click a row to navigate to that position.

## Piece Mobility

Select a piece in the first selector, then optionally choose another in **Compare…**. You can also **click a piece on either the mini board or the full board to select it**, or **Shift-click a piece to compare it**. Full-board selection applies while the Piece Mobility tab is open and the board shows a game position, rather than an engine prediction.

The gold and blue lines chart each selected piece’s available destinations after each move. Hover the graph for the position and counts. Click or drag it to navigate the game. On the mini board, outlines identify the selected pieces and colored dots mark their available destinations.

Mobility is calculated for either color independently of whose turn it is. The count respects pins and king safety, excludes castling and en passant, and counts a promotion square once rather than counting each promotion choice separately. Each physical piece is tracked through moves and promotion; its line ends when it is captured. Pieces are named by their color, original type, and starting square, so two knights remain distinguishable.

More destinations do not automatically mean a stronger piece: defending a key square or blocking an attack can also be valuable.

## King Safety

Choose **White king** or **Black king** to change the diagram. Both players’ cards remain visible with an assessment and concrete reasons, including nearby shield pawns, missing shelter, pawn-open files, and enemy pressure.

| Assessment | Meaning |
| --- | --- |
| **Sheltered** | The structural checks find nearby pawn shelter without the exposure or pressure signals used by this view. |
| **Weakened** | Some shelter, file, or pressure concerns are present. |
| **Exposed** | Several concerns combine, or substantial enemy pressure surrounds the king in a low-material position. |
| **In check** | An enemy piece currently attacks the king. This takes priority over the other assessments. |
| **Endgame** | Low remaining material makes king activity more relevant; missing shelter and open files alone do not lower the assessment. |

Shield pawns are friendly pawns one or two ranks ahead of the king on its file or an adjacent file. A pawn-open file contains no pawn of either color; a nearby file without a friendly pawn can also weaken shelter.

The low-material rule uses **26 or fewer combined non-pawn material points**: knights and bishops count as 3, rooks as 5, and queens as 9. This king-safety rule applies directly to the selected position; unlike the game-phase summaries, it does not reserve the first ten full moves for the opening.

The diagram uses **gold** for the king, **green** for shield pawns, **red** for attacked king-area squares, and **amber** for nearby pawn-open files. **Safety evolution** lists changes in either king’s shelter or pressure. Click a row to inspect the position, even if the assessment label has stayed the same.

King Safety is a structural guide, independent of Stockfish’s evaluation. Pressure includes pinned attackers, and a pawn-open file may still be blocked by other pieces. Use engine analysis to investigate whether a threat leads to a forced attack.

## Expand the graph or print a report

Above the evaluation graph, **Expand** opens the full-screen evaluation view. The positional insight tabs remain in the main analysis panel. The gold **Print analysis** button beside Expand opens the report prompt once at least one position has engine analysis; pause or stop a running analysis first. Choose **Save as PDF** in the browser’s Print dialog to save the report.

See the [Analysis gallery](/gallery/#analysis) for screenshots of all three positional views.

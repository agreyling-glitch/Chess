---
title: "Game phases and critical moves"
description: "How Ironwood groups moves and chooses moments for a closer look."
category: "analysis"
keywords: [opening, middlegame, endgame, phase, critical, turning point, verification]
weight: 70
---

## Opening, middlegame, endgame

Ironwood uses the following rules to classify each move:

| Phase | Rule |
| --- | --- |
| **Opening** | The first **20 half-moves** in the game: 10 moves by White and 10 by Black. |
| **Middlegame** | After the opening, the combined non-pawn material total is **above 26 points**. |
| **Endgame** | After the opening, the combined non-pawn material total is **26 points or fewer**. |

One half-move is a move by one player. The opening rule takes priority even if little material remains during those first 20 half-moves.

To calculate the material total, count the pieces remaining for **both players together**:

| Piece | Points per piece |
| --- | --- |
| Knight | 3 |
| Bishop | 3 |
| Rook | 5 |
| Queen | 9 |
| Pawn or king | Not counted |

For example, if each player has one rook, one bishop, and one knight, the total is **22 points**: `(5 + 3 + 3) × 2`. After the opening, that is classified as endgame. Adding one queen makes the total **31 points**, which is middlegame.

The classification uses the board **before each move**. A capture that reduces the total to 26 or below therefore changes the phase for the following move. Material is checked again for each move; a promotion can raise the total back above the threshold.

These are approximate study categories, shared by the expanded graph, phase summaries, and printed reports. They may differ from how a player describes the position. The phase summary shows accuracy and average CPL for moves in each group; use those numbers to help choose what to study.

## Phases on the expanded graph

Right-click the analysis graph and choose **Expand analysis graph**. Colored background bands mark the phases across the move timeline: **blue** for opening, **purple** for middlegame, and **green** for endgame. A colored strip and boundary lines show where each phase starts and ends; phase names appear above bands when there is room. The legend remains visible even for a short phase.

Hover a position to see its phase in the tooltip, or click it to see **Game phase** in the details below. Phase shading also covers unanalyzed positions; gaps in the evaluation line still mean those positions have not been analyzed. The graph uses the same phase classification as the summaries and report. These are approximate study categories, so their boundaries may differ from how you would describe the game yourself.

## Critical moves

An **inaccuracy**, **mistake**, or **blunder** may be a critical move because it changes the player's chances. Mate-related swings can also be critical. Ironwood can revisit positions immediately before and after these moves with a larger search budget. This is **verification**: a second look intended to make the judgments more reliable.

A **turning point** is a position highlighted because the evaluation or move quality changed sharply. It is a place to start asking what changed on the board. It does not automatically mean that one move alone decided the game.

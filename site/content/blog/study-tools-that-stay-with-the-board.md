---
title: "Study tools that stay with the board"
date: 2026-10-05T00:00:00-05:00
description: "Floating notes, tactical patterns, and editable drawings make room for the move list, while phase highlights bring structure to game review."
slug: "study-tools-that-stay-with-the-board"
---

A game review often starts with a question: what changed here? You might want to write an explanation, trace a piece’s route, or check whether a newly exposed piece creates a tactic. Those tools are useful beside the board, but putting every control in Game Moves makes it harder to follow the game.

Ironwood now gives **Draw on board**, **Notes**, and **Tactical Map** their own translucent floating windows. Their icons sit above the board, to the left of **2D/3D**. Open the tools you need, drag their titles to arrange them, and close them when you want more space.

## Keep the position and its context together

The Notes window has two tabs. **Note** follows the displayed position; **Start** lets you read the game’s introduction without taking the board away from the move you are studying. The title names the move in notation and plain language, such as **Notes: Nxb5 (Knight to B5)**.

Double-click the note text to edit it, or double-click the empty-note prompt to start writing. Double-clicking a move in Game Moves opens its note editor directly. The collapsible **All notes** list gathers the game’s written observations in one place, including the starting note. [Notes help](/help/taking-notes/) explains editing and sharing.

## Draw a plan, then refine it

The drawing window offers four colors, squares and circles with solid or dotted outlines, and solid, dashed, or curved arrows. A curve can show a route without covering another line; dotted outlines can distinguish one group of squares from another.

You can change an arrow before releasing it. Keep holding left-click and tap right-click to switch between solid and dashed, or to reverse a curve’s direction. The preview updates immediately. Drawing also stays aligned as you zoom the 3D board.

Expand **Drawings** to change a color, move a shape to another square, adjust an arrow’s endpoints, or delete one entry. The list grows when you make the window taller. The window keeps a compact width, and its header has a drawing switch, Clear, and Undo. **Ctrl+Z** undoes drawing changes for the displayed position while the tools are open, including a deletion or Clear.

Saved games and Analysis JSON preserve the styles, as do annotated board images. Annotated PGN includes an Ironwood extension for styles; another chess app may show simpler arrows or square outlines. [Drawing help](/help/drawing-on-the-board/) covers the controls and export behavior.

## Trace a tactical clue

Open the bullseye to inspect Tactical Map findings. **Danger** identifies attacked pieces that gained attackers or lost defenders after the previous move. **Pins** finds pieces shielding their king from a sliding attacker. **Forks** lists legal candidate moves that attack at least two enemy pieces other than pawns.

Select a card to see its relevant squares and attack lines on either board view. These findings help you choose what to examine; they do not prove a material win. Use Stockfish to test the opponent’s replies. The map is hidden during rated training games, active online play, best-move exercises, and prediction previews. [Tactical Map help](/help/tactical-map/) describes its limits.

## See where the game changes phase

The expanded analysis graph now shades the **opening in blue**, **middlegame in purple**, and **endgame in green**. Boundary lines and labels show the ranges, while the legend and selected-position details keep the phase readable even when a band is narrow. Evaluation gaps still mean positions have not been analyzed.

The graph shares the summaries’ practical classification: the first 20 half-moves are opening. After that, Ironwood counts both sides’ non-pawn material before each move—three points for a knight or bishop, five for a rook, and nine for a queen. A total above 26 is middlegame; 26 or fewer is endgame. These are study categories, so they may differ from how you describe the position yourself.

Use the bands to find a part of the game to review, then inspect the moves, add a note, or draw the plan you missed. The [phase guide](/help/game-phases-and-critical-moves/) explains the rules with examples, and the [Gallery](/gallery/) shows the updated graph.

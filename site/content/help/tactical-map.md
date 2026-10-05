---
title: "Tactical Map"
description: "Explore changes in danger, absolute pins, and legal fork candidates on the board."
category: "analysis"
keywords: [tactics, tactical map, danger, pins, forks, threats]
weight: 18
---

The Tactical Map helps you inspect threats and tactical patterns in the displayed position. It works on both the 2D and 3D boards without requiring an engine analysis run.

## Open and arrange the map

Click the **bullseye icon** immediately to the left of **2D/3D** above the board to open the translucent **Tactical Map** window. The Notes icon is to its left, followed by the Draw on board pencil farther left. The map has its own window rather than a section in Game Moves.

Drag the window to reposition it or resize it. Click the bullseye again, or the close button, to close the window and hide its overlays.

## Filter and inspect patterns

The header contains **Danger**, **Pins**, and **Forks** filter buttons with counts for the displayed position. Click a filter to show or hide that kind of finding. The color matches the corresponding cards and board markings.

Select a finding card to outline its relevant squares and trace attack lines. Select the same card again to clear the selection. A fork card identifies the candidate’s starting and destination squares, for example **Fork candidate · d1 to d7**; its description names the targets. Selecting it illustrates the candidate without playing the move.

## What the findings mean

- **Danger** compares the displayed position with the previous move. It identifies surviving pieces on the same squares that are attacked and have gained attackers or lost defenders. Coral outlines mark these pieces.
- **Pins** identifies pieces shielding their king from an enemy bishop, rook, or queen. Purple outlines mark the pinned pieces.
- **Forks** lists legal moves for the side to move that make the moving piece attack at least two enemy pieces other than pawns. Select a candidate to see its destination and targets in blue. All promotion choices are considered.

The current map reports Danger, absolute Pins, and Fork candidates; it does not provide a skewer category or a complete heatmap of every vulnerable square.

These are geometric clues, not engine-confirmed wins. Attack and defender counts include pinned pieces; they do not establish which captures are legal or profitable. Test the opponent's responses with analysis before concluding that a tactic wins material. Danger does not cover all weak or vulnerable empty squares.

## When the map is available

The map is hidden during rated training games, active online play, best-move exercises, and prediction previews. Its controls are temporary for the current session. Automatic findings do not alter your saved notes or personal board drawings.

Use [Draw on board](/help/drawing-on-the-board/) to add your own persistent arrows and shapes, or [Notes](/help/taking-notes/) to record what you learned from a position.

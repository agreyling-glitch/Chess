---
title: "Following the position, then choosing what to practice"
date: 2026-10-13T00:00:00-05:00
description: "More direct board controls, clearer tactical candidates, and a route from training analysis into phase puzzles."
slug: "following-the-position-then-choosing-what-to-practice"
---

This update connects the act of reviewing a game with the next practice session. It also brings more of the same interaction to the 2D and 3D boards.

## Keep your attention on the board

You can now drag a piece to its destination in either view. A translucent preview follows your pointer; in 3D it uses the selected model and camera angle. Dropping outside the board or on an illegal square cancels the move, as does Escape.

The mouse wheel steps through game history: down moves forward and up moves backward. Select a move in Engine prediction to explore that line with the same gesture, then select Game Moves to return. Active training and online games keep wheel navigation disabled.

For the 3D camera, hold the right mouse button while scrolling to zoom. Hold Shift during a right-drag to adjust tilt while keeping rotation fixed. These controls also work in Focus mode, where active-game buttons now remain beneath the board so resignation and available draw actions stay accessible.

## Make feedback useful during play and quiet during review

**Settings → Move feedback** now offers a saved check-square highlight and a capture dissolve. Live captured pieces briefly shrink and fade with a gold pulse; the 3D model also sinks slightly. Replaying or stepping through a game does not trigger the effect. System reduced-motion preferences suppress capture and live-move animations.

Tactical Map now places Knight reach ghosts and attacker/defender badges on the 3D board. They follow rotation, tilt, and zoom, helping you inspect routes in the view you are already using.

Fork cards also explain more of their immediate context. A checking fork against a major piece, or a fork against multiple major pieces, receives the strong-candidate label only when the forking piece has no immediate legal capture. Capturable candidates list those legal replies. This is a pattern check, not proof of a forced material win: the opponent may have other ways to answer.

## Turn a training recommendation into practice

When the training dashboard has enough analyzed moves to recommend a phase, **Practice this phase** opens a puzzle with Opening, Middlegame, or Endgames selected. Choose the difficulty before launching, or change it later in the Practice tab. Those phase themes are also available directly in Puzzles.

These are tactics drawn from the selected phase. Difficulty uses Lichess's anonymous baseline of 1500 rather than your Ironwood training rating, so the two numbers should not be read as equivalent. Puzzles now load into the main board and record played attempts under Saved Games → Puzzles. Save any game you want to keep before switching, then reopen it from Saved Games afterward. Finish active training or online play before opening puzzles.

See [board controls](/help/board-and-view/), [Tactical Map](/help/tactical-map/), and [training practice](/help/training-vs-ai/#finding-areas-to-focus-on) for the details, or browse the [complete release notes](/changelog/2026-10-13-board-controls-and-focused-practice/).

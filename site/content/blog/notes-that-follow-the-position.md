---
title: "Notes that follow the position"
date: 2026-10-02T00:00:00-05:00
description: "Bring your own reasoning into game review with visible notes, manual symbols, and colored board drawings."
slug: "notes-that-follow-the-position"
---

A saved observation is useful when it appears where you need it. Ironwood already supported notes on moves and the starting position; this update brings those notes into the main board workspace. As you step through a game, the note panel follows the displayed position. Add or edit your thoughts there, and scroll longer notes without losing the board.

The initial position now has its own visible row in Game Moves. A Starting note can explain an opening plan, introduce a puzzle, or describe a custom setup. Later in the game, **View starting note** takes you back to that context. Saved Games shows a short preview before you open the game, and the larger board preview displays the full Starting note beneath its labeled final position.

## Your assessment beside the engine's

Stockfish provides one view of a position; your own reasoning is also worth recording. **Add annotation…** in a move's right-click menu lets you choose a move-quality symbol such as `!`, `!?`, or `?`, plus a position assessment such as equal, unclear, or an advantage for either side.

These manual symbols appear in gold beneath the move text and stay separate from Stockfish's assessments. You can change a selection, toggle it off, or clear it. A written note can explain why a move seemed interesting or what you missed. Slight and clear advantage symbols use readable text forms so they display reliably with the app's fonts.

## Show the plan on the board

Some ideas are easier to draw. An arrow can show a pawn push or a piece route; an outlined square can identify a target or a weakness. Ironwood now supports green, red, and yellow drawings on both its 2D and 3D boards.

Use **Draw on board** and choose a color, then click a square or drag between squares. Keyboard users can draw temporarily with Alt, adding Shift for red or Ctrl for yellow. Repeat a drawing to remove it, redraw it in another color to change it, or clear all drawings for the position. Turn drawing mode off to play moves again.

Drawings belong to positions, including the initial board, and return when you revisit them. They remain separate from engine arrows and are hidden while you step through a predicted engine line.

## Keep the work with the game

Saved games and Analysis JSON preserve notes, manual symbols, and drawings. Annotated PGN carries manual symbols as standard numeric annotation glyphs and drawings as arrow and square comment extensions. Other chess programs may support those extensions differently. Plain PGN provides the game record without personal annotations, while printed reports continue to include written notes.

The help center now has guides for [taking notes and adding annotations](/help/taking-notes/) and [drawing arrows and highlighting squares](/help/drawing-on-the-board/), including shortcuts, removal, and export behavior.

[Explore the features →](/features/)

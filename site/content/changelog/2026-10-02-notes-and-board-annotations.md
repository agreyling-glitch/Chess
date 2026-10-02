---
title: "Notes beside the board and personal annotations"
date: 2026-10-02T00:00:00-05:00
description: "Visible Starting notes, manual annotation symbols, colored board drawings, and dedicated help guides."
slug: "2026-10-02-notes-and-board-annotations"
version: "2026.10.02"
---

## Added

- **Add annotation…** in the Game Moves right-click menu, with manual move-quality and position-assessment symbols. Select one from each group, replace or toggle a selection, or use **Clear annotation**. Gold manual symbols remain separate from Stockfish's assessments.
- Colored arrows and square outlines on the 2D and 3D boards. Use **Draw on board** and Green, Red, or Yellow controls, or Alt-click/drag shortcuts: Alt for green, Alt+Shift for red, and Alt+Ctrl for yellow.
- Position-specific drawings that toggle off when repeated, change color when redrawn, and can be removed with **Clear drawings**. Saved games, annotated PGN, and Analysis JSON preserve them.
- Dedicated help guides for [taking notes and adding annotations](/help/taking-notes/) and [drawing arrows and highlighting squares](/help/drawing-on-the-board/).

## Improved

- The main board workspace shows the displayed position's note, with Add/Edit controls and scrolling for long notes in both Compact and Expanded layouts.
- A **Starting position** row in Game Moves opens the initial board and its note. **View starting note** keeps that context accessible from later positions.
- Saved Games shows a two-line Starting note preview. Clicking it opens the starting position; the larger final-board preview displays the full Starting note beneath the board.
- Manual symbols round-trip through standard PGN numeric annotation glyphs. Drawings use `[%cal ...]` and `[%csl ...]` comment extensions; compatibility with other programs depends on their support for those extensions. Plain PGN omits personal annotations.
- The Features page has dedicated notes and drawing sections, with links to help. Each feature heading remains paired with all its paragraphs in the desktop and narrow-screen layouts.

## Fixed

- Missing font glyphs in the annotation menu: slight and clear advantage assessments use readable text equivalents such as `+=` and `+/-`.

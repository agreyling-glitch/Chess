---
title: "Board controls and focused practice"
date: 2026-10-13T00:00:00-05:00
slug: "2026-10-13-board-controls-and-focused-practice"
version: "2026.10.13"
description: "Drag moves, wheel navigation, capture feedback, 3D tactical overlays, and phase puzzles from training recommendations."
---

## Playing and reviewing

- Drag pieces to legal destinations in 2D and 3D, with translucent previews using the current 3D model and camera. Illegal drops, drops outside the board, and Escape cancel the drag; promotions keep the piece chooser.
- Scroll down over either board to advance through moves and up to go backward. Select an Engine prediction move to navigate that line, then a Game Moves entry to return to game history. Active training and online play disable wheel navigation.
- Hold the right mouse button while scrolling to zoom in 3D. Shift while right-dragging locks rotation and changes tilt; Shift plus right-button scrolling still zooms.
- Added saved **Capture dissolve** and **Highlight king in check** preferences under **Settings → Move feedback**. Capture effects run during live captures only; capture and live-move animations respect system reduced-motion settings.
- Kept active-game controls beneath the board in Focus mode. Training resignation requires confirmation and records a loss. Local two-player draw offers can be accepted or declined, with agreed results saved. Engine opponents do not negotiate draw offers.

## Tactical Map and practice

- Added 3D Knight reach ghosts, route interaction, and board-surface attacker/defender badges that follow camera changes.
- Distinguished strong, checking, and capturable fork candidates, with target badges, matching overlay colors, and immediate legal captures of the forking piece. The labels do not prove a material win.
- Added Opening, Middlegame, and Endgames puzzle themes. **Practice this phase** launches a matching puzzle from a training recommendation with a difficulty selector, using Lichess's anonymous 1500 baseline independently of training ratings.
- Standardized puzzle-board pieces on Merida, refreshed the training status card, and redesigned the disconnected Lichess lobby around sign-in and reconnect actions.

## Documentation

- Updated [Features](/features/) and the board, game, training, puzzle, and Tactical Map help pages.
- Added the journal post [Following the position, then choosing what to practice](/blog/following-the-position-then-choosing-what-to-practice/).

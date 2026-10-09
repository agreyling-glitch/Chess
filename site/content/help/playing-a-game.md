---
lastmod: 2026-10-09T00:00:00-05:00
title: "Playing a game"
description: "Start a local or online game, choose a position, and use the basic game controls."
category: "using-ironwood"
keywords: [new game, local, online, engine, Chess960, take back, resign, flip board]
weight: 11
---

Open **Game → New game** to choose how to play. A **local** game runs on your device. You can play against Stockfish or make moves for both sides. Choose your color and game settings in the New Game dialog.

In **You vs Engine**, enable **Limit to Elo rating** and choose a target from 1320 to 3190 before starting. With Elo limiting off, Ironwood uses your current skill-level setting. For a profile with separate White and Black ratings and alternating colors, choose [Training vs AI](/help/training-vs-ai/).

You can move with the mouse or [type moves with the keyboard](/help/keyboard-input/), such as `e4` or `nc3`, then press Enter.

The starting-position choices include **Standard** (the usual chess setup), **Chess960** (a legal shuffled back rank), and **Custom FEN** (a position you provide). If you have a board image or want to place the pieces yourself, choose Custom FEN and open the [Position Editor](/help/position-editor/). A [FEN](/help/fen/) describes one position; it is not a full game record.

During an ordinary local game (outside Training vs AI), **Game → Take back** undoes your last move. If Stockfish has already replied, it also undoes that reply. **Game → Resign** gives up the current game after confirmation. **Game → Flip board** changes which color appears at the bottom; it does not change whose turn it is.

For an online game, choose the Online option, then **Lichess** or **FICS**. See [Play on Lichess](/help/play-on-lichess/) or [Playing online with FICS](/help/online-play/). The **Lichess-Online** and **FICS-Online** menus provide each service's controls. Lichess computer setup offers Standard, Chess960, and From Position, with Unlimited, Real time, or Correspondence timing. See the Lichess help page for its FEN input and strength settings. When a game ends, you can [analyze it](/help/getting-started/).


## Focus on the board

Click the four-corner **Focus on board** icon immediately to the right of **Flip board** in the board toolbar. Focus mode hides the main menus, move list, analysis panels, status bar, observation tabs, game-action buttons, and floating windows. The board expands into the available space while player names, clocks, results, captured pieces, coordinates, and the board-view, flip, and focus controls remain visible. The drawing, Notes, and Tactical Map toolbar icons are hidden. This works in both 2D and 3D; moves, clocks, online connections, and engine play continue normally. Real-time position analysis pauses during focus mode and resumes after you leave if it was active or requested by a position change. Full-game analysis continues.

Press **Shift+Enter** to toggle focus mode when you are not editing a text field. Click the icon again or press **Escape** to restore the workspace. Opening the theme chooser exits focus mode to show that tool. Focus mode is temporary and is not saved with your games.

## Adjust piece appearance

Open **View → Board → Theme…** to choose from eight 2D themes and five 3D themes. Click a card’s gear to select and adjust that theme, or use **View → Board → Theme adjustments…** for the current theme. Adjustments save separately for each theme on this device. See [Board themes and appearance](/help/board-themes/) for board palettes, gradients, inner shading, outlines, materials, shadows, and lighting.

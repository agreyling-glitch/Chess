---
lastmod: 2026-10-30T00:00:00-05:00
title: "Playing a game"
description: "Start a local or online game, choose a position, and use the basic game controls."
category: "using-ironwood"
keywords: [new game, local, online, engine, Chess960, take back, resign, flip board]
weight: 11
---

The **Play** menu offers **vs AI Engine**, **Online via Lichess**, **Online via FICS**, and **Self-play**. The online choices open their lobby directly; the local choices open their own setup window. Local games run on your device; Self-play lets you make moves for both sides.

In **vs AI Engine**, choose Built-in Stockfish or an installed engine from a connected Companion using the **Engine** dropdown. The refresh icon reloads that list. For Stockfish, enable **Limit to Elo rating** and choose a target from 1320 to 3190 before starting; with limiting off, the current skill setting applies. Other engines show their advertised strength controls; engines without Elo controls use their Engine settings. For a profile with separate White and Black ratings and alternating colors, choose [Train against AI](/help/training-vs-ai/).

Click a piece and then its destination, or **hold the left mouse button on a piece and drag it** to a legal square. A translucent piece follows the pointer while you drag. Release outside the board or on an illegal square to cancel; press **Escape** to cancel a drag. Promotion opens the usual piece chooser. You can also [type moves with the keyboard](/help/keyboard-input/), such as `e4` or `nc3`, then press Enter.

On a 3D board, the dragged piece uses the current 3D model, camera angle, and appearance settings.

The starting-position choices include **Standard** (the usual chess setup), **Chess960** (a legal shuffled back rank), and **Custom FEN** (a position you provide). If you have a board image or want to place the pieces yourself, choose Custom FEN and open the [Position Editor](/help/position-editor/). A [FEN](/help/fen/) describes one position; it is not a full game record.

During an ordinary local game (outside AI training), **Take back** beneath the board undoes your last move. If Stockfish has already replied, it also undoes that reply. **Resign…** beneath the board gives up the current game after confirmation. **Flip board** above the board changes which color appears at the bottom; it does not change whose turn it is.

Active games show **Resign…** and draw controls beneath the board in both 2D and 3D. Local two-player games use **Offer draw…**, followed by the other player choosing **Accept draw** or **Decline / keep playing**. Engine opponents do not negotiate draw offers, so that button is disabled against an engine. Ordinary local games also show **Take back**; training games do not. Lichess includes draw, takeback, abort, and claim actions supported by its server; FICS includes **Offer draw**, **Accept offer**, and **Decline offer**. Finished games and replay boards hide these actions.

For an online game, choose **Play → Online via Lichess** or **Play → Online via FICS**. See [Play on Lichess](/help/play-on-lichess/) or [Playing online with FICS](/help/online-play/). The bottom **Lichess** and **FICS** buttons open each service's window directly. Lichess computer setup offers Standard, Chess960, and From Position, with Unlimited, Real time, or Correspondence timing. See the Lichess help page for its FEN input and strength settings. When a game ends, you can [analyze it](/help/getting-started/).


## Focus on the board

Click the four-corner **Focus on board** icon immediately to the right of **Flip board** in the board toolbar. Focus mode hides the main menus, move list, analysis panels, status bar, observation tabs and floating windows. The board expands into the available space while player names, clocks, results, captured pieces, coordinates, and the board-view, flip, and focus controls remain visible. The study sidebar, including Tactical and Annotations, is hidden. This works in both 2D and 3D; moves, clocks, online connections, and engine play continue normally. Real-time position analysis pauses during focus mode and resumes after you leave if it was active or requested by a position change. Full-game analysis continues.

Press **Shift+Enter** to toggle focus mode when you are not editing a text field. Click the icon again or press **Escape** to restore the workspace. Opening the theme chooser exits focus mode to show that tool. Game-action buttons remain beneath an active board in Focus mode. Focus mode is temporary and is not saved with your games.

## Adjust piece appearance

Open **Settings → Board → Theme…** to browse the available 2D piece sets and five 3D themes. Click a card’s **Adjust** button to select and adjust that theme, or use **Settings → Board → Theme adjustments…** for the current theme. Adjustments save separately for each theme on this device. See [Board themes and appearance](/help/board-themes/) for board palettes, gradients, inner shading, outlines, materials, shadows, and lighting.

## Local game setup

**Play → vs AI Engine** opens a resizable window with a 3D gold frame, **Your Side** cards, and a sunken **Game Settings** section. Choose the engine, strength, time control, and starting position, then start the game. **Play → Self-play** lets you move both sides. Online play and training have their own menu entries instead of tabs in this window.

## Move times and navigation

The **Time** column in Game Moves shows White and Black’s time spent, stacked beside each move pair. Hover for the available elapsed and remaining clock values. A dash means timing is unavailable. Timings are retained in saved games and [PGN exports](/help/pgn/#time-spent-and-time-remaining).

Scroll down over the board, mini graph, expanded graph, or move list to advance; scroll up to go back when review is allowed. The move list keeps its viewport still while the selected row is visible. During an engine-preferred preview, **Return to game moves** restores played-game navigation.

---
title: "Getting started"
description: "Find a game, open the analysis workspace, and learn what to look at first."
category: "using-ironwood"
keywords: [game, review, start, library, analyze]
weight: 10
lastmod: 2026-11-02T00:00:00-06:00
---

## Open a game

Play a game in Ironwood or open one from your saved games. You can also import a PGN, the common text format for chess moves. The game review shows the move list and lets you step forward and backward through positions.

Open **Game library → Import…** and choose **Paste a game or collection**, **Lichess**, **Chess.com**, or **Scoresheet**. Fetch recent games by username from either online source, or one completed Lichess game by its link. See [Import games](/help/import-games/) for the workflow.

For a handwritten game, choose **Tools → Enter a scoresheet…** or **Game library → Import… → Scoresheet → Enter scoresheet**. Type the moves beside your scoresheet photo, check them on the board, and import or download the PGN. Your unfinished draft saves on this device. See [Import a game from a scoresheet](/help/import-a-scoresheet/).

## Sidebar navigation

The sidebar replaces the top menus. Its collapsed rail keeps the selected mode visible; move the pointer over it to expand the menu.

- **Play:** New game against AI, online lobbies, Self-play, and Save current game.
- **Analysis:** open the analysis workspace directly.
- **Training:** Train against AI, Lichess Puzzles, then Study.
- **Game library:** Saved games, imports, Backup and Restore, and Storage information.
- **Settings:** engines, Companion connections, board appearance, and local-data controls.

The gold gear at the bottom of the sidebar configures quick-access shortcuts: enable or hide each shortcut and move it up or down. Lichess Profiles, Board Theme, and Study are available but start disabled for new users. Choices are remembered.

Online choices open their lobby without automatically signing in. Use the lobby's sign-in or reconnect control when needed. New game against AI opens a content-sized Quick setup panel connected to the sidebar.

## Find your way around

The live board stays on the left. Choose a tab on the right:

- **Moves:** Game Moves, notation, evaluations, annotations, playback, and per-move time spent. Hover the Time column for available remaining clocks.
- **Prediction:** the current engine evaluation and expected continuation.
- **Analysis:** full-game engine analysis, saved runs, the graph, and the review summary.
- **Position:** Pawn Structure, Piece Mobility, and King Safety.
- **Explore:** **Openings** for Opening Explorer and **Endgames** for Tablebases.
- **Tactical:** Tactical Map, with **Insights**, **Activity**, and **Patterns**.
- **Annotations:** **Drawings**, **Notes**, and **Chess Symbols**.

Study is a separate **Training → Study** workspace with chapter comments, variations, and practice. [Read and practice a study](/help/lichess-studies/).

Use the gold gear at the end of the tab row to enable or hide panels and change their order. Your preferences are remembered. Analysis is first for new users; an existing custom order is preserved.

Each analysis panel shows the selected position and move, for example **Position 9 of 44 (Qe3)**. Gold-accented subtabs inside sunken panels keep related tools together. The Moves panel fits its contents; the board fits the available space as you resize the browser. Switching between 2D and 3D preserves the live board's size. Some study tools are unavailable during active online play or training.

Select a move in Moves, then switch tabs to inspect that position. [Compare saved analyses](/help/saved-analysis/), [inspect positions](/help/positional-insights/), or [add annotations](/help/taking-notes/).

## Run game analysis

Select the **Analysis** tab and click **Analyze full game with Stockfish 19**. After a completed analysis, the button becomes **Analyze again with Stockfish 19**. Ironwood asks Stockfish to examine each position in turn. You can pause and resume the work. A second, deeper check may run on critical positions where a move appears to change the game sharply.

A Lichess sign-in or open lobby does not prevent analysis. Analysis is disabled while an online game is engaged. The **Analysis** tab shows **Game chat** during Lichess play and returns to analysis when the game ends. To review a saved game while a correspondence game remains ongoing, use **Game library → Saved games**.

**Analysis → Summary** compares both players with accuracy and average CPL charts for Overall, Opening, Middlegame, and Endgame. **Critical Analysis** contains the turning point, the Move quality stacked bars (open by default), and Move Analysis with played and engine-preferred lines beside an evaluation-swing bar. **Time per move**, **Clock burn-down**, and **Time spent vs. move quality quadrant** help review recorded clock use. [Timing-chart guide](/help/time-management/).

The results are estimates from a chess engine. They can change if Stockfish searches longer or uses different settings. Wait for analysis and any critical-move verification to finish before treating the summary as final.

For several saved games at once, select their checkboxes in Saved Games and choose **Analyze N games**. The engine selector, optional critical second pass and progress modal are described in [Batch analysis](/help/saved-games-and-backups/#batch-analysis).

## Open the right-hand panel in a separate window

Click **↗** beside the right-hand panel's tabs to open a live **Study panel**
window. It includes **Moves, Prediction, Analysis, Position, Explore, Tactical,
and Annotations**. Tab changes, move selection, scrolling, notes, and engine-line
playback act on the same game in the main window.

Move the popout to another monitor or resize it. The panel lays out its controls
for that window's size and draws text and charts at the monitor's native resolution.
The **Text size** selector changes the interface scale with a fresh layout, rather
than enlarging a picture. The panel is removed from the main window while popped
out, giving the board that space. The main window remains open and owns the game.
**Show main window** brings the board forward. **Dock panel**, closing the
popout's window, or **Dock study panel** in the main window returns the panel
to its original place without losing the selected move or analysis.
Clicking **↗** again focuses an existing popout.

Allow browser pop-ups for Ironwood if the browser blocks the new window. The
popout pauses its view and input while the main panel is hidden by focus or puzzle
mode. Closing or reloading the main game closes the popout; open it again from
the main window after reloading. Expanded analysis graphs open inside the Study panel. Other dialogs still open in the main window.

## Read the summary

Start with the [evaluation](/help/evaluation/) graph to see when either player gained an advantage. Then inspect the [move labels](/help/move-quality/) and [symbols](/help/move-symbols/) around the biggest changes.

**Analysis → Summary** compares both players in two bar charts: **Accuracy** (higher
is better) and **Average centipawn loss** (lower is better). Gold identifies White;
sage green identifies Black. Compare **Overall** and the three game phases,
hover for details, or click a phase bar to visit that player's first move in it.
Read [Accuracy and CPL](/help/accuracy-and-cpl/#compare-the-players) for move counts,
missing results, and how the metrics differ. The actual positions explain what
happened.

## Explore one position

Select a move in **Moves**, then open **Prediction** to see the selected engine's suggested move and its [engine line](/help/engine-line/). Step through the line on the board. Ask what threat the suggestion addresses, which piece improves, or what the played move allowed. The engine gives a candidate continuation; it does not give a human explanation by itself.

## Watch, compare, and keep games

Open [Lichess player profiles](/help/lichess-player-profiles/) to compare accounts, ratings and notable games. Click a game’s mini board to watch its moves, or **Review on Ironwood** to analyze it on the full board. Choose **Play → Save current game** to keep a temporary profile review.

In [Saved Games](/help/saved-games-and-backups/), browse the categories or sort by move count. Puzzle playback is separate from puzzle progress, and training profiles keep their completed results separately from the replay library. Back up local data before changing browsers or devices.

## Find the right guide

Use Help search for a feature, button label, or chess term. For a detailed graph walkthrough, see [Expanded game analysis graph](/help/expanded-analysis-graph/). For loading problems or an unresponsive page, see [Troubleshooting](/help/troubleshooting/).

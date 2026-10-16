---
title: "Player profiles and engine prediction boards"
date: 2026-10-15T00:00:00-05:00
slug: "2026-10-15-player-profiles-and-prediction-boards"
version: "2026.10.15"
description: "Compare Lichess profiles and engine lines, preview recent games, and keep the games you choose to save."
---

## Lichess player profiles

- Added detailed performance from time-control rating chips, including ratings, rank/percentile, records, opponent averages, streaks, and playing time. Best wins and worst losses reuse enlarged previews and Ironwood review; loading is cached per window/time control and handles partial game errors and rate limits.

- Added public biography, ratings, game record, daily rating history by time control, and the latest 12 completed games for your account or any username.
- Added Follow/Unfollow, a searchable following list, and Player profile in the Lichess-Online menu. Window titles include the player's name and ID; multiple green-bordered profile windows keep independent filters and geometry.
- Made biography links and FIDE ratings open in new tabs. Recent-game mini boards use the library's gold hover outline and magnifying-glass indicator and open enlarged playback with player colors and results.
- Added **Review on Ironwood** for temporary full-board review. **Save current game** now saves the complete reviewed game, even from the starting position. Recent-game controls occupy an aligned right column rather than adding height below the details.
- Improved narrow-window rating chips, game-record counters, and recent-game layouts. Stabilized card heights during resizing and kept resized profile windows from growing back.

## Engine prediction boards and study surfaces

- Added a raised **Expand** button beneath Engine prediction. Each window shows one mini board per predicted move, move labels, and highlighted move squares; clicking a board inspects the position when active play permits it.
- Added independent prediction snapshots for comparing lines. Titles include variation, evaluation, first move, and depth. Windows retain their own size, position, and maximize/restore state.
- Scaled mini boards to available window space, with scrolling when needed. Added gold window borders and one shared sunken panel with stronger procedural paper grain.
- Gave profile rating graphs their own sunken paper-grain section and applied the stronger grain to the bottom Full Game Analysis details panel.

## Saved Games

- Excluded games with zero moves and added move-count sorting.
- Added a **Puzzles** category for puzzle move history, separate from puzzle progress counts.
- Added White/Black player labels and outcomes to enlarged playback in both Saved Games and player profiles.

Updated [Help](/help/), [Features](/features/), and the project README.

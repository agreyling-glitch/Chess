---
title: "Library consoles, batch analysis, and training progress"
date: 2026-11-02T00:00:00-06:00
slug: "2026-11-02-library-console-and-batch-analysis"
version: "2026.11.02"
description: "Search saved games with a compact console, analyze selected games in two passes, and follow training progress by phase."
---

- Redesigned Saved Games search into a sunken Search console with icons, aligned fields, gold active-filter chips, and Filters collapsed by default. Search, category, profile, source, date, opening, analysis, game length, folder, sorting and library management remain available.
- Added an Analysis column with Complete, Partial and Not analyzed states. Retained saved runs contribute to status, and matching retained analysis can be restored when an empty current snapshot is reopened. The table retains stable column widths and an opaque sticky header while scrolling.
- Grouped selection actions into Export, Organize and More menus, with a separate inset analysis section, engine selector, critical-move checkbox and gold Analyze button. Existing folder, transfer, deletion and export actions are preserved.
- Added sequential batch analysis for selected games using browser Stockfish or an available Companion engine. Pass 1 uses one second per position. Optional Pass 2, enabled by default, checks positions before and after critical moves for five seconds each. The modal names the pass and reports progress, completed games, partial results and failures.
- Batch results are read back before success is reported. Earlier runs and annotations are retained, and cancelling saves available work. The completion message explains that a Not analyzed filter hides successful entries and replaces them with the next matching games.
- Redesigned Train against AI as a Training console with next-game information and Progress, Focus areas and Recent games tabs. Manage profiles retains creation, engine choices, rename, import, export and deletion.
- Added phase-progress charts to Focus areas: gold rolling averages over ten completed games, individual-game dots, a common vertical scale, Last 20 / Last 50 / All games, and hover details. Missing numerical analysis remains a gap, with sample and coverage information visible.
- Added configurable sidebar shortcuts and a dedicated Analysis navigation section. The bottom gold gear controls shortcut visibility and order; Lichess Profiles, Board Theme and Study start disabled for new users. Analysis is first in the default panel order while existing custom orders remain intact.
- Moved Study into its own Training workspace with direct public-study URL import, selectable saved-study cards, Browser/Companion storage selection, and Study-mode reopening. Study and Puzzle panels now provide their own Annotations tabs.
- Saved puzzles reopen in Puzzle mode. Initial-position puzzle annotations can be saved without making a move, and Save current game now confirms successful storage or reports an error or pending Companion synchronization.
- Refined the compact Prediction console, gold controls, matching icons, grain, panel spacing and window alignment throughout the workspace.
- Added a shared position slider with start, previous, next and final-position buttons across analysis tabs, inside a subtly grained sunken border.
- Opening Explorer waits for navigation to settle before requesting a new position, spaces network requests, and continues to show cached results immediately.
- Fixed player-profile review retaining an earlier study, and preserved precise clock timings supplied by Lichess.
- Updated the gallery with the Prediction console, saved-analysis controls, Critical Analysis and three time-management charts.

See [saved games and batch analysis](/help/saved-games-and-backups/), [training progress](/help/training-vs-ai/), [sidebar navigation](/help/getting-started/), [Lichess studies](/help/lichess-studies/), and [puzzles](/help/puzzles/).

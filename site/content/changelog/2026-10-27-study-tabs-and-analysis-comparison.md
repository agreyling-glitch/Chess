---
title: "Study tabs, annotations, and faster analysis comparison"
date: 2026-10-27T00:00:00-05:00
slug: "2026-10-27-study-tabs-and-analysis-comparison"
version: "2026.10.27"
description: "Dedicated Position, Explore, Tactical, and Annotations tabs, complete NAG preservation, saved-analysis shortcuts, and cached engine discovery."
---

## A workspace organized around the task

- The right-hand tabs are now **Moves, Prediction, Analysis, Position, Explore, Tactical, and Annotations**.
- **Position** contains Pawn Structure, Piece Mobility, and King Safety. Engine analysis stays in **Analysis**.
- **Explore** groups Opening Explorer under **Openings** and Tablebases under **Endgames**.
- **Tactical** brings Tactical Map into the sidebar, with gray **Insights**, **Activity**, and **Patterns** subtabs inside a sunken panel.
- **Annotations** replaces the drawing and note toolbar shortcuts and floating windows with **Drawings**, **Notes**, and **Chess Symbols**. Notes are edited inside the panel; common symbols remain in the right-click menu.
- Every tab after Moves shows the current position and move. Game Moves fits its contents, and switching between 2D and 3D preserves the live board's dimensions, including during artwork loading.

## Compare saved analysis runs

- Moved **Saved Analysis** below the Analyze controls.
- Added numbered gray buttons in both the normal Analysis view and expanded graph. Click them or press **1–9** to switch runs; the dropdown and active button stay synchronized.
- Added per-run engine and settings tooltips. The expanded graph's engine heading follows the selected historical run rather than the engine configured for future analysis.
- Number shortcuts respect text input and unavailable switching states.

## Keep the full annotation vocabulary

- Preserved all valid PGN NAG values **$0–$255** through saved storage, annotated PGN, and Analysis JSON.
- Added descriptions for the original standard set, plus numeric fallbacks for unfamiliar or program-specific codes.
- Added a searchable **More symbols** section under Chess Symbols and an applied-symbol list for individual removal.
- Common-symbol edits preserve unrelated advanced annotations. Move tooltips, notes, and reports expose annotations that previously had no displayed symbol.

## Faster Companion engine selection

- Cached engine capabilities in memory for the browser session, avoiding a repeated discovery launch when returning to a previously discovered engine.
- Added immediate picker feedback and a preparation message for first-time selections. Settings wait until preparation succeeds; discovery failures remain visible.
- Refreshing installed engines or changing connections or credentials clears cached capabilities. Opening Engine settings no longer forces a refresh. Actual engine startup and first discovery can still take time.

Read the [workspace guide](/help/getting-started/), [saved-analysis guide](/help/saved-analysis/), or [annotations guide](/help/taking-notes/).

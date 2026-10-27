---
title: "Compare saved analyses"
description: "Switch saved engine runs with numbered buttons or keys 1–9, and inspect each run's recorded settings."
category: "analysis"
keywords: [Saved Analysis, compare, engine, dropdown, numbered buttons, shortcuts, settings, hover, expanded graph]
weight: 22
lastmod: 2026-10-27T00:00:00-05:00
---

## Choose a saved run

Open a game and select **Analysis**. The **Analyze / Analyze again** controls and engine selector appear first; **Saved Analysis** is below them. Choose a run in its dropdown. When multiple runs are available, numbered gray raised buttons provide a quicker way to switch.

Click a numbered button or press **1–9** for the corresponding run. Numbers follow the dropdown order; buttons beyond 9 remain clickable. The dropdown and highlighted number stay synchronized after the selected run loads. Refreshing the list or saving another run can change the order.

Switching is unavailable while full-game analysis is running or while a run is loading. Number shortcuts do not select runs while you are typing or using a modified key combination. In the Analysis view, these numbers take priority over observed FICS-game shortcuts.

## Compare inside the expanded graph

Choose **Expand** in the graph's upper-right corner. The expanded view includes the same numbered buttons and **1–9** shortcuts, so you can compare runs without closing it. The graph, results, and engine heading follow the selected saved run. Returning to the main view shows the same selection in the dropdown.

Selecting a historical run does not change the engine configured for your next analysis. To create new results, choose the desired engine beside Analyze and start a new run. Your earlier saved runs and personal annotations remain separate.

## Inspect the engine and settings

Hover any numbered button, in either view, for the run's recorded engine, analysis budget, critical-move verification status, and engine options such as Threads and Hash. The tooltip also includes its saved-run label and number shortcut. Older runs may not contain every setting; missing information is marked as unavailable rather than taken from your current engine configuration.

Use **Manage analyses** for saved-run management. Use the refresh control when you need to reload the available runs. Companion-hosted results need their connected archive or a locally cached copy; an unavailable run reports a loading error rather than replacing the current results.

See [Analysis settings](/help/search-depth-and-settings/) and [Expanded graph navigation](/help/expanded-analysis-graph/).

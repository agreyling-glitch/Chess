---
title: "Import history, Chess.com timings, and collapsible analysis controls"
date: 2026-11-04T00:00:00-06:00
slug: "2026-11-04-import-history-and-compact-analysis"
version: "2026.11.04"
description: "Reuse recent import usernames, calculate Chess.com move durations, and give analysis details more room on smaller screens."
---

- Added separate **Recent usernames** dropdowns for Lichess and Chess.com imports. Each remembers the last 20 distinct usernames submitted with Fetch in this browser, newest first. Reusing a name moves it to the top; Lichess game and study links stay out of the list.
- Calculated time spent per move from imported Chess.com clock comments and simple time controls, including first moves and increments. Explicit elapsed times are preserved, and missing clocks leave gaps instead of charging several moves to one.
- Fixed Chess.com imports stopping when a listed monthly archive returns 404 or 410. Ironwood now skips unavailable months and tries earlier months within the selected dates, while retaining rate-limit and other error handling.
- Added **Hide controls & graph** beside the **Analysis** heading to collapse engine controls, saved results, and the evaluation graph. Summary, Critical Analysis, and timing tabs gain vertical space; progress and errors remain visible. **Show controls & graph** restores the section.
- Fixed the Analysis heading being shortened after adding the collapse control.

See [Chess.com import](/help/import-from-chess-com/), [Lichess import](/help/import-from-lichess/), and [time-use review](/help/time-management/).

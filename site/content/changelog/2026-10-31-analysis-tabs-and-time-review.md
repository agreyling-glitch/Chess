---
title: "Analysis tabs and time-use review"
date: 2026-10-31T00:00:00-05:00
slug: "2026-10-31-analysis-tabs-and-time-review"
version: "2026.10.31"
description: "Dedicated critical-analysis and timing views, compact Companion help, and safer saved-game separation."
---

- Split the analysis details into Summary and Critical Analysis. Turning point, Move quality, and Move Analysis now live in Critical Analysis; Move quality starts expanded.
- Added Time per move, Clock burn-down, and Time spent vs. move quality quadrant tabs. Recorded timings, remaining clocks, and estimated CPL help identify rushed errors and expensive thinks. Hover details and clickable chart marks connect the charts to the board.
- Added the dashed even-pace clock budget and red 60-second reference. Clock values retain increments, and missing timing data stays unavailable.
- Styled timing charts with sunken frames and subtle grain. Clock legends use drawn solid, dotted, and dashed samples to avoid missing font glyphs.
- Replaced the long disconnected engine-provider hint beside Refresh with a compact gold ? help button and a link to Companion connections. Other engine status messages truncate to fit, with full text on hover.
- Fixed explicit saving of temporary profile reviews to create a separate game entry instead of reusing the previous game's storage identity. Temporary reviews no longer display the previous game's saved-analysis selector.
- Made incompatible or unreadable saved-analysis selections report a visible error. Existing mismatched payloads remain preserved; this change does not automatically repair older mixed-game records.

Read the [timing guide](/help/time-management/), [saved-analysis help](/help/saved-analysis/), and [journal entry](/blog/where-the-thinking-time-goes/).

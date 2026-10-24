---
title: "Library cleanup, responsive panels, and safer imports"
date: 2026-10-24T00:00:00-05:00
slug: "2026-10-24-library-cleanup-and-responsive-panels"
version: "2026.10.24"
description: "Find short games, clear library filters, fit the board to your window, maximize training profiles, and import PGN and ZIP collections with stronger safeguards."
---

## Find and clean up short games

- Added **Special filters** to Saved Games, with **Fewer than 2 full moves** as its first option. It matches games with fewer than four played half-moves and works with the other filters and **Select all matching**.
- Added **Clear filters** immediately to the right of **Filters**. It appears when a filter is active and resets all filters together while retaining the chosen sort order.

## Fit the workspace to your window

- Made the two main panels adjust automatically as the browser resizes. The live board grows to fit the available vertical space; the Moves, Prediction and Analysis panel fills the remaining width.
- Added a gray **Maximize / Restore** button to Training Profiles. Restore returns the window to its previous size and position.

## Import PGN and ZIP collections

- Added ZIP selection through **Browse…** and drag-and-drop imports. PGN files inside the ZIP, including files in folders, use the existing game-selection and duplicate-detection workflow.
- Discarded imported PGN games with fewer than **two full moves (four half-moves)** and added a separate discarded-game count to the import result.
- Added PGN size, game-count, move-count and nesting limits, including a **64 MiB** collection limit, up to **100,000 games**, and **1 MiB** per game. Invalid or excessive input shows an error.
- Replaced repeated variation scanning with single-pass move counting and moved selected games into the paced import queue without retaining another full copy. Comments, variations and duplicate detection remain supported.

## Protect online connections

- Added global and per-client FICS connection limits, command and output budgets, and deadlines for login, stalled writes, unread output, idle connections and session lifetime.
- Preserved guest and registered sign-in, console commands and game observation. These changes protect bridge resources; the legacy FICS transport is unchanged.

See [Features](/features/), [Import Games help](/help/import-games/) and [Saved Games help](/help/saved-games-and-backups/).

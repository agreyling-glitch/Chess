---
title: "Optional statistics and clearer engine help"
date: 2026-10-11T13:00:00-05:00
description: "Optional usage statistics, expanded engine settings guidance, and easier deferred updates."
slug: "2026-10-11-optional-statistics-and-engine-help"
---

## Added

- Optional usage statistics with explicit consent, inline privacy details, and an Analytics settings button to change your choice.
- A privacy notice describing the selected usage events and excluded game content. Session recording and automatic interaction capture are disabled.
- Expanded engine help covering threads, memory, analysis presets, the two full-game analysis passes, and troubleshooting.

## Changed

- Deferred app updates can be installed through **Help → Update and Reload**.
- The Storage menu now appears immediately before Help.

## Fixed

- Analysis could stall before searching when Stockfish increased its thread count. Engine workers now finish loading before thread settings are applied.
- Production now ships the matching patched engine script with the app under a new filename, avoiding an older script from the separate engine host or browser cache.

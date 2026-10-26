---
title: "Windows Companion, desktop engines, and local archives"
date: 2026-10-26T00:00:00-05:00
slug: "2026-10-26-windows-companion"
version: "2026.10.26"
description: "Ironwood Companion 0.1.4 is available for Windows, with desktop engines, named archive locations, compressed imports, and verified updates."
---

## Ironwood Companion for Windows

- Released **Companion 0.1.4** for Windows 11 on Intel/AMD 64-bit computers. Companion runs alongside Ironwood to provide desktop engines and a game archive on your computer.
- Added a public [download and setup page](/downloads/companion/), plus **Help → Companion setup…** and **Help → Companion connections…**. No diagnostics switch is needed.
- Added a status dashboard with connection and engine counts, folder shortcuts, import activity, and update controls.
- Published the Windows download and signed update channel. Companion checks updates against Ironwood's release key and package checksum. The executable does not have a Windows publisher signature; see the download page before installing.

## Install and manage desktop engines

- Added **Auto-install** and **Manual** tabs. Auto-install downloads supported packages directly from their original publishers; Manual registers an engine you already have.
- Added controls to manage the selected engine's name, executable, and launch arguments, and to remove its registration. Companion-installed packages can also have their installed files removed.
- Engine lists show the Companion name so installations on different computers are easier to distinguish. Playing and analysis options remain in Engine settings.

## Choose where your games live

- Added a consistent location dropdown to **Storage information**, **Saved games**, and **Import**. Choose Browser or a named connected Companion.
- Added archive import, browsing, filtering, and PGN export. Browser storage remains separate; connecting Companion does not automatically move your existing games.
- Standardized opening descriptions through an ECO lookup and added **All [ECO code] games** choices, so filtering by a code includes its opening variations.
- Formatted library counts, selection totals, page totals, and indexing counts with commas.

## Import collections and follow progress

- Added **7z** support alongside PGN and ZIP through Browse and drag-and-drop, including supported archives containing matching Scid files. Scid source files are read without changing them.
- Added automatic Companion folder imports for PGN, ZIP, 7z, and matching Scid databases, with reports and restart checkpoints after saved batches.
- Improved archive import batching and overlapped preparation with saving. Progress shows counts, speed, estimated time remaining, and waiting or retrying messages.
- Added opening-index progress to the main status bar, including indexed games, remaining games, speed, and estimated time remaining. Indexing can continue after an import finishes.

Read the [Companion guide](/help/companion-engines-and-storage/) or the [release journal](/blog/desktop-engines-and-a-library-of-your-own/).

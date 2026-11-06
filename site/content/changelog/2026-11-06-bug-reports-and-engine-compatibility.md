---
title: "In-app bug reports and Chess960 engine compatibility"
date: 2026-11-06T00:00:00-06:00
slug: "2026-11-06-bug-reports-and-engine-compatibility"
version: "2026.11.06"
description: "Report a bug from Help or the startup screen, and find engines that support Chess960."
---

- Added **Help → Report a bug…** and a report button on the startup error screen. Review the diagnostics before sending, add a reply email if you want a response, and optionally attach a previewable PGN from the board. Reports work with usage statistics turned off. [Reporting bugs](/help/optional-usage-statistics/#report-a-bug)
- With usage statistics enabled, startup error reports identify the stage, failure category, and HTTP status when available. A bug report can reference the matching startup error.
- Filtered engine choices for Chess960 play, position analysis, and full-game analysis. Compatible engines include built-in Stockfish and Companion engines that advertise Chess960 support. [Companion engines and Chess960](/help/companion-engines-and-storage/#chess960-limitations)
- Updated the Windows x64 Companion Auto-install catalog to include Laser 1.7 and Supernova 2.4 for standard chess. The catalog you see depends on your installed Companion version.
- Added connection status and a reconnect message to detached Lichess player profiles.

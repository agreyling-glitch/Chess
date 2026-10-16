---
title: "Troubleshooting and recovery"
description: "Recover from a frozen page, diagnose loading problems, and protect locally saved games."
category: "settings"
keywords: [freeze, frozen, stuck, loading, network, refresh, retry, backup, browser, offline, errors]
weight: 80
lastmod: 2026-10-15T00:00:00-05:00
---

## A frozen page

If buttons and the board stop responding, reload the same Ironwood tab. If the browser cannot reload it, close that tab and reopen Ironwood. Reopening restores data that was already saved in this browser; work that had not been saved may be lost.

Keep the same browser and site address when recovering. Local development ports, browsers, devices, and private sessions can have separate storage. Do not clear site data as a first troubleshooting step. Make a [backup](/help/saved-games-and-backups/) before resetting data whenever the app is responsive enough.

For a repeatable freeze, record the action immediately before it, the browser, whether 2D or 3D was active, and whether engine analysis was running. A screenshot and errors from **F12 → Console** can help diagnose it. A UI deadlock may produce no console error.

## Analysis is slow or appears stuck

Full-game analysis works through positions and may then verify critical moves at a higher budget. Check whether the position counter is advancing. Pause/resume if the controls respond. If it stays stuck, reload and retry. Lower quality or fewer threads can reduce CPU demand, but cannot repair a software deadlock. See [Search depth and settings](/help/search-depth-and-settings/).

## Online information does not load

Check your connection and refresh the affected Lichess profile section, activity list, or game preview. Wait before repeated retries if the service is limiting requests. Public data may be absent for a player even when the connection works. Sign in again when an account action explicitly requires renewed authorization.

An online game's clock or correspondence deadline continues on its server while Ironwood is closed or disconnected. Reopen the lobby and resume the game; reloading does not resign it. See [Play on Lichess](/help/play-on-lichess/) and [FICS](/help/online-play/).

## Artwork or 3D does not load

Some themes download artwork on first use. Keep using the 2D fallback while it loads, or choose another theme. Reconnect and use **Retry** when offered. Cached artwork can work offline; uncached online features cannot. See [Board themes](/help/board-themes/#loading-artwork-and-offline-use).

## Saved games seem missing

Confirm you are using the original browser, device, and site address. Check the library category, profile filter, and sort order. Games with zero moves are intentionally excluded. A game opened from a Lichess profile must be explicitly saved to enter the library.

Deleting saved puzzle playback does not reset puzzle progress; deleting a saved training game does not erase its completed profile result. **Reset all local data** is broader and should be preceded by a backup. Browser data removal can erase local records.

## Reporting a problem

Include steps to reproduce, expected and actual behavior, browser, relevant settings, and any error text. Avoid sharing authorization tokens or private notes. Optional usage statistics currently measure visits and usage; automatic freeze reporting has not been implemented. See [Optional usage statistics](/help/optional-usage-statistics/).

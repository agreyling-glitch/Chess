---
title: "Import games from Chess.com"
description: "Filter completed Chess.com games by date and time control, fetch up to 200, and choose which to save."
category: "using-ironwood"
keywords: [Chess.com, chesscom, import, username, PGN, archives, recent games, duplicates]
weight: 19
lastmod: 2026-10-05T00:00:00-05:00
---

Open **Game → Import…** and select **Chess.com**. Enter a Chess.com username and click the gold **Fetch games** button. Status messages appear below the input. No login or API token is required; fetching needs an internet connection.

Before fetching, choose **Games** (20, 50, 100, or 200) and **Time control** (All, Bullet, Blitz, Rapid, or Daily). Optionally enter **From** and **Through** dates in `YYYY-MM-DD` format. Dates use UTC and the full Through date is included. Leave either date blank for an open range. **Reset filters** restores 20 games, all time controls, and no dates.

Ironwood reads matching monthly archives and collects the newest completed standard chess or Chess960 games, up to your chosen count. Dates filter by game completion. Unsupported variants and unfinished games are skipped. Fewer games may be returned if there are fewer matches. Chess.com caches these archives, so a just-finished game may not appear immediately.

If no games match, widen the date range or choose another time control. Invalid dates or a start date after the end date show an error below the input.

When fetching finishes, **Paste a game or collection** opens with the PGNs. Click **Continue**, choose the games you want to keep, and use the existing import flow to save them. Duplicate games already in your library are skipped.

Open an imported game and choose **Game Analysis** to analyze it. Import does not automatically start engine analysis.

The Import window also has **Lichess** and **Scoresheet** tabs. Use the first tab to paste PGN or Analysis JSON from any source. Chess.com import accepts usernames; for a single game, download its PGN and paste it in the first tab. See [Import games](/help/import-games/) for the four-tab workflow.

If the API reports a rate limit, wait a full minute before fetching again. See [the Analysis gallery](/gallery/#analysis) for tools available after import.

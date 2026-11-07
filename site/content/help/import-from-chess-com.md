---
title: "Import games from Chess.com"
description: "Filter completed Chess.com games by date and time control, fetch up to 200, and choose which to save."
category: "using-ironwood"
keywords: [Chess.com, chesscom, import, username, PGN, archives, recent games, duplicates]
weight: 19
lastmod: 2026-11-05T00:00:00-06:00
---

Open **Import games**, or **Manage library → Import games…** in Saved Games, and select **Chess.com**. Enter a Chess.com username and click the gold **Fetch games** button. Status messages appear below the input. No login or API token is required; fetching needs an internet connection.

Enter just the username, such as `hikaru`, rather than a profile URL. **Recent usernames** lets you reuse the last 20 distinct usernames submitted with Fetch. The newest is first; using a name again moves it to the top. The list stays in this browser on this device and is separate from Lichess history.

Before fetching, choose **Games** (20, 50, 100, or 200) and **Time control** (All, Bullet, Blitz, Rapid, or Daily). Optionally enter **From** and **Through** dates in `YYYY-MM-DD` format. Dates use UTC and the full Through date is included. Leave either date blank for an open range. **Reset filters** restores 20 games, all time controls, and no dates.

Ironwood reads matching monthly archives and collects the newest completed standard chess or Chess960 games, up to your chosen count. Dates filter by game completion. Unsupported variants and unfinished games are skipped. Fewer games may be returned if there are fewer matches. Chess.com caches these archives, so a just-finished game may not appear immediately.

If no games match, widen the date range or choose another time control. Invalid dates or a start date after the end date show an error below the input.

If Chess.com lists a monthly archive that returns Not Found (404) or Gone (410), Ironwood skips it and tries earlier months within your date range. The returned games may therefore be older than expected. If no games are found and some months were unavailable, the message explains this; try again later or choose an earlier range. Rate limits, connection failures, and other server errors still stop the request.

When fetching finishes, collections open game selection directly. Choose the games you want to keep and import them. A single returned game proceeds directly to review or the destination’s selection screen. You do not need to pass through the PGN text box or click **Continue**. Duplicate games already in your library are skipped.

Open an imported game and choose **Analysis** to analyze it. Import does not automatically start engine analysis.

Clock comments are preserved. For games with a simple starting time and optional increment, Ironwood calculates time spent from each player's previous clock minus the new clock plus the increment, including the first move. Explicit elapsed times take priority. Missing clocks break the calculation for that player until consecutive clock records are available again. See [time-use review](/help/time-management/).

The Import window also has **Lichess** and **Scoresheet** tabs. Use the first tab to paste PGN or Analysis JSON from any source. Chess.com import accepts usernames; for a single game, download its PGN and paste it in the first tab. See [Import games](/help/import-games/) for the four-tab workflow.

If the API reports a rate limit, wait a full minute before fetching again. See [the Analysis gallery](/gallery/#analysis) for tools available after import.

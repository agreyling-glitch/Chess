---
title: "Import games from Lichess"
description: "Filter completed Lichess games by date and time control, fetch up to 200, or import one game by its link."
category: "using-ironwood"
keywords: [Lichess, import, username, profile, game URL, PGN, recent games, completed games, duplicates, analyze]
weight: 13
lastmod: 2026-11-05T00:00:00-06:00
---

Bring completed Lichess games into Ironwood without downloading a PGN file first. You do not need to sign in to Lichess or provide an API token. Fetching games requires an internet connection.

## Import a player's recent games

1. Open **Import games**, or **Manage library → Import games…** in Saved Games.
2. On the **Lichess** tab, enter the player's **username**, such as `Dr_Tiger`.
3. Click the gold **Fetch games** button. Follow the status below the input while Ironwood retrieves up to the chosen number of matching completed games (20 by default).
4. A collection opens the game-selection window directly. There is no Paste tab or **Continue** step.
5. Select the games you want, or use **Select matching** to select all games matching the current player filter. Click **Import selected**.
6. Open **Game Library** and choose **Imported Games** to find the imported games. Open a game to review its moves or start analysis.

Ironwood skips games that are already in Saved Games. Importing a collection saves the selected games; analyze them individually after opening them.

Enter `Dr_Tiger`, rather than the profile URL `https://lichess.org/@/Dr_Tiger`. The import field currently accepts usernames and individual game links, but not profile links.

Use **Recent usernames** to select one of the last 20 distinct usernames submitted with Fetch. Newest names appear first, and using a name again moves it to the top. The history stays in this browser on this device, separately from Chess.com history. Game and study links are not added to the username list.

## Filter a username import

Choose **Games** (20, 50, 100, or 200) and **Time control** (All, Bullet, Blitz, Rapid, Classical, or Correspondence). All includes standard chess and Chess960; a specific speed uses that Lichess performance category. The newest matching games are fetched first, up to your chosen count.

Optionally enter **From** and **Through** dates in `YYYY-MM-DD` format. Dates use UTC and include the whole Through date. Leave either blank for an open range. **Reset filters** restores 20 games, all time controls, and no dates. Invalid dates or a reversed range show an error; if no games match, widen the range or choose another time control.

## Import one game by its link

Filters apply to usernames. When you paste a game link, the filter controls are disabled and Ironwood fetches that single game.

Paste the game's link, such as `https://lichess.org/abcdefgh`, on the **Lichess** tab and click **Fetch games**. Replace the example with a real completed-game link. The fetched game proceeds directly to review, or to game selection when the chosen destination requires it.

Start analysis afterward from **Analysis**, using the same controls as any imported game. Repeated downloads of the same completed-game link can reuse a copy held in memory for up to 30 minutes. Username collections and study refreshes still fetch fresh data.

## Paste PGN or restore analysis JSON

You can still paste a [PGN game or collection](/help/pgn/) directly into **Paste a game or collection**, or drag a `.pgn` file onto the app. An [Ironwood analysis JSON](/help/saving-and-sharing/) file restores its saved analysis without rerunning Stockfish.

**Clear** empties the game-text box. The **×** at the top right closes the import window. See [Import games](/help/import-games/) for all four tabs and their shared review workflow.

## If fetching does not work

- **User or game not found:** check the username or completed-game link.
- **No completed games found:** the player may have no completed games available to import.
- **Game still in progress:** wait until it finishes, then fetch it again.
- **Connection error or timeout:** check your internet connection and try again.
- **Rate limit:** wait at least a full minute, or longer if Lichess supplies a longer retry interval. Online play requests, imports, and study refresh share this pause; disconnecting does not clear it.

Imported games are stored locally in this browser on this device. Use a [backup](/help/saved-games-and-backups/) to keep a separate copy.

## Browse without importing

Use [Lichess player profiles](/help/lichess-player-profiles/) when you only want to inspect a player or watch a recent or notable game. Mini-board playback does not add it to the library. **Review on Ironwood** opens a temporary full-board review; **Save current game** keeps it locally. Import remains the workflow for adding selected games or a collection directly to Saved Games.

## Timings when reviewing profile games

**Review** in a Lichess player profile requests clock history for recent and notable games. The Game Moves **Time** column shows time spent, calculated as the player’s previous remaining clock minus the new clock plus the increment. Hover over a value to see the remaining clock.

Ironwood uses either PGN clock comments or Lichess JSON clock history. It preserves explicit elapsed times, follows Lichess’s zero-time first move for each player, and handles checkmate/stalemate without adding an unawarded increment. Berserk players use zero increment when that information is supplied. Games without enough clock history show a dash. Refresh the profile after updating Ironwood to fetch clock history for previously cached games.

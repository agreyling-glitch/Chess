---
title: "Import games from Lichess"
description: "Filter completed Lichess games by date and time control, fetch up to 200, or import one game by its link."
category: "using-ironwood"
keywords: [Lichess, import, username, profile, game URL, PGN, recent games, completed games, duplicates, analyze]
weight: 13
---

Bring completed Lichess games into Ironwood without downloading a PGN file first. You do not need to sign in to Lichess or provide an API token. Fetching games requires an internet connection.

## Import a player's recent games

1. Open **Game → Import…**.
2. On the **Lichess** tab, enter the player's **username**, such as `Dr_Tiger`.
3. Click the gold **Fetch games** button. Follow the status below the input while Ironwood retrieves up to the chosen number of matching completed games (20 by default).
4. **Paste a game or collection** opens with the game text. Click **Continue** to open the game-selection window.
5. Select the games you want, or use **Select matching** to select all games matching the current player filter. Click **Import selected**.
6. Open **Game → Load saved game** and choose **Imported Games** to find the imported games. Open a game to review its moves or start analysis.

Ironwood skips games that are already in Saved Games. Importing a collection saves the selected games; analyze them individually after opening them.

Enter `Dr_Tiger`, rather than the profile URL `https://lichess.org/@/Dr_Tiger`. The import field currently accepts usernames and individual game links, but not profile links.

## Filter a username import

Choose **Games** (20, 50, 100, or 200) and **Time control** (All, Bullet, Blitz, Rapid, Classical, or Correspondence). All includes standard chess and Chess960; a specific speed uses that Lichess performance category. The newest matching games are fetched first, up to your chosen count.

Optionally enter **From** and **Through** dates in `YYYY-MM-DD` format. Dates use UTC and include the whole Through date. Leave either blank for an open range. **Reset filters** restores 20 games, all time controls, and no dates. Invalid dates or a reversed range show an error; if no games match, widen the range or choose another time control.

## Import one game by its link

Filters apply to usernames. When you paste a game link, the filter controls are disabled and Ironwood fetches that single game.

Paste the game's link, such as `https://lichess.org/abcdefgh`, on the **Lichess** tab and click **Fetch games**. Replace the example with a real completed-game link. Click **Continue** to open the game for review.

After fetching, the **Paste a game or collection** tab opens with the PGN. Click **Continue** to review and import it. Start analysis afterward from **Game Analysis**, using the same controls as any imported game.

## Paste PGN or restore analysis JSON

You can still paste a [PGN game or collection](/help/pgn/) directly into **Paste a game or collection**, or drag a `.pgn` file onto the app. An [Ironwood analysis JSON](/help/saving-and-sharing/) file restores its saved analysis without rerunning Stockfish.

**Clear** empties the game-text box. The **×** at the top right closes the import window. See [Import games](/help/import-games/) for all four tabs and their shared review workflow.

## If fetching does not work

- **User or game not found:** check the username or completed-game link.
- **No completed games found:** the player may have no completed games available to import.
- **Game still in progress:** wait until it finishes, then fetch it again.
- **Connection error or timeout:** check your internet connection and try again.
- **Rate limit:** wait a full minute before trying again.

Imported games are stored locally in this browser on this device. Use a [backup](/help/saved-games-and-backups/) to keep a separate copy.

---
title: "Import games from Lichess"
description: "Fetch completed Lichess games by username or game link, then save and analyze them in Ironwood."
category: "using-ironwood"
keywords: [Lichess, import, username, profile, game URL, PGN, recent games, completed games, duplicates, analyze]
weight: 13
---

Bring completed Lichess games into Ironwood without downloading a PGN file first. You do not need to sign in to Lichess or provide an API token. Fetching games requires an internet connection.

## Import a player's recent games

1. Open **Game → Import…**.
2. In **From Lichess**, enter the player's **username**, such as `Dr_Tiger`.
3. Click **Fetch games**. Ironwood retrieves their latest **20 completed games** and fills the game-text box.
4. Click **Continue** to open the game-selection window.
5. Select the games you want, or use **Select matching** to select all games matching the current player filter. Click **Import selected**.
6. Open **Game → Load saved game** and choose **Imported Games** to find the imported games. Open a game to review its moves or start analysis.

Ironwood skips games that are already in Saved Games. Importing a collection saves the selected games; analyze them individually after opening them.

Enter `Dr_Tiger`, rather than the profile URL `https://lichess.org/@/Dr_Tiger`. The import field currently accepts usernames and individual game links, but not profile links.

## Import one game by its link

Paste the game's link, such as `https://lichess.org/abcdefgh`, into **From Lichess** and click **Fetch games**. Replace the example with a real completed-game link. Click **Continue** to open the game for review.

To analyze a single game immediately, enable **Analyze the full game after import** before clicking **Continue**. Analysis uses your Full game quality setting, and you can pause or stop it. Otherwise, start analysis later from the game's analysis workspace.

## Paste PGN or restore analysis JSON

You can still paste a [PGN game or collection](/help/pgn/) directly into **Paste a game or collection**, or drag a `.pgn` file onto the app. An [Ironwood analysis JSON](/help/saving-and-sharing/) file restores its saved analysis without rerunning Stockfish.

**Clear** empties the game-text box. The **×** at the top right closes the import window.

## If fetching does not work

- **User or game not found:** check the username or completed-game link.
- **No completed games found:** the player may have no completed games available to import.
- **Game still in progress:** wait until it finishes, then fetch it again.
- **Connection error or timeout:** check your internet connection and try again.
- **Rate limit:** wait a full minute before trying again.

Imported games are stored locally in this browser on this device. Use a [backup](/help/saved-games-and-backups/) to keep a separate copy.

---
title: "Import games"
description: "Choose an import source, review a game collection, or enter a handwritten scoresheet."
category: "using-ironwood"
keywords: [import, tabs, PGN, JSON, collection, Lichess, Chess.com, scoresheet]
weight: 12
---

Open **Game → Import…** to choose among four tabs. The window keeps the same size as you switch tabs. On narrow windows, scroll the tab row horizontally to reach every tab. Panel contents scroll vertically when needed. Close the window with **×**.

## Paste a game or collection

Paste [PGN](/help/pgn/) for one game or a collection, or paste an Ironwood Analysis JSON export. You can also drag a `.pgn` or `.json` file onto the app. **Clear** empties the text and **Continue** opens the game or collection for review. Analysis JSON restores saved analysis immediately.

For a collection, select games to keep and click **Import selected**. Use the player filter and **Select matching** to select matching games. Games already in your library are skipped. Find saved imports under **Game → Load saved game → Imported Games**.

## Lichess

Enter a player's username to fetch up to the chosen number of matching completed games (20 by default), or paste a completed-game link to fetch one game. Click the gold **Fetch games** button and watch the status below the input. No login or API token is needed. [Lichess import instructions](/help/import-from-lichess/).

## Chess.com

Enter a Chess.com username and click **Fetch games** for up to the chosen number of completed standard chess or Chess960 games (20 by default). No login or API token is needed. Recently finished games may take time to appear in the cached archives. [Chess.com import instructions](/help/import-from-chess-com/).

Both panels offer **Games** (20, 50, 100, or 200), **Time control**, and optional **From** and **Through** dates in `YYYY-MM-DD` format. Dates use UTC and include the entire Through date; leave either blank for an open range. The newest matching games are fetched first, up to your chosen count. **Reset filters** restores 20 games, all time controls, and no date limits. Filters apply to usernames, not individual Lichess game links. Chess.com uses game completion dates. Lichess uses its game-date filters. Selecting a specific Lichess speed restricts results to that performance category.

Both online sources require an internet connection. After fetching, **Paste a game or collection** opens with the game text; choose **Continue** to review and import it.

## Scoresheet

The three-step guide explains how to add a reference photo, enter and check moves, and import the finished game. Click **Enter scoresheet** to open the workspace with a board preview and live move validation. You enter moves manually; the photo is a reference. Photos and unfinished drafts stay on your device, and a photo is optional. [Scoresheet entry instructions](/help/import-a-scoresheet/).

## Analyze after importing

Importing does not automatically start full-game analysis. Open an imported game and start it from **Game Analysis** when ready. [Analysis JSON](/help/saving-and-sharing/) restores existing results without another engine run.

---
title: "Saved games and backups"
description: "Find your games, understand local browser storage, and preserve a backup."
category: "using-ironwood"
keywords: [save, saved games, library, backup, restore, browser storage, data]
weight: 14
---

Use **Game → Save current game** to keep the current game, and **Game → Load saved game** to open the library. Ironwood also restores the current board and related preferences when you return to the same browser and site.

Saved games live in this browser's storage on this device. A different browser, private window, or device will not automatically share that library. Clearing browser site data can remove it.

Use [Import Games](/help/import-games/) to paste PGN or Analysis JSON, fetch completed games from Lichess or Chess.com, or enter a scoresheet. Selected games appear under **Imported Games** in the library, and duplicate imports are skipped.

For a copy you control, use **Storage → Backup all data** and keep the downloaded backup file somewhere safe. **Storage → Restore backup** imports it later. **Storage information** shows what is stored. The **Clear saved games** and **Reset all local data** commands remove data, so use the backup first if you may want it again.

To share one game with chess software, export or copy its [PGN](/help/pgn/). To preserve Ironwood's richer analysis of one game, use [analysis JSON](/help/saving-and-sharing/).

Games with a Starting note show a short preview in the library. Click that note to open the starting position, or enlarge the board preview to read the full note beneath the final board. See [Taking notes and adding annotations](/help/taking-notes/) for more.

## Larger board preview

Click a game's small board to open the larger preview. Use **First**, **Previous**, **Next**, and **Last** beneath the board to browse its positions. **Left Arrow** and **Right Arrow** also move backward and forward.

Select **Auto play** to advance one move every **0.5 seconds**. If the preview is already at the final position, autoplay starts from the beginning. **Pause**, a navigation button, or an arrow key stops autoplay. It also stops at the last move and when you close the preview. Previewing does not change the current game.

After confirming a deletion, you return to Saved Games. Deleting the currently loaded game resets the board and reopens the library with your filters, page, and maximized state preserved.

## Training profiles in backups

**Backup all data** includes [training profiles](/help/training-vs-ai/), their completed results, per-color rating history, and progress statistics. Merge restores combine results by unique game ID so a repeated backup does not count the same result twice. Keep backups before clearing browser data or switching devices.

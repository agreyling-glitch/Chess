---
title: "Saved games and backups"
description: "Find your games, understand local browser storage, and preserve a backup."
category: "using-ironwood"
keywords: [save, saved games, library, backup, restore, browser storage, data]
weight: 14
lastmod: 2026-10-30T00:00:00-05:00
---

Use **Play → Save current game** to keep the current game, and **Game History → Saved games** to open the library. Ironwood also restores the current board and related preferences when you return to the same browser and site.

While signed in to Lichess, loading a saved game does not open the lobby. If you are playing correspondence, Ironwood puts that online game aside and keeps your connection; resume it from **Your ongoing games** later. During a live online game, loading a saved game disconnects online play without resigning on Lichess. Its clock continues. See [Play on Lichess](/help/play-on-lichess/).

Saved games live in this browser's storage on this device. A different browser, private window, or device will not automatically share that library. Clearing browser site data can remove it.

Full-game analyses are saved as separate runs. Use the dropdown beside **Saved Analysis** to switch between engines or settings without replacing previous results. **Manage analyses** lists each run's engine, settings, status and date; deleting selected runs asks for confirmation and keeps the game, notes and other runs. Runs are stored with the Browser game and included in Browser backups from **Game History → Backup and Restore…**.

Use [Import Games](/help/import-games/) to paste PGN or Analysis JSON, fetch completed games from Lichess or Chess.com, or enter a scoresheet. Selected games appear under **Imported Games** in the library, and duplicate imports are skipped.

Open **Game History → Backup and Restore…**, choose **Backup** or **Restore**, then select **Game History Location**. The list includes **Browser** and each configured Companion under its saved name; disconnected Companions cannot be selected. Each action affects only the chosen location.

The Backup and Restore window uses a 3D gold frame, heavy outer grain, and a fine-grain sunken section for its controls.

Browser backups include games, collections, training profiles and preferences. Browser restore lets you review the file and choose Merge or Replace.

**Companion 0.1.6 and later** back up large archives directly on the server as SQLite database snapshots. Select a named Companion to see its **Backup folder** and available files. Choose **Configure backup folder…** to change the absolute path on that machine, or set **Backup folder** in Companion's Windows status window. Accessible NAS shares or mounted folders are supported when Companion's account has read/write permission. The default folder is `backups` beside its archive database.

Choose **Back up on Companion** to save a new snapshot there. Restore selects a snapshot already in that Companion's folder; copy a backup file to another Companion's folder before restoring there. Progress stays in the window, while the work continues on the server if you close it. Games are not downloaded through the browser. Snapshots include games, analyses and collections; tokens, engines and machine settings are not included. Older Companion versions show an update message.

Companion restore requires an empty archive and validates the backup before copying it. During backup and restore, archive requests wait and automatic imports pause; engine sessions remain available. Keep backups on a separate disk or another machine for protection against drive failure.

To share one game with chess software, export or copy its [PGN](/help/pgn/). To preserve Ironwood's richer analysis of one game, use [analysis JSON](/help/saving-and-sharing/).

Games with a Starting note show a short preview in the library. Click that note to open the starting position, or enlarge the board preview to read the full note beneath the final board. See [Taking notes and adding annotations](/help/taking-notes/) for more.

## Find and select games

Use the **Category** dropdown and expand **Filters** to access the **Player**, **Source**, **Date**, **Opening** and **Analysis** filters to narrow the library. Player, Source, Date and Opening each have a searchable dropdown; type inside it, then select an entry. Opening entries show the ECO code and opening name together, and you can search using either. Dates appear newest first. Source uses the saved import source, or the PGN Site value when no source was saved. Choose **All** to clear a filter or **Unknown** for missing information. Choose a sort order and use **Previous** and **Next** to move through pages. **Select page** selects the displayed page; **Select all matching** selects all games matching the current filters. **Clear selection** unselects games across all pages. Export or delete the selected games using the controls above the list. Click a game title to open it directly; its hamburger menu contains Favorite, Move and Delete actions. Deletion asks for confirmation.

Drag the Saved games title to move the window, drag an edge or corner to resize it, or use the title-bar maximize/restore button. Choose **Open saved games in a new window** (↗) to browse in a separate window. Your filters and selection carry over, and the game cards adjust to the available width. Saved Games remembers its size and position, as well as whether Filters is expanded. The game list scrolls within the window.

## Larger board preview

Click a game's small board to open the larger preview. Use **First**, **Previous**, **Next**, and **Last** beneath the board to browse its positions. **Left Arrow** and **Right Arrow** also move backward and forward.

Select **Play** to advance one move every **2 seconds** by default. Right-click **Play** to choose a speed; the preview remembers your choice. If the preview is already at the final position, autoplay starts from the beginning. **Pause**, a navigation button, or an arrow key stops autoplay. It also stops at the last move and when you close the preview. Previewing does not change the current game.

After confirming a deletion, you return to Saved Games. Deleting the currently loaded game resets the board and reopens the library with your filters, page, and maximized state preserved.

## Training profiles in backups

A **Browser** backup from **Game History → Backup and Restore…** includes [training profiles](/help/training-vs-ai/), their completed results, per-color rating history, and progress statistics. Merge restores combine results by unique game ID so a repeated backup does not count the same result twice. Keep backups before clearing browser data or switching devices.

## Saved-game windows

The saved-game library uses a paper-grain outer surface, a fine-linen recessed control section, and sunken game-list sections. Enlarged playback uses a grain outer surface and matching inset areas for its heading, board, and navigation buttons. These visual changes keep the existing selection, backup, import, and playback behavior.

## Sort and review your library

Use the move-count sort to find short or long games, or return to the most recent order. Games with zero moves are not added to Saved Games.

Hover a small board for the gold outline and magnifying-glass indicator. The larger move viewer identifies White, Black, and the game result. The same viewer is available from recent games in [Lichess player profiles](/help/play-on-lichess/#player-profiles-and-followed-players).

A profile game's **Review on Ironwood** session stays temporary until you choose **Play → Save current game**. Saving keeps the complete game, regardless of the move currently displayed.

The **Puzzles** category stores puzzle move history separately from ordinary games and training games. Removing a saved puzzle playback entry does not reset puzzle progress counts.

## Storage information

{{< companion-help >}}

Open **Game History → Storage information** for local record counts and the browser’s reported storage estimate. Drag the title to move the window and its edges or corners to resize it. The record sections scroll inside the gold-bordered window. A capacity chart compares estimated usage with quota when available; it describes this site’s browser storage, rather than the size of your exported backup or free disk space.

Use **ECO code** to filter an exact code (for example, `B90`) or a prefix (`B`), and **Opening name** to search part of a name (for example, `Sicilian`). The filters work together with player, category and analysis filters. Games without opening tags are classified from their moves using the deepest recognized opening position, including transpositions. Existing opening tags are preserved; custom starting positions and unrecognized lines can remain unclassified.


## Edit game metadata

Open a game's hamburger menu in Saved Games and choose **Edit metadata…**. Edit player names, event, venue/site, date, round, result, opening/ECO, ratings and FIDE IDs, then choose **Save metadata**. The changes affect library display and search details. Original PGN, moves, notes, clock timings and saved analyses stay intact. Dates use `YYYY.MM.DD`; unknown date parts may use question marks. Training names, date and result remain tied to their recorded session.

For an archive game, changes go only to the Companion selected in Saved Games. If another window changes that game first, saving reports a revision conflict instead of overwriting it; refresh and reopen the editor. Metadata also travels with game records when copying between Browser and Companion.


Training engine players use a stable engine/model name, such as **Maia3 79M (GPU)**. Opponent Elo or level/node settings appear separately in the game details and remain in the saved training session. Existing Browser games with historical rating suffixes are grouped under the stable engine name, without changing original move records or training results.


## Copy games to an archive

In Browser Saved Games, choose **Copy to Archive** from a game's hamburger menu or select several games and use **Copy to Archive** above the list. When several Companions are connected, choose the destination by its saved name. Copies retain metadata, notes, timings and saved analyses. Browser originals and the currently open game stay in place. **Move to Archive** remains available when you want to transfer and remove the Browser originals instead.


## Collections in Browser and Companion storage

Use the **Collection** selector beside Category to show **All games** or one named collection in the current storage location. Browser and each Companion own separate collections; switching storage switches the available list.

Choose **New collection**, **Rename collection**, or **Delete collection** above the list. Deleting a collection keeps its games and analyses. A game can belong to multiple collections. Select games and use **Add to collection** or **Remove from collection**, or use those actions from a single game's menu.

Copy and move actions ask for a destination collection, including **No collection**. They add destination membership before removing any source for a move. **Copy to Browser** keeps archive originals. Browser backups preserve collection names and membership; Merge resolves conflicting collection IDs, and Replace restores the backup's collection list. Companion database backups already include their collections. Companion filtering requires Companion 0.1.7 or later. The older **Your game archive** window has been removed; Saved Games is the shared management window.

## Storage and cleanup menus

**Storage information**, **Clear saved games…**, and **Reset all local data…** are at the bottom of **Game History**. The separate Storage menu and its duplicate Backup/Restore entries have been removed. Use **Game History → Backup and Restore…** for the chosen Browser or named Companion location. Read each cleanup confirmation before proceeding; resetting local data also removes local profiles and preferences.

Collections are memberships of stored games, so a game in several collections keeps one stored game record. Categories describe the kind of game; collection filters can be combined with categories and other filters.

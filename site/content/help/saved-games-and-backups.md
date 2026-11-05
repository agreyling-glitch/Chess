---
title: "Saved games and backups"
description: "Find your games, understand local browser storage, and preserve a backup."
category: "using-ironwood"
keywords: [save, saved games, library, backup, restore, browser storage, data]
weight: 14
lastmod: 2026-11-05T00:00:00-06:00
---

Use **Play → Save current game** to keep the current game, and **Game library → Saved games** to open the library. A confirmation appears after saving succeeds. Errors are reported instead of a success message; a Companion save can report that the local copy is safe while synchronization is pending. Ironwood also restores the current board and related preferences when you return to the same browser and site.

While signed in to Lichess, loading a saved game does not open the lobby. If you are playing correspondence, Ironwood puts that online game aside and keeps your connection; resume it from **Your ongoing games** later. During a live online game, loading a saved game disconnects online play without resigning on Lichess. Its clock continues. See [Play on Lichess](/help/play-on-lichess/).

Saved games live in this browser's storage on this device. A different browser, private window, or device will not automatically share that library. Clearing browser site data can remove it.

Full-game analyses are saved as separate runs. Use the dropdown beside **Saved Analysis** to switch between engines or settings without replacing previous results. **Manage analyses** lists each run's engine, settings, status and date; deleting selected runs asks for confirmation and keeps the game, notes and other runs. Runs are stored with the Browser game and included in Browser backups from **Game library → Backup and Restore…**.

Use [Import Games](/help/import-games/) to paste PGN or Analysis JSON, fetch completed games from Lichess or Chess.com, or enter a scoresheet. Selected games appear under **Imported Games** in the library, and duplicate imports are skipped.

Imported Lichess studies appear in the Browser library's **Studies** category, with one entry for each study and its chapter count. Older separate chapter saves from the same Lichess study are combined while keeping notes, analysis, favorites, and collection membership. Original chapter records remain inside the consolidated record in browser backups. Open the study entry to return to its chapters in **Study mode**. [Read and practice studies](/help/lichess-studies/).

Open **Game library → Backup and Restore…**, choose **Backup** or **Restore**, then select **Game History Location**. The list includes **Browser** and each configured Companion under its saved name; disconnected Companions cannot be selected. Each action affects only the chosen location.

The Backup and Restore window uses a 3D gold frame, heavy outer grain, and a fine-grain sunken section for its controls.

Browser backups include games, collections, training profiles and preferences. Browser restore lets you review the file and choose Merge or Replace.

**Companion 0.1.6 and later** back up large archives directly on the server as SQLite database snapshots. Select a named Companion to see its **Backup folder** and available files. Choose **Configure backup folder…** to change the absolute path on that machine, or set **Backup folder** in Companion's Windows status window. Accessible NAS shares or mounted folders are supported when Companion's account has read/write permission. The default folder is `backups` beside its archive database.

Choose **Back up on Companion** to save a new snapshot there. Restore selects a snapshot already in that Companion's folder; copy a backup file to another Companion's folder before restoring there. Progress stays in the window, while the work continues on the server if you close it. Games are not downloaded through the browser. Snapshots include games, analyses and collections; tokens, engines and machine settings are not included. Older Companion versions show an update message.

Companion restore requires an empty archive and validates the backup before copying it. During backup and restore, archive requests wait and automatic imports pause; engine sessions remain available. Keep backups on a separate disk or another machine for protection against drive failure.

To share one game with chess software, export or copy its [PGN](/help/pgn/). To preserve Ironwood's richer analysis of one game, use [analysis JSON](/help/saving-and-sharing/).

Games with a Starting note show a short preview in the library. Click that note to open the starting position, or enlarge the board preview to read the full note beneath the final board. See [Taking notes and adding annotations](/help/taking-notes/) for more.

## Find and select games

The Search console has **Search games or players** and **Manage library** on its top row. **Show** selects a category. **Filters** starts collapsed each time you open the library, even if it was expanded before.

Expand Filters for aligned **Player**, **Source**, **Date**, **Opening**, **Analysis**, **Game length**, and **Folder** fields. Training adds a **Profile** field. Player, Source, Date and Opening retain searchable dropdowns. Opening combines the ECO code and opening name; dates are newest first. Choose All to clear a field or Unknown for missing information.

Active filters appear as removable gold chips outside the collapsed grid. Remove one chip or use **Clear all** to reset the view while keeping your sort order. An active **Not analyzed** filter removes newly analyzed games from the results and fills the page with the next matching games. Clear it to see completed analyses.

The table's **Analysis** column shows **Complete**, **Partial**, or **Not analyzed**, taking retained saved runs into account. Hover for the number of analyzed positions. An unfinished critical verification pass remains Partial. Click **Moves** or **Saved** to sort; other sorting choices, refresh and folder management remain under **Manage library**.

Click a game title to open it directly. Select another part of the row to inspect its preview. Checkboxes select games independently of the preview: **Select page**, **Select all matching**, and **Clear selection** work across pages. The grouped selection console provides **Export** (PGN, annotated PGN, Analysis JSON), **Organize** (folder and available Companion transfer actions), and **More** (Delete selected). Deletion still asks for confirmation. Each row's menu retains the individual game actions.

Drag the title or window edges to move or resize Saved Games, maximize/restore it, or use **Open saved games in a new window**. The table keeps stable column widths across pages. Size and position are remembered; filters reopen collapsed.

## Batch analysis

1. Select at least two saved games using their checkboxes, **Select page**, or **Select all matching**.
2. In the inset analysis section, choose **Stockfish 19 · Browser** or an available engine from a connected Companion.
3. Leave **Verify critical moves** checked for the deeper second pass, or uncheck it for a first pass only.
4. Choose **Analyze N games**.

The modal reports the current game, pass, position and overall progress. Pass 1 analyzes each position for one second. Pass 2 uses Ironwood's critical-move criteria to check positions before and after flagged moves for five seconds each. It covers inaccuracies, mistakes, blunders and mate-related changes, with duplicate positions checked once.

Games are processed sequentially without replacing the current board. Existing notes and analysis runs are preserved. Each saved result is read back before the game is counted as complete. The final report lists complete, partial and failed games; failures include an explanation. Companion connection or revision conflicts are reported rather than silently overwriting another edit.

**Cancel analysis** stops the batch and saves available first-pass and verification results. Completed games remain saved; an unfinished verification pass remains Partial. The dialog can be closed after saving finishes. Keep the browser tab open while the batch runs. Study workspaces require individual chapter analysis rather than batch analysis of the whole study.

If **Not analyzed** is selected, successful games disappear from that filtered view. This does not delete them. Remove that chip, or choose **Fully analyzed**, to find them again. Reopen a game to inspect its saved run and critical-pass status. [Compare saved analyses](/help/saved-analysis/).

## Larger board preview

Click a game's small board to open the larger preview. Use **First**, **Previous**, **Next**, and **Last** beneath the board to browse its positions. **Left Arrow** and **Right Arrow** also move backward and forward.

The **☰** options button beside the preview’s close button offers the same actions as that saved game’s library menu, including **Open Game**, **Export PGN**, and named Companion destinations. Choosing an action closes the preview and carries out that action.

Select **Play** to advance one move every **2 seconds** by default. Right-click **Play** to choose a speed; the preview remembers your choice. If the preview is already at the final position, autoplay starts from the beginning. **Pause**, a navigation button, or an arrow key stops autoplay. It also stops at the last move and when you close the preview. Previewing does not change the current game.

After confirming a deletion, you return to Saved Games. Deleting the currently loaded game resets the board and reopens the library with your filters, page, and maximized state preserved.

## Training profiles in backups

A **Browser** backup from **Game library → Backup and Restore…** includes [training profiles](/help/training-vs-ai/), their completed results, per-color rating history, and progress statistics. Merge restores combine results by unique game ID so a repeated backup does not count the same result twice. Keep backups before clearing browser data or switching devices.

## Saved-game windows

The saved-game library uses a table with a selected-game preview alongside it. Select a row to inspect its board, players, result, analysis details and notes, then choose **Open game**. Click the preview board for the full playback viewer. On narrow windows the preview appears below the table.

Use **Search games or players** to find a game, **Show** to choose a game category, and **Filters** for player, source, date, opening, analysis status or game length. Active filters appear as removable chips; **Clear all** resets them. The **Folder** picker selects the library's collections. **Manage library** contains **Import games…**, folder management, refresh and sorting options. Import opens the same four-tab window used by **Game library → Import…**. A separate Saved Games window focuses the main app for import; keep it open. If that app is no longer available, the library window opens the app’s import page. You can also sort by clicking **Moves** or **Saved** in the table header.

**Source** groups links as **Lichess games**, **Lichess puzzles**, **Lichess studies**, and **Chess.com**. Each game's original source URL remains in its details and exports. Other source names remain unchanged. Older Companion builds may need an update to filter these groups across the archive.

Checkboxes select games for batch operations independently of the preview. Each row's options menu and the preview's **More options** retain export, metadata, collection, category, favorite, deletion and available Companion transfer actions. **Game location** switches between Browser and named Companion libraries; **Manage connections** opens their settings. Category, collection and all existing filters remain available. Click **Moves** or **Saved** in the table header to sort, or use sorting choices under **Manage library**.

## Sort and review your library

Use the move-count sort to find short or long games, or return to the most recent order. Ordinary games with zero moves are not added. Studies and puzzles can be saved at their initial position, including their annotations.

Hover a small board for the gold outline and magnifying-glass indicator. The larger move viewer identifies White, Black, and the game result. The same viewer is available from recent games in [Lichess player profiles](/help/play-on-lichess/#player-profiles-and-followed-players).

A profile game's **Review on Ironwood** session stays temporary until you choose **Play → Save current game**. Saving keeps the complete game, regardless of the move currently displayed.

The **Puzzles** category stores puzzle move history separately from ordinary games and training games. Removing a saved puzzle playback entry does not reset puzzle progress counts.

## Storage information

{{< companion-help >}}

Open **Game library → Storage information** for local record counts and the browser’s reported storage estimate. Drag the title to move the window and its edges or corners to resize it. The record sections scroll inside the gold-bordered window. A capacity chart compares estimated usage with quota when available; it describes this site’s browser storage, rather than the size of your exported backup or free disk space.

Use the searchable **Opening** dropdown to find an ECO code (for example, `B90`) or an opening name (for example, `Sicilian`). Choose an individual opening or an available **All [ECO] games** entry. The filters work together with player, category and analysis filters. Games without opening tags are classified from their moves using the deepest recognized opening position, including transpositions. Existing opening tags are preserved; custom starting positions and unrecognized lines can remain unclassified.


## Edit game metadata

Open a game's hamburger menu in Saved Games and choose **Edit metadata…**. Edit player names, event, venue/site, date, round, result, opening/ECO, ratings and FIDE IDs, then choose **Save metadata**. The changes affect library display and search details. Original PGN, moves, notes, clock timings and saved analyses stay intact. Dates use `YYYY.MM.DD`; unknown date parts may use question marks. Training names, date and result remain tied to their recorded session.

For an archive game, changes go only to the Companion selected in Saved Games. If another window changes that game first, saving reports a revision conflict instead of overwriting it; refresh and reopen the editor. Metadata also travels with game records when copying between Browser and Companion.


Training engine players use a stable engine/model name, such as **Maia3 79M (GPU)**. Opponent Elo or level/node settings appear separately in the game details and remain in the saved training session. Existing Browser games with historical rating suffixes are grouped under the stable engine name, without changing original move records or training results.


## Copy games to an archive

In Browser Saved Games, use a game’s **☰** menu, the same menu in its larger preview, or select several games and use the controls above the list. With one configured Companion, **Copy to [Companion name]** and **Move to [Companion name]** use the name you chose for that connection. With several Companions, **Copy to…** and **Move to…** open destination submenus listing those names. Disconnected destinations are disabled.

Copies retain metadata, notes, timings and saved analyses. Browser originals and the currently open game stay in place. Moving transfers the games and removes the Browser originals after the destination is saved. When browsing a Companion, **Copy to Browser** and **Move to Browser** transfer games back to this browser.


## Collections in Browser and Companion storage

Use **Filters → Folder** to show **All games** or one named collection in the current storage location. Browser and each Companion own separate collections; switching storage switches the available list.

Choose **New folder**, **Rename folder**, or **Delete folder** under **Manage library**. Deleting a collection keeps its games and analyses. A game can belong to multiple collections. Select games and use **Organize → Add to folder…** or **Organize → Remove from folder**, or use those actions from a single game's menu.

Copy and move actions ask for a destination collection, including **No collection**. They add destination membership before removing any source for a move. **Copy to Browser** keeps archive originals. Browser backups preserve collection names and membership; Merge resolves conflicting collection IDs, and Replace restores the backup's collection list. Companion database backups already include their collections. Companion filtering requires Companion 0.1.7 or later. The older **Your game archive** window has been removed; Saved Games is the shared management window.

## Storage and cleanup menus

**Storage information** is in **Game library**. Cleanup actions are under **Settings → Local data → Clear saved games…** and **Settings → Local data → Reset all local data…**. Use **Game library → Backup and Restore…** for the chosen Browser or named Companion location. Read each cleanup confirmation before proceeding; resetting local data also removes local profiles and preferences.

Collections are memberships of stored games, so a game in several collections keeps one stored game record. Categories describe the kind of game; collection filters can be combined with categories and other filters.

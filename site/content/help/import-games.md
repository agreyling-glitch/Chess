---
title: "Import games"
lastmod: 2026-10-24T00:00:00-05:00
description: "Import PGN, ZIP and Scid databases, fetch online games, or enter a handwritten scoresheet."
category: "using-ironwood"
keywords: [import, tabs, PGN, ZIP, Scid, si4, si5, database, compressed, drag and drop, JSON, collection, Lichess, Chess.com, scoresheet]
weight: 12
---

Open **Game History → Import…** to choose among four tabs. Drag the title or outer background to move the window; drag an edge or corner to resize it. The tab row wraps on narrow windows, and panel contents scroll vertically when needed. The gold-bordered window has a heavy paper-grain outer background and recessed content sections. Close it with **×** or Escape.

## Paste a game or collection

{{< companion-help >}}

Paste [PGN](/help/pgn/) for one game or a collection, or paste an Ironwood Analysis JSON export. Use the gray **Browse…** button to choose a `.pgn`, `.json`, or `.zip` file. Browsed game collections open the game-selection screen directly, just like drag-and-drop. Single games and JSON fill the paste area; choose **Continue** for those. **Clear** empties the text. Analysis JSON restores saved analysis when you continue. You can also drag a PGN, JSON or ZIP file onto the app to enter the normal game or collection import flow.

### Import a ZIP collection

Choose a ZIP with **Browse…**, or drop the ZIP onto the app. Ironwood reads the archive locally and combines every `.pgn` file, including files inside folders. Collections open game selection directly without rendering the entire PGN in the paste area. A ZIP containing only one game fills the paste area when browsed; choose **Continue** for that game. Other files are ignored; ZIP archives of Analysis JSON exports are not supported. Select the games you want to save in the review window. Existing duplicate detection still applies.

Both the ZIP file and the combined uncompressed PGN content can be up to **64 MiB**, with at most **1,000 archive entries**, including folders and unrelated files. Files are checked for corruption before the PGN text enters the import flow. Standard stored and deflated ZIP files are supported. Unzip password-protected, ZIP64, multi-part or unsupported archives first, then choose their PGN files. If the browser cannot decompress the ZIP, unzip it first as well.

### Import a .7z archive

Choose a `.7z` file with **Browse…**, or drop it onto the app. Ironwood extracts it locally in a background worker, with progress and a **Cancel** button. Archives containing PGNs enter the usual game review and duplicate checks. An archive containing one complete Scid 5 database opens the range-selection window; Scid 4 databases use the usual conversion and review flow. Scid archives must include the three matching index, name and game files.

Large Scid 5 archives use temporary browser storage rather than loading the database into memory. Enough browser storage must be available for the extracted files. Archives and expanded content are limited to **64 GiB** and **1,000 entries**; combined PGNs and Scid 4 databases remain limited to **64 MiB**. Temporary files are removed after review or when the Scid window closes. Password-protected archives, unsupported compression methods, mixed PGN/Scid archives, or archives containing several Scid databases must be extracted manually first. Original files remain unchanged.

### Import a Scid database (.si4)

Choose **Browse…** and select all three matching files together: for example, `mygames.si4`, `mygames.sn4`, and `mygames.sg4`. After conversion, collections open the usual game selection and duplicate detection; for a single game, choose **Continue**. You can also drop the three files together onto the app. Unzip a Scid database first; ZIP import reads PGN files only.

Scid import runs locally and is read-only: the original files are never changed. For `.si4`, the three files together, and the resulting PGN text, must each fit within **64 MiB**. A progress indicator and **Cancel reading** are available during conversion through Browse. Games marked deleted in Scid are skipped. Invalid files or an undecodable game stop `.si4` conversion before anything is saved. Only `.si4` version 400 is supported; export `.si3` databases to PGN first.

The conversion includes standard game details, ratings, ECO codes, moves, variations, move comments, supported move symbols, and custom starting positions. It does not preserve custom PGN tags, pre-game comments, or Scid-specific flags and database settings. Only one supported move symbol per move is retained. Keep the original database if those details matter.

### Import a large Scid 5 database (.si5)

In the browser app, choose **Browse…** and select the matching `.si5`, `.sn5`, and `.sg5` files together, or drop all three files onto the app. Extract `.7z` and ZIP archives first. A separate Scid import window opens with the database's game count. Scid 5 index and game files are read in small chunks, so multi-gigabyte databases do not need to fit in memory. The name file is loaded once and can be up to **128 MiB**. Scid 4, PGN and ZIP limits are unchanged.

Drag the title or outer background to move the gold-bordered window; drag any edge or corner to resize it. The game list scrolls inside the window. Clearing **Through game** restores the database's last game when you leave the field or use an action; clearing **From game** restores game 1.

Use **From game**, **Through game** and **Player name contains** to choose a range or filter. **Preview first 100**, **Next 100** and **Previous 100** browse matching games without decoding the entire database. Check games and choose **Review selected games** to send multiple games directly to the usual selection screen. For a single game, choose **Continue** in the import screen.

Choose **Import to Browser** to save the range directly; the default range covers the entire database. Games are decoded and saved in batches of at most 32 using the same PGN preparation, duplicate fingerprints and minimum-length rule as ordinary imports. Duplicate checks use a disk index. Progress lists saved, duplicate, short, deleted and failed games. Undecodable games are counted as failed and the first error is shown. Invalid index data or a storage failure stops the operation; previously saved batches remain intact.

**Stop** finishes the current batch. To resume, select the same three files again and choose **Resume last import**. The resume point is saved on this device; retrying an earlier range also safely skips duplicates. Browser storage quota still applies, and saved games may occupy more space than the compressed Scid database. Imports stop if a storage batch cannot be committed. Keep the original files and start with a smaller range when checking a new database. If prompted during a storage update, close other Ironwood tabs and retry.

Resize the window vertically for a larger paste area. The middle sunken section expands with it; Clear and Continue remain directly below on the outer background.

For a collection, select games to keep and click **Import selected**. Drag the review window’s title to move it or its edges to resize it. Use the player filter and **Select matching** to select matching games. A progress bar shows the number processed and percentage for a large import. The result lists games saved, already saved and failed; if importing stops early, it also shows games not attempted. Games already in your library are skipped. Find saved imports under **Game History → Saved games → Imported Games**.

## Lichess

Enter a player's username to fetch up to the chosen number of matching completed games (20 by default), or paste a completed-game link to fetch one game. Click the gold **Fetch games** button and watch the status below the input. No login or API token is needed. [Lichess import instructions](/help/import-from-lichess/).

## Chess.com

Enter a Chess.com username and click **Fetch games** for up to the chosen number of completed standard chess or Chess960 games (20 by default). No login or API token is needed. Recently finished games may take time to appear in the cached archives. [Chess.com import instructions](/help/import-from-chess-com/).

Both panels offer **Games** (20, 50, 100, or 200), **Time control**, and optional **From** and **Through** dates in `YYYY-MM-DD` format. Dates use UTC and include the entire Through date; leave either blank for an open range. The newest matching games are fetched first, up to your chosen count. **Reset filters** restores 20 games, all time controls, and no date limits. Filters apply to usernames, not individual Lichess game links. Chess.com uses game completion dates. Lichess uses its game-date filters. Selecting a specific Lichess speed restricts results to that performance category.

Both online sources require an internet connection. After fetching, **Paste a game or collection** opens with the game text; choose **Continue** to review and import it.

## Scoresheet

For direct access, choose **Tools → Enter a scoresheet…**. This opens the same workspace as the Scoresheet tab's Enter scoresheet button.

The three-step guide explains how to add a reference photo, enter and check moves, and import the finished game. Click **Enter scoresheet** to open the workspace with a board preview and live move validation. You enter moves manually; the photo is a reference. Photos and unfinished drafts stay on your device, and a photo is optional. [Scoresheet entry instructions](/help/import-a-scoresheet/).

## Analyze after importing

Importing does not automatically start full-game analysis. Open an imported game and start it from **Analysis** when ready. [Analysis JSON](/help/saving-and-sharing/) restores existing results without another engine run.

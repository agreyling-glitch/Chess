---
title: "A local-first chess library that can survive an update"
date: 2026-09-24T10:00:00-05:00
description: "How Ironwood grew from one restored position into a searchable, portable game library without giving up local privacy."
slug: "local-first-library-and-safe-updates"
---

Ironwood originally remembered one game. That was enough to make a refresh feel safe, but it was not enough to build a personal chess practice around. Imported master games, engine analysis, favorite positions, and display preferences all need clearer ownership and a reliable way to move between browser versions.

The new Saved Games library keeps personal and imported games in IndexedDB. Games can be filtered by player and analysis state, marked as favorites, and reopened with their analysis intact. The application still works locally: browsing the library, restoring a game, and running Stockfish do not require an account or send a position to a server.

Large PGN files can contain many games. Ironwood now opens a selection screen for those collections: search or filter by player, choose matching games or individual entries, and watch import progress. It checks for duplicates before saving, so importing the same collection again does not fill the library with copies. Imported games can then be opened and analyzed individually.

## More ways to play

A new game can now begin with you playing White, playing Black, or letting Ironwood choose a side at random. When you choose Black, the board turns to your perspective and Stockfish makes the opening move. Several piece sets let you change the character of the board, while move animation and sounds make play easier to follow. These display choices are global preferences, so reopening a different saved game does not unexpectedly change your preferred pieces or feedback.

## The upgrade path matters

Adding a database is easy compared with safely introducing one to an installed application. Existing players already have a game stored by the original release. On first launch, the new build migrates that legacy game into the library rather than treating the new schema as an empty start.

The update process is deliberately visible. Ironwood compares the running application shell, WebAssembly package, Stockfish cache, server manifest, and active service worker. A waiting release is offered as an update instead of silently replacing a session. The About window exposes those versions so a stale component can be identified without guessing.

Before release, the production upgrade is tested as a complete sequence: preserve a recognizable game in the existing installed app, apply the update, confirm that the migrated game appears in Saved Games, and then reopen it offline. A successful development refresh is not a substitute for that test.

## Backups remain understandable

The Storage menu exports a versioned JSON document containing complete game records, analysis, favorites, and global display preferences. JSON is intentionally the first backup format: it is inspectable, straightforward to validate, and does not require a proprietary archive reader.

Restore validates the document before changing storage and offers two explicit choices. **Merge** keeps the existing library and adds the backup. **Replace existing** clears the game library first. Clearing saved games and resetting all local data are also separate operations, because deleting a library should not accidentally remove preferences or the large offline engine.

This is less glamorous than a new evaluation graph, but it is foundational. A private chess application should make local ownership concrete: players can see what is stored, copy it, restore it, and understand exactly what an update will do.

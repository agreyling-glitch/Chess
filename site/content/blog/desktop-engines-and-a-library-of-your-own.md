---
title: "Desktop engines and a library of your own"
date: 2026-10-26T00:00:00-05:00
description: "Windows Companion brings desktop engines and a local archive to Ironwood, with named storage locations and visible import progress."
slug: "desktop-engines-and-a-library-of-your-own"
---

Ironwood began with a board and an engine running in the browser. That remains a useful way to play and study: open the app, load a game, and start analyzing. Larger collections and desktop engines bring a different set of needs, though. They need a place on disk, an engine process, and work that can keep running while you use the board.

**Ironwood Companion 0.1.4 is now available for Windows.** It runs on your computer alongside Ironwood, providing desktop engines and a local game archive. It is optional: you can continue using browser Stockfish and Browser storage without installing it.

## Give each collection a place

Storage information, Saved games, and Import now share a location dropdown. Choose **Browser** for this browser's library, or choose a named Companion for its archive. If you connect more than one Companion, their names help identify where you are browsing or saving.

These are separate libraries. Connecting Companion does not move your existing browser games. Choosing a destination explicitly makes it easier to keep a small study collection in the browser and a larger collection in an archive, while checking the location before exporting or deleting games.

Opening filters also need to make sense across a large collection. Games keep their ECO code, with descriptions supplied by a shared lookup. **All [ECO code] games** lets you search across the variations under that code instead of accidentally selecting only one description.

## Bring a desktop engine to the board

The Add engine window separates two jobs. **Auto-install** offers packages supported by the selected Companion and downloads them directly from the original publisher. **Manual** lets you register an engine you already have, keeping its executable and supporting files together.

Manage engines handles the installation: its name, executable, launch arguments, and removal. Engine settings handles how you use it for play and analysis. Renaming an engine keeps its registration identity, so a clearer display name does not create a different engine registration.

## Make long imports understandable

Collections can arrive as PGN, ZIP, 7z, or supported Scid databases. Browse and drag-and-drop are available in the app. Companion also watches its Import folder, so you can place a collection there and follow its progress from Ironwood. Compressed collections can contain PGN or matching Scid files; Scid sources remain unchanged.

The progress display distinguishes games read, saved, duplicated, skipped as short or deleted, and failed. Speed and estimated time remaining help explain how the work is moving, while waiting and retrying messages explain pauses. Folder imports save checkpoints after committed batches so supported imports can resume after a restart.

Saving games and building the opening index are separate tasks. An import can finish while indexing is still catching up. The main status bar shows indexed and remaining games, speed, and an estimate for that work too.

## Start with Windows

The public package has been tested on Windows 11 with an Intel/AMD 64-bit processor, including a clean Windows virtual machine. Other platforms are not offered as public downloads yet.

Companion's executable does not have a Windows publisher signature. Its update verification uses Ironwood's release key and package checksum; that is separate from Windows publisher trust. The [download page](/downloads/companion/) explains installation, verification, and connecting with a token. Keep that token private.

To get started, download Companion and open **Help → Companion connections…** in Ironwood. The [setup guide](/help/companion-setup/) walks through the connection, and the [engine and storage guide](/help/companion-engines-and-storage/) covers the next steps.

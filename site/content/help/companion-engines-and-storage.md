---
title: "Engines and storage with Companion"
description: "Install and manage desktop engines, choose an archive, and follow automatic imports."
category: "using-ironwood"
keywords: [Companion, archive, engine, auto-install, manual, Import folder, ZIP, 7z, progress, backup]
weight: 62
draft: false
lastmod: 2026-10-27T00:00:00-05:00
---

## Install an engine

Connect Companion using the [Windows setup guide](/help/companion-setup/). Open **Game → Engine settings…** and choose the **+** button next to the engine list.

**Auto-install** shows packages supported by the selected Companion. Choose a package to download and install. Companion downloads directly from the original publisher, checks the package, and registers the engine. Downloads can continue after closing the window. Engines and any models are stored on the Companion computer.

**Manual** is for engines you already have. Keep the executable and its supporting files together under Companion's Engines folder. Choose the folder and executable, review the detected engine name, then choose **Save engine**. Expand the arguments section only if the engine needs additional launch arguments.

## Manage an installed engine

Select the engine in Engine settings, then choose the sliders icon, **Manage engines**. This opens the selected engine; you do not need to choose it again. Change its display name, executable, or launch arguments, then save and reselect the engine to start a new session.

The **Engine options** tab controls the engine's advertised playing and analysis options. **Play**, **Realtime Analysis**, and **Full game Analysis** control the work for those uses. Managing the installation does not replace these controls.

Renaming an engine changes its display label and keeps its registration identity. **Remove from Companion…** removes that registration. For packages installed by Companion, you can also choose to delete its installed files. Manually added engine files are kept. Saved games and training history are kept; a profile using a removed engine cannot start until that engine is restored.

## Import into an archive

Open **Game → Import…** and choose the named Companion in the location list. Choose a supported file with **Browse…**, drag it onto the app, or paste PGN. Review the selection and destination before importing. Choose **Browser** instead if the collection should stay in this browser.

For Scid databases, clearing **Through game** returns the range to the database's last game. Use the full matching set of Scid files, or a supported archive containing that set. Scid import reads the source without changing it.

## Use the Import folder

In Companion's status window, choose **Import** to open its watched folder. Place a PGN, ZIP, or 7z file there. ZIP and 7z archives can contain PGN files or matching Scid database files. For an uncompressed Scid database, keep all matching files together. The original source is read locally.

In Ironwood's Import window, select that Companion to see its folder-import progress. Counts distinguish read, saved, duplicate, short, deleted, and failed games. Speed and estimated time remaining describe the import; opening-index progress is a separate task and can trail behind saved games. Estimates vary as the workload changes.

The queue reports when it is waiting, retrying, stopped, complete, or needs attention. Use the **Reports** folder for the import report. On restart, supported folder imports resume from the last saved batch rather than treating earlier committed batches as new work.

## Back up the right location

Browser backups and Companion archives are separate. Check the selected location before exporting or deleting games. Export games you need in PGN for interchange. For a complete Companion data-folder backup, quit Companion before copying the data folder so the archive is not changing during the copy. Keep engine models and configuration with the backup if you need a complete recovery.

See [Companion troubleshooting](/help/companion-troubleshooting/) for connection and import errors.

## Switch engines

Use the engine list in **Game → Engine settings…**, or the selector beside Analyze, to choose a Companion engine. Your choice appears immediately while its capabilities are prepared. Returning to a previously discovered engine in the same browser session skips the discovery probe. Refreshing installed engines, or changing connections or credentials, clears that cache. First-time discovery and starting the actual engine can still take time.

To compare results already saved for a game, use [Saved Analysis](/help/saved-analysis/) instead of changing the engine for the next run.

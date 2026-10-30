---
title: "Companion 0.1.6: server-side archive backups"
date: 2026-10-30T13:00:00-05:00
slug: "2026-10-30-companion-server-backups"
version: "Companion 0.1.6"
description: "Configure a backup folder on each Companion and back up large archives without downloading them into the browser."
---

- Added a configurable Backup folder in Companion's Windows settings and connection settings. Paths refer to the machine running Companion, including accessible NAS shares or mounted storage.
- Game History → Backup and Restore now lists snapshots from the selected named Companion's folder and runs SQLite backup/restore jobs on that server, with progress and results in Ironwood.
- Snapshots preserve games, analyses, collections and indexes without building a giant JSON response. Incomplete backups are not published. Restore validates the snapshot and requires an empty archive.
- Browser backup and restore retain their existing JSON and Merge/Replace behavior.

Companion **0.1.6 is a development build**. The immutable published Windows download and signed feed remain unchanged at 0.1.4.

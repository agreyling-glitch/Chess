---
title: "Companion troubleshooting"
description: "Resolve startup, connection, import, and update problems without losing your archive."
lastmod: 2026-10-28T00:00:00-05:00
category: "settings"
keywords: [Companion, failed to fetch, token, permission denied, import paused, updates, Windows, connection, startup, sign in, 0.1.5]
weight: 63
draft: false
---

## Companion will not start

Extract the entire ZIP first; do not run the executable inside the compressed archive. Keep the package files together, and use the Windows x64 package on a supported computer. See the [download page](/downloads/companion/) for requirements and Windows publisher-signature information. If security software blocks the package, verify its source and checksum; do not disable protection to make it run.

## Companion does not start when I sign in

The built-in **Start Companion when I sign in** checkbox requires **0.1.5 or later**.
The current public Windows release is **0.1.8**. Use **Check for updates…**, or download the new ZIP and follow the manual update instructions while preserving your data folder.
In a supported build, open the status window and check the Settings row. If you
moved the application folder, clear and enable the option again from its new
location. Startup runs after Windows sign-in, so it will not run while the PC
is waiting at the sign-in screen. If Windows has disabled Companion under
**Settings → Apps → Startup**, enable it there as well.

If enabling the checkbox reports an error, read that message and confirm you can
write to your account's Startup folder. The checkbox returns to the actual
shortcut state if the change fails. You can still start Companion manually.

To stop future automatic starts, clear **Start Companion when I sign in**.
Choosing **Quit** only stops the current session. For folder moves, custom data
paths, and removing the portable app, follow the
[sign-in startup guide](/help/companion-setup/#start-automatically-when-you-sign-in).

## Failed to fetch or disconnected

Check that Companion is running and says **Ready**. Confirm the selected connection's address and name. For the same computer, use `http://127.0.0.1:8790`; for a remote Companion, use the configured remote address. A firewall, browser local-network permission, unavailable server, or disallowed website origin can also prevent access. If the browser asks about local-network access, review that request yourself for the Ironwood site you are using.

Adding a website to allowed origins does not make a locally listening Companion reachable over the network. See [Set up Companion](/help/companion-setup/#use-another-computer).

## Connection token rejected

Use **Show token…** on the Companion you intend to reach. Paste it into that connection's **Connection token** field and save. Do not use another Companion's token or a release-download credential. Rotating the owner token invalidates the old token; update each browser connection afterward. Do not share tokens in screenshots or reports.

## Saved games seem missing

Check the location list: **Browser**, **Win11**, and other names refer to separate libraries. Clear category, player, opening, and other filters, then refresh. Opening indexing can continue after import completes. Do not reset the archive or clear browser data as a first troubleshooting step.

## Import is waiting or paused

Read the waiting message before retrying. Request-capacity waits and retries are different from a stopped import. Speed and time estimates can pause when no batch is advancing. If the queue says **Permission denied**, confirm the Companion account can write its configured data, Import, and Reports folders. On a NAS, the mapped folders also need permissions for the container's configured user; Windows folder instructions will not repair a NAS mapping.

For **Archive operation failed**, preserve the source file and read the import report. Do not repeatedly reset or reimport the whole database. Include the filename, error text, selected Companion, and version when reporting a problem, without sending its token or private configuration.

## Update check fails

Keep using the installed version if it is working. Check the connection and retry later. An unavailable release service or failed verification is not an “up to date” result. Companion must verify the signed release metadata and package checksum before installing an update; do not work around a verification failure with an untrusted package. A failed check leaves the installed version available; retry when the release service is reachable.

## Updating from 0.1.4 or 0.1.7

Those versions can verify an update but fail to restart it because the updater encoded the executable name incorrectly. Rollback retains the previous version. Download the current Windows ZIP and perform one manual update: quit Companion, replace only its application files in the same folder, and keep its configured data folder. Version 0.1.8 fixes the executable-name handoff for future updates. Never disable signature verification or delete the data folder to work around this problem.

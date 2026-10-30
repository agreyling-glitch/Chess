---
title: "Companion 0.1.5: optional Windows sign-in startup"
date: 2026-10-28T00:00:00-05:00
slug: "2026-10-28-companion-sign-in-startup"
version: "Companion 0.1.5"
description: "Choose whether Ironwood Companion starts when you sign in to Windows."
draft: true
---

Companion **0.1.5** is a development build. Its public Windows package and signed
update feed have not been published; the public download remains **0.1.4**.

## Start Companion when you sign in

- Added **Settings → Start Companion when I sign in** to Companion's status window.
- Checking it creates a Windows Startup shortcut for your account. Clearing it
  removes the shortcut, without requiring administrator permission.
- The shortcut preserves your current data directory and configuration path,
  including paths with spaces and non-English characters. Application updates
  at the same folder location keep the shortcut working.
- If a shortcut change fails, Companion shows the error and restores the
  checkbox to the actual shortcut state.

See the [setup guide](/help/companion-setup/#start-automatically-when-you-sign-in)
for folder moves and disabling startup before removing the portable app.

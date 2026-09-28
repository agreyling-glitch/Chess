---
title: "FICS sign-in and menu fixes"
date: 2026-09-28T00:00:00-05:00
description: "More reliable FICS sign-in and challenges, clearer connection feedback, and in-place game-list refreshes."
slug: "2026-09-28-fics-connection-and-menus"
version: "2026.09.28"
---

## Fixed

- The FICS sign-in window stays open while entering a handle or password. The direct-player challenge window also stays open while entering a handle and time controls.
- An unregistered FICS handle now produces a clear error instead of leaving the interface at **Connecting to FICS**.
- Refreshing **Available games** or **Observe a game** updates the list without closing its menu.

## Improved

- Redesigned the FICS sign-in window with clearer fields, spacing, and connection guidance.
- The FICS console now shows server dialogue as soon as the connection opens, including the sign-in exchange before the session is ready.

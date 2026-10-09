---
title: "A board to focus on and make your own"
date: 2026-10-09T00:00:00-05:00
slug: "a-board-to-focus-on-and-make-your-own"
description: "Focus mode, personal opening history, online player statistics, and a more individual look for every board theme."
---

A busy analysis workspace is useful when you are comparing moves. Sometimes you want the board to fill the space. Ironwood’s new focus mode hides the menus and study panels while keeping the players, clocks, captures, and essential board controls visible. Use the focus icon beside Flip board or press **Shift+Enter**; **Escape** brings the workspace back. Real-time position analysis pauses during focus mode and resumes afterward when needed.

The online connections now offer more context beside the board. Lichess displays your overall record and time-control ratings after Connected, with more account details on hover. FICS shows Lightning, Blitz, and Standard ratings and exposes the account’s rating records on hover. Guests are labeled separately.

Opening Explorer also connects positions to your own history. **My games** and **Player** let you filter a player’s results, examine continuations, and find recent matching games to import for analysis. Lichess may need time to index a player’s games; the explorer now shows that progress rather than presenting an initial empty response as a settled record of zero games.

## Give each theme its own character

Classic Staunton, Neon Geometric, and Art Deco Faceted join the 2D chooser. Cream and walnut suit the Staunton set; navy squares frame the cyan and magenta geometric pieces; jade and gold complement the faceted set. Those palettes appear in the board, chooser previews, and copied board images.

Each theme card has a gear that opens its adjustments. Settings belong to that theme, so experimenting with one look leaves your other themes ready to return to. The grouped controls keep proportions together and give colors, depth, shadows, and lighting their own space.

For illustrated 2D pieces, choose solid side colors or gradients with separate endpoints for White and Black. Inner shading adds volume within the artwork, while contrasting outlines help the silhouette stand apart from the board. The 3D controls include height and material finish, plus the overall appearance and radial light below the board. Right-button drag rotates the 3D camera, right-button double-click resets it, and the wheel zooms.

The smoothing work also exposed a performance problem: the small adjustments preview and live board could repeatedly rebuild differently sized versions of the same piece. That preview is removed, sprite resolution is bounded, and processed artwork is cached. The outline filter now uses a linear-time distance pass, so its cost does not grow with a large neighborhood search around every pixel.

See [Board themes and appearance](/help/board-themes/), [Playing a game](/help/playing-a-game/), and [Opening Explorer](/help/opening-explorer/) for the controls, or read the [full changelog](/changelog/2026-10-09-focus-themes-and-player-statistics/).

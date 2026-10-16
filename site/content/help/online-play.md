---
lastmod: 2026-10-15T00:00:00-05:00
title: "Playing online"
description: "Connect through FICS-Online to find an opponent, observe games, and use game controls."
category: "using-ironwood"
keywords: [online, FICS, lobby, guest, seek, challenge, connection, rated]
weight: 12
---

Ironwood connects to **FICS**, the Free Internet Chess Server, for online games. Open **Game → New game → Online → FICS** to reach the lobby. You can play as a guest or sign in with an existing FICS account. You can reopen it through **FICS-Online**, which has its own connection, account, available-game, observation, matchmaking, challenge, and console controls. **Lichess-Online** contains the separate Lichess controls. The FICS lobby lets you seek an unrated game, join an available game, or challenge a player's FICS handle. To play with a Lichess account, see [Play on Lichess](/help/play-on-lichess/).

FICS confirms online moves and supplies the official board and clocks. Keep your connection open while playing. If a move does not seem to register, wait for the server's response rather than assuming the board has advanced.

When an online game finishes, Ironwood keeps the moves for local review and saves the finished game on this device. An active online game is not in the Saved Games library until it ends. You can run [Stockfish analysis](/help/getting-started/) afterward.

Use a unique password for a FICS account. Guest play does not require you to create one.

## Console and chats in a second window

Open **FICS-Online → FICS console & chats**, then click the **↗** pop-out button beside Close in the console title bar. You can move this resizable window to another monitor. It has the same console, command suggestions, player chats, channel tabs, and copy controls as the main window. Commands and messages use the main window's existing FICS connection; keep that window open and connect or sign in there. Existing drafts are copied when the pop-out opens, and subsequent drafts are independent in each view.

To bring the pop-out forward, reopen the main console and click **↗** again. Closing the pop-out leaves your FICS connection running. If the browser blocks it, allow pop-ups for Ironwood.


## FICS account statistics

Beside **FICS · Connected**, Ironwood shows your handle and Lightning, Blitz, and Standard ratings. Hover over the summary to see each category’s wins, draws, losses, rated-game total, rating deviation (RD), and best rating when available. A dash means no rating is available. These records come from your own FICS `finger` rating table and refresh when you sign in and after your game finishes. Guest accounts show a **Guest** label because they have no permanent ratings or records. Disconnecting clears the displayed statistics.

## Keep a Lichess account session

You can sign in through **Lichess-Online** without disconnecting FICS or replacing its board. This supplies Opening Explorer access for later study. Tablebases work without a Lichess login. Only one game can control the board, and analysis tools remain unavailable during live play. See [Lichess account access alongside FICS](/help/play-on-lichess/#lichess-account-access-alongside-fics).

---
title: "Lichess play, with a place to return"
date: 2026-10-08T00:00:00-05:00
description: "Play through your Lichess account, keep correspondence games ongoing, and return to Ironwood's analysis workspace when you need it."
slug: "lichess-play-and-a-place-to-return"
---

Playing online and studying a game belong in the same workspace, but they need different controls. Ironwood now connects to your Lichess account so you can find opponents, play on the board you already use, and return to your saved games for review.

## Start with the lobby

The **Lichess-Online** menu opens a lobby with quick-pairing tiles, custom game setup, correspondence, friend challenges, and computer play. **FICS-Online** keeps its own server controls. Both menus use the Game menu's text size, with enough room to keep their items on one line.

You authorize Ironwood on Lichess without entering your password into Ironwood. The status bar gives a compact connection indicator, and the account link opens in a new tab. Opening Explorer shares this sign-in, removing its separate token field.

The lobby opens when you ask for it. Starting a local game or opening a saved game no longer brings it back just because your account is connected.

## Make the move feel immediate

A server-confirmed move can take time to reach the board. Ironwood now shows a translucent preview immediately after you move. Lichess's confirmation makes it solid; a rejection clears the preview. The official history and clocks still come from Lichess.

During play, the Game Analysis panel becomes **Game chat**. When the game ends, it returns to analysis and the finished game saves to your local library. Draw offers, takeback requests, aborting, resignation, and claims are available near the board and under Game, as well as in the lobby. Lichess decides which actions are permitted.

Connecting to the lobby leaves analysis available. Engaging an online game disables engine assistance until that game finishes or a correspondence game is put aside for local study.

## Keep correspondence in the background

Correspondence gives each player days to reply. Its deadline continues while the app is closed, so a game can remain active across several visits.

**Your ongoing games** shows the opponent, your side, whose turn it is, and game mode, with start-time and time-control details when available. Select a game to resume it. **Return to lobby · keep this game ongoing** puts correspondence aside without resigning or signing out.

You can also load a saved game while correspondence remains ongoing. Refreshing reconnects your account without automatically engaging an existing game or replacing the saved board. Ironwood displays one online game at a time; your other games remain on Lichess.

## Choose the computer game you want

The computer setup follows Lichess's arrangement: a position selector, time tabs, eight strength buttons, and large White, Random, and Black cards. Ironwood uses its own piece artwork and gold accents.

Choose Standard, a randomized Chess960 start, or From Position with a validated standard-chess FEN. Chess960 castling is handled throughout online replay. Custom Chess960 FEN is not part of the From Position option.

Unlimited removes the clock and move deadline. Real time uses minutes and increment. Correspondence sets days per move. These choices use Lichess's computer; local Stockfish play remains available through New game.

The [Lichess help guide](/help/play-on-lichess/) covers setup, rapid time controls, chat, game actions, and the difference between leaving online play and signing out.

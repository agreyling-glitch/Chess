---
title: "Lichess play and correspondence"
date: 2026-10-08T00:00:00-05:00
slug: "2026-10-08-lichess-play-and-correspondence"
version: "2026.10.08"
description: "Lichess account play, a redesigned lobby and computer setup, game chat, and correspondence switching that preserves local study."
---

## Added

- Lichess account authorization, rapid and classical quick pairing, custom matchmaking, friend challenges, incoming challenge responses, computer games, and resuming ongoing games.
- Immediate translucent previews for pending online moves, followed by server confirmation or rejection.
- Game chat in the Game Analysis panel during active Lichess play.
- Resign, abort, draw, takeback, and claim controls below the board, in the lobby, and beneath Resign in the Game menu, subject to Lichess availability.
- Lichess computer setup with Standard, Chess960, and From Position; validated standard-chess FEN input; Unlimited, Real time, and Correspondence tabs; strength buttons 1–8; and White, Random, and Black side cards.
- Chess960 support in online move replay and castling, including saved-game variant metadata.
- Richer ongoing-game entries with opponent, side, turn, mode, and available start-date and time-control details.
- Correspondence controls that keep a game ongoing while returning to the lobby, opening another game, or loading a saved game.

## Changed

- Redesigned the Lichess lobby with quick-pairing tiles, clearer setup controls, content-sized gold action buttons, and a consistent, taller window across tabs.
- Renamed the online menus to FICS-Online and Lichess-Online, separated their controls, matched Game menu font sizes, and widened menus to avoid wrapped items.
- Simplified connection information and replaced missing status glyphs with drawn indicators. Lichess account links open in a new tab.
- Opening Explorer now shares the Lichess sign-in session instead of requiring a separate token.
- The lobby opens manually instead of reopening on every new or saved game.
- Connecting to the lobby preserves local analysis. Engaging an online game disables engine assistance; completion restores full game analysis and saves the finished game locally.
- Refreshing reconnects the account without automatically engaging an existing correspondence game or overwriting a loaded saved game.
- Clarified Leave online play versus Sign out: leaving disconnects streams while keeping authorization; signing out revokes authorization.

## Help

- Updated Lichess play, FICS play, local game setup, saved games, analysis, and Opening Explorer help, including rapid time-control explanations and a standard middlegame FEN example.

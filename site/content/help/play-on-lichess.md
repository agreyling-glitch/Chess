---
title: "Play on Lichess"
description: "Use the Lichess lobby, set up computer games, resume correspondence games, and chat during play."
category: "using-ironwood"
keywords: [lichess, online, rapid, classical, matchmaking, challenge, correspondence, sign-in, chat, Chess960, FEN, computer, unlimited]
weight: 24
---

## Connect and open the lobby

Open **Lichess-Online → Lichess lobby and game controls…**, then choose **Sign in with Lichess**. You can also use **Game → New game → Online → Lichess**. Authorize Ironwood on Lichess, then return to Ironwood. Ironwood never asks for your Lichess password.

The lobby opens manually. Being connected does not make it reopen whenever you start a local game or load a saved game. Refreshing reconnects your account without automatically engaging an existing correspondence game. Open the lobby and select a game when you want to resume it.

The status bar shows whether Lichess is connected. The account link opens Lichess in a new browser tab. Your authorization stays in this browser session and is excluded from saved games and backups.

## Quick pairing and creating a game

**Quick pairing** offers time-control tiles. The first number is each player's starting time in minutes; the second is the seconds added to that player's clock after each move.

| Option | Time per player | Added after each move |
| --- | --- | --- |
| 10+0 | 10 minutes | None |
| 10+5 | 10 minutes | 5 seconds |
| 15+10 | 15 minutes | 10 seconds |
| 30+0 | 30 minutes | None |
| 30+20 | 30 minutes | 20 seconds |

The first three are rapid choices; the 30-minute choices are classical. Your clock runs only during your turn. Increment gives you extra time for each move, including in the endgame.

Choose **Custom** or **Create a game** to set minutes, increment, rated or casual play, and your preferred side. **Find opponent** starts matchmaking. Ironwood's Lichess Board API connection supports rapid and classical matchmaking; direct challenges can also use blitz. Bullet games must be played on Lichess.

**Challenge a friend** lets you enter a Lichess username and send a challenge. Direct challenges expire after 20 seconds if not accepted. Incoming challenges appear in the lobby with accept and decline controls.

**Cancel matchmaking** cancels an active real-time seek. Correspondence seeks remain on Lichess until someone joins them; closing Ironwood does not cancel them.

## Play against computer

Choose **Play against computer** in the lobby. The setup has three position choices:

- **Standard** starts from the usual chess position.
- **Chess960** starts with a legal randomized back rank. Ironwood supports Chess960 moves and castling.
- **From Position** starts standard chess from a position you supply. Paste a valid [FEN](/help/fen/) into the input that appears. The start button stays disabled until the position is valid. This option accepts standard-chess FEN, not a custom Chess960 FEN.

For a standard middlegame test, use:

```text
r1bq1rk1/ppp2ppp/2np1n2/2b1p3/2B1P3/2NP1N2/PPP2PPP/R1BQ1RK1 w - - 0 8
```

Choose a time mode:

- **Unlimited** has no move deadline or clock.
- **Real time** uses starting minutes and increment.
- **Correspondence** gives you a selected number of days for each move.

Choose **Strength** from 1 to 8, then **White**, **Random side**, or **Black**. Press the gold **Play against computer** button to start a casual game against the Lichess computer. Strength 1 is the easiest and 8 is the strongest. These games use Lichess's computer, separately from Ironwood's local Stockfish games.

## Correspondence and ongoing games

In the **Correspondence** tab, choose 1, 2, 3, 5, 7, 10, or 14 days per move, then Casual or Rated and **Find a correspondence opponent**. Colors are assigned automatically. The deadline keeps running while you are away; for example, two days allows up to 48 hours for each reply. Correspondence is also available against the computer through its setup screen.

**Your ongoing games** lists games still active on Lichess. Entries show the opponent, speed, rated or casual mode, your color, and whether it is your turn. Start date and time and time-control details appear when available from Lichess. Select an entry to resume it.

While playing correspondence, choose **Return to lobby · keep this game ongoing** to put that game aside without resigning or signing out. You can resume another game or create a new one. Ironwood displays one online game at a time, while Lichess keeps your other games ongoing.

**Game → Load saved game** also puts an active correspondence game aside and keeps your Lichess sign-in. The saved game remains on the board instead of being replaced by the correspondence game. Resume the online game manually from the lobby later. Loading a saved game during a live online game disconnects online play; it does not resign the game on Lichess, and its clock continues.

## Moves, chat, and game controls

Play on Ironwood's board, including promotion and castling. A move appears immediately as a translucent ghost while waiting for Lichess. It becomes solid when confirmed or disappears if rejected. This preview does not advance the official move history. Clocks and results come from Lichess. After a dropped connection, Ironwood reconnects and recovers the full move history.

During an active Lichess game, the **Game Analysis** panel becomes **Game chat** beside the board. Type a message and press Enter or Send. This is the game's player chat, separate from spectator chat. After the game ends, the panel returns to full game analysis.

Game controls appear below the board, in the lobby, and under **Game**, beneath Resign:

- **Resign…** asks for confirmation before giving up the game.
- **Abort game** ends a game when Lichess allows an abort.
- **Offer draw** sends a draw offer. When your opponent offers a draw, you can accept or decline it.
- **Request takeback** asks your opponent to undo a move. An incoming request can be accepted or declined.
- **Claim draw** asks Lichess to recognize a claim. A victory claim may appear when the opponent has left and Lichess permits it.

Lichess decides whether each action is allowed in the current position.

## Analysis and leaving online play

Connecting your account or browsing the lobby leaves local position analysis and full game analysis available. Engine assistance is disabled when an online game is engaged. When the game finishes, Ironwood saves it to the local library and restores review and analysis.

**Leave online play** disconnects the game streams and returns to the local workspace, while keeping your sign-in for reconnecting. It clears an unfinished online board locally. It does not resign or abort that game on Lichess.

**Sign out** revokes Ironwood's Lichess authorization. It is available when you are not actively playing. Use it when you want to remove the account connection.

Leaving the Play page or closing the app stops that page's live connection; it does not end your game on Lichess. Live clocks and correspondence deadlines continue. When you return, open the lobby to resume an ongoing game.

The [Lichess Board API documentation](https://lichess-org.github.io/api/#tag/Board) describes supported time controls and game actions.

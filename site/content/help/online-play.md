---
lastmod: 2026-10-30T00:00:00-05:00
title: "Playing online with FICS"
description: "Use the FICS window to connect, chat, find opponents, watch public games, and preview or review player history."
category: "using-ironwood"
keywords: [online, FICS, lobby, guest, seek, challenge, connection, rated, console, chat, history, finger, watch, observe, preview, review, remember handle]
weight: 12
---

Ironwood connects to **FICS**, the Free Internet Chess Server, for online games and spectating. Choose **Play → Online via FICS** or click **FICS** in the bottom status bar to open the **Free Internet Chess Server** window directly. An existing FICS session is reused. Its main tabs are **Console**, **Chat**, **Lobby**, **History**, **Watch**, and **Play**. The bottom **Lichess** button opens the separate [Lichess lobby](/help/play-on-lichess/).

Connecting as a guest or signing in preserves your current board, moves, analysis, and layout. FICS takes over the main board when a game starts or you open a watched game. During live online play, FICS supplies the authoritative moves and clocks and engine assistance is disabled. Finished games are saved on this device for later review and analysis.

## Connect and sign in

Open **Connection** in the FICS window:

- **Create Account…** opens FICS registration in your browser.
- **Sign in…** opens the FICS Account form for your handle and password.
- **Connect as guest** connects without a registered account.
- **Disconnect** closes the FICS connection.

Choose **Remember handle** to save your handle on this device and fill it in after reloads. Unchecking it removes the saved handle. Ironwood does not save the FICS password; it clears the password after sign-in. Use a password unique to FICS: the connection beyond Ironwood to FICS is unencrypted.

The bottom status bar separates the **FICS** button, connection state, and connected handle. The server window also shows connection status in its top row.

## Console: commands and replies

**Console** contains server messages, command suggestions, and a command field with **Send**. Use it to read notices, check a rejected command, or run FICS commands directly. Select text to copy server output.

For example, `finger PlayerName` asks for a player's profile, and `history PlayerName` lists recent games. Lobby and History provide buttons for these tasks, so typing commands is optional.

## Chat: conversations and channels

Choose **Chat** to see a second row of conversation tabs, including server contacts such as ROBOadmin and adminBOT. **+ New chat** opens a conversation by player name or joins a channel by number (0–255). Messages wrap to the conversation pane's width. Each conversation retains its own draft and unread count; the main Chat tab shows the combined unread count.

The active main tab is gold; the active conversation is sage green. Closing a conversation tab does not unsubscribe from its FICS channel. Chats last for the current session. FICS account and channel permissions determine where you can send messages; check Console for server replies if a guest message is refused.

## Lobby: available games and player information

**Lobby** displays available games in a sunken table with Name, Rating, Time, Game, Rated, and Player type columns. **Time** uses minutes + increment in seconds, such as `5+3`. **Refresh** requests a fresh list.

Each game has three controls:

- **Join** responds to that game's advertisement. FICS may start the game or ask its owner to accept your challenge.
- **Finger** opens a translucent, scrollable popup with that player's FICS profile output. Click outside to dismiss it.
- **History** opens the History tab and looks up that player.

Player names and type labels use matching colors. The gold **?** beside **Player type** opens a one-type-per-line legend. FICS markers include `(C)` computer, `(U)` unregistered guest, `(*)` administrator, `(B)` blindfold account, `(T)` team, `(TD)` tournament director or bot, `(CA)` chess advisor, `(SR)` service representative, and chess titles. Unrecognized markers remain visible. See the [FICS marker guide](https://www.freechess.org/Help/HelpFiles/who.html).

## History: recent games, previews, and reviews

In **History**, enter a handle in **History for**, then choose **Look up** or press Enter. The results table shows game number, result, side played, ratings, opponent, game type, time control, ECO, ending, and date. Win, loss, and draw results have distinct colors. The game-type and ending abbreviations come from FICS; see its [history guide](https://www.freechess.org/Help/HelpFiles/history.html). A player with no history gets a status message instead of an empty results table.

**ECO** is the Encyclopaedia of Chess Openings classification, such as A04 for the Réti Opening. It identifies the opening, not the quality of play.

- **Preview** opens the existing moves player for the archived game, keeping your main board and current game unchanged.
- **Review** loads the game onto the main board at its starting position for navigation and analysis. Finish active online play or training before reviewing another game. The FICS connection stays open.

Reviewing an archived game is temporary. Use **Play → Save current game** if you want it in your local library. Games can become unavailable as a player's server history changes or if access is restricted.

## Watch: public games

**Watch** displays running public games in a table with game number, White and Black players and ratings, game type, and time control. Choose **Refresh** to update the list.

**Watch** asks FICS to send a game's board and moves. **Watching · Open** selects a game you already watch on the main board. The first watched game opens automatically when you are not playing; subsequent games can be selected using the observation tabs above the board. Ironwood retains up to 10 watched games. **Stop watching all** removes the current observations.

FICS also accepts `follow PlayerName` in Console to observe that online player's current and future games automatically. Ironwood receives these through the observation path, but it does not have dedicated Follow controls or automatically switch the main board to every subsequent followed game. Retained games count toward the same ten-game limit. Use `follow` without a name to cancel following; stopping observations alone does not cancel FICS follow. [FICS follow guide](https://www.freechess.org/Help/HelpFiles/follow.html).

## Play: matchmaking and challenges

**Play** groups three activities into sunken sections:

1. **Find an opponent:** choose Minutes and Increment (seconds), then **Find unrated opponent**. Use **Cancel search** to withdraw your advertisement.
2. **Challenge a specific player:** enter a FICS handle and choose **Send challenge**. The challenge uses the displayed unrated time control from the matchmaking settings.
3. **Challenge requests:** choose **Accept** or **Decline** for a server challenge. Recent challenge notices appear here; Console contains the complete server replies.

Connect before using these controls, and finish your current game before starting another. While playing, the main board's game controls remain available for resignation and draw actions. Wait for FICS to confirm moves and game actions.

## Move the FICS window to another monitor

Click the raised **↗** button beside the **×** close button to open FICS in a separate browser window. It shares the main window's existing connection and offers the same tabs and account controls. Keep the main Ironwood window open: commands are sent through it, and full-board reviews load there. Existing chat drafts are copied when the pop-out opens; subsequent drafts are independent in each view.

To bring the pop-out forward, reopen the main FICS window and click **↗** again. Closing the pop-out leaves the connection running. Allow pop-ups for Ironwood if your browser blocks it.

## FICS account statistics

The status bar shows your connected handle. Hover it for available account statistics: Lightning, Blitz, and Standard records, rated-game totals, rating deviation, and best ratings when supplied. These come from your FICS `finger` rating table and refresh at sign-in and after a game finishes. Guests have no permanent ratings or records. Disconnecting clears the displayed account statistics.

## Keep a Lichess account session

Open the bottom **Lichess** button to access its lobby and sign-in. A Lichess account session supplies Opening Explorer access for later study; tablebases require no login. Only one game controls the board, and study tools remain unavailable during live online play. See [Lichess account access alongside FICS](/help/play-on-lichess/#lichess-account-access-alongside-fics).

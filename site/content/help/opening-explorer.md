---
lastmod: 2026-10-09T00:00:00-05:00
title: "Opening Explorer"
description: "Explore common opening moves and game results without changing your game."
category: "using-ironwood"
keywords: [opening, explorer, masters, Lichess, statistics, preview, ratings]
weight: 20
---

## Connect to Lichess

**Read about [ECO code] on ChessCodex** opens the matching opening category, such as **A04** for the Zukertort Opening. The link appears when Lichess supplies an ECO code and follows recognized openings as you explore. An ECO category may cover several variations.

Opening Explorer uses your existing Ironwood Lichess sign-in session. Sign in through **Lichess-Online**; no separate API token is needed. Signing out clears access and cached results. If your session is rejected, sign out and sign in again.

Opening Explorer is available for local review while your account is connected. Engine assistance and exploration are unavailable while an online game is engaged; finish the game or put a correspondence game aside before reviewing another game.

Open a game and select **Game Analysis → Opening Explorer** below the evaluation graph. Choose **Masters**, **Lichess games**, **My games**, or **Player**. Lichess games can be filtered by speed and rating group. **My games** uses your signed-in username; **Player** lets you enter another Lichess username and click **Apply filters**. Personal and player results can be filtered by the player’s color, speed, and an inclusive month range (`YYYY-MM`). Leave either month blank for an open boundary. Click **Apply filters** after editing a username or dates; color and speed apply immediately. **Reset** clears date and speed filters and selects White. An internet connection is required; statistics come from Lichess's public Opening Explorer service.

The table shows up to twelve common moves, game counts, and result bars: light for White wins, gray for draws, and dark for Black wins. Hover over a bar for counts and percentages. Results describe game outcomes, not engine evaluations or the best move.

Click a move to preview its continuation on the main board and explore further moves. **Back** undoes one preview move; **Back to game** returns to the selected game position. Your game moves stay intact. Selecting another game position resets the preview. The board is read-only during a preview; switch tabs or return to the game to resume normal board interaction.

No matches may mean a rare position or restrictive filters. Connection errors offer **Retry**; after a rate limit, wait one minute. Opening Explorer supports standard chess positions.

## Review your opening history

**My games** and **Player** show the selected player’s wins, draws, and losses at the displayed position, along with the existing White/draw/Black result bars for each continuation. Lichess indexes player games on demand; statistics update while indexing runs and may initially be incomplete. While waiting, Ironwood shows the indexing queue position when supplied and does not treat an empty initial response as a final zero result. Results are cached for the current sign-in session; **Refresh player statistics** fetches updated results once the current indexing request finishes.

Up to eight matching recent games appear below the moves. Choose **Import for analysis…** to fetch a completed game into Ironwood’s existing Import window. Review and import it there, then use local Stockfish analysis and notes as usual. Duplicate checking and the restriction against importing unfinished games still apply. Your current board stays intact until you choose to import the game.

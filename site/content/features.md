---
title: "Features"
lastmod: 2026-09-30T00:00:00-05:00
description: "Play and analyze chess in 2D or 3D, meet FICS opponents, import Lichess games, and keep your games on your device."
layout: "features"
url: "/features/"
---

## Play your way

Start a game as White or Black, or let Ironwood choose a side at random. When you play Black, Stockfish makes the first move and the board follows your perspective. Choose **You vs Engine** or **Play Both Sides** in a fixed-size New Game window. Engine settings keep Play, Realtime Analysis, and Full Game Analysis in separate tabs, with scrollable controls. Choose among five piece sets: System, Cburnett, Merida, Royal Rascals, and Undead Court. A dark board frame with gold coordinates and piece shadows are on by default; move animation and sounds add feedback for moves, captures, and checks.

## Choose a 2D or 3D board

Switch between 2D and 3D from the **View** menu or the button beside Flip Board. The 3D board sits inside the same game workspace, with a fixed perspective and scroll-wheel zoom. Choose Marble, Wood, or Glass from **View > Board > 3D theme**. Wood uses Omie's CC0 chess set. Adjust its appearance with one slider for brightness, contrast, and gloss. Coordinates, move highlights, best-move arrows, and a graphical promotion chooser remain available. Beneath each player's name, piece icons show their captures and a +score shows a material advantage; these follow the selected position as you review moves. **Copy Board to Clipboard** in Game Moves captures the selected position in the active 2D or 3D view.

## Meet opponents online

Connect to the Free Internet Chess Server (FICS) as a guest or sign in with an existing account. Browse available games, seek an unrated opponent with your chosen time and increment, challenge a specific player, and respond to incoming challenges. Live boards and clocks follow the server. Finished games are saved on this device for later review and analysis.

## Watch live games

Browse running FICS games and watch up to 10 games simultaneously. Spacious, scrollable game lists make it easier to compare players and time controls. Follow the moves and clocks and switch between observation tabs. Once a game finishes, run full-game analysis to review its moves and key moments.

## Keep conversations together

The FICS console opens automatically when you connect. Separate chat tabs keep player conversations and channels alongside the server console, with unread counts and a draft for each conversation. Start a chat by player name or channel number, and use the console for server commands and replies. Chats last for the current session; FICS account and channel permissions determine where you can send messages.

## Explore different starting positions

Play from the standard starting position, try a random Chess960 setup, or enter a custom FEN. The position editor lets you arrange pieces and review the side to move, castling rights, and other position details before starting. For Chess960 castling, move your king onto its rook.

## Bring a board from an image

Upload, paste, or drop a chessboard screenshot into the position editor. Recognition runs on your device and produces an editable position. Crop closely to the board for the best results, then check the pieces, orientation, and game-state details before using it. You can correct the board manually; screenshot recognition is a starting point for review.

## Analyze the whole game

Run Stockfish 19 with its full NNUE network directly in your browser. Review evaluations, best moves, principal variations, move quality, accuracy, and the turning point of a game. Pause and resume full-game analysis or open the evaluation graph in a full-screen view with move labels, mistake markers, position navigation, and detailed scores. Add a personal note to any move or the starting position. Notes stay separate from engine analysis and appear in the expanded graph and printed report. The Game Moves right-click menu also copies the selected position as FEN or a board image, or copies a partial or full PGN. Choose letter or figurine notation for the move list.

## Import games from Lichess

Bring your Lichess games into Ironwood for review and analysis. Enter a player's username to fetch their latest 20 completed games, or paste an individual game link. No Lichess login or API token is needed. Choose the games you want to keep; Ironwood skips duplicates and saves imported games in your local library. Open a game to explore its moves and run Stockfish analysis, or choose immediate analysis when importing a single game. See [Import games from Lichess](/help/import-from-lichess/) for the steps. Fetching games requires an internet connection.

## Build your library

Saved Games keeps My Games, Observed games, and Imported games together on this device, with an All Games view. Finished watched games save automatically with their source and analysis progress, so you can reopen them and resume partial analysis. Import a single PGN or choose games from a multi-game PGN collection; search by player, select the games you want, and skip duplicates. Search by player, source, or played date, filter by Not analyzed, Partial, or Complete analysis, mark favorites, and reopen a game with its saved analysis. Maximize the library or click a board preview for a larger view. Select individual games, a page, or all matching games for deletion or export. Export plain PGN, annotated PGN with notes, or Analysis JSON; multiple JSON files are bundled in a ZIP.

## Keep control of your data

Ironwood saves games and display preferences locally. Download a versioned JSON backup, then restore it by merging with your library or replacing existing games. Storage Information shows game counts and browser usage. Install the app and explicitly store the engine for offline play and analysis without an account. FICS play, spectating, and chat require an internet connection and send game activity and messages to the server; local engine analysis and screenshot processing run on your device.

## Device and browser compatibility

Ironwood is currently designed for desktop and laptop screens. Use a current browser and graphics device with WebGPU support and hardware acceleration enabled. WebGPU availability and 3D performance vary by browser, operating system, and graphics hardware. The full Stockfish 19 engine is a separate 94.5 MiB download; allow space for it if you enable offline play. We have not established a reliable minimum CPU, RAM, or GPU model, so those are not listed as requirements.

---
title: "Features"
lastmod: 2026-10-04T00:00:00-05:00
description: "Train against Stockfish with personal Elo profiles, type your moves, review games in 2D or 3D, and keep your chess data on your device."
layout: "features"
url: "/features/"
---

## Analyze without limits

**No analysis usage cap. No paywall. No subscription.** Analyze as many games and positions as you want with Stockfish 19 and its full NNUE network running directly in your browser. Choose your own search time, depth, or node budget, including unlimited search; speed and available resources depend on your device. Review evaluations, best moves, principal variations, move quality, accuracy, and the turning point of a game. Pause and resume full-game analysis or open the evaluation graph in a full-screen view with move labels, mistake markers, position navigation, and detailed scores. Use **Print analysis report** to open a printable report with player accuracy, move quality, game phases, critical-position diagrams, engine lines, and your written notes. Choose **Save as PDF** in your browser's Print dialog to keep or share the analysis.

## Train against an adaptive opponent

Create a local profile in **Training vs AI** and start at a 1320 training rating. Alternate White and Black games against Stockfish, whose target follows the rating for that color. The lower color rating determines overall progress. Track separate color rating histories, results, and clock losses. A progress dashboard shows analysis coverage and comparable average-loss bars for opening, middlegame, and endgame, with mistake counts and a practice suggestion based on the analyzed sample. Rename profiles without losing their ratings or history, maximize the dashboard, and continue with **Start next training game**. Training games have their own Saved Games category and profile filter; profiles and completed results are included in full backups. [How training ratings and focus areas work](/help/training-vs-ai/).

## Take notes and annotate positions

Record your plans, questions, and lessons with a personal note on any move or the starting position, without needing to run analysis. Read, add, edit, or delete the displayed position's note beside the live board in the Compact workspace. Notes follow the board as you step through moves, with markers in Game Moves and the evaluation graph. Long notes scroll within the panel. Select the **Starting position** row or **View starting note** to revisit the game's initial context. Notes also appear beside analysis details in the expanded graph and in the printed report.

Add your own move-quality symbols—`!`, `!!`, `!?`, `?!`, `?`, or `??`—and position assessments from **Add annotation…** in the move's right-click menu. Choose one symbol from each group, change the selection, or clear it. Your gold symbols stay separate from Stockfish's assessments, so you can explain your own reasoning in a note. See [Taking notes and adding annotations](/help/taking-notes/) for instructions.

## Illustrate plans on the board

Draw green, red, or yellow arrows and outline important squares on either the 2D or 3D board. Use **Draw on board** and its color controls, or Alt-click and Alt-drag shortcuts. Repeat a drawing to remove it, change its color, or use **Clear drawings** for the displayed position. Drawings belong to individual positions, including the starting position, and reappear when you return to them. See [Drawing arrows and highlighting squares](/help/drawing-on-the-board/) for shortcuts and touch controls.

Notes, manual symbols, and drawings remain attached to saved games and are preserved in annotated PGN and Analysis JSON. Annotated PGN uses standard numeric codes for symbols and arrow/square comment extensions for drawings; support for the drawing extensions varies between chess programs. Plain PGN omits personal annotations.

## Play your way

Start a game as White or Black, or let Ironwood choose a side at random. When you play Black, Stockfish makes the first move and the board follows your perspective. Choose **You vs Engine**, **Play Both Sides**, **Training vs AI**, or **Online** in the New Game window. Set a Stockfish target of 1320–3190 Elo directly in **You vs Engine** by enabling **Limit to Elo rating**. Engine settings keep Play, Realtime Analysis, and Full Game Analysis in separate tabs, with scrollable controls. Choose among five piece sets: System, Cburnett, Merida, Royal Rascals, and Undead Court. A dark board frame with gold coordinates and piece shadows are on by default; move animation and sounds add feedback for moves, captures, and checks.

## Type your moves

Start typing during your turn and a translucent overlay shows the move. Enter plays it, Backspace edits, and Escape cancels. Type `e4`, `nc3`, captures, castling, or promotions; lowercase piece letters and coordinate notation such as `e2e4` work too. Illegal moves stay visible for correction. Keyboard entry follows normal turn and clock rules and works in local, training, and online games. [Keyboard move input and shortcuts](/help/keyboard-input/).

The Compact workspace gives Game Moves the available vertical space, with larger top tabs for moving between the game and its analysis.

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


## Import games from Lichess

Bring your Lichess games into Ironwood for review and analysis. Enter a player's username to fetch their latest 20 completed games, or paste an individual game link. No Lichess login or API token is needed. Choose the games you want to keep; Ironwood skips duplicates and saves imported games in your local library. Open a game to explore its moves and run Stockfish analysis, or choose immediate analysis when importing a single game. See [Import games from Lichess](/help/import-from-lichess/) for the steps. Fetching games requires an internet connection.

## Import handwritten scoresheets

Turn a handwritten game into PGN with **Game → Import → Enter scoresheet**. Type White's and Black's moves beside a photo of the sheet while a board preview and validated move list follow your entries. Zoom and drag the photo to read the handwriting. Live validation catches illegal or missing moves, accepts lowercase piece letters, and adds check and checkmate notation automatically. Correct an earlier move without losing the continuation; gold board highlights and an automatically scrolling move list help you follow the game.

Moves, game details, and the photo save locally so you can stop and return to the unfinished sheet later. Enter the players, event, date, and result, then download PGN or pass the game into Ironwood's existing import flow for review and analysis. See [Import a game from a scoresheet](/help/import-a-scoresheet/) for instructions or [the gallery](/gallery/) for a preview.

## Build your library

Saved Games keeps My Games, Training games, Observed games, and Imported games together on this device, with an All Games view. Finished watched games save automatically with their source and analysis progress, so you can reopen them and resume partial analysis. Import a single PGN or choose games from a multi-game PGN collection; search by player, select the games you want, and skip duplicates. Search by player, source, or played date, filter by Not analyzed, Partial, or Complete analysis, mark favorites, and reopen a game with its saved analysis. Games with a Starting note show a two-line preview; click the note to open the initial position. Maximize the library or click a board preview for a larger view with the full Starting note. Browse with First, Previous, Next, and Last controls or the Left and Right arrow keys. Auto play advances every 0.5 seconds and pauses when you navigate manually. After deleting a game, return to the library with the current filters and window state preserved. Select individual games, a page, or all matching games for deletion or export. Export plain PGN, annotated PGN with notes, or Analysis JSON; multiple JSON files are bundled in a ZIP.

## Keep control of your data

Ironwood saves games and display preferences locally. Download a versioned JSON backup, then restore it by merging with your library or replacing existing games. Storage Information shows game counts and browser usage. Install the app and explicitly store the engine for offline play and analysis without an account. FICS play, spectating, and chat require an internet connection and send game activity and messages to the server; local engine analysis and screenshot processing run on your device.

## Device and browser compatibility

Ironwood is currently designed for desktop and laptop screens. Use a current browser and graphics device with WebGPU support and hardware acceleration enabled. WebGPU availability and 3D performance vary by browser, operating system, and graphics hardware. The full Stockfish 19 engine is a separate 94.5 MiB download; allow space for it if you enable offline play. We have not established a reliable minimum CPU, RAM, or GPU model, so those are not listed as requirements.

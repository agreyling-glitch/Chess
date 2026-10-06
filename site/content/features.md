---
title: "Features"
lastmod: 2026-10-05T00:00:00-05:00
description: "Analyze with Stockfish, explore pawn structure, piece mobility, king safety, and tactical patterns, and play in 2D or 3D."
layout: "features"
url: "/features/"
---

## Analyze without limits

**No analysis usage cap. No paywall. No subscription.** Analyze as many games and positions as you want with Stockfish 19 and its full NNUE network running directly in your browser. Choose your own search time, depth, or node budget, including unlimited search; speed and available resources depend on your device. Review evaluations, best moves, principal variations, move quality, accuracy, and the turning point of a game. Pause and resume full-game analysis or open the evaluation graph in a full-screen view with move labels, mistake markers, position navigation, and detailed scores. Colored bands on the expanded graph highlight opening, middlegame, and endgame, with phase labels and a legend matching the analysis summaries. Use **Print analysis** beside **Expand** to open a printable report with player accuracy, move quality, game phases, critical-position diagrams, engine lines, and your written notes. Choose **Save as PDF** in your browser's Print dialog to keep or share the analysis.

## Explore opening choices

Connect using a Lichess API token with no permissions selected. Opening Explorer keeps the token only for the current app session and sends it directly to Lichess; reloading or disconnecting clears it.

Explore common continuations in **Game Analysis → Opening Explorer**, with Masters or Lichess game statistics, speed and rating filters, and White/draw/Black result bars. Preview moves on the main board without changing your game, then return with **Back to game**. Requires an internet connection and supports standard chess. [Opening Explorer](/help/opening-explorer/).

## Understand pawn structure

Open **Pawn Structure** below the Game Analysis graph to highlight chains, isolated pawns, and passed pawns. A pawn diagram sits beside separate White and Black counts and square lists. Follow pawn changes in an aligned evolution table with ten visible rows and click a row to inspect its position. These insights work without waiting for engine analysis. [Using Pawn Structure](/help/positional-insights/#pawn-structure).

## Compare piece mobility

Open **Piece Mobility**, select a piece, and optionally compare a second. Click a piece on the mini board or full board to select it; Shift-click to compare it. Gold and blue graph lines show available destinations through the game, with clickable position navigation and destination highlights on the mini board. Pieces retain their identity through moves, castling, and promotion; a captured piece’s line ends. Mobility respects pins and king safety, excludes castling and en passant, and measures either side independently of the move turn. [Using Piece Mobility](/help/positional-insights/#piece-mobility).

## Inspect king safety

Open **King Safety** for separate White and Black assessments with explanations of pawn shelter, nearby pawn-open files, and enemy pressure. Choose a king to highlight its shelter and attacked neighborhood on the diagram, then use the ten-row safety timeline to explore changes. Low-material endgames allow for useful king activity instead of penalizing missing shelter alone. These structural assessments complement the engine evaluation. [Using King Safety](/help/positional-insights/#king-safety).

## Train against an adaptive opponent

Create a local profile in **Training vs AI** and start at a 1320 training rating. Alternate White and Black games against Stockfish, whose target follows the rating for that color. The lower color rating determines overall progress. Track separate color rating histories, results, and clock losses. A progress dashboard shows analysis coverage and comparable average-loss bars for opening, middlegame, and endgame, with mistake counts and a practice suggestion based on the analyzed sample. Rename profiles without losing their ratings or history, maximize the dashboard, and continue with **Start next training game**. Training games have their own Saved Games category and profile filter; profiles and completed results are included in full backups. [How training ratings and focus areas work](/help/training-vs-ai/).

## Take notes and annotate positions

Record your plans, questions, and lessons with a personal note on any move or the starting position, without running analysis. Open the translucent **Notes** window from the document icon above the board. The **Note** tab follows the displayed position; **Start** shows the starting note while you keep the board on the current move. The title identifies the move in notation and plain language, such as **Notes: Nxb5 (Knight to B5)**.

Double-click the note text to edit, or double-click **No note for this position.** to add a note. You can also double-click a move in **Game Moves** to open its note editor directly. Expand **All notes** to browse every note in the game; click an entry to view it or double-click to edit. Drag the window by its title and resize it to suit your workspace. Notes also appear beside analysis details in the expanded graph and in printed reports.

Add your own move-quality symbols—`!`, `!!`, `!?`, `?!`, `?`, or `??`—and position assessments from **Add annotation…** in the move's right-click menu. Choose one symbol from each group, change the selection, or clear it. Your gold symbols stay separate from Stockfish's assessments, so you can explain your own reasoning in a note. See [Taking notes and adding annotations](/help/taking-notes/) for instructions.

## Illustrate plans on the board

Open **Draw on board** with the pencil icon above the board, to the left of Notes. Its translucent floating window keeps drawing tools out of Game Moves. Choose green, red, yellow, or blue, then mark squares or circles with solid or dotted outlines, or draw solid, dashed, and curved arrows on either board view.

While holding left-click to draw an arrow, tap right-click to switch between solid and dashed lines or reverse a curve’s direction. Expand **Drawings** to change an individual drawing’s color or starting and ending squares, or delete it. Use **Clear** to remove the position’s drawings, and the undo icon or **Ctrl+Z** to undo additions, edits, deletions, or Clear. Drawings belong to individual positions and reappear when you return to them.

The header’s slide control enables or disables drawing; closing the window returns to playing moves. Drag the title to reposition the window and resize it vertically to give the scrolling list more room. Alt-click and Alt-drag shortcuts remain available. See [Drawing arrows and highlighting squares](/help/drawing-on-the-board/) for the full controls.

Notes, manual symbols, and drawings stay attached to saved games and are preserved in annotated PGN and Analysis JSON. Ironwood preserves drawing styles with its own PGN extension; other chess programs may display simpler arrows or square outlines. **Copy Annotated Board to Clipboard** captures personal drawings and their styles in the active 2D or 3D view. Plain PGN omits personal annotations.

## Explore tactical patterns

Open **Tactical Map** from the bullseye beside the **2D/3D** button. Its translucent floating window lists newly exposed pieces, absolute pins, and legal fork candidates, with color-coded **Danger**, **Pins**, and **Forks** filters and counts. Select a finding to highlight its squares and trace attack lines on the board; select it again to clear the overlay. Drag its title to reposition the map, or resize the window alongside your notes and drawing tools.

The map provides geometric clues without requiring an engine analysis run. A candidate is not a proven material win: use Stockfish to check the opponent’s replies. Tactical Map is available for review and analysis and is hidden during rated training games, active online play, best-move exercises, and prediction previews. See [Tactical Map](/help/tactical-map/) for its scope and controls.

## Play your way

Start a game as White or Black, or let Ironwood choose a side at random. When you play Black, Stockfish makes the first move and the board follows your perspective. Choose **You vs Engine**, **Play Both Sides**, **Training vs AI**, or **Online** in the New Game window. Set a Stockfish target of 1320–3190 Elo directly in **You vs Engine** by enabling **Limit to Elo rating**. Engine settings keep Play, Realtime Analysis, and Full Game Analysis in separate tabs, with scrollable controls. Choose among five piece sets: System, Cburnett, Merida, Royal Rascals, and Undead Court. A dark board frame with gold coordinates and piece shadows are on by default; move animation and sounds add feedback for moves, captures, and checks.

## Type your moves

Start typing during your turn and a translucent overlay shows the move. Enter plays it, Backspace edits, and Escape cancels. Type `e4`, `nc3`, captures, castling, or promotions; lowercase piece letters and coordinate notation such as `e2e4` work too. Illegal moves stay visible for correction. Keyboard entry follows normal turn and clock rules and works in local, training, and online games. [Keyboard move input and shortcuts](/help/keyboard-input/).

The workspace places **Game Moves** to the left of the live board, **Real-Time Analysis** immediately to its right, and **Game Analysis** on the far right. Resize the side panels to suit your display. The White/Black balance bar follows the board height, and engine predictions use a compact scrolling table with separate White and Black columns.

## Choose a 2D or 3D board

Switch between 2D and 3D from the **View** menu or the button beside Flip Board. The 3D board sits inside the same game workspace, with right-button drag rotation, right-button double-click to reset the view, and scroll-wheel zoom. Open **View → Board → Theme…** for a full-size chooser with previews of all five 2D piece sets and the Marble, Wood, Glass, Art Deco, and Egyptian 3D themes. Click a card to apply it and switch board views. The 3D appearance and radial-light controls remain under Board. Wood uses Omie's CC0 chess set. Adjust its appearance with one slider for brightness, contrast, and gloss. Coordinates, move highlights, best-move arrows, and a graphical promotion chooser remain available. Beneath each player's name, piece icons show their captures and a +score shows a material advantage; these follow the selected position as you review moves. **Copy Board to Clipboard** in Game Moves captures the selected position in the active 2D or 3D view.

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


## Choose how to import your games

Filter Lichess and Chess.com username imports by time control and an optional date range, and request 20, 50, 100, or 200 matching games. Newest matches come first. Dates use UTC with an inclusive end date, and either boundary can be left open. Reset restores 20 games, all time controls, and no dates. Individual Lichess game links fetch one game independently of the filters. [Import filters](/help/import-games/).

Open **Game → Import…** for four tabs: **Paste a game or collection**, **Lichess**, **Chess.com**, and **Scoresheet**. The window stays the same size across tabs, with large gold-accented tabs and scrollable contents. Paste PGN or Analysis JSON, fetch online games, or enter a handwritten scoresheet. Review collections and choose which games to keep; duplicate games are skipped. Start full-game analysis when ready, or restore saved results immediately with Analysis JSON. [Import Games guide](/help/import-games/).

The online import panels provide clear username fields, gold **Fetch games** buttons, and status messages. Fetched games open in the Paste tab; choose **Continue** to review and import them. The Scoresheet tab provides a three-step guide and an **Enter scoresheet** button to open the entry workspace.

## Import games from Lichess

Bring your Lichess games into Ironwood for review and analysis. Enter a player's username to fetch up to the chosen number of matching completed games (20 by default), or paste an individual game link. No Lichess login or API token is needed. Choose the games you want to keep; Ironwood skips duplicates and saves imported games in your local library. Open a game to explore its moves and start Stockfish analysis from Game Analysis. See [Import games from Lichess](/help/import-from-lichess/) for the steps. Fetching games requires an internet connection.

## Import games from Chess.com

Select **Chess.com** in the tabbed Import window and enter a username to fetch up to the chosen number of completed standard chess or Chess960 games (20 by default). Ironwood reads recent monthly archives sequentially, then lets you choose games to save through its existing duplicate-aware import workflow. No account connection or API token is needed. Chess.com’s cached archives may take time to include a just-finished game. The same window provides **Paste a game or collection**, **Lichess**, and **Scoresheet** tabs. [Import games from Chess.com](/help/import-from-chess-com/).

## Import handwritten scoresheets

Turn a handwritten game into PGN with **Game → Import… → Scoresheet → Enter scoresheet**. Type White's and Black's moves beside a photo of the sheet while a board preview and validated move list follow your entries. Zoom and drag the photo to read the handwriting. Live validation catches illegal or missing moves, accepts lowercase piece letters, and adds check and checkmate notation automatically. Correct an earlier move without losing the continuation; gold board highlights and an automatically scrolling move list help you follow the game.

Moves, game details, and the photo save locally so you can stop and return to the unfinished sheet later. Enter the players, event, date, and result, then download PGN or pass the game into Ironwood's existing import flow for review and analysis. See [Import a game from a scoresheet](/help/import-a-scoresheet/) for instructions or [the gallery](/gallery/) for a preview.

## Build your library

Saved Games keeps My Games, Training games, Observed games, and Imported games together on this device, with an All Games view. Finished watched games save automatically with their source and analysis progress, so you can reopen them and resume partial analysis. Import a single PGN or choose games from a multi-game PGN collection; search by player, select the games you want, and skip duplicates. Search by player, source, or played date, filter by Not analyzed, Partial, or Complete analysis, mark favorites, and reopen a game with its saved analysis. Games with a Starting note show a two-line preview; click the note to open the initial position. Maximize the library or click a board preview for a larger view with the full Starting note. Browse with First, Previous, Next, and Last controls or the Left and Right arrow keys. Auto play advances every 0.5 seconds and pauses when you navigate manually. After deleting a game, return to the library with the current filters and window state preserved. Select individual games, a page, or all matching games for deletion or export. Export plain PGN, annotated PGN with notes, or Analysis JSON; multiple JSON files are bundled in a ZIP.

## Keep control of your data

Ironwood saves games and display preferences locally. Download a versioned JSON backup, then restore it by merging with your library or replacing existing games. Storage Information shows game counts and browser usage. Install the app and explicitly store the engine for offline play and analysis without an account. FICS play, spectating, and chat require an internet connection and send game activity and messages to the server; local engine analysis and screenshot processing run on your device.

## Device and browser compatibility

Phones and tablets can browse the website, but Ironwood blocks app startup, installation, and offline engine setup on mobile devices. Open the app on a desktop or laptop computer.

Ironwood is currently designed for desktop and laptop screens. Use a current browser and graphics device with WebGPU support and hardware acceleration enabled. WebGPU availability and 3D performance vary by browser, operating system, and graphics hardware. The full Stockfish 19 engine is a separate 94.5 MiB download; allow space for it if you enable offline play. We have not established a reliable minimum CPU, RAM, or GPU model, so those are not listed as requirements.

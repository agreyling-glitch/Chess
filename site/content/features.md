---
seo_title: "Chess analysis, online play, and board themes — Ironwood Chess"
title: "Features"
lastmod: 2026-10-27T00:00:00-05:00
description: "Play and analyze in your browser, or add desktop engines and a local archive with Ironwood Companion for Windows. Explore online play, puzzles, and study tools."
layout: "features"
url: "/features/"
---

## Play your way

Start a game as White or Black, or let Ironwood choose a side at random. When you play Black, Stockfish makes the first move and the board follows your perspective. Choose **You vs Engine**, **Play Both Sides**, **Training vs AI**, or **Online** in the New Game window. Set a Stockfish target of 1320–3190 Elo directly in **You vs Engine** by enabling **Limit to Elo rating**. Engine settings keep Play, Realtime Analysis, and Full Game Analysis in separate tabs, with scrollable controls. Choose among nine flat-board themes: System, Cburnett, Merida, Royal Rascals, Undead Court, Classic Staunton, Neon Geometric, Art Deco Faceted, and Hybrid Marble. The three newest themes have coordinated board palettes. Board frames and piece shadows are on by default; coordinate colors follow the selected theme. Move animation and sounds add feedback for moves, captures, and checks.

## Keep the board beside your study tools

The live board stays on the left, with **Moves · Prediction · Analysis · Position · Explore · Tactical · Annotations** on the right. Analysis contains engine results; Position contains pawn structure, piece mobility, and king safety. Explore groups **Openings** and **Endgames**. Tactical provides **Insights**, **Activity**, and **Patterns**. Annotations keeps **Drawings**, **Notes**, and **Chess Symbols** together in sunken panels with gray subtabs.

Every tab after Moves identifies the selected position and move. The Game Moves panel fits its contents, while the board follows the available space when the browser is resized. Switching between 2D and 3D keeps the same live-board size. [Workspace guide](/help/getting-started/#find-your-way-around).

## Analyze without limits

**No analysis usage cap. No paywall. No subscription.** Analyze as many games and positions as you want with Stockfish 19 and its full NNUE network running directly in your browser. Choose your own search time, depth, or node budget, including unlimited search; speed and available resources depend on your device. Review evaluations, best moves, principal variations, move quality, accuracy, and the turning point of a game. Pause and resume full-game analysis or open the evaluation graph in a full-screen view with move labels, mistake markers, position navigation, and detailed scores. Colored bands on the expanded graph highlight opening, middlegame, and endgame, with phase labels and a legend matching the analysis summaries.

Saved analyses remain separate runs. Below the Analyze controls, select a run from **Saved Analysis** or click its numbered gray button. Press **1–9** to select the corresponding run in Analysis or the expanded graph. The dropdown, highlighted button, and expanded graph engine name follow the selected run. Hover a numbered button for its recorded engine, budget, verification status, and engine options. [Compare saved analyses](/help/saved-analysis/). Replay the engine continuation directly in Move Analysis with a mini player and a rounded outline around the current move. **Try the best move** appears when the engine recommends a different move.

For full-game analysis, choose a **node budget** or **1–60 seconds per position**. Node mode keeps Quick, Standard, Deep and Custom quality choices. Turn **Verify critical moves after analysis** on or off; after the first pass, **Verify critical moves** lets you request the deeper check manually. Choose the engine beside the Analyze controls; the bottom-bar **Engine** button opens its settings. [Analysis budgets and settings](/help/search-depth-and-settings/).

{{< companion-features >}}

## Inspect the full game graph

In **Analysis**, use **Expand** inside the graph’s upper-right corner for a full-screen evaluation graph with phase bands, move-quality markers, written-note markers, and a framed position preview. Hover to inspect a position, click or drag to select it, or navigate with Previous, Next, the Position slider, and the mouse wheel. Scroll down advances a move; scroll up goes back.

Two move-aligned rows identify captured white and black pieces. Large **Merida** icons keep captures consistent across board themes; hover for the capture move. The preview follows board orientation and displays the inspected position’s saved drawings and the main board’s best-move arrows when applicable. Notes can be read and edited alongside move details. Separate sunken sections give the graph, details, notes, and board room to breathe. [Expanded graph guide](/help/expanded-analysis-graph/).

## Compare engine lines on mini boards

Click the gray raised **Expand** button beneath Engine prediction in the **Prediction** tab to see one board after each predicted move. The title identifies the variation, evaluation, first move, and depth. Open multiple independent prediction snapshots to compare lines; resize or maximize each window and click a board to inspect its position. Each board has a sunken frame and coordinates on all four sides. Boards scale with the available space inside a shared sunken paper-grain panel, with scrolling when needed to keep pieces readable. The full-game analysis details use matching recessed sections. [Engine prediction help](/help/engine-line/#expand-the-prediction-into-boards).

## Explore Lichess player profiles

Open your account or any username from **Lichess lobby → Player profile**. Browse biography links, ratings, game record, rating graphs by time control, and recent games. Follow or unfollow players and open profiles from your following list. Compare players in separate gold-bordered windows, each with independent graph filters and a sunken paper-grain chart. The raised **↗** title-bar button opens a profile in its own browser window for another monitor, sharing the main window’s Lichess connection; full-board reviews return to the main board.

Click a time-control rating chip for detailed performance: peak and lowest ratings, rank and percentile, opponent averages, record, streaks, and playing time. Best wins and worst losses show up to five games each with the same enlarged playback and full-board review controls as recent games. Statistics are cached separately by player window and time control, with refresh controls for retrying loads. The Puzzles chip does not open performance details.

Expand or collapse **Recent games** to show the latest 12 completed games across time controls. Hover a mini board for its gold outline and magnifying-glass indicator, then click for enlarged playback with player colors and outcome, or choose **Review on Ironwood** for the full board. Reviews stay temporary until **Save current game** adds the complete game to your library. [Player profile help](/help/lichess-player-profiles/).

## Follow player activity

Expand **Activity** in a Lichess profile to load recent daily results, grouped by date and time control. See wins, losses, draws, before/after ratings when available, and puzzle results supplied by Lichess. Activity loads on demand, with its own refresh control and per-window cache. Expand **Following** to search your followed players and open their profiles directly; Follow and Unfollow update your list on Lichess after authorization. Public biography and website links, FIDE links, and original game links open in new tabs. [Profiles and activity](/help/lichess-player-profiles/).

## Train against an adaptive opponent

Create a local profile in **Training vs AI** and choose its **Starting target Elo** from 1320–3190; 1320 remains the default. Both color ratings begin at your chosen value. Alternate White and Black games against Stockfish, whose target follows the rating for that color. The lower color rating determines overall progress. Track separate color rating histories, results, and clock losses. A progress dashboard shows analysis coverage and comparable average-loss bars for opening, middlegame, and endgame, with mistake counts and a practice suggestion based on the analyzed sample. Rename profiles without losing their ratings or history, move, resize or maximize the dashboard, restore its previous size and position, or pop it out onto another monitor, and continue with **Start next training game**. Training games have their own Saved Games category and profile filter; profiles and completed results are included in full backups. [How training ratings and focus areas work](/help/training-vs-ai/).

## Practice with puzzles

Open **Game → Puzzles…** for the Lichess daily puzzle or practice by theme and difficulty, with automatic opponent replies, promotion choices, hints, and solution reveal. Puzzles load into the main board, and attempts with played moves are recorded under Saved Games → Puzzles. Save any game you want to keep before switching. Review missed puzzles and track results locally without signing in or submitting a Lichess rating. New puzzles require internet access; saved mistakes can be retried offline. [Puzzles help](/help/puzzles/).

## Practice a recommended game phase

Choose Opening, Middlegame, or Endgames in puzzle practice. When your training dashboard recommends a phase, **Practice this phase** opens a matching puzzle at your chosen difficulty. Difficulty uses the anonymous Lichess baseline of 1500, independently of your training rating. Puzzles use the main board and its selected appearance. [Training practice](/help/training-vs-ai/#finding-areas-to-focus-on).

## Replay puzzle attempts without losing progress

Puzzle attempts with played moves appear under **Puzzles** in Saved Games, using the same larger playback viewer as ordinary games. Playback records are separate from the results used by Puzzle Progress and Review mistakes: deleting an attempt’s saved replay does not reset its progress count. Completed training results similarly remain in the training profile when their replay game is deleted. [Puzzles](/help/puzzles/) · [Saved Games](/help/saved-games-and-backups/).

## Explore opening choices

Opening Explorer uses your existing Lichess sign-in session in Ironwood. Sign in through the bottom Lichess button; no separate API token is needed. Signing out clears access and cached results.

Explore common continuations in **Explore → Openings**, with Masters or Lichess game statistics, speed and rating filters, and White/draw/Black result bars. Choose **My games** or **Player** for personal opening history, filter by player color, speed, and month range, see the player’s wins, draws, and losses at the displayed position, and import matching recent completed games for local analysis. Lichess indexes player games on demand; Ironwood shows indexing progress and lets you refresh the statistics afterward. Preview moves on the main board without changing your game, then return with **Back to game**. Requires an internet connection and supports standard chess. [Opening Explorer](/help/opening-explorer/).

## Explore endgames with Syzygy

Open **Explore → Endgames** to look up standard endgames with seven pieces or fewer, including both kings, and no castling rights. Results come from Lichess's tablebase service and require internet access, but no sign-in or local tablebase download. Read win, draw, and loss outcomes, including draws under the 50-move rule, alongside per-move **DTZ** and **DTM** badges when available.

Hover a continuation to show its arrow on the live board; click to explore the resulting position and replies. An exploration tree keeps alternate branches available as you compare them. Back, Reset preview, and Clear give you control over the study without changing recorded game moves. Move labels follow your Letters or Figurines preference. Inspect the root's 50-move clock, refresh cached results, or open the selected position on Lichess. [Syzygy endgame tablebases](/help/syzygy-tablebases/) explains the outcomes, distance counts, and preview controls.

## Understand pawn structure

Open **Pawn Structure** below the graph in **Analysis** to highlight chains, isolated pawns, and passed pawns. A pawn diagram sits beside, or in narrow panes above, compact White and Black panels with counts and square lists. Follow pawn changes in an aligned evolution table with ten visible rows and click a row to inspect its position. These insights work without waiting for engine analysis. [Using Pawn Structure](/help/positional-insights/#pawn-structure).

## Compare piece mobility

Open **Piece Mobility**, select a piece, and optionally compare a second. Click a piece on the mini board or full board to select it; Shift-click to compare it. Gold and blue graph lines show available destinations through the game, with clickable position navigation and destination highlights on the mini board. Pieces retain their identity through moves, castling, and promotion; a captured piece’s line ends. Mobility respects pins and king safety, excludes castling and en passant, and measures either side independently of the move turn. [Using Piece Mobility](/help/positional-insights/#piece-mobility).

## Inspect king safety

Open **King Safety** for compact White and Black assessments with explanations of pawn shelter, nearby pawn-open files, and enemy pressure. Choose a king to highlight its shelter and attacked neighborhood on the diagram, then use the ten-row safety timeline to explore changes. Low-material endgames allow for useful king activity instead of penalizing missing shelter alone. These structural assessments complement the engine evaluation. [Using King Safety](/help/positional-insights/#king-safety).

## Explore tactics and positional plans

Open **Tactical** to explore the position directly on the board, with gray Insights, Activity, and Patterns subtabs inside a sunken panel. Its tools work without an engine analysis run:

- **Opponent threats:** inspect candidate checks, captures, and forks for the other side if the position stays unchanged. Click a row to trace the move in pink and its checking or fork attacks in gold. Candidates update with the position; resolve check before exploring a hypothetical opponent turn.
- **What changed?:** step through a game to see opened attack lines in blue, lost defenders with gold dashed lines from their previous squares, and new attacks in red. Explanations connect each change to the affected pieces.
- **Positional activity:** click any real piece, of either color, for a 0–100 score rated **Restricted**, **Balanced**, **Active**, or **Dominant**. Concrete reasons describe mobility, central control, pressure, support, pins, and piece-specific roles. During a live game against the AI, activity follows your selection while normal moves still work.
- **Knight reach · two hops:** select either side's knight to preview first-hop ghosts, potential second-hop squares, capture rings, and major-piece fork opportunities. Hover a ghost to focus its branch. In review, click a ghost to lock its route; click again or use **Unlock route** to release it. Live-game clicks continue to select and move pieces.
- **Pin-aware Destination safety:** hover a knight destination to distinguish geometric support in gray, legal recapturers in green, unavailable defenders in amber, and geometric attackers in red. Capture-specific details account for pins, king safety, captures, and newly opened lines.
- **Attack vs Defense:** red bubbles count geometric attackers; green bubbles show **geometric defenders / legal recapturers**. The built-in legend explains `2/-` (no legal enemy capture), `2/0` (no legal recapturer), and `2/1` (one legal recapturer). Hover a piece for details against each candidate capture. Bubbles remain above board markings and arrows so their numbers stay readable.
- **Patterns:** filter **Danger**, **Pins**, and **Forks** using the colored buttons directly above their cards. Click a card to trace its relevant squares and attack lines; click again to clear it.

Geometric counts include pinned pieces; legal recapture counts check the position after each candidate capture. Ghost annotations do not affect real-piece counts. Activity ratings are positional heuristics, and hypothetical routes assume other pieces remain stationary. None of these tools proves a winning exchange: use Stockfish to check replies. The map requires no engine analysis run and is hidden during rated training games, active online play, best-move exercises, and prediction previews. See [Tactical Map](/help/tactical-map/) for all controls and legends.

## Take notes and annotate positions

Open **Annotations → Notes** to record a plan, question, or lesson for the selected move or starting position. Add and edit notes inside the panel with **Save note**, **Cancel**, and **Delete note**. Double-clicking a move in Game Moves, or choosing Add note from its right-click menu, opens the same editor. **All notes** browses the game's written annotations.

Use **Annotations → Chess Symbols** for common move-quality and position assessments. Expand **More symbols** to search descriptions or numeric codes. All valid PGN NAG values ($0–$255) are preserved in annotated interchange; the original standard set has descriptions, and other codes keep a numeric fallback rather than an assumed program-specific meaning. The right-click annotation menu keeps common choices close at hand. Symbols remain your assessment, separate from engine classifications. [Notes and symbols](/help/taking-notes/).

## Illustrate plans on the board

Open **Annotations → Drawings** to use the drawing tools beside the board. The drawing switch enables or disables board input; leaving Drawings turns drawing input off. Choose **green, red, yellow, blue, black, or white**. Mark squares with solid or dotted outlines, circles, translucent filled squares, or crosses. Filled squares keep pieces visible; crosses can mark targets or squares to avoid.

Draw solid, dashed, or curved arrows, or choose an **L-shaped knight arrow** with a solid or dashed line. Knight arrows follow the longer leg first and keep their bend aligned with the board in 3D. While holding left-click to draw, tap right-click to switch solid/dashed straight or knight arrows, or reverse a curve’s direction.

Add **ghost pieces** to illustrate possible placements without changing the real position or engine analysis. Choose a White or Black piece symbol, then click a square to place a translucent ghost in the selected color. Repeat the same piece and color on that square to remove it. Ghosts work in both board views; in 3D they remain flat symbols over the square.

Expand **Drawings** to edit a drawing’s color and squares or delete it. **Clear**, the undo icon, and **Ctrl+Z** cover additions, replacements, edits, deletions, and clearing. Each position keeps its own drawings. The panel’s slide control enables or disables drawing; leaving Drawings turns drawing input off. Alt-click and Alt-drag shortcuts remain available. [Drawing arrows and highlighting squares](/help/drawing-on-the-board/).

Notes, manual symbols, and drawings stay attached to saved games and are preserved in annotated PGN and Analysis JSON. Ironwood’s PGN extensions preserve shape and arrow styles and colored ghost pieces; other chess programs may simplify styles or ignore ghosts. **Copy Annotated Board to Clipboard** captures the selected position’s drawings and any displayed Stockfish recommendation arrow in the active 2D or 3D view. Printed analysis also includes personal annotations. Plain PGN omits them.

## Print and share a complete review

In **Analysis**, use **Print analysis** beside the **Saved Analysis** heading to open a report with player accuracy, move quality, game phases, critical-position diagrams, and engine lines. **Your notes and drawings** brings your written notes, manual annotation symbols, and board drawings into the same report. Positions with arrows, shapes, or ghost pieces receive annotated diagrams even when they are outside the engine’s critical-position list. Starting-position annotations are included too.

Pause or stop full-game analysis before printing. Choose **Save as PDF** in your browser’s Print dialog to keep or share the report. [Saving and sharing analysis](/help/saving-and-sharing/).

## Build your library

Saved Games keeps **My Games**, **Training**, **Puzzles**, **Observed**, and **Imported Games** together on this device. Choose a **Category**, expand **Filters**, and search dedicated Player, Source, Date and Opening dropdowns. Opening entries combine ECO codes and opening names; games without opening tags are classified from their moves using the bundled opening dataset. Filter by analysis status, sort by recency or move count, and mark favorites. **Special filters → Fewer than 2 full moves** finds games with fewer than four played half-moves for review, export or deletion. When a filter is active, **Clear filters** appears beside **Filters** and clears all filters together while keeping your sort order. Click a game title to reopen it, or use its hamburger menu for other actions.

Move, resize or maximize Saved Games, or open it in a separate window. Its size, position and expanded Filters state are remembered, and game cards reflow with the available space. Board Theme and Training Profiles pop-outs also adapt to their window size. Select individual games, a page, or all matching games; **Clear selection** clears every page. A progress bar tracks large exports and disappears when preparation completes. Export plain PGN, annotated PGN with notes, symbols and drawings, or Analysis JSON; multiple JSON files are bundled in a ZIP.

Click a small board for a larger player with a move table beside it. The current move has a rounded outline, and the list scrolls to keep it visible. Browse with the navigation controls or arrow keys, or use **Play** for automatic playback. Right-click Play to choose **Very Fast (0.5 s)**, **Fast (1 s)**, **Normal (2 s)**, **Slow (3 s)** or **Analysis (5 s)**. The preview remembers your speed. Games with a Starting note show a short preview and retain the full note in the larger player. Deleting a game returns to the library with the current filters and window state preserved.

## Choose how to import your games

Filter Lichess and Chess.com username imports by time control and an optional date range, and request 20, 50, 100, or 200 matching games. Newest matches come first. Dates use UTC with an inclusive end date, and either boundary can be left open. Reset restores 20 games, all time controls, and no dates. Individual Lichess game links fetch one game independently of the filters. [Import filters](/help/import-games/).

Open **Game → Import…** for four tabs: **Paste a game or collection**, **Lichess**, **Chess.com**, and **Scoresheet**. Move or resize the gold-bordered window to fit your workspace; the tab row wraps on narrower windows. The paste area grows with the window, with Clear and Continue directly below it. Paste PGN or Analysis JSON, fetch online games, or enter a handwritten scoresheet. Move or resize the collection review window, filter by player and choose which games to keep; duplicate games are skipped. A beveled progress bar shows games processed and percentage during large imports. PGN imports discard games with fewer than **two full moves (four half-moves)**. The result separates saved games, duplicates, discarded games and failures, and reports any games not attempted. Start full-game analysis when ready, or restore saved results immediately with Analysis JSON. [Import Games guide](/help/import-games/).

Use **Browse…** to select a PGN, Analysis JSON or ZIP file. A ZIP can contain one or more PGN files, including files in folders; Ironwood combines them for the normal import flow. Browse fills the paste area, then **Continue** opens the game or collection. You can also drag a ZIP onto the app. ZIP imports run locally, check file integrity, and use the existing duplicate detection. The ZIP and its combined uncompressed PGNs can each be up to **64 MiB**, with at most **1,000 entries**. [ZIP import help](/help/import-games/#import-a-zip-collection).

PGN imports use a single pass to count moves through comments and variations. File size, game count, move count and nesting limits protect the app from excessively large or malformed notation. PGN collections can contain up to **100,000 games** within **64 MiB**, with a **1 MiB** limit per game.

The online import panels provide clear username fields, gold **Fetch games** buttons, and status messages. Fetched games open in the Paste tab; choose **Continue** to review and import them. The Scoresheet tab provides a three-step guide and an **Enter scoresheet** button to open the entry workspace.

## Import games from Lichess

Bring your Lichess games into Ironwood for review and analysis. Enter a player's username to fetch up to the chosen number of matching completed games (20 by default), or paste an individual game link. No Lichess login or API token is needed. Choose the games you want to keep; Ironwood skips duplicates and saves imported games in your local library. Open a game to explore its moves and start Stockfish analysis from Game Analysis. See [Import games from Lichess](/help/import-from-lichess/) for the steps. Fetching games requires an internet connection.

## Import games from Chess.com

Select **Chess.com** in the tabbed Import window and enter a username to fetch up to the chosen number of completed standard chess or Chess960 games (20 by default). Ironwood reads recent monthly archives sequentially, then lets you choose games to save through its existing duplicate-aware import workflow. No account connection or API token is needed. Chess.com’s cached archives may take time to include a just-finished game. The same window provides **Paste a game or collection**, **Lichess**, and **Scoresheet** tabs. [Import games from Chess.com](/help/import-from-chess-com/).

## Import handwritten scoresheets

Turn a handwritten game into PGN with **Tools → Enter a scoresheet…**, or **Game → Import… → Scoresheet → Enter scoresheet**. Both open the same workspace. Type White's and Black's moves beside a photo of the sheet while a board preview and validated move list follow your entries. Zoom and drag the photo to read the handwriting. Live validation catches illegal or missing moves, accepts lowercase piece letters, and adds check and checkmate notation automatically. Correct an earlier move without losing the continuation; gold board highlights and an automatically scrolling move list help you follow the game.

Moves, game details, and the photo save locally so you can stop and return to the unfinished sheet later. Enter the players, event, date, and result, then download PGN or pass the game into Ironwood's existing import flow for review and analysis. See [Import a game from a scoresheet](/help/import-a-scoresheet/) for instructions or [the gallery](/gallery/) for a preview.

## Explore different starting positions

Play from the standard starting position, try a random Chess960 setup, or enter a custom FEN. The position editor lets you arrange pieces and review the side to move, castling rights, and other position details before starting. For Chess960 castling, move your king onto its rook.

## Bring a board from an image

Upload, paste, or drop a chessboard screenshot into the position editor. Recognition runs on your device and produces an editable position. Crop closely to the board for the best results, then check the pieces, orientation, and game-state details before using it. You can correct the board manually; screenshot recognition is a starting point for review.

## Meet opponents online

Play with your Lichess account on Ironwood's board. Sign in through the **Lichess lobby** using Lichess authorization, then use quick-pairing tiles, create a custom rapid or classical game, challenge a friend, or resume an ongoing game. Pending moves appear immediately as translucent pieces and become solid when Lichess confirms them. Live clocks and game results follow the server.

Set up a casual game against the Lichess computer with **Standard**, **Chess960**, or **From Position** with a validated standard-chess FEN. Choose Unlimited, Real time, or Correspondence, strength 1–8, and White, Random, or Black. The setup uses large strength buttons, piece cards, and a gold start button.

Correspondence games stay ongoing while you return to the lobby, play another game, or load a saved game for study. The ongoing-game list shows your side, turn, game mode, and available start-time and clock details. Refreshing reconnects your account without automatically replacing your local board with an existing correspondence game.

During active Lichess play, **Game Analysis** becomes **Game chat**. Resign, abort, draw, takeback, and claim controls are available below the board, in the lobby, and under **Game**, subject to Lichess's rules. Analysis remains available while connected to the lobby and returns when a game ends; finished games save locally. **Leave online play** keeps your authorization for reconnecting, while **Sign out** revokes it. [Playing on Lichess](/help/play-on-lichess/).

The bottom **Lichess** button opens the lobby directly. Connection state and your username appear separately; click your username or choose Player profile in the lobby for your record, ratings, graphs, and recent games.

Move or resize the Lichess lobby, or use its raised **↗** button to open it on another monitor. The pop-out shares the main window's connection and applies matchmaking and game commands there. Only the selected Quick pairing, Create a game, or Correspondence tab is gold; inactive tabs are gray. A larger account name and subtle paper-grain panels make the lobby easier to scan. Keep the main window open while using the pop-out.

Open the bottom **FICS** button for the **Free Internet Chess Server** window, with Console, Chat, Lobby, History, Watch, and Play tabs. Connect as a guest or use the Connection menu to create an account or sign in. Remember handle fills your saved handle after reloads; the password is not saved. Connecting preserves your local board and analysis until a game starts or you open a watched game. Live boards and clocks follow FICS; completed games save locally.

Connection limits, login and idle timeouts, and bounded command and output queues protect the FICS bridge from excessive resource use. Guest sign-in, registered sign-in, console commands and game observation remain available.

The FICS status bar separates its window button, connection state, and connected handle. Hover the handle for available account records. Lobby lists available games in a color-coded table, with a player-type legend and Finger profile popups. Play groups opponent search, time controls, named-player challenges, and Accept/Decline controls. [FICS guide](/help/online-play/).

## Watch live games

The **Watch** tab lists public FICS games in a sunken table with players, ratings, game type, and time controls. Refresh the list, start watching, reopen a game you already watch, or Stop watching all. Watch up to 10 games and switch between observation tabs above the board. Finished games can be analyzed afterward.

## Preview and review FICS history

Enter any player handle in **History** to see a table of recent games, results, opponents, ratings, time controls, opening codes, and dates. **Preview** uses the moves player without changing the main board. **Review** opens the archived game on the main board at its starting position, keeping FICS connected; save it explicitly if you want it in your library. Lobby’s History button looks up its selected player. [History help](/help/online-play/#history-recent-games-previews-and-reviews).

## Keep conversations together

The FICS **Console** shows server commands and replies. **Chat** has a separate row of player and channel tabs, wrapped messages, unread counts, and independent drafts. Start a conversation or join a channel with New chat. The raised pop-out button opens the server window on another monitor, sharing the main connection. Chats last for the current session; FICS permissions determine where you can send messages.

## Choose a 2D or 3D board

Switch between 2D and 3D from the **View** menu or the button beside Flip Board. The 3D board sits inside the same game workspace, with right-button drag rotation, right-button double-click to reset the view, and right-button-plus-wheel zoom. Open **View → Board → Theme…** for a movable, resizable chooser with **2D Pieces** and **3D Boards** tabs. It always opens on **2D Pieces**; the Marble, Wood, Glass, Art Deco, and Egyptian previews load when you select **3D Boards**. Click a card to apply it and switch board views. Each card has an Adjust button for per-theme settings. The 2D controls include gradient colors, inner shading, outlines, and shadows. The 3D controls include size, height, materials, shadows, overall appearance, and radial lighting. See [Board themes and appearance](/help/board-themes/). Wood uses Omie's CC0 chess set. Adjust its appearance with one slider for brightness, contrast, and gloss. Coordinates, move highlights, best-move arrows, and a graphical promotion chooser remain available. Beneath each player's name, piece icons show their captures and a +score shows a material advantage; these follow the selected position as you review moves. **Copy Board to Clipboard** in Game Moves captures the selected position in the active 2D or 3D view.

## Put a 3D board on another monitor

Open the **3D ↗** companion window from the board toolbar to watch the current game in a separate browser window. It follows moves and game history while keeping its own theme and camera. Flip its board independently, rotate and zoom, or reset the view; move it to another monitor without giving up the main study workspace. The main window must remain open for synchronization. [Companion board controls](/help/board-and-view/).

## Make each theme your own

**Hybrid Marble** adds a flat board with shaded 3D pieces viewed from above. Choose Marble, Wood, Glass, Art Deco, or Egyptian pieces in its theme settings. **Complement** selects board colors to suit the piece style; **Custom** lets you choose the light and dark squares yourself.

Choose **Smooth, Marble veins, Wood grain, Linen, Slate, Glass, or Sand** as the board texture. All textured choices add beveled edges and soft depth lighting; Glass adds reflections, while Smooth stays flat. Texture and color choices are independent of the piece style. Your hybrid preferences are saved on this device and appear in copied board images.

Classic Staunton pairs its pieces with cream and walnut squares. Neon Geometric uses navy squares and cyan coordinates; Art Deco Faceted uses jade and teal with gold accents. The palettes follow the theme in the live board, chooser previews, and copied board images.

Open a theme card’s **Adjust** button or **View → Board → Theme adjustments…** to customize it. Changes apply immediately and save separately for each theme on this device. **Reset this theme** restores only that theme’s defaults.

For illustrated 2D pieces, choose separate White and Black colors or gradients with independent start and end colors. Select vertical, horizontal, diagonal, or center-outward gradients, or start from a preset. **Inner shading** adds rounded volume inside the artwork; smoothed contrasting outlines help the silhouettes stand out. Size and shadow strength/softness are adjustable too. Processed piece images are cached for normal redraws.

The 3D dialog groups size with height, then provides side colors, material finishes, shadows, overall appearance, and radial lighting. Contrasting outlines are limited to 2D. See [Board themes and appearance](/help/board-themes/) for the available controls.

## Move and review in either board view

Drag pieces to legal destinations in 2D or 3D, with a translucent preview following the pointer. Scroll down over the board to advance through game moves and up to go back. Select an engine prediction to navigate that line instead. Wheel navigation is disabled during active training and online play. In 3D, hold the right mouse button while scrolling to zoom; Shift while right-dragging locks rotation while adjusting tilt.

Under **View → Move feedback**, choose **Capture dissolve** and **Highlight king in check**. Preferences save locally. Capture effects apply only to live captures, and capture and live-move animations respect system reduced-motion settings. Active-game actions remain beneath the board in Focus mode, including training resignation. Local two-player games can accept or decline draw offers. [Board controls](/help/board-and-view/) · [Playing a game](/help/playing-a-game/).

Tactical Map shows Knight reach ghosts and attacker/defender counts on the 3D board, following its camera. Fork cards distinguish strong, checking, and capturable candidates and show immediate legal captures of the forking piece. These clues do not prove a material win. [Tactical Map](/help/tactical-map/).

## Type your moves

Start typing during your turn and a translucent overlay shows the move. Enter plays it, Backspace edits, and Escape cancels. Type `e4`, `nc3`, captures, castling, or promotions; lowercase piece letters and coordinate notation such as `e2e4` work too. Illegal moves stay visible for correction. Keyboard entry follows normal turn and clock rules and works in local, training, and online games. [Keyboard move input and shortcuts](/help/keyboard-input/).

The workspace places **Game Moves** to the left of the live board, **Real-Time Analysis** immediately to its right, and **Game Analysis** on the far right. Resize the side panels to suit your display. The White/Black balance bar follows the board height, and engine predictions use a black scrolling table with separate White and Black columns and navigation controls inside the table, matching Game Moves. Analysis controls sit to the left above the graph, with gold Expand and Print analysis buttons on the right.

## Focus on the board

Click the focus icon beside **Flip board** or press **Shift+Enter** when you are not editing text. The board expands while menus, move lists, analysis panels, floating tools, and the status bar are hidden. Player names, clocks, results, captured pieces, coordinates, and essential board controls remain visible. Focus mode works in 2D and 3D. Real-time position analysis pauses while focused; engine play, online clocks, and full-game analysis continue. Press **Escape**, **Shift+Enter**, or the focus icon to restore the workspace. [Focus mode](/help/playing-a-game/#focus-on-the-board).

## Personalize the app background

Open **View → Background…** for a dedicated settings window. Choose **Gradient Only**, the built-in **Default** Ironwood knight artwork, or **Uploaded Image**; the selected mode is highlighted in gold. Upload PNG, JPEG, or WebP images, switch between your saved uploads, and remove images you no longer want. Upload and remove controls appear for the uploaded-image option.

Adjust image opacity, background gradient strength, and the 3D backdrop independently. A translucent 3D backdrop reveals the app background while the board and real pieces remain opaque. Paper grain and fine linen distinguish recessed study surfaces. The expanded analysis view uses grain around solid sunken detail panels, keeping text and board previews readable.

First launch uses **Gradient Only**, a **17% gradient**, and **0% 3D background opacity**. Returning users retain their saved choices. Changes apply immediately; uploaded backgrounds and preferences stay on this device. [Background settings](/help/board-and-view/#app-background).

## A consistent study workspace

Board Theme and Background open at normal sizes and can be moved, resized, or opened in separate browser windows with **↗**. Both have raised **×** buttons, gold 3D borders, heavy outer paper grain, and linen and subtle-grain inset sections. Theme choices and background changes in a pop-out apply to the main workspace; theme **Adjust** controls open in the main window. Import Games uses matching gold borders and textured sections. Engine Settings marks its selected context tab with a raised gold finish and dark text. Allow browser pop-ups for Ironwood and keep the main window open when using companion windows.

A resizable **New Game** window separates Play Mode, Engine Settings, Game Settings, Your Side, and online server choices. Compact engine controls and centered start/connect buttons leave more room for setup.

Sunken sections, gold accents, and raised primary controls now unify the menus, training profiles and results, saved games and enlarged playback, engine settings, online lobbies, backgrounds, board themes, and study tools. Fine linen adds depth to the main move list, Engine Prediction score and continuation sections, and the Saved Games controls. Paper grain appears around training profiles, engine settings, Tactical Map, Notes, Saved Games, and larger playback. Profile graphs and expanded prediction boards have their own sunken textured surfaces. Training graphs use a gold paper texture. Notes has explicit Add/Edit buttons, and the Position Editor groups board setup, piece selection, import, position details, and FEN controls.

Tactical Map separates **Insights**, **Activity**, and **Patterns** into tabs. Threat and change lists resize with the available height. Pawn Structure, Piece Mobility, King Safety, Opening Explorer, and Tablebases use matching recessed panels. The workspace tab row stays above the right panel’s scrolling content. Tactical and Annotations use the same gray subtabs and sunken panels as Explore.

Lichess account sign-in can remain available alongside FICS without replacing the current board. Opening Explorer uses that account session; tablebases require no sign-in. One game controls the board at a time, and study tools remain unavailable during live online play and training.

## Keep control of your data

Ironwood saves games, personal annotations, uploaded backgrounds, and display preferences locally. Download a versioned JSON backup, then restore it by merging with your library or replacing existing games. Move and resize Storage Information to inspect game counts and browser usage in scrollable recessed sections. Install the app and explicitly store the engine for offline play and analysis without an account. Lichess play and chat, and FICS play, spectating, and chat require an internet connection and send game activity and messages to the server; local engine analysis and screenshot processing run on your device.

## Optional usage statistics

Choose whether to share visits and usage events with PostHog, and change your choice from **Analytics settings**. Session recording and automatic interaction capture are disabled; game positions, moves, notes, and player names are excluded from these usage events. Local play and analysis work with either choice. Automatic freeze reporting is not currently implemented. [Usage statistics and privacy](/help/optional-usage-statistics/).

## Device and browser compatibility

Phones and tablets can browse the website, but Ironwood blocks app startup, installation, and offline engine setup on mobile devices. Open the app on a desktop or laptop computer.

Ironwood is currently designed for desktop and laptop screens. Use a current browser and graphics device with WebGPU support and hardware acceleration enabled. WebGPU availability and 3D performance vary by browser, operating system, and graphics hardware. The full Stockfish 19 engine is a separate 94.5 MiB download; allow space for it if you enable offline play. We have not established a reliable minimum CPU, RAM, or GPU model, so those are not listed as requirements.

## Searchable help and recovery guides

The [Help center](/help/) covers play, training, puzzles, online connections, imports, analysis, annotations, themes, and local storage. Search article text with multiple terms, filter by topic, and use each article’s contents links to reach a specific control. Dedicated guides cover Lichess profiles, expanded graphs, and recovery from loading problems or a frozen page.

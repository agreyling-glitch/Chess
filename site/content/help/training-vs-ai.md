---
title: "Train against AI"
description: "Choose a training engine, build a profile, play both colors, and follow your progress."
category: "using-ironwood"
lastmod: 2026-11-02T00:00:00-06:00
keywords: [training, profile, Elo, rating, progress, statistics, Stockfish]
weight: 14
---

Choose **Training → Train against AI…** and choose **Manage profiles** to create a profile for Built-in Stockfish or a supported installed Companion engine. Each profile retains its engine and options. For Stockfish, enter a profile name, choose **Starting target Elo** from 1320–3190, and select **Create profile**. The default is 1320. Start the game to play White against Stockfish at your chosen target. Games use the standard starting position with unlimited thinking time for you. Stockfish gets up to one second of search time per move. Unfinished training games from earlier versions also become untimed when reopened.

Each profile starts with separate White and Black ratings at its chosen starting target. After a completed game, the next game uses the other color. A color's next opponent target follows its rating. **Overall** progress is the lower of the two ratings, so advancing requires results with both colors.

Wins count as 1, draws as 0.5, and losses as 0. The rating changes by `32 × (score − expected score)`, rounded to a whole point. Expected score uses the usual Elo formula with the opponent's recorded target. Checkmate, resignation, and draws all count, as do historical clock losses from earlier versions. Unfinished games do not change a rating. Reopening or analyzing a finished game does not apply its rating change again.

Stockfish's target range is 1320–3190. Your training rating can fall below 1320; the engine stays at its lowest target until you recover. These are personal **Ironwood training ratings**, not FIDE, Chess.com, or Lichess ratings. Stockfish's strength limiter is approximate, and the game's conditions affect its effective strength. [Stockfish documents its Elo target and calibration](https://official-stockfish.github.io/docs/stockfish-wiki/UCI-Protocol-and-Stockfish-Commands.html#uci_elo).

Hints, analysis, takebacks, and strength changes are held until the game finishes. Training automatically records a draw for stalemate, insufficient material, threefold repetition, or 50 moves without a pawn move or capture.

To resign, choose **Resign…** beneath the board, then confirm **Resign game**. Choose **Keep playing** to cancel. The button is also available in Focus mode. A confirmed resignation records a training loss and updates your profile.

## Your profile and progress

Open **Training → Train against AI…** or **Profile & progress** in Game Moves. Choose a profile to see:

- White, Black, and overall rating histories.
- Win/draw/loss totals for each color and rating changes over its last ten games.
- A score rate and a broad descriptive 95% range, including its sample size. This range is not uncertainty in your Elo or proof of improvement; opponents adapt, and your own skill changes.
- Clock losses and recent completed games.

The console groups **Progress**, **Focus areas**, and **Recent games** in separate icon tabs. The next-game inset shows your side, opponent difficulty, and unlimited clock. Use the gold **Start training game** button to continue with the next color. While a training game is underway, finish it before starting the next one. Training Profiles is movable and resizable, with a gold 3D border. Drag its header or resize its edges. The raised **↗** button opens it in a separate browser window; **×** closes it. Profile edits are shared between windows, and Start training game in the pop-out starts play on the main board. Keep the main window open. Puzzle practice is launched from Training Profiles in the main window.

### Rename a profile

Open **Manage profiles**, check the row for one profile, choose **Rename**, edit the name, and choose **Save name**. Profile creation, engine choices, import, export and deletion remain in this management window. **Cancel** keeps the existing name. Renaming changes the display name, while its internal ID, ratings, and completed-game history stay linked. Saved Games titles and the profile filter display the current profile name; previously recorded game data can retain the name used when the game was played.

You can also [type your moves](/help/keyboard-input/), including lowercase piece letters such as `nc3`, during training.

## Finding areas to focus on

After a training game finishes, use **Analysis** to analyze it. The profile updates its statistics as positions are analyzed, without changing the game's rating result.

The dashboard counts only your moves. An analysis-coverage indicator shows how many player moves have been analyzed. Opening, middlegame, and endgame cards show average centipawn loss as bars on a shared scale, plus numerical evaluation counts, mistakes, and blunders. Lower average loss is better; 100 centipawns equals one pawn. The highlighted practice card suggests a phase to review. Expand **How these statistics are calculated** for the definitions. Mistakes and blunders follow Ironwood's existing move-quality classifications. Mate scores are excluded from the average centipawn loss and tracked separately as missed or allowed mates.

Phase labels are heuristics: the first ten full moves are the opening; later positions with at most 26 points of non-pawn material are endgames; other positions are middlegames. A phase needs at least 20 moves with numerical evaluations and a mistake or blunder before it can be recommended as a recurring focus area. Partial analysis can bias the summary. Use the suggested phase and mate counts to choose positions to review, rather than treating them as a diagnosis of your chess ability.

When **Your next practice focus** recommends a phase, choose **Puzzle difficulty** and click **Practice this phase** to load a matching [Lichess puzzle](/help/puzzles/). The profile window closes and Puzzles opens on Practice with Opening, Middlegame, or Endgames selected. Difficulty uses Lichess's anonymous baseline of 1500, not your Ironwood training rating. Finish active training or online play before starting puzzles.

### Progress over time

In **Focus areas**, the three sunken history charts track average loss for Opening, Middlegame and Endgame. Gold lines show a rolling average of the available per-game means within each ten-completed-game window; faint dots show individual games. A shorter starting window is used until ten games are available. **Last 20 games**, **Last 50 games**, and **All games** change the visible history, while rolling averages still include earlier games in their ten-game window. All three charts share the same vertical scale.

Lower is better. Hover a chart for the game date, color, average loss, rolling average, sampled-game count, numerical evaluations and game coverage. Games without numerical evaluations for that phase remain gaps, not zero loss. Mate scores are excluded. The coverage line distinguishes games with analysis from known-move coverage; older games without recorded move totals are excluded from that percentage. Partial analysis and changing opponents limit comparisons.

## Saved games and backups


**Saved Games → Training** keeps training games separate from My Games, Imported Games, and Observed games. Filter the Training section by profile to replay games, add notes, and run analysis. Each game records the profile, color, opponent target, and rating change.

Profiles and their completed-result histories are included in **Settings → Storage → Backup and Restore**. Restoring a backup merges profile results by their unique game IDs to avoid counting a result twice. Deleting a game from Saved Games removes its replay record; its completed rating result and aggregate analysis remain in the profile history. **Reset all local data** removes profiles too.

Profiles are stored in this browser on this device. Make a backup before clearing browser data or moving devices. Native development previews keep profiles in memory for the current session.

## Training window layout

The Training console separates the next-game controls from Progress, Focus areas and Recent games, with profile management behind **Manage profiles**. Gold icons and primary controls, aligned spacing, and sunken sections match the rest of Ironwood. The rating graph has a gold paper surface, and the training result card uses the same recessed treatment. Primary actions such as **Start training game** use raised gold buttons.

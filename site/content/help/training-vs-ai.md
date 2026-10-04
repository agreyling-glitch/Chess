---
title: "Training vs AI"
description: "Build a training profile, play both colors, and follow your progress against Stockfish."
category: "using-ironwood"
keywords: [training, profile, Elo, rating, progress, statistics, Stockfish]
weight: 14
---

Choose **Game → New game → Training vs AI**, enter a profile name, and select **Create profile**. Start the game to play White against Stockfish at a 1320 Elo target. Games use the standard starting position and a 10-minute clock with five seconds added after each move.

Each profile starts with separate White and Black ratings of 1320. After a completed game, the next game uses the other color. A color's next opponent target follows its rating. **Overall** progress is the lower of the two ratings, so advancing requires results with both colors.

Wins count as 1, draws as 0.5, and losses as 0. The rating changes by `32 × (score − expected score)`, rounded to a whole point. Expected score uses the usual Elo formula with the opponent's recorded target. Checkmate, resignation, clock losses, and draws all count. Unfinished games do not change a rating. Reopening or analyzing a finished game does not apply its rating change again.

Stockfish's target range is 1320–3190. Your training rating can fall below 1320; the engine stays at its lowest target until you recover. These are personal **Ironwood training ratings**, not FIDE, Chess.com, or Lichess ratings. Stockfish's strength limiter is approximate, and the game's conditions affect its effective strength. [Stockfish documents its Elo target and calibration](https://official-stockfish.github.io/docs/stockfish-wiki/UCI-Protocol-and-Stockfish-Commands.html#uci_elo).

Hints, analysis, takebacks, and strength changes are held until the game finishes. Training automatically records a draw for stalemate, insufficient material, threefold repetition, or 50 moves without a pawn move or capture.

## Your profile and progress

Open **Game → Training profiles…** or **Profile & progress** in Game Moves. Choose a profile to see:

- White, Black, and overall rating histories.
- Win/draw/loss totals for each color and rating changes over its last ten games.
- A score rate and a broad descriptive 95% range, including its sample size. This range is not uncertainty in your Elo or proof of improvement; opponents adapt, and your own skill changes.
- Clock losses and recent completed games.

Use **Start next training game** to continue with the next color. While a training game is underway, finish it before starting the next one. The profiles window opens tall by default; use the maximize button beside Close to expand it and the same button to restore its size.

### Rename a profile

Select the profile, choose **Rename profile**, edit the name, and choose **Save name**. **Cancel** keeps the existing name. Renaming changes the display name, while its internal ID, ratings, and completed-game history stay linked. Saved Games titles and the profile filter display the current profile name; previously recorded game data can retain the name used when the game was played.

You can also [type your moves](/help/keyboard-input/), including lowercase piece letters such as `nc3`, during training.

## Finding areas to focus on

After a training game finishes, use **Game Analysis** to analyze it. The profile updates its statistics as positions are analyzed, without changing the game's rating result.

The dashboard counts only your moves. An analysis-coverage indicator shows how many player moves have been analyzed. Opening, middlegame, and endgame cards show average centipawn loss as bars on a shared scale, plus numerical evaluation counts, mistakes, and blunders. Lower average loss is better; 100 centipawns equals one pawn. The highlighted practice card suggests a phase to review. Expand **How these statistics are calculated** for the definitions. Mistakes and blunders follow Ironwood's existing move-quality classifications. Mate scores are excluded from the average centipawn loss and tracked separately as missed or allowed mates.

Phase labels are heuristics: the first ten full moves are the opening; later positions with at most 26 points of non-pawn material are endgames; other positions are middlegames. A phase needs at least 20 moves with numerical evaluations and a mistake or blunder before it can be recommended as a recurring focus area. Partial analysis can bias the summary. Use the suggested phase and mate counts to choose positions to review, rather than treating them as a diagnosis of your chess ability.

## Saved games and backups

**Saved Games → Training** keeps training games separate from My Games, Imported Games, and Observed games. Filter the Training section by profile to replay games, add notes, and run analysis. Each game records the profile, color, opponent target, and rating change.

Profiles and their completed-result histories are included in **Storage → Backup all data**. Restoring a backup merges profile results by their unique game IDs to avoid counting a result twice. Deleting a game from Saved Games removes its replay record; its completed rating result and aggregate analysis remain in the profile history. **Reset all local data** removes profiles too.

Profiles are stored in this browser on this device. Make a backup before clearing browser data or moving devices. Native development previews keep profiles in memory for the current session.

---
title: "Training both sides, one game at a time"
date: 2026-10-04T00:00:00-05:00
description: "Personal training profiles turn Stockfish games into a record of progress, with typed moves and quicker game review."
slug: "training-both-sides-and-typing-moves"
---

A single win against an engine tells you something about one game. A sequence of games with both colors can give you more useful material to study. Ironwood's new **Training vs AI** mode keeps that sequence under a local profile, with separate White and Black ratings starting at 1320.

Finish a game with White, then play Black. Stockfish's next target follows the rating for that color, within its 1320–3190 range. Overall progress is the lower of the two ratings, so one stronger side cannot hide the other. These are personal Ironwood training ratings, rather than an official rating or a precise measure of human playing strength.

## Turn analysis into a practice plan

The profiles dashboard shows rating histories, results, and recent changes. After a game, analyze it to add your moves to the phase statistics. Opening, middlegame, and endgame cards use a shared bar scale for average centipawn loss, with exact values, evaluation counts, mistakes, and blunders beside them.

Analysis coverage makes the missing data visible. A practice suggestion requires at least 20 numerical evaluations and a mistake or blunder in a phase. Partial analysis, changing opponents, and small samples all limit what the numbers can tell you. Treat the highlighted focus as a place to begin reviewing positions, rather than a verdict on your ability.

You can rename a profile while keeping its internal ID and history, expand the dashboard, and start the next training game from it. Training games have their own Saved Games category and profile filter. Full backups include profiles and completed results; restoring the same results repeatedly does not apply the Elo changes again.

## Make a move without reaching for the mouse

Start typing `e4` or `nc3` during your turn and a translucent overlay appears. Enter plays the move, Backspace edits, and Escape cancels. Lowercase piece letters, captures, castling, promotions, and coordinate moves are accepted. Illegal moves stay visible so you can correct them. The clock keeps running while you type.

The same entry works in ordinary local games and online play. For an ordinary game against Stockfish, the New Game window now exposes the Elo target directly. The Compact workspace gives the move list more vertical room and makes its tabs easier to read.

## Review from the library

Click a small board in Saved Games to open the larger preview. Step through moves with its buttons or Left and Right arrows, or select Auto play to advance every half-second. Manual navigation pauses playback, and closing the preview stops it. You can inspect a game without replacing the current board.

Read the guides for [Training vs AI](/help/training-vs-ai/), [keyboard move input](/help/keyboard-input/), and [Saved Games and backups](/help/saved-games-and-backups/), or browse the [release notes](/changelog/2026-10-04-training-and-keyboard-moves/).

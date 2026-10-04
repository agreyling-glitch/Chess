---
title: "Training profiles, keyboard moves, and preview playback"
date: 2026-10-04T00:00:00-05:00
description: "Adaptive Stockfish training with per-color Elo, typed move entry, and move playback in Saved Games previews."
slug: "2026-10-04-training-and-keyboard-moves"
version: "2026.10.04"
---

## Added

- **Training vs AI** with local profiles starting at 1320 for both White and Black, alternating completed-game colors, and a Stockfish Elo target matched to the next color's rating. Overall progress uses the lower rating.
- Completed-game Elo updates with K=32 and protection against counting the same result twice. Training uses standard chess with a 10-minute clock and five-second increment; hints, takebacks, analysis, and strength changes pause until the game finishes.
- A **Training profiles** dashboard with per-color rating histories, results, recent changes, analysis coverage, average-loss bars by phase, mistake and blunder counts, and sample-aware practice suggestions.
- Profile renaming, maximize/restore, and a prominent **Start next training game** button. Profiles and completed-result history are included in full backups and merged by game ID.
- A separate **Training** Saved Games category and profile filter. Titles and filters use the profile's current name after renaming.
- **Keyboard move input**: the first keystroke opens a translucent overlay. Enter plays, Backspace edits, and Escape cancels. Supports lowercase piece letters, captures, castling, promotions, and coordinate notation; illegal moves remain available for correction.
- A **1320–3190 Elo control** directly in **New game > You vs Engine**.
- First, Previous, Next, and Last controls beneath larger Saved Games previews, plus **Auto play/Pause** at 0.5 seconds per move. Left and Right arrow keys navigate and pause playback.

## Improved

- Compact is the sole workspace layout. Game Moves uses the available vertical space and its top tabs have larger text.
- Dialog close controls share the Scoresheet window styling. Removed the Storage window's maximize control.
- Redesigned training setup with profile and rating cards, clearer next-game details, larger dashboard text, section spacing, and a taller default profiles window.
- Centered the New Game engine settings card and removed redundant Engine and Context labels in Engine settings.
- Replaced unsupported arrow glyphs in training labels with readable text.
- Deleting the currently loaded saved game resets the board and returns to Saved Games with filters, page, and maximized state preserved.
- Updated [Training vs AI help](/help/training-vs-ai/), added [Keyboard move input help](/help/keyboard-input/), and refreshed related play, preview, and backup guides.

---
title: "Lichess studies and prediction hints"
date: 2026-11-01T00:00:00-05:00
slug: "2026-11-01-lichess-studies-and-prediction-hints"
version: "2026.11.01"
description: "Study chapters in one workspace, compare live PV moves on the board, and keep saved alternatives visible with blue ghosts."
---

- Added a Lichess study workspace for chapter navigation, author comments, variations, personal notes, drawings, and practice as either side. Public studies can be refreshed or copied as complete PGN while retaining local work.
- Added **Studies** to the Browser library. Chapters from the same Lichess study share one entry; older separate chapter saves are consolidated while preserving local annotations, analysis, favorites, collections, and original records in browser backups.
- Refined the study layout with a single chapter selector, the chapter's ECO code and opening name, a sunken lesson panel with subtle grain, gold move labels, and softer commentary. Fixed hover resizing and alignment of chapter, practice, and source/export controls.
- Added **Show best next move** as a prominent On/Off card in Prediction. It compares available PV lines for the displayed position, marking the strongest move in green and alternatives in red, with score bubbles at destination squares. Scores use White's perspective; duplicate candidate moves share a marker.
- Enhanced **Analyze position** with a split-button engine menu for browser Stockfish and available Companion engines. Switching clears stale live predictions and resumes an active search after the new engine is ready.
- Added **Compare saved line**, enabled by default and remembered across positions and reloads. A dashed blue arrow and translucent ghost distinguish the saved recommendation before the played move from current live predictions. Labels identify that earlier position; the 3D ghost follows the board's piece model, camera, scale, and orientation.
- Preserved multiline study comments and drawing directives through import and export, separate analysis for study branches, manual board orientation, and local work when public-source material changes. Practice follows the chosen side across chapter changes.

See [Lichess studies](/help/lichess-studies/), [prediction and board markers](/help/engine-line/), and [saved-analysis comparison](/help/saved-analysis/).

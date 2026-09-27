---
title: "A clearer way to study and share a game"
date: 2026-09-27T12:00:00-05:00
description: "Move notes, an expanded analysis graph, board-image sharing, and batch tools for a local-first chess library."
slug: "studying-and-sharing-your-games"
---

The useful part of an analysis session is often the thought you want to remember afterward: why a move looked tempting, where a plan changed, or what you would try next time. Ironwood now lets you attach a text note to a move or the starting position. A small marker shows where notes are present in the move list and on the evaluation graph. Open the graph full screen to read the note alongside Stockfish's score, search depth, and best continuation.

Notes belong to the game, not to an engine run. Pausing or rerunning analysis does not replace them. They survive reopening a saved game and switching among observed games. They also travel with annotated PGN and Ironwood Analysis JSON, and appear in the printable report. Plain PGN remains free of analysis and notes when you want a simple game record.

## More room to see the game

The analysis graph can be expanded from its right-click menu. The larger view adds evaluation gridlines, move labels, mistake markers, and controls to step through positions. From the Game Moves right-click menu, you can copy the selected position as FEN or as a PNG board image using your current pieces, orientation, coordinates, frame, and shadows. You can also copy PGN through the selected move or for the whole game.

The board itself gained a dark frame with gold coordinates and optional piece shadows, both enabled by default. The New Game dialog has a stable size as you switch between playing Stockfish and playing both sides. Engine settings put play and analysis controls in separate tabs, so changing one context does not make the window jump around.

## A library that handles more than one game at a time

Saved Games can fill the browser window, and each small board preview opens into a larger view. You can select games individually, select a page, or select all games matching the current filters. Deleting a selection asks for confirmation and applies the batch together. The library's Export menu produces a single file for one game, a combined PGN for multiple games, or a ZIP of individual Analysis JSON files so each remains importable.

These are local operations. Notes, game records, analysis, and rendered board images stay on the device unless you choose to export or paste them elsewhere.

[Explore the features →](/features/)

---
title: "Getting started"
description: "Find a game, open the analysis workspace, and learn what to look at first."
category: "using-ironwood"
keywords: [game, review, start, library, analyze]
weight: 10
---

## Open a game

Play a game in Ironwood or open one from your saved games. You can also import a PGN, the common text format for chess moves. The game review shows the move list and lets you step forward and backward through positions.

Open **Game → Import…** and choose **Paste a game or collection**, **Lichess**, **Chess.com**, or **Scoresheet**. Fetch recent games by username from either online source, or one completed Lichess game by its link. See [Import games](/help/import-games/) for the workflow.

For a handwritten game, choose **Game → Import… → Scoresheet → Enter scoresheet**. Type the moves beside your scoresheet photo, check them on the board, and import or download the PGN. Your unfinished draft saves on this device. See [Import a game from a scoresheet](/help/import-a-scoresheet/).

## Run game analysis

Open the Analysis workspace and start full game analysis. Ironwood asks Stockfish to examine each position in turn. You can pause and resume the work. A second, deeper check may run on critical positions where a move appears to change the game sharply.

The results are estimates from a chess engine. They can change if Stockfish searches longer or uses different settings. Wait for analysis and any critical-move verification to finish before treating the summary as final.

## Read the summary

Start with the [evaluation](/help/evaluation/) graph to see when either player gained an advantage. Then inspect the [move labels](/help/move-quality/) and [symbols](/help/move-symbols/) around the biggest changes. [Accuracy and average CPL](/help/accuracy-and-cpl/) summarize the whole game, but the actual positions explain what happened.

## Explore one position

Select a move to see Stockfish's suggested move and its [engine line](/help/engine-line/). Step through the line on the board. Ask what threat the suggestion addresses, which piece improves, or what the played move allowed. The engine gives a candidate continuation; it does not give a human explanation by itself.

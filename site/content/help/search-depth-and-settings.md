---
title: "Search depth and settings"
description: "Set up Stockfish for play, realtime analysis, and full-game analysis, with defaults and lighter-PC suggestions."
category: "terms"
keywords: [engine settings, depth, nodes, nps, threads, hash, quality, quick, standard, deep, custom, performance, verification]
weight: 60
lastmod: 2026-10-21T00:00:00-05:00
---

## Open engine settings

{{< companion-help >}}

1. In the app's bottom status bar, click the gray raised **Engine** button beside the engine name and Ready/Analyzing state. You can also use **Game → Engine settings…**.
2. Select **Play**, **Realtime Analysis**, or **Full game Analysis**. The selected tab has a raised gold finish and dark text.
3. Adjust the settings, then click **Apply**. **Cancel** discards your changes.

Engine settings are unavailable while full-game analysis is running or paused, and during a live training game. Stop full-game analysis before changing its settings.

**Play** controls the engine opponent. **Realtime Analysis** controls analysis of an individual position. **Full game Analysis** sets the work budget for reviewing an entire game. Full-game and realtime analysis share **Search threads** and **Hash memory**, so changing either affects both.

## Full-game defaults and quality

These are the defaults for a fresh setup. Your saved preferences or an imported analysis can use different settings.

| Setting | Default |
| --- | --- |
| Budget | Nodes |
| Verify critical moves after analysis | On |
| Analysis quality | Standard |
| Nodes per position, first pass | 1,000,000 |
| Search threads | Detected from your device, capped at 8 |
| Hash memory | 64 MiB |
| Candidate lines | 1, fixed |
| Strength | Maximum: Skill 20, Elo limiting off |
| Network | Stockfish 19, Full NNUE, fixed |

On **Full game Analysis**, choose **Budget → Nodes** for a node budget or **Budget → Seconds** for **1–60 seconds per position**. Seconds mode replaces the quality/node controls with **Seconds per position**. It sets an engine search-time target for each first-pass position; startup, processing and a second pass add to the total game-analysis time.

In Nodes mode, choose **Analysis quality**:

| Quality | First-pass nodes per position | Use |
| --- | --- | --- |
| Quick | 100,000 | A faster first review |
| Standard | 1,000,000 | General game review; the default |
| Deep | 5,000,000 | More work on difficult games |
| Custom | 10,000–50,000,000 | Set your own budget using Nodes per position |

More nodes mean more work and usually more time. They can improve difficult evaluations, but do not guarantee that every tactical or positional issue is resolved. Changing a realtime search limit does not replace the full-game quality budget.

## Why analysis has two passes

**Pass 1** examines the game's positions in sequence. Progress counts positions, including the starting position, rather than complete moves.

**Verify critical moves after analysis** controls whether **Pass 2** starts automatically. This deeper check examines selected critical positions. In Nodes mode its budget is five times the first-pass budget, with a minimum of 1,000,000 and a maximum of 50,000,000 nodes per selected position. Standard therefore uses **5,000,000 nodes** for each selected position in this pass. In Seconds mode the deeper check uses five times the first-pass time target for each selected position.

Leave the automatic check off for a first-pass review, then use **Verify critical moves** in **Analysis** when you want the deeper check. **Resume verification** continues an interrupted verification pass without discarding the first-pass results.

A display such as **Pass 2 · 14/18** means 14 of 18 selected verification positions have completed. It does not mean the whole game is being analyzed again. The second pass can take noticeably longer per position. Summaries remain **Provisional** until verification finishes; evaluations and move labels may change.

Use **Pause** and **Resume** when you need to interrupt the work. Live position analysis is held while full-game analysis runs. See [Game phases and critical moves](/help/game-phases-and-critical-moves/) for interpreting the results.

## Choose settings for your computer

These are starting points to try, not requirements or speed guarantees. Available thread choices depend on your device.

| Goal | Full-game quality | Search threads | Hash memory |
| --- | --- | --- | --- |
| Keep a modest laptop responsive | Quick | 1–2 | 32–64 MiB |
| Everyday game review | Standard | 2–4 | 64 MiB |
| Deeper review with spare resources | Deep | 4 or more, if available | 128–256 MiB |

Start with Quick and fewer threads if the computer becomes hot, fans become loud, or other apps feel sluggish. Close other tabs running chess analysis. Increase one setting at a time so you can judge its effect. A 3D board also uses graphics resources; a 2D board is a useful comparison when investigating responsiveness.

**Search threads** are workers Stockfish can use at once. More threads may improve speed, but also increase CPU load; doubling threads does not guarantee twice the speed. Ironwood initially uses the detected thread count up to its eight-thread cap, so the default can be demanding on a busy computer.

**Hash memory** stores information about positions already searched. Larger hash can help some searches, but is not a general speed switch. It is only one part of the app's memory use: the engine, neural network, browser, and board rendering need memory too. A 64 MiB hash setting does not mean the whole browser tab uses only 64 MiB.

## Realtime analysis and engine play

On **Realtime Analysis**, choose a **Search limit** and edit its corresponding control:

- **Time:** set **Time limit**, in milliseconds.
- **Depth:** set **Search depth**.
- **Nodes:** set **Search nodes**. The default is 1,000,000 nodes.
- **Infinite:** search until you stop analysis.

Inactive controls retain their values for when you choose that mode. **Candidate lines** ranges from 1 to 5, with 3 as the realtime default. More candidate lines ask Stockfish to analyze alternatives and can slow progress on the best line. Full-game analysis uses one fixed line.

On **Play**, use **Strength mode** to choose **Skill level** or **Elo rating**, then adjust **Skill level** or **Target Elo**. These affect the opponent, not the strength used for analysis. When a local game clock is active, the clock controls engine search time and manual search limits are inactive. See [Elo and ratings](/help/elo-rating/) for the meaning of a target Elo.

## Depth, nodes, and speed

**Depth** is measured in half-moves called *plies*. Depth 20 corresponds to 20 half-moves, or about ten full moves, along a line. Engines extend some lines and shorten others, so depth is not a promise that every possible continuation was examined equally far.

**Nodes** count positions examined during search. **NPS** means nodes per second: the engine's search speed on your device. It is not a quality score for your moves. Depth reached within a fixed node budget varies with the position.

## Slow analysis or no progress

A long second pass can be normal if its position counter continues advancing. A position search that stays at **depth 0 and zero nodes** for a long time needs investigation; choosing a larger budget does not explain an indefinite lack of output.

Try pausing and resuming full-game analysis. If it remains stuck, record any errors in your browser's **F12 → Console**, then refresh the same tab and retry. In a private browsing session, keep the private window open to retain its locally stored data. For persistent problems, report the quality, threads, hash memory, pass and position counter, and any console error.

## Restore defaults

Select a settings tab, click **Reset this context**, then **Apply**.

- **Play** resets the opponent's settings.
- **Realtime Analysis** resets realtime settings, including the threads and hash shared with full-game analysis.
- **Full game Analysis** resets its quality to Standard and its custom budget to 1,000,000. It does **not** reset the shared thread or hash settings.

## Full-game progress

**Analyze full game with Stockfish 19** uses a raised gold button and a beveled progress bar. The balance bar holds its displayed value while full-game analysis runs and resumes realtime updates afterward. This reduces animation work; it does not change the engine’s analysis budget or establish a measured speed increase.

---
title: "Search depth and settings"
description: "Plain explanations of depth, nodes, quality presets, threads, and hash memory."
category: "terms"
keywords: [depth, nodes, nps, threads, hash, quality, quick, standard, deep, custom]
weight: 60
---

## Depth and nodes

**Depth** is how many moves ahead the engine has searched along a line, measured in half-moves called *plies*. Depth 20 is roughly ten moves by each side in a straightforward line, but chess engines search some lines more deeply than others. Greater depth usually gives a more informed estimate, not a guarantee.

**Nodes** are positions the engine examined. More nodes generally mean more work. **NPS** means nodes per second: how fast the engine is examining positions on your device. It is a speed measure, not a quality score for your moves.

## Full game quality

Ironwood's Quick, Standard, and Deep options set different node budgets per position. **Custom** lets you choose the budget. A larger budget takes longer and may improve the analysis of difficult positions. Ironwood may then spend more nodes verifying critical moves. If a label changes after verification, use the completed result.

## Threads and hash

**Threads** are workers the engine can use at once. More threads can speed up searching on capable devices, while also using more processing power. **Hash** is memory for positions Stockfish has already examined so it can reuse that work. More hash can help some searches, but it uses more of your device's memory.

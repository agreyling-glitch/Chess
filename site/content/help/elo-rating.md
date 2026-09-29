---
title: "What is Elo?"
description: "Understand chess ratings and Ironwood's illustrative Elo calculator."
category: "terms"
keywords: [Elo, ELO, rating, expected score, calculator, K factor, FIDE, Glicko]
weight: 88
---

**Elo** is a rating system that estimates a player's results against other rated players. A higher rating generally indicates stronger past results in that rating pool. A rating is not a percentage, an engine evaluation, or a prediction of any one game. Ratings from different sites or playing speeds are not directly interchangeable.

If two players have equal ratings, the basic Elo formula expects each to score about half a point per game on average. A win scores **1**, a draw **0.5**, and a loss **0**. Winning against a much higher-rated opponent normally gains more points than winning against a much lower-rated one.

## Use Ironwood's calculator

Open **Tools → Elo calculator**. Enter your rating and your opponent's rating. Ironwood shows your **expected score**, then estimated rating changes for a win, draw, or loss. Select a result to see an estimated new rating. Under **Advanced**, you can choose a rating-system example and adjust the **K-factor**, which controls how much one result changes the estimate.

This is an **illustration**, not an official rating update. FIDE may apply additional rules. Chess.com and Lichess use Glicko-based systems, where rating uncertainty also affects changes, so their actual updates can differ from Ironwood's estimate.

The engine's **Target Elo** setting is a rough strength target for an engine opponent; it is not a rating you earn by playing it.

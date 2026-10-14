---
title: "A consistent study workspace and independent account access"
date: 2026-10-14T00:00:00-05:00
slug: "2026-10-14-sunken-workspace-and-account-access"
version: "2026.10.14"
description: "Sunken tool panels, responsive study layouts, clearer notes, and Lichess account access alongside FICS."
---

## Workspace and study tools

- Added sunken sections, gold accents, and consistent close controls across training, saved games, enlarged playback, engine settings, online lobbies, backgrounds, board themes, drawing tools, Notes, and the Position Editor. Primary actions use raised gold buttons; the training graph has a gold paper surface.
- Kept main-menu hover effects from shifting neighboring buttons. Game Analysis scrolls below its controls and tabs.
- Split Tactical Map into Insights, Activity, and Patterns. Threat and change lists share available vertical space as the window resizes.
- Applied recessed sections to Opening Explorer, Tablebases, King Safety, Piece Mobility, and Pawn Structure. Narrow panes stack diagrams or selectors; the wide mobility selector row no longer leaves a large gap.
- Added explicit Add/Edit Note actions and a sunken writing area. Restored dragging from floating tool titles and aligned the Position Editor trash button with its palette.
- Painted hybrid pieces after the board rim so its edge does not cross them. Connection-status dots are drawn directly instead of relying on a font glyph.

## Analysis and account access

- Styled Analyze full game and its progress bar in raised gold. The balance bar holds its displayed value during full-game analysis, then resumes updates; no measured speed gain is claimed.
- Separated Lichess account sign-in from board ownership. Signing in preserves FICS and its current board while supplying Opening Explorer access. Tablebases need no login.
- Prevented background Lichess game events from replacing an active FICS game. Only one game controls the board; live-game analysis restrictions remain.

## Documentation

Updated the relevant [Help](/help/) pages and [Features](/features/), and added [A calmer place to study](/blog/a-calmer-place-to-study/) to the Journal.

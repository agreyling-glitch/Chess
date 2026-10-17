---
title: "FICS workspace, lobby pop-outs, and window controls"
date: 2026-10-17T00:00:00-05:00
slug: "2026-10-17-online-lobbies-and-window-controls"
version: "2026.10.17"
description: "Use the expanded FICS workspace, pop out the Lichess lobby and appearance tools, and arrange resizable study windows."
---

## FICS workspace and online access

- Moved FICS access from the former FICS-Online menu to the bottom **FICS** button. The Free Internet Chess Server window groups **Console**, **Chat**, **Lobby**, **History**, **Watch**, and **Play** tabs under a Connection menu.
- Added guest and account controls, a **Remember handle** option, wrapped chat messages, and secondary conversation tabs. Passwords are not saved.
- Added a textured Lobby table with colored player types, a **?** legend, Finger information, and History lookups. History displays recent games in a table: **Preview** uses the moves player, while **Review** opens the archived game on the main board.
- Moved watchable games, opponent search, named challenges, and challenge acceptance into Watch and Play. Watch supports up to ten observed games and a raised **Stop watching all** button.
- Kept the local board, moves, and analysis intact when connecting to FICS until a played or watched game takes the board.
- Removed the Lichess-Online menu; the bottom **Lichess** button opens the lobby directly. Connection indicators show each service's state and username separately.

## Movable windows and pop-outs

- Made **Import Games**, **Board Theme**, **Background**, and **Lichess lobby** movable and resizable. Board Theme opens at a normal size instead of filling the workspace.
- Added raised **↗** and **×** controls to Board Theme, Background, and Lichess lobby, matching Training Profiles. Gold 3D borders, heavy paper grain, linen controls, and subtle-grain sections unify the appearance tools.
- Board Theme pop-outs apply selections to the main board; **Adjust** opens the controls in the main window. Background pop-outs share image controls and send opacity and gradient changes to the main window.
- Lichess lobby pop-outs share the main connection and game state. Matchmaking and game commands use that connection; profile and resignation controls open in the main window. Quick-pairing choices wrap as the lobby narrows.
- Increased the Lichess account name size, made the selected lobby tab gold and inactive tabs gray, and removed the bottom recessed frame while retaining Leave online play and Sign out.
- Added gold 3D borders to Lichess profiles and the lobby. Training Profiles supports moving, resizing, and a separate browser window, with game starts routed to the main board.

## Imports and engine settings

- Added **Tools → Enter a scoresheet…**, opening the same workspace as Import Games → Scoresheet.
- Restyled Import Games and scoresheet entry with textured backgrounds, gradients, and sunken panels. Kept titles outside the recessed sections.
- Removed horizontal scrolling from the Import Games tabs. The paste area grows with the window; Clear and Continue sit directly on the outer background.
- Made the active **Engine Settings** context tab a raised gold button with dark text. The status bar groups the engine name, Ready/Analyzing state, and a gray raised **Settings** button.

See [Features](/features/) and [Help](/help/) for current instructions. Browser pop-outs require permission and the main Ironwood window to remain open.

---
title: "A 3D board in the Ironwood workspace"
date: 2026-09-29T00:00:00-05:00
description: "How the new 3D board fits into play and analysis, and what desktop players should expect."
slug: "chess-in-two-and-three-dimensions"
---

Ironwood now offers a 3D board alongside its 2D board. Use the **View** menu or the 2D/3D button beside Flip Board to switch while keeping the same game, move list, and analysis tools in place. The 3D scene is drawn inside the egui workspace rather than opening a separate viewer.

The board uses a fixed perspective so the squares remain easy to follow. Scroll to zoom, and use the 3D appearance slider to adjust brightness, contrast, and gloss together. The marble board and textured pieces have lighting and soft contact shadows. Coordinates, highlighted moves, and Stockfish's suggested arrow are drawn in the 3D view too.

The surrounding game tools still matter in three dimensions. Beneath each player's name, icons show the pieces they have captured and a +score marks their material advantage. The display follows the selected position when you step through a game. The promotion chooser shows graphical choices. From the Game Moves menu, **Copy Board to Clipboard** renders the selected position in whichever board view is active, ready to paste elsewhere.

Ironwood is presently designed for desktop and laptop screens. Its browser renderer uses WebGPU, whose availability and performance depend on the browser and graphics device. The full Stockfish 19 engine is a separate 94.5 MiB download if you choose to store it for offline play. We have not measured a dependable minimum CPU, memory, or GPU specification; the [Features page](/features/) keeps the current compatibility guidance in one place.

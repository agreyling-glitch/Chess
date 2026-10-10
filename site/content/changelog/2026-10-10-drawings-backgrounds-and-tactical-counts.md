---
title: "Drawing tools, tactical counts, and background settings"
date: 2026-10-10T00:00:00-05:00
slug: "2026-10-10-drawings-backgrounds-and-tactical-counts"
version: "2026.10.10"
description: "Ghost pieces, new drawing shapes, Attack vs Defense bubbles, printed annotations, and a dedicated background settings window."
---

## Added

- **Attack vs Defense** in Tactical Map: red top-left bubbles count enemy attackers and green top-right bubbles count friendly defenders on every real piece in 2D, including zero counts. Counts update with the position, include pinned pieces, and respect blocked sliding attacks. They are geometric control counts rather than legal-capture or exchange evaluations. The overlay follows Tactical Map’s availability rules and is hidden in 3D.
- **Ghost pieces** in Draw on board, with White and Black piece symbols using the selected drawing color. Click a square to place a translucent annotation without changing the position; repeat the same piece and color to remove it. Ghosts support saving, editing, deletion, Clear, Undo, annotated board copies, and printed analysis. They remain flat symbols in the 3D view.
- **Black and white** drawing colors alongside green, red, yellow, and blue.
- **Translucent filled squares** and **crosses**, plus solid and dashed **L-shaped knight arrows**. Knight arrows follow the longer leg first and use board-aligned bends in 3D. Right-click while dragging toggles solid/dashed knight arrows.
- A blue **drawing indicator** in Game Moves, including drawing-only moves and positions with ghost pieces. Moves with notes and drawings show both indicators; hovering shows the drawing count.
- **Your notes and drawings** in printed analysis, including written notes, manual symbols, and diagrams for positions with drawings. Starting-position annotations and noncritical positions are included.
- **View → Background…** opens a modal with Gradient Only, Default, and Uploaded Image choices, a switchable uploaded-image library, and separate image opacity, gradient, and 3D backdrop sliders. PNG, JPEG, and WebP uploads are stored on this device.
- Built-in Ironwood background artwork with the circular emblem removed, derived from the owner-supplied knight image.

## Changed and fixed

- First launch now selects **Gradient Only**, with a **17% background gradient** and **0% 3D background opacity**. Returning users retain their saved settings. Upload/remove actions appear only for the uploaded-image option.
- Added subtle paper texture to the app background and adjustable image opacity. A transparent 3D backdrop reveals the app background while real board pieces remain opaque.
- Made Game Moves and engine prediction table backgrounds black, with prediction navigation controls inside the table. Reduced the balance bar width.
- Made the expanded analysis view translucent, with a more opaque lower details area.
- Drawing controls open in a taller window and share matching button sizes and column spacing.
- Analysis actions align left above the chart; Expand and Print analysis align right. Analyze again and Expand use gold styling, with additional spacing above the chart.
- Reduced padding, text size, and spacing in White and Black pawn-structure and king-safety panels.
- Prevented stale local engine WASM responses from being reused during development, addressing a truncated cached engine download that blocked real-time analysis.
- Fixed background texture caching that could leave the app blank.

## Help and site

- Updated drawing, notes, background, sharing, and Tactical Map guides, including the 2D-only Attack vs Defense overlay and geometric-count limitations.
- Added the drawing-controls screenshot to **Choose a color and shape** in Help.
- Refreshed Features with current analysis, annotation, background, and workspace capabilities.
- Added the journal post [Illustrating plans and counting defenders](/blog/illustrating-plans-and-counting-defenders/).

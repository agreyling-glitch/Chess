---
lastmod: 2026-10-14T00:00:00-05:00
title: "Board themes and appearance"
description: "Choose and customize 2D or 3D themes, with guidance on artwork downloads, Retry, and offline use."
category: "using-ironwood"
keywords: [theme, appearance, board colors, gradient, shading, outlines, shadows, 2D, 3D, hybrid, marble, wood, glass, texture, sand, complement, custom, loading, artwork, download, offline, cache, fallback, retry]
weight: 13
---

## Choose a theme

Open **View → Board → Theme…** to browse the **2D Pieces** and **3D Boards** tabs. The chooser always opens on **2D Pieces**, so browsing flat-board themes does not load 3D previews. Select **3D Boards** to load those previews. Switching tabs leaves your current board unchanged. Click a preview to apply a theme; double-click to apply it and close the chooser. Choosing a card also switches to its 2D or 3D board view.

The flat-board themes are System (Unicode), Cburnett, Merida, Royal Rascals, Undead Court, Classic Staunton, Neon Geometric, Art Deco Faceted, and Hybrid Marble. Several themes include coordinated square, frame, and coordinate colors:

| Theme | Board palette |
| --- | --- |
| Classic Staunton | Cream and walnut, with warm coordinates |
| Neon Geometric | Two navy shades, with cyan coordinates |
| Art Deco Faceted | Pale jade and deep teal, with gold coordinates |

These palettes follow the selected theme automatically and appear in the live board, theme previews, and copied board images.

The 3D themes are Marble, Wood, Glass, Art Deco, and Egyptian. Switch dimensions with the **2D / 3D** button above the board or the View menu. Right-click that button to open the theme chooser.

## Loading artwork and offline use

In the web app, some artwork downloads only when needed:

- **Hybrid Marble pieces:** the Marble, Wood, and Glass piece styles download when you use them or view their preview. Merida pieces keep the flat board usable while the selected style loads.
- **3D boards:** artwork downloads when you open a 3D preview or switch to a 3D board. Wood has its own download; the other 3D themes share artwork. The live board stays usable in 2D until the selected 3D artwork is ready, then switches automatically. Your position and selected 3D theme are preserved.

Connect to the internet the first time you use this artwork. After a complete download, it can be reused offline while it remains cached on this device. Installing Ironwood alone does not download every theme, and clearing browser data or the browser removing cached files can require another download.

If artwork is unavailable, reconnect and choose **Retry** beside its status in **Board Theme** or **Theme adjustments…**. For a 3D board, **Retry** also appears above the live 2D fallback board. You can keep playing with the fallback or choose another available theme. Wait until 3D artwork is ready before copying a 3D board image.

## Rotate, zoom, and reset the 3D board

With a 3D theme selected:

- **Hold the right mouse button and drag** on the board to rotate the view and change its tilt.
- **Hold Shift while right-dragging** to keep rotation fixed and adjust only the viewing height/tilt with vertical movement. Horizontal movement is ignored. You can press or release Shift during the drag.
- **Double-click the right mouse button** on the board to reset the camera view.
- **Hold the right mouse button and scroll the mouse wheel** over the board to zoom in or out.
- **Shift + right mouse button + mouse wheel** still zooms in or out, so you can adjust distance while keeping rotation locked.
- **Scroll without holding the right mouse button** to navigate game moves, or the selected engine prediction, one move at a time.

Rotation, reset, and zoom change only your view. Normal scrolling changes the displayed position. These controls also work in focus mode.

## Hybrid Marble: flat board with 3D pieces

Choose **Hybrid Marble** in the **2D** tab for a flat, square board with shaded pieces rendered from a more top-down 3D view. The pieces rise above their square centers. Board clicks, move arrows, drawings, and [tablebase previews](/help/syzygy-tablebases/) work as they do on the other flat boards. Hybrid Marble uses fixed piece images; use a full 3D theme when you want to rotate or zoom the camera.

Open the Hybrid Marble card's **gear**, or **View → Board → Theme adjustments…**, to customize it:

| Setting | Choices and behavior |
| --- | --- |
| **3D piece style** | Marble, Wood, Glass, Art Deco, or Egyptian. Changes the pieces while keeping the board flat. |
| **Board theme: Complement** | Chooses square colors to suit the piece style. Switching styles updates the palette: Wood uses warm tones, Glass uses cool tones, and the other styles use their coordinated palettes. |
| **Board theme: Custom** | Shows separate **Light squares** and **Dark squares** color pickers. Changes appear immediately. Switching piece styles while Custom is active keeps those colors. |
| **Board texture** | Smooth, Marble veins, Wood grain, Linen, Slate, Glass, or Sand. Textures overlay the selected square colors, so they work with both Complement and Custom. |

**Smooth** is flat and untextured. The other textures add beveled edges and soft lighting for depth. Glass adds reflections; Sand adds fine grain. A Glass board texture and Glass pieces are independent choices—you can combine any piece style with any board texture.

The usual flat-board size, side-color, gradient, outline, and shadow controls also apply. Your selected piece style and Hybrid Marble appearance settings are saved on this device. The hybrid styles share one set of appearance adjustments. Copied board images include the selected piece style, square colors, and texture.

## Adjust one theme

Click a theme card’s **gear** to select it and open its adjustments. You can also use **View → Board → Theme adjustments…** for the current theme. Changes appear immediately and save separately for each theme on this device. Switching themes restores that theme’s settings. **Reset this theme** restores only the selected theme’s defaults.

## 2D adjustments

- **Piece proportions:** change piece size.
- **Piece colors:** enable custom colors and choose White and Black colors separately, or use a preset.
- **Gradient colors:** enable a gradient and choose separate start and end colors for each side. Choose top-to-bottom, left-to-right, diagonal, or center-outward direction. Warm ivory, Ice & plum, and Copper & teal presets provide starting points. Gradients replace the solid side colors while retaining the artwork’s shading.
- **Depth & outlines:** increase **Inner shading** to add rounded volume inside illustrated pieces. At 0%, no extra shading is applied. **Contrasting piece outlines** adds a smoothed border to help pieces stand out.
- **Piece shadows:** enable shadows below pieces, then adjust strength and softness.

Gradient colors and inner shading are available for illustrated piece sets; System uses font glyphs. There is no separate 2D preview in the adjustments dialog: use the live board to judge changes. Processed piece images are cached so normal board redraws reuse them.

## 3D adjustments

- **Piece proportions:** adjust size and height together.
- **Colors & material:** choose separate side colors and a Theme default, Matte, Satin, Glossy, Metallic, or Glass finish. Glass gives a translucent appearance.
- **Piece shadows:** enable shadows and adjust strength and softness.
- **Board lighting:** use **3D appearance** to adjust gloss, brightness, and contrast together. Enable or disable the radial light below the board here.

Contrasting outlines are available only in 2D. The 3D appearance and radial-light controls are in the adjustments dialog rather than the Board menu.

## Preview cards

The chooser groups its tabs and theme previews in sunken sections. The active theme has a gold outline and **Selected** label. Choose **Adjust** on a card to apply that theme and open its appearance controls. Hybrid pieces render in front of the board rim, including pieces extending above the back rank.

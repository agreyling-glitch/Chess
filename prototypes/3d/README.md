# Ironwood 3D board

Ironwood's **View → Show 3D board** switches the in-game board inside egui.
The Rust game supplies the same legal moves and position updates as the 2D view,
including Stockfish and online-game paths. The in-game renderer draws the board
with wgpu inside egui; the CPU renderer handles PNG export and fallback. The browser
loads the in-game Wood and shared Marble/Glass asset packs on demand, with a usable
2D board while loading and a separate cache for offline reuse. Native builds keep
the assets embedded. The renderer remains part of the application.

Run `npm run dev:3d`, then open <http://127.0.0.1:8080/3d/>. The build goes
to `web/3d/`, which is ignored by Git and generated during normal and Cloudflare
builds. `npm run build:3d` builds without starting the server.

The `/3d/` route remains a standalone FEN inspection page: enter a position,
drag to orbit, scroll to zoom, or flip the viewpoint. It does not alter the game.

The six piece models come from `assets/3d/polyhaven-chess-set/` (CC0 1.0).
Three.js is pinned in `package-lock.json` (MIT). The build includes its MIT
license notice. The standalone viewer is excluded from PWA installation downloads.
With Ironwood's service worker active, a complete online visit caches the viewer's
files for offline revisits. Revisit online after an app update or browser cache
clearing. The viewer uses the existing manifest but does not register the worker
itself.

# Ironwood 3D board

Ironwood's **Tools → Show 3D board** switches the in-game board inside egui.
The Rust game supplies the same legal moves and position updates as the 2D view,
including Stockfish and online-game paths. The in-game renderer rasterizes the
board on the CPU, then displays it through egui's wgpu renderer. The models and
renderer are included in the existing PWA offline shell.

Run `npm run dev:3d`, then open <http://127.0.0.1:8080/3d/>. The build goes
to `web/3d/`, which is ignored by Git and generated during normal and Cloudflare
builds. `npm run build:3d` builds without starting the server.

The `/3d/` route remains a standalone FEN inspection page: enter a position,
drag to orbit, scroll to zoom, or flip the viewpoint. It does not alter the game.

The six piece models come from `assets/3d/polyhaven-chess-set/` (CC0 1.0).
Three.js is pinned in `package-lock.json` (MIT). The build includes its MIT
license notice. The 3D page uses Ironwood's existing manifest and service worker.

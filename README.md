# Ironwood Chess

A browser-first chess game built with Rust/WASM and egui. Legal move generation,
castling, en passant, promotion, checkmate, and stalemate are handled in Rust.
Stockfish 19 runs behind a dedicated worker boundary; its full NNUE WebAssembly
build uses SharedArrayBuffer, Atomics, and pthread workers when the page is
cross-origin isolated.

## Run

```powershell
npm install
npm run build
npm run dev
```

Open <http://127.0.0.1:8080>. Production hosting must send these headers on every
document, worker, JavaScript, WASM, and NNUE response:

```text
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Resource-Policy: same-origin
```

To use Cloudflare Wrangler instead, build once and start its asset server:

```powershell
npm install
npm run build
npm run dev:wrangler
```

Open <http://127.0.0.1:8788>. `wrangler.jsonc` serves a generated app-shell
directory, while the launcher provides the 94.5 MiB full-NNUE engine through a same-origin local
proxy. This split is necessary because Cloudflare Workers rejects individual
static assets larger than 25 MiB. `web/_headers` applies the isolation headers
needed by Stockfish pthreads. The localhost engine stream runs at full speed and
uses normal browser caching, matching the production delivery behavior.

The board/game state is intentionally separated from rendering so 2D glyphs can
later be replaced with timeline-driven Battle Chess animations without changing
the legality model or engine protocol.

## Saved games

The browser automatically saves the current FEN, undo history, last-move
highlight, board orientation, and engine preference in local storage after each
change. Reloading `/play/` or returning from the landing page restores the game;
if a saved position is waiting for Black, Stockfish resumes after initialization.
`New game` replaces the saved state with the initial position. Storage is local
to the browser and origin, so `localhost` and `127.0.0.1` keep separate games.

## Stockfish license

The bundled Stockfish.js 19 engine is GPLv3 software and remains separate from
the proprietary Ironwood Chess application. `npm run engine` copies the full
GPLv3 text from the pinned npm package into the deployed site. Public attribution,
license, and exact corresponding-source links are available at
`/open-source-notices.html`. Do not remove that page, its links, or the deployed
license file when publishing the application.

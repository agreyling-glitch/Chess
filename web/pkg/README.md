# Ironwood Chess

A browser-first chess game built with Rust/WASM and egui. Legal move generation,
castling, en passant, promotion, checkmate, and stalemate are handled in Rust.
Stockfish 19 runs behind a dedicated worker boundary; its full NNUE WebAssembly
build uses SharedArrayBuffer, Atomics, and pthread workers when the page is
cross-origin isolated.

## Import from Lichess

In the browser app, choose **Game → Import…** and enter a Lichess username
or an `https://lichess.org/…` game link, then click **Fetch games**. Username
imports fetch the latest 20 completed games without requiring login. Click
**Continue** to choose games in the existing batch importer, which skips
duplicates. A single game opens for review; enable **Analyze the full game
after import** to analyze it immediately. Imported games remain available in
Saved Games for later review and analysis.

Fetching requires an internet connection. Ironwood makes one Lichess request
at a time and waits at least one minute after a rate-limit response. Live
games cannot be imported. No additional libraries or Lichess assets are bundled.

## Play on Lichess

Choose **Game → New game → Online → Lichess**, or open **Lichess-Online → Lichess
lobby and game controls…**. Sign in on Lichess to grant the Board API and
challenge permissions. Ironwood uses OAuth with PKCE; it never asks for your
Lichess password. The token stays in session storage, separate from saved games
and backups. **Sign out** revokes the token. **Leave online play**
disconnects streams while preserving authorization; it does not resign or abort the game on Lichess.

The lobby supports rated or casual standard chess, rapid/classical matchmaking,
correspondence seeks, direct player challenges (including blitz), Lichess AI games, incoming
challenge acceptance/decline, and resuming ongoing games. Unsupported variants
have a link to play on Lichess. Correspondence seeks remain on Lichess until
matched; the cancel button applies to real-time matchmaking.

Play on Ironwood's existing 2D or 3D board. Pending moves appear immediately as
translucent ghosts and become solid when Lichess confirms them, with streamed
clocks and full-history recovery after a reconnect.
The Game Analysis panel becomes Game chat during active Lichess play. Use the controls below the board, the Game menu, or the Lichess lobby for draw offers, takeback requests, aborting,
claiming a draw/victory when permitted by Lichess, and resigning. Finished games
are saved on this device and become available for analysis. Engine assistance
and analysis are disabled while an online game is engaged; signing in or browsing the lobby leaves local analysis available. Leaving an unfinished online game
clears its position from the local workspace; reconnect to resume it. Correspondence can instead be put aside without disconnecting to load a saved game or resume another game. Refreshing does not automatically engage existing ongoing games.

Computer setup supports Standard, Chess960, and From Position with standard-chess FEN, Unlimited/Real time/Correspondence timing, strength 1–8, and side selection.

This integration is original Ironwood code calling the [public Lichess
API](https://lichess.org/developers). It bundles no Lichess source, SDK, artwork,
or engine assets, and introduces no new licensing dependencies. API details and
Board restrictions are documented at <https://lichess-org.github.io/api/>.

Validate with `node --test tests/*.test.mjs`, `cargo test --lib`, and
`cargo check --lib --target wasm32-unknown-unknown`. OAuth/stream tests use fake
accounts and transports; a live account sign-in requires the user to approve
the Lichess authorization screen.

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

For Cloudflare Workers Builds, use `npm run build:cloudflare` as the build
command and `npx wrangler deploy` as the deploy command. The compiled application
WASM in `web/pkg` is a committed deployment artifact, so HTML, CSS, Worker, and
other static-only changes do not recompile Rust in CI. The CI build validates
the required artifacts and prepares `.wrangler-assets` for Wrangler.

After changing Rust code, run `npm run build` locally and commit the refreshed
`web/pkg` files along with the source change. This makes the compiled artifact
reviewable and ensures Cloudflare deploys exactly the build that was tested.

The public journal, changelog, and help center are generated by Hugo from `site/content`.
Help articles are Markdown files in `site/content/help/`. Give each new article a
title, description, category (`analysis`, `terms`, or `using-ironwood`), keywords,
and weight in its front matter. `npm run build:site` builds these pages in isolation;
the Wrangler preparation step merges them under `/blog/`, `/changelog/`, and
`/help/` without replacing the hand-built
landing page. Cloudflare Workers Builds includes Hugo Extended; pin its version
with the `HUGO_VERSION` build variable when reproducible framework builds are
required. Once the Help → Contents link is in the game, article-only changes
need a site build and deployment, with no WASM rebuild. Each article has a direct
HTML URL in the sitemap so search engines can index its text.
If `npm run dev:wrangler` is already running, use `npm run refresh:site` after
editing Markdown. It rebuilds Hugo and refreshes `.wrangler-assets` without
rebuilding the game WASM or the position-editor model. Reload `/help/` to see
the new search index.

Production runtime variables are declared in `wrangler.jsonc`, because Wrangler
deployments treat the repository configuration as authoritative and replace
dashboard-only variables. `ENGINE_ORIGIN` points at the public R2 origin that
stores the versioned Stockfish engine files.

The board/game state is intentionally separated from rendering so 2D glyphs can
later be replaced with timeline-driven Battle Chess animations without changing
the legality model or engine protocol.

The in-game 3D board uses bundled CC0 Poly Haven, Omie, and Polyy.AI chess assets inside the
same egui canvas as the rest of the workspace. Choose **View > Show 3D board**
or use the 2D/3D button beside Flip Board to switch views; the board, legal moves,
clocks, and other controls stay in place. The game board uses a fixed perspective;
scroll over it to zoom. Choose Marble, Wood, Glass, Art Deco,
or Egyptian in **View > Board > 3D theme**.
In Game Moves, **Copy Board to Clipboard** captures the
selected position as a 2D or 3D image according to the active view.
The in-game view draws the 3D board with wgpu inside egui. A CPU renderer
remains available for PNG export and renderer fallback. The separate `/3d/` FEN viewer
is generated by `npm run build:3d` and uses Three.js.

## Play online through FICS

Open `/play/`, choose **New game → Online**, and use the FICS lobby window to seek
an unrated game, join a listed game, or challenge a FICS handle. Ironwood connects
as a guest by default. The lobby also lets you sign in to an existing FICS account
or open the [official FICS registration page](https://www.freechess.org/Register/).
FICS confirms each move and sends the authoritative board and clocks.
When a game ends, Ironwood keeps its moves for local Stockfish analysis and saves
the finished game on this device. Active online games require a connection and
are not stored in the Saved Games library until they finish.

The browser connects over a same-origin WebSocket to the Cloudflare Worker, which
opens a TCP connection to `freechess.org:5000`. Use `npm run dev:wrangler` for a
local end-to-end connection; `npm run dev` serves the page but does not provide
the WebSocket bridge. The FICS service and its availability are independent of
Ironwood. The FICS TCP hop uses its legacy unencrypted protocol. Ironwood does
not save the sign-in password, and you should use a unique FICS password.

## Saved games

The browser automatically saves the current FEN, undo history, last-move
highlight, board orientation, and engine preference in local storage after each
change. Reloading `/play/` or returning from the landing page restores the game;
if a saved position is waiting for Black, Stockfish resumes after initialization.
`New game` replaces the saved state with the initial position. Storage is local
to the browser and origin, so `localhost` and `127.0.0.1` keep separate games.

## Progressive Web App

`web/service-worker.js` makes the landing page, game shell, application WASM,
journal, and changelog available offline. The 94.5 MiB Stockfish engine is kept
in a separate versioned cache and is downloaded only when the player chooses
**Enable offline play** on the landing page. The same control removes that
offline engine copy. Bump `SHELL_CACHE` whenever a release needs to invalidate
all previously cached shell assets; versioned engine upgrades must use a new
engine cache name and new asset filenames.

## License

Ironwood Chess's original application code, interface, and project artwork are
licensed under the [GNU General Public License, version 3 or later](LICENSE)
(`GPL-3.0-or-later`). The corresponding source is available in this repository.

Bundled third-party components retain their own licenses. These include
Stockfish.js (GPLv3), the Cburnett and Merida chess pieces (GPLv2 or later),
the Rust logo (CC BY 4.0), Fenshot (MIT), and ONNX Runtime Web (MIT). See the
[open-source notices](web/open-source-notices.html) for attribution, license
texts, and source links. `npm run engine` copies the Stockfish license from the
pinned npm package into the deployed site. Keep the notices, licenses, and
corresponding-source links with published builds.

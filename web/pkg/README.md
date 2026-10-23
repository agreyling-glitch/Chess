# Ironwood Chess

A browser-first chess game built with Rust/WASM and egui. Legal move generation,
castling, en passant, promotion, checkmate, and stalemate are handled in Rust.
Stockfish 19 runs behind a dedicated worker boundary; its full NNUE WebAssembly
build uses SharedArrayBuffer, Atomics, and pthread workers when the page is
cross-origin isolated.

## Syzygy endgame tablebases

In Game Analysis, select **Tablebases** and **Look up this position** to query
Lichess for a standard position with up to seven pieces (including kings) and no
castling rights. Clicking a continuation previews the move and automatically
looks up the resulting position, using cached results when available.

The **Exploration tree** keeps the moves you explore in a branching hierarchy.
Click a node to revisit its board position; select a different continuation to
add a sibling branch. Back and Reset preview preserve branches; **Clear** removes
the tree.
Click the small triangle beside Game position or a move with children to collapse
or expand its continuations. This preserves branches and makes no API request.
Selecting a node automatically looks up its position, using cached
results when available. Once the tree has branches, **Look up this position**
is hidden; Back and Reset preview also look up their destination automatically.
Clearing the tree restores the initial lookup button. Automatic lookups never
force a refresh.
Results describe the side to move; move rows describe the player making that move. Cursed wins and blessed
losses are shown as draws under the 50-move rule. DTZ measures plies to a pawn move,
capture or mate and may be rounded; it is not generally a distance to mate.
DTM is shown only when provided.
The root 50-move clock defaults to the recorded game history and can be edited;
check it for imported setup positions, whose starting counter Ironwood does not
retain. Preview moves update that counter, resetting it on pawn moves or captures.
The result card shows **Lichess API** for a fresh request or **Cached result** when
revisiting a position already held in Ironwood's memory. **Look up this position**
uses the cache when available. Inside **Position settings & API details**,
**Refresh tablebase result** bypasses that cache and requests a fresh result.
**Open this position on Lichess**, in the same settings section, opens the selected
game or exploration position in a new tab with its FEN and move counters populated.

Each lookup sends the position FEN to `https://tablebase.lichess.ovh/standard`.
No sign-in is needed. Requests are serialized, cached in memory (up to 100
positions), time out after 15 seconds, and pause for at least a minute after a
rate limit. Offline or missing-table errors can be retried. Tablebases are
unavailable during active training or FICS play; Lichess play uses its game-chat
panel instead of Game Analysis. These results do not replace Stockfish scores.

Licensing: this is original Ironwood client code accessing the
[public HTTP API](https://lichess.org/developers), with the endpoint and response
documented by [lila-tablebase](https://github.com/lichess-org/lila-tablebase#http-api).
No server code, probing library, or tablebase files are copied or distributed.
The server's AGPL-3.0-or-later license covers that server code; this integration
adds no third-party code or asset license to Ironwood's distribution.

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
challenge and follow permissions. Ironwood uses OAuth with PKCE; it never asks for your
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

For a release, run `npm run release:prepare` before the release checks and commit
its updated data and build artifacts. This downloads the latest Lichess opening
snapshot, verifies its CC0 licence and dataset declaration, checks all five TSV
files and every opening's legal moves, then runs the full browser build. The
upstream commit, per-file row counts and SHA-256 hashes are recorded in
`assets/openings/SOURCE.json`; the app continues to classify openings offline.
A failed refresh stops preparation and preserves the existing snapshot. A loss
of more than 20% of a volume's rows also stops preparation for review.

`npm run openings:update` refreshes only the data. To reproduce a prior snapshot,
use `npm run openings:update -- --revision <full-upstream-commit>`, then rebuild.
Repeating an unchanged update does not rewrite its provenance timestamp. Routine
WASM-only development remains offline with `npm run build:wasm`.

For Cloudflare Workers Builds, use `npm run build:cloudflare` as the build
command and `npx wrangler deploy` as the deploy command. The compiled application
WASM in `web/pkg` is a committed deployment artifact, so HTML, CSS, Worker, and
other static-only changes do not recompile Rust in CI. The CI build validates
the required artifacts and prepares `.wrangler-assets` for Wrangler.

After changing Rust code, run `npm run build` locally and commit the refreshed
`web/pkg` files along with the source change. This makes the compiled artifact
reviewable and ensures Cloudflare deploys exactly the build that was tested.

Marble, Wood, and Glass 2D piece images load on demand. The default pieces remain
embedded, and Merida pieces keep the board usable while an optional theme loads
or is unavailable offline. Once downloaded, theme images use a separate browser
cache that survives app updates; browsers can still evict cached data.
`npm run build:piece-themes` generates immutable copies and a manifest under
`web/piece-themes/` from the existing CC0 sprites in `web/pieces/`. It runs before
the WASM build, local development server, and Wrangler asset preparation. These
generated copies are ignored by Git; keep the original sprites and provenance.

The browser also loads the CC0 Wood and shared Marble/Glass 3D models and textures
on demand. The theme chooser opens on its 2D tab; 3D previews load only when the
3D tab is selected. Art Deco and Egyptian use the shared Marble board assets. A usable 2D
board remains visible while a required 3D pack loads or is unavailable; Retry
loads it again after a failed request. Complete packs are cached separately from
the app shell for offline reuse across updates, subject to browser cache eviction.
`npm run build:board-themes` creates content-hashed assets and a manifest under
`web/board-themes/` from the originals in `assets/3d/`. It runs with the same build,
development, and deployment hooks as the 2D packs. Native builds keep embedded
assets. Both generated directories are ignored by Git.

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

The in-game 3D board uses Poly Haven, Omie, and Polyy.AI chess assets inside the
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

The wgpu renderer caches the board and glow independently from each square's
piece and shadow geometry. Moves and history navigation rebuild and upload only
changed squares; selection and move highlights have their own small buffer.
GPU geometry skips the CPU vertex-lighting calculations because the shader
supplies lighting. Previews, PNG export, and the CPU fallback retain CPU lighting.
Theme, camera rotation, size, and appearance changes still rebuild geometry.
Zoom uses the same shader adjustment for retained and updated geometry. To
compare warm CPU scene construction with highlight updates, run
`cargo test --release --lib benchmark_full_scene_rebuild_versus_highlight_updates -- --ignored --nocapture`.
For moves and history navigation, run
`cargo test --release --lib benchmark_full_scene_rebuild_versus_board_updates -- --ignored --nocapture`.
These benchmarks compare full reconstruction with cached updates and report
median CPU construction times and vertex bytes; they exclude GPU drawing and
command encoding, so they do not measure frame rate. Separate square buffers
reduce uploads at the cost of more draw calls, especially with outlines enabled.

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

The standalone `/3d/` viewer downloads only when opened. With Ironwood's service
worker active, a complete online visit caches its files for offline revisits.
After an app update or browser cache clearing, visit it online again.

The optional built-in background image is also excluded from installation
downloads. Selecting or restoring **Default** in **View → Background…** loads
the image and uses the normal runtime cache for offline reuse. It may need an
online load again after an app update or browser cache clearing. Uploaded
backgrounds remain in their separate IndexedDB library.

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


### Engine worker startup

`npm run engine` copies the pinned Stockfish.js package and applies the GPLv3 glue patch in
`scripts/stockfish-thread-pool.mjs`. It prepares the supported pthread pool asynchronously
before applying thread settings, avoiding a synchronous thread-resize stall. The patched
JavaScript ships with the app as `stockfish-19-ironwood-pool-v1.js`, so production cannot
reuse the old glue from the separate engine origin or HTTP cache. The WebAssembly
engine and NNUE network are unchanged. The adapter rejects an unexpected engine layout;
review it when upgrading Stockfish. Run `node --test tests/engine-worker.test.mjs` to validate
the startup handshake and worker-loading barrier.

### Optional PostHog analytics

Analytics require explicit acceptance and are disabled on localhost and preview hosts by default.
Localhost and `127.0.0.1` enable the statistics choice automatically, without a special URL parameter. Open `http://127.0.0.1:8788/play/`, then choose **Allow statistics**. Unanswered or declined consent sends no events.
Test events go to the configured project with `environment = local_test`; filter production dashboards to `environment = production`.
Use **Analytics settings** to decline or allow again. Do not use a personal API key in frontend code.

Rebuild analytics with `node scripts/build-analytics.mjs` (also runs during `build:site`).
The Apache 2.0/MIT SDK is pinned in package-lock.json and notices are copied into web/licenses.
Run `node --test tests/analytics.test.mjs` to check consent and context filtering.

Events: `$pageview`, `app_opened`, `game_started` (local new games), `game_analysis_started` (full-game analysis),
and `active_usage` with `active_seconds`. Active time requires a focused, visible app with input in the last two minutes.
For active minutes, sum active_seconds / 60, grouped by PostHog session ID; average or take the median across sessions.
Unique visitors represent browsers that accepted analytics, not all people. Blocked/offline activity is not queued by Ironwood.
Session replay, automatic interaction capture, surveys, performance capture, and exception capture are disabled.
Maintain separate Production and Local Test dashboards. Use `environment = production` and
`environment = local_test` respectively, including filters embedded in SQL queries. Use separate
copies of insights so changing a test chart does not change the production chart.

### Lichess player profiles

Click your account summary in the connected status bar, or **Player profile** in the signed-in Lichess lobby. Load your account or any Lichess username. The profile shows public biography, ratings, total/rated games and wins/losses/draws, daily history by time control, and the latest 12 completed games with final-position boards and links to Lichess. Graph filters offer 1M, 3M, 6M, YTD, 1Y and ALL; hover to inspect a date and rating. Narrow windows stack the biography below the graph.

Profile, history and games load on demand using the existing sign-in, remain in memory for the session, and can be refreshed. Viewing another player does not change your account or board. Missing profile fields and unavailable time controls are shown explicitly; successful sections remain visible if a later request fails. Public profile browsing adds no dependencies or artwork; following actions use the permissions described below.

### Saved-game move counts

Games enter Saved Games after their first move; a zero-move board can still be restored as your current session. Existing library entries are retained. Use **Sort → Fewest moves first** or **Most moves first** to order filtered results before pagination. **Recently saved** restores the default order. The sort choice is remembered on this device. Move counts follow the existing library convention: each move by either player counts once (half-moves).

### Saved puzzle move history

**Saved Games → Puzzles** contains puzzle move lines, with one saved record per attempt. Correct moves, automatic replies, and revealed solutions update that attempt; loading a new puzzle or retry starts another. Empty attempts are not added. Entries can be opened, previewed, sorted, favorited, exported, and deleted like other saved games. Older entries identified as Lichess puzzles also appear in this category. Backups preserve the category. Deleting a saved puzzle line does not change puzzle progress counts or Review mistakes, which remain in separate device-wide puzzle history.

### Following Lichess players

Open **Lichess-Online → Player profile…**, then expand **Following** to search the players followed by your signed-in account and open their profiles. **Follow** and **Unfollow** update your actual Lichess list. The profile window title includes the player’s name and account ID. New sign-ins request `follow:read` and `follow:write`; older sessions can use **Sign in again for follow access** if Lichess reports missing permissions. Requests use the existing session-only token and add no dependencies or assets.

Player profiles support multiple independent windows for comparison. Use **Load in new window** with the desired username, or open **Lichess-Online → Player profile…** again. Each window keeps its own player, graph filters, size, and position, with a fluorescent green border. Record counters wrap as complete blocks on narrow windows. Follow status stays synchronized across open profiles.

Profile biography and website links open in new tabs. The FIDE rating opens the linked FIDE player profile when Lichess provides an ID, or the FIDE ratings directory otherwise. Click a recent-game mini board to use the Saved Games move viewer directly from memory, with step controls and autoplay. **Review on Ironwood** closes profile overlays and opens a temporary analysis session without adding the game to Saved Games; **Game → Save current game** explicitly saves the full reviewed game, even from its starting position. Starting or importing a normal game resumes normal saving. Active online play and training must finish first.

### Expanded engine prediction windows

The gray raised **Expand** button beneath Engine prediction opens a snapshot with one mini board after each predicted move. Each title includes variation, evaluation, first move, and depth. Open multiple snapshots for comparison; each has its own size, position, maximize/restore state, and captured line. Boards scale to available space and scroll when needed. Clicking a board navigates the captured prediction when active play permits it. Gold borders and a shared sunken panel with stronger procedural paper grain match the bottom Full Game Analysis details panel; profile rating graphs use a subtler grain in their own sunken section.

### Player-profile companion windows and activity

Use the raised **↗** button beside Close in a player profile’s header to move it into a separate browser window. Pop-outs retain player, time-control, range, and performance selections and fill the browser area; in-app profile headers remain draggable. The main window owns the Lichess connection and publishes profile data to its same-origin companions. Profile requests are forwarded to that connection, and **Review on Ironwood** queues a review on the main board with its normal active-play guards. Keep the main window open. Blocked pop-ups leave the original profile available.

Rating chips open detailed performance, including records, peaks, streaks, and up to five best wins/worst losses. Notable games use a batch export request for browser-compatible previews. Recent games can be collapsed. Activity loads on demand with daily games, puzzle results, and rating changes, cached per profile window.

### Expanded game graph

The expanded evaluation graph supports hover/click inspection, Previous/Next, a position slider, and wheel navigation (down advances, up goes back). Two capture-event rows use 34px Merida icons regardless of the board theme. The framed position preview has coordinates, follows orientation, and renders saved position drawings plus the main board’s applicable best-move arrows. The graph, details, notes, and preview have spaced sunken sections. Input reads occur outside egui data locks; a regression test exercises graph opening after the freeze fix.

### Workspace and documentation

Fine linen and procedural paper grain distinguish recessed study surfaces without adding third-party artwork. Engine predictions use plain table headings and a centered gray Expand button; snapshot boards have frames and coordinates. New Game groups setup by mode and supports vertical resizing without growing horizontally when Training vs AI is selected. Saved Games controls, Notes, Draw on board, Tactical Map, engine settings, and training profiles use the shared visual treatment.

Puzzles now synchronize with the **main board**, using its appearance. Played attempts are saved in the Puzzles library category separately from device-wide progress. Save a temporary profile review explicitly before switching to a puzzle; reopen saved games from the library afterward.

The Help center supports article-text search with multiple terms, topic filters, quick links, and per-article contents. Dedicated guides cover player profiles, expanded graphs, and recovery. Features includes navigation across the current workflows. Optional analytics still excludes automatic freeze reporting; discussing a watchdog did not enable one.

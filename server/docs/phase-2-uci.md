# Phase 2: UCI engine bridge

Companion runs owner-installed native UCI executables and bundles no engines or
networks. The private-preview Companion flag gates the engine UI.

In **Game → Companion engines → Add engine**, select an engine folder and its executable
from the list shown on entry, wait for automatic UCI validation, then click
**Save engine**. The engine's UCI name is detected automatically and can be edited
before saving. Optional arguments remain in Advanced as a JSON array. Changing
arguments invalidates the result and requires validation again. No second terminal,
approval phrase, or five-minute approval deadline is involved.

The authenticated `GET /api/v1/engine-files?path=...` endpoint lists folders and
executable candidates, not file contents. On all platforms it starts inside
the data directory's `engines` folder and cannot browse or register outside it.
There is no drive picker or arbitrary folder input. Selecting an engine posts to
`/api/v1/engine-registrations` and starts the bounded UCI/isready handshake. Poll
`/{id}` for `validated` or `failed`; POST `/{id}/save` with an optional `label`
persists the validated executable and adds it to the live catalog without restart.
Validation alone never changes config.json. Closing the dialog cancels its request.
Failed validation, cancellation and concurrent configuration edits prevent saving.
Pending validation results are memory-only, bounded to 16, and lost on restart.

The owner credential authorizes browsing and launching installed executables on
the Companion host. Existing authentication, origin restrictions, process limits,
and handshake deadlines apply. No executable uploads or shell expansion are used.
This is an owner administration operation, not a sandbox for untrusted engines.

For Docker, mount Linux engines read-only at `/data/engines`, for example
`./engines:/data/engines:ro`. The picker starts there and cannot browse or register
executables outside it, including through traversal or symlinks. Set the server
process environment `IRONWOOD_ENGINE_DIRECTORY` to use a different mounted root.
The mounted files must be executable and readable by container UID 10001, and
must have runtime dependencies compatible with the image. The sample Compose
configuration includes this mount; no engines are distributed with Companion.

The new flow requires an updated Companion binary. Older binaries return an error
for this endpoint. Manual configuration remains available: stop Companion, add an
`engines` array, then restart:

```json
{
  "id": "my-engine",
  "label": "My installed engine",
  "executable": "C:/ChessEngines/engine.exe",
  "args": [],
  "options": {}
}
```

This object is one entry in `engines`, alongside the existing listen, origin and
token configuration. Relative executable paths resolve against the configuration
directory. Arguments are separate strings; no shell expands them. Engine IDs,
paths and installation are owner-local; analysis requests select a registered ID.
Engine executables and any networks must have suitable explicit licenses.

On Linux use a Linux executable with compatible runtime dependencies. Docker
requires an explicit read-only engine mount (for example `./engines:/data/engines:ro`)
and the executable's container path. A Windows executable cannot run in this
Linux container. Container resource limits remain recommended for owner engines.

Optional `engine_limits` defaults are: `max_processes: 4`, `handshake_ms: 3000`,
`stop_ms: 1000`, `max_search_ms: 300000`, `max_threads: 8`, `max_hash_mib: 1024`.
Even infinite searches have the server's wall-clock deadline. Process limits are
global; each authenticated socket owns at most one engine. Advertised Threads,
Hash and MultiPV ranges are bounded by server limits. The authenticated owner may
edit advertised spin/check/combo/string options in Engine settings → Engine options.
Values are checked against UCI types/ranges, with no newlines or command injection.
Strings (including engine-local file paths) are owner administration, like engine
registration. Button actions use the typed idle-only `set_option` message and are
never replayed as search overrides. Pondering stays disabled; Chess960 is controlled
by the position request. Browser Stockfish's bundled-file options remain managed.

Full-game budgets support Nodes or Seconds (1–60 per position), remembered by
engine identity. New Companion-engine preferences default to 5 seconds; built-in
Stockfish retains node quality presets. Timed verification uses five times the
first-pass time (up to 300 seconds). Pause/resume, game saves and JSON exports retain
the budget kind and verification milliseconds. The existing per-engine verification
checkbox still controls automatic pass two.

In Ironwood open **Game → Companion engines**, enter the server address and owner
credential, connect, select an engine, discover options, and apply it. The shared connection can optionally be remembered on this device. Built-in Stockfish remains
available without Companion. Rated training continues to use built-in Stockfish;
provider switching is blocked during active training, online play and full-game
analysis. Changing engines/options clears cached analysis but preserves moves and
notes. Reconnecting the same provider preserves completed game analysis.

Disconnects pause engine work; they never silently switch engine or replay moves.
Reconnect explicitly through this dialog, then continue play or analysis. Output
is correlated by session, search and position; cancelled/stale results are ignored.
The server validates FEN, move history, PV moves and best moves with Shakmaty.
Startup, cancellation, disconnect and shutdown bound process cleanup; stderr is
bounded and private. These controls are not a sandbox for untrusted executables.

## Validation

```sh
cargo build --locked -p ironwood-server
cargo test --locked -p ironwood-server
cargo clippy --locked -p ironwood-server -p ironwood-protocol --all-targets -- -D warnings
node --test tests/companion-server.test.mjs tests/companion-lifecycle.test.mjs tests/companion-uci.test.mjs tests/companion-engine-adapter.test.mjs
cargo test --locked -p battle-chess --lib
npm run build:wasm
npm run refresh:site
```

The original Node fixture tests handshake/partial lines, option bounds, concurrent
clients, cancellation, crash, illegal best moves, oversized output, stderr floods,
timeouts and child cleanup. Adapter tests cover stale identities, duplicate best
moves, cancellation ordering and failure without replay. Windows browser testing
on a separate localhost origin verified discovery, options, engine selection and
analysis (+0.23, depth 7 from the deterministic fixture), and casual play applying
exactly one legal reply to e4. Malformed/illegal fixture output fails closed.

Linux Docker execution, real owner-installed engines, clean-machine packaging,
and deployed HTTPS-to-loopback browser permissions remain release acceptance
checks. No public package/image has been published for this phase.

# Adaptive Archive import pacing validation

Validated 2026-10-23 on Windows, Chromium 155. Private-preview implementation record; not public release notes.

Removed the one-second batch timer. Authenticated Companion responses expose the operations budget and whether an engine session is active. Imports reserve 25% of the operations budget (except a one-request budget), wait for the window reset when necessary, and allow one import request in flight per client. Active engines receive a 200–1000 ms pause based on prior import latency. Older servers without feedback use a conservative 120-import/minute fallback. Existing Retry-After handling and bounded retries remain; busy responses without Retry-After use shorter exponential backoff. Exact documents are retained across retries.

## Browser measurements

Same 512 distinct, legal 33-ply PGNs, real browser WASM preparation/annotated export, production HTTP client, disposable archives, release server binaries. Initial-import elapsed time includes preparation and import, excludes verification. Single measurements, not a statistical guarantee or a full file-picker UI timing.

| Scenario | Fixed one-second pacing | Adaptive pacing | Improvement |
| --- | ---: | ---: | ---: |
| Idle | 3.203 s | 0.930 s | 3.44x |
| Stockfish searching | 3.212 s | 1.371 s | 2.34x |

Idle baseline comes from the immediately preceding parsing validation. Busy baseline and both adaptive cases were measured this turn. Busy cases use the existing GPL-3.0 Stockfish 19 lite single-thread WASM build through Node, registered in the isolated Companion. Actual engine depth output is required before timing; it searches continuously until the initial import finishes. This is one engine workload, not an lc0/GPU benchmark.

Each case re-imported all games and verified 512 duplicates, exact saved JSON including 512 notes, and 1,024 saved analysis payloads and metadata. No user archive was used. The benchmark harness is scripts/browser-import-benchmark.mjs; --engine-busy enables the engine workload and --before serves the saved pre-change client for the baseline. Local result artifacts are tmp/browser-pgn-adaptive-release-idle.json, tmp/browser-pgn-adaptive-release-busy.json and tmp/browser-pgn-paced-busy.json.

## Checks

- 260 JavaScript tests passed, including budget boundaries, old-server fallback, malformed response metadata and serialized imports recovering after failure.
- 40 native server library tests passed, including independent read/operations budgets and exposed feedback headers.
- Required browser WASM rebuild and site asset refresh succeeded.
- Default Windows development package rebuilt at output/companion/current, retaining the private update channel; existing data directory preserved.
- Companion remains behind the existing shared feature flag. No deployment, Git push or public release changes.

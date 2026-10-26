# Ironwood release checks

For releases, run `npm run release:prepare` before release checks and committing build artifacts. It refreshes and validates the CC0 opening dataset at one upstream commit, then rebuilds the application. If the refresh fails, resolve it before releasing; do not silently fall back to old data. Routine offline development can still use `npm run build:wasm`.

Always rebuild the browser WASM bundle after application changes with `npm run build:wasm`; a compile check alone is not sufficient.

The local preview at port 8788 serves `.wrangler-assets`, not `web` directly. After rebuilding WASM or changing browser assets, run `npm run prepare:wrangler` to refresh that preview folder. If website content changed, run `npm run build:site` before preparing the preview. Before telling the owner the preview is updated, verify that `http://127.0.0.1:8788/pkg/battle_chess_bg.wasm` matches `web/pkg/battle_chess_bg.wasm`; rebuilding WASM alone leaves the running preview stale.

The production PWA update flow was checked for a previous release. Do not require a recurring PWA test reminder or confirmation before GitHub pushes. Choose validation appropriate to the changes being released.

# Licensing requirements

Only recommend or implement solutions with a clear, straightforward licensing path for Ironwood's intended use and distribution. Verify licensing before proposing or integrating engines, libraries, model weights, artwork, or other third-party assets. Do not assume a repository's code license also covers its model weights or other assets. Exclude options with unclear terms, pending permissions, or complicated licensing obligations; prefer suitable alternatives with explicit, easy-to-comply-with licenses.

# Ironwood Companion public Windows release

Use `scripts/package-companion.ps1` with its default output for Windows development
updates. Keep the current build at `output/companion/current` and reuse that path;
do not create a new output folder for each Companion change. Stop a running copy
before replacing it, and preserve its configured data directory.

The owner authorized the public Windows x64 release on October 26, 2026. Companion setup and downloads must be discoverable without an About/diagnostics checkbox. Keep connection tokens, exact-origin checks and update-signature verification mandatory. Public download access must never expose private-preview releases or owner credentials.

The public Features, Help and Windows download pages may describe Companion. Offer only the published Windows x64 package; macOS, Linux and QNAP remain developer-supported rather than public downloads. Match download metadata to the immutable published package and signed feed. Never replace a published versioned ZIP, publish private keys, or weaken verification to work around a release problem.

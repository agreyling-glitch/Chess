---
title: "Why Ironwood runs Stockfish in your browser"
date: 2026-09-21T00:00:00-05:00
description: "A look at the architecture that keeps full-NNUE Stockfish local while the board remains responsive."
slug: "stockfish-in-your-browser"
---

Ironwood Chess began with a demanding premise: the browser version should not quietly substitute a cut-down chess engine or send positions to a server. The game should run a serious engine locally, preserve the complete NNUE evaluation network, and keep the board responsive while Stockfish calculates.

That requires more than compiling an engine to WebAssembly. Stockfish runs behind a dedicated Worker boundary so search never occupies the browser's rendering thread. `SharedArrayBuffer` and `Atomics` provide the primitives its pthread build needs for parallel evaluation, while cross-origin isolation headers make those capabilities available safely.

## A deliberately split delivery path

The application itself is compact: Rust supplies game state and legal move handling, while egui and WebGPU render the board. That WebAssembly package is deployed with the site.

The Stockfish 19 engine is different. Its full WebAssembly binary and NNUE data are large enough that they live in Cloudflare R2. A same-origin Worker route streams those versioned files to the browser, allowing long-lived caching without weakening the isolation required for multithreading.

The first visit therefore includes a substantial engine download. Later games normally start from the browser cache. The loading indicator reports real transfer progress, and the interface remains separate from the engine throughout initialization and search.

## A URL instead of an installer

A native chess application has to be packaged separately for Windows, macOS, and Linux, often with additional builds for different processor architectures. Each package brings its own installer, signing requirements, update mechanism, and security prompts. Before a player reaches the board, they have already been asked to download an executable and trust it with access to their computer.

Ironwood replaces that process with a URL. A modern browser already provides the runtime, so the same application can open across desktop and mobile operating systems without asking the player to choose an installer or understand whether their machine uses ARM or x86. Browser and device capabilities still vary, but the distribution path remains one codebase, one deployment, and one address.

Updates follow the same model. The application shell can be replaced centrally, and the browser checks its fixed application assets for changes. Large versioned engine files remain cached until an intentional engine upgrade receives a new URL. This avoids a population of old desktop installations while also avoiding a 94.5 MiB download after every small interface change.

## Native machinery inside a safer boundary

Running in a browser is also a useful security constraint. A downloaded native executable can potentially request broad filesystem, process, and operating-system access. WebAssembly executes inside the browser sandbox instead. Ironwood does not need arbitrary access to the player's files or machine: it needs memory, computation, rendering, and a carefully limited path to its own hosted assets.

That boundary no longer means reducing the engine to a single slow thread. Modern browser primitives give demanding applications much of the machinery historically associated with desktop software:

- `SharedArrayBuffer` and `Atomics` support shared memory and coordination between Stockfish threads.
- Web Workers keep engine initialization and search away from the interface thread.
- WebAssembly provides compact, predictable execution for the Rust game and the C++ engine.
- WebGPU gives the renderer a modern path to the graphics hardware.

Performance still depends on the browser, device, thread count, and WebAssembly implementation, so a browser build should not be described as identical to every optimized native binary. The important result is that players can get strong, multithreaded, full-NNUE analysis without installing an operating-system-specific program—and without sending each position to a remote chess service.

The same delivery model also supports an app-like experience. Ironwood's service worker stores the application shell for offline use, while the landing page offers an explicit **Enable offline play** action for the much larger Stockfish files. The engine is never silently added to a mobile data bill: the player sees its size, chooses whether to store it, and can remove the offline copy later. Once enabled, the installed app can reopen the board and engine without a connection.

## Why build it this way?

Local evaluation keeps the current position on the player's device. It also creates a stable foundation for what comes next: richer analysis controls, carefully calibrated playing strengths, and an animated presentation inspired by classic battle chess without coupling animation state to chess legality.

It also changes the economics of serious analysis. A server-based chess service pays for every position searched and every second its engines continue calculating. Those costs naturally lead to subscription tiers, daily analysis allowances, restricted depth, shared compute, or queues during busy periods. Once Ironwood's full Stockfish engine has reached the browser, continued search does not consume an Ironwood server. The player's own machine supplies the compute.

That makes long, private analysis possible without an account, a metered server allowance, or a premium engine tier. A future analysis workspace can let Stockfish continue exploring a critical position for as long as the player chooses, use the locally available thread budget, and compare lines without uploading the game. The practical limits become the device's speed, memory, battery, thermal behavior, and the time the player is willing to spend—not a remote service's daily cap.

The word *full* matters here. Ironwood is preserving Stockfish 19's complete NNUE network and multithreaded search rather than presenting a lightweight imitation under the Stockfish name. The browser remains a delivery and security boundary; it is not intended to become an artificial product boundary around the engine's depth.

## Open-source licensing

Stockfish is free software licensed under the GNU General Public License version 3. Ironwood publishes the license, attribution, and a link to the exact corresponding Stockfish source for the engine version it distributes on its [open-source notices page](/open-source-notices.html). That provenance matters when powerful code is being asked to run on a player's machine, even inside a browser sandbox.

Ironwood's original application code, interface, and project artwork are also free software, licensed under GPL version 3 or later. The [project source](https://github.com/agreyling-glitch/Chess) and [license](https://github.com/agreyling-glitch/Chess/blob/main/LICENSE) are available for anyone who wants to study, modify, or share them. Third-party components retain the licenses listed on the notices page.

Today the board is intentionally restrained. Calibrated playing strengths and a dedicated analysis workspace are the next functional layers; the longer road leads toward the animated battlefield the architecture was designed to support.

[Play the current build →](/play/)

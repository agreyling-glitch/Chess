---
title: "Initial public build"
date: 2026-09-21T00:00:00-05:00
description: "The first public foundation for private, full-NNUE browser chess."
slug: "2026-09-21-initial-public-build"
version: "2026.09.21"
---

## Added

- A complete playable chess board with legal move validation, castling, en passant, promotion, checkmate, and stalemate handled in Rust.
- Stockfish 19 running in a dedicated Web Worker with its full NNUE network.
- Multithreaded engine evaluation through `SharedArrayBuffer`, `Atomics`, and pthread workers.
- Automatic game restoration after a refresh or a return from the landing page.
- Engine download progress reporting and persistent browser caching.
- A responsive two-dimensional interface designed to support future battle animations.
- A public landing page, open-source notices, sitemap, robots policy, and complete favicon set.
- An installable Progressive Web App with an automatically cached application shell and an explicit, progress-aware offline Stockfish download.

## Infrastructure

- Large, versioned engine assets are delivered through Cloudflare R2 and a same-origin Worker route.
- Application WebAssembly is committed as a reviewed deployment artifact, avoiding unnecessary Rust compilation for content-only releases.
- Cross-origin isolation headers keep multithreaded engine execution available without blocking the rendering thread.

This entry establishes the public changelog format. Future entries will focus on changes players can see or feel rather than reproducing private Git history.

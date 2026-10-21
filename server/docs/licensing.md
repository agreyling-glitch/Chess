# Phase 0 dependency and distribution review

Reviewed 2026-10-17. Companion and its protocol crate use the repository's
GPL-3.0-or-later license. No engines, weights, opening books, games, artwork,
or tablebase files are added by Phase 0.

Direct dependencies:

| Component | Verified package license | Purpose |
| --- | --- | --- |
| Axum 0.8.9 | MIT | HTTP routing, upgrade handling |
| Tokio 1.53.3 | MIT | Native runtime, TCP, timers, shutdown |
| Serde / serde_json | MIT OR Apache-2.0 | Configuration and wire types |
| getrandom 0.3.4 | MIT OR Apache-2.0 | OS-backed random connection tickets |
| Tower 0.5.3 (tests) | MIT | Request-level integration tests |
| ws 8.21.0 (Node tests only) | MIT | Native HTTP/WebSocket process test client |

Sources: downloaded Cargo registry package manifests/license files, linked
repositories in `dependency-licenses.json`, [Axum upstream manifest](https://github.com/tokio-rs/axum/blob/main/axum/Cargo.toml),
and [Tokio upstream manifest](https://github.com/tokio-rs/tokio/blob/master/tokio/Cargo.toml).

`node scripts/companion-licenses.mjs` regenerates the exact resolved dependency
inventory for Windows x64 and Linux x64, including development dependencies.
The reviewed graph has explicit MIT, Apache-2.0, BSD, Unicode-3.0, or selectable
MIT/Apache alternatives, plus Ironwood's own GPL packages. Choose MIT where an
alternative is offered; retain required notices for conjunctive BSD/Unicode terms.
There are no unknown/missing license declarations in this inventory. This is a
straightforward path compatible with the project's GPL distribution, rather than
an approval of unrelated assets hosted in those repositories.

Before distribution, collect actual required license/copyright/NOTICE texts for
the shipped graph and provide GPL corresponding source/build instructions. The
inventory records terms and versions; it does not itself replace those notices.
Regenerate/review it when the lockfile changes.

The Docker development recipe uses official Rust and Debian images. Rust has
MIT/Apache licensing; Debian contains separately licensed OS packages. Inspect
the resolved image's package copyright files, retain notices, identify source
availability obligations, and pin the base digest before publication. This image
has not been built/audited here and is not a released artifact. Excluding a release
approval is deliberate: no Linux image compliance claim is inferred from Cargo
metadata alone.

Phase 2 reuses Shakmaty 0.30.1, already used by Ironwood. Its downloaded Cargo
manifest explicitly declares GPL-3.0-or-later, compatible with this GPL project.
The native source package must include its corresponding vendored source and
notices. The fake UCI engine is original test code; no engine/network is bundled.

Phase 3 uses `rusqlite` 0.40.1 and `libsqlite3-sys` 0.38.2 (MIT), with bundled
SQLite (public domain), `pgn-reader` 0.29.0 (GPL-3.0-or-later, matching Shakmaty),
and `sha2` 0.10.9 (MIT OR Apache-2.0). Exact downloaded Cargo manifests and the
upstream licensing declarations were checked before integration. Only bundled
core SQLite/backup features are enabled; no SQLCipher, extension loading or
external datasets are included. Keep MIT notices and corresponding GPL source in
packages; `scripts/package-companion.ps1` vendors the matching sources/notices.
The dependency inventory was regenerated for native Windows and Linux.
Sources: https://github.com/rusqlite/rusqlite/blob/master/README.md,
https://sqlite.org/copyright.html, https://github.com/niklasf/shakmaty,
and https://github.com/RustCrypto/hashes/tree/master/sha2.

Opening-book readers and optional datasets remain unselected. Their licenses and dataset terms require independent
review when those phases begin. No download/recommendation of a particular
engine or neural network is necessary for this connection-only prototype.

Phase 5 uses `shakmaty-syzygy` 0.28.0, explicitly GPL-3.0-or-later, compatible
with this GPL project and pinned to the existing Shakmaty 0.30.1 dependency.
The downloaded manifest declares Rust 1.88 compatibility. No optional mmap
feature is enabled. Its corresponding source and notices are included by the
existing vendored-source packaging flow. New transitive byteorder 1.5.0 declares
Unlicense OR MIT; tracing-attributes declares MIT. The Windows/Linux inventory
has been regenerated. Upstream: https://github.com/niklasf/shakmaty and
https://docs.rs/crate/shakmaty-syzygy/0.28.0/source/Cargo.toml.

Phase 6 now uses a selected real Syzygy dataset for development validation.
The generator author explicitly permits redistribution of generated table files;
this dataset-specific statement was verified separately from all code licenses.
Pinned permission evidence, file hashes, mirror URLs and checksum provenance
are in `server/validation/syzygy-fixtures.json` and
[the validation record](phase-6-tablebase-validation.md). The files stay in ignored
tmp storage and are not distributed with Companion or the browser application.
The separate development reference `chess` 1.11.2 is GPL-3.0-or-later and pinned
by source-archive hash; it adds no native/runtime dependency. Original invalid
fixtures and bare-king unit tests remain available without dataset downloads.

The Phase 6 development validation toolbox uses the official MIT Node Docker
recipe and existing MIT ws 8.21.0, with a minimal locked npm graph. It is separate
from the production image and introduces no Companion runtime dependency. Base
Debian package notices and release digest audit still apply before publication.
References: https://github.com/nodejs/docker-node/blob/main/LICENSE and
https://github.com/websockets/ws/blob/master/LICENSE. Windows packaging uses the
installed MSVC toolchain's static CRT support; no separate Microsoft DLL or
redistributable installer is copied into the package.

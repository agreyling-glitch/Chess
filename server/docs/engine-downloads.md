# Private-preview publisher-direct engine downloads

Catalog review (2026-10-25): Windows x64 now also offers Arasan 26.1 and Weiss 2.0.
Arasan's official `arasan26.1.zip` SHA-256 is
`57af9ca4d8a9f57662421c1b54652b246b02cce011f9e2ed26c2252813420690`.
Its included LICENSE explicitly applies MIT terms to the engine, network and book
directories; preserve the full package, including Syzygy attribution. Select
`arasanx-64.exe` (baseline SSE2), retaining its NNUE and book alongside it.
Publisher/source: https://github.com/jdart1/arasan-chess/releases/tag/v26.1 .
Weiss's official v2.0 Windows collection SHA-256 is
`2bbc728d8eb9ac1e60c2cc57b843ca43a4887854cc790db8f8fe238873c255ca`.
Select `weiss-nopopcnt.exe` for baseline compatibility; this release uses classical
evaluation, with no external model weights. Code license is GPL-3.0; publisher
and corresponding source: https://github.com/TerjeKir/weiss/tree/v2.0 .
Both packages were downloaded directly from their publishers, installed in isolated
Companion folders, UCI-registered, tested for a legal opening move, and checked
again after Companion restart. No engines were added to the owner's registration.

Lc0 CPU 0.32.1 is now enabled for Windows x64. The original CPU-DNNL ZIP is
https://github.com/LeelaChessZero/lc0/releases/download/v0.32.1/lc0-v0.32.1-windows-cpu-dnnl.zip
(24,001,097 bytes, SHA-256
`b9cfcfbd3dabffbfd452f6e8e087c22273721bef48eba19640b6d92006c142f5`).
Preserve GPL-3.0-or-later COPYING and the package's DNNL/third-party/mimalloc notices.
The ZIP's default 791556 network is deliberately skipped during extraction.
Instead, fetch the unmodified T1 network from the original project storage:
https://storage.lczero.org/files/networks-contrib/t1-256x10-distilled-swa-2432500.pb.gz
(37,118,673 bytes, SHA-256
`bc27a6cae8ad36f2b9a80a6ad9dabb0d6fda25b1e7f481a79bc359e14f563406`).
The creator masterkni6 explicitly permits anyone to use this exact network however
they want: https://github.com/orgs/LeelaChessZero/discussions/2430 (2026-08-05 reply).
This is a specific permission grant, not an inferred GPL license or a claim about
other networks. Save its source, creator, quote and permission link in the installed
NETWORK-PERMISSION.txt; show network-source/permission links in the download card.
Both artifacts stream to separate resumable files with pinned sizes and digests.
Progress counts their combined 61,119,770 bytes. Publish/register only after both
verify; launch with an explicit absolute --weights path and --backend=blas.
The receipt includes the selected network and permission record, so existing
verification and optional uninstall include them. A missing/unselected net fails
installation verification. Isolated download/install, actual chess search and
restart checks passed without modifying any existing owner engine.

Deferred: Maia3's official model card states AGPLv3, but its
Python/PyTorch environment requires a separate pinned runtime installer; there is
no official native ZIP release for the current installer. It is not in this catalog.

The app's existing Add an engine window includes a download panel only when the
shared Companion preview flag is enabled. Companion owns each job, so closing the
app or panel stops polling without cancelling the download. Engine payloads and
network files are never hosted by Ironwood; only catalog metadata is bundled.

The initial catalog pins official Stockfish 19 universal Windows x64/ARM64 release
packages, sizes, SHA-256 digests, executable names and source/license links.
The official release embeds its NNUE networks in the executable. The publisher's
license, source, authors and documentation remain with the extracted package.
No third-party catalog or engine artwork is copied from En Croissant. Other engines
and standalone networks require separately verified publisher URLs and licenses
before being admitted to the catalog. Linux/macOS catalog entries are not enabled
in this first version; local registration continues to work on those platforms.

Authenticated API:

- GET `/api/v1/engine-download-catalog`: platform-compatible metadata.
- GET `/api/v1/engine-downloads`: current/persisted job summaries (read budget).
- POST `/api/v1/engine-downloads` with `{package: catalogId}`: start/resume.
- POST `/api/v1/engine-downloads/<catalogId>/cancel` with `{}`: cancel and retain
  partial download for an explicit Resume.

Requests cannot supply URLs, executable paths or download destinations. A pooled
HTTPS client downloads directly from the official GitHub release; redirects are
limited to approved GitHub release/CDN hosts. No owner credential or app cookies
are attached to publisher requests. Response chunks stream to disk under the
engine directory's `.downloads/<catalogId>`. Resume sends an exact byte range;
206 must match the expected offset/size, while an ignored Range/200 truncates
before writing. Size limits and the pinned publisher digest must pass before
extraction. Network failures retry with visible waits, then pause recoverably.

ZIP extraction streams into a unique bounded staging directory, rejects unsafe
paths/links/duplicates, retains support/license files and verifies integrity.
Publication uses a same-volume directory rename and never replaces an existing
installation. Per-file hashes support recovery/validation. Temporary staging is
excluded from manual engine browsing/registration. UCI validation precedes atomic
configuration persistence; a synchronized cancellation check guards registration.
Existing engines, credentials, archive and other configuration fields are retained.

Jobs persist progress locally. Restart marks incomplete jobs paused; explicit
Resume reuses partial bytes or verifies the already-published installation and
finishes registration. Cancellation/extraction failure does not register an engine.
The UI reports bytes, recent transfer speed, ETA and each verification/install phase.
ETA/speed are cleared while waiting, retrying, verifying or stopped.

Dependencies: reqwest 0.13.5 (MIT OR Apache-2.0; native TLS) and zip 9.1.0 (MIT,
deflate only), with exact resolved licenses in dependency-licenses.json. Windows
uses OS TLS. Linux container/runtime definitions include libssl3 and certificate
roots; matching source and notices are bundled by the existing packaging script.

Validation includes bounded HTTP Range/ignored-Range/wrong-Range cases, persisted
restart state, cancellation before configuration commit, traversal rejection and
license retention. The opt-in `node scripts/test-engine-publisher-download.mjs`
uses an isolated local Companion and downloads directly from the official source.
It verified the 81,431,614-byte x64 package, extracted it, passed UCI validation,
registered it and retained the registration after a server restart. An optional
local copy at output/stockfish-19-official.zip seeds a prefix to exercise real resume;
no engine binary is added to the repository or package.

Final development validation: 86 server library tests and four tests per Windows
binary passed (three opt-in tests ignored); 39 engine/archive/browser integration
tests passed. The final panel lifecycle checks passed again (34-test focused set).
Browser WASM rebuilt and port 8788 refreshed. The default Companion package was
installed with the existing data directory/configuration. The live owner catalog
reports Windows x64 / Stockfish 19 available; its job list is empty, so installation
is user-initiated. Publisher download smoke tests use disposable local data only.

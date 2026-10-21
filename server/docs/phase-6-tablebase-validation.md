# Phase 6: real-tablebase validation

The first Phase 6 gate is complete on the Windows x64 development machine.
The offline validator compared Companion's authenticated HTTP responses against
**python-chess 1.11.2**, a separate Python implementation, using real Syzygy files.
This is a selected three-to-five-piece corpus, not certification of every
position, all materials, six/seven-piece datasets, or Linux Docker.

## Result recorded on 2026-10-17

[Machine-readable evidence](../validation/windows-x64-real-tablebases.json)
records the binary, validator, reference source, corpus and fixture-manifest
SHA-256 hashes, platform, root results, rule coverage and failures.

- 136 root positions and 1,384 legal continuations passed with no mismatches.
- Root outcomes include 45 wins, 28 losses, 38 draws, nine cursed wins,
  14 blessed losses, and one rounding-ambiguous result in each direction.
- The corpus includes genuine cursed/blessed five-piece outcomes at clock zero,
  fifty-move boundaries, legal en passant (and the same placement without it),
  all four promotions, mate/stalemate, and 15 color-mirrored roots.
- Every legal UCI move, SAN, signed DTZ, child-side outcome and zeroing flag is
  checked. DTM must remain unavailable. No +/-1 DTZ tolerance is used.
- Python's public probe does not expose rounding provenance. At exactly 100
  plies the oracle permits the definite or appropriately signed ambiguous
  category; any ambiguous response must set `dtzRounded`. Draws and directly
  proved mates have exact DTZ. This is not a complete independent verification
  of the precision flag for every table encoding.
- Disposable copies verified WDL-only installation, missing capture successors,
  corrupt WDL/DTZ, restoration, removal/reinstallation, invalid FEN, castling
  and positions over seven pieces. Input fixtures were hashed again afterward.
- Development debug-build HTTP latency was 3.5 ms minimum, 30.0 ms median and
  188.0 ms maximum, including all continuations and fresh table handles. These
  are local observations, not release performance budgets or cold-disk results.

The real-data checks exposed a bug: library WDL conversion applied the clock to
terminal mate DTZ=-1, producing a blessed loss at clock 100. Companion now gives
checkmate precedence. It also recognizes a directly proved mate in one as an
exact win rather than a rounded boundary ambiguity. Terminal roots and mating
continuations are both covered by the corpus; the terminal rule also has a
file-free native regression test.

Supporting checks passed: 19 native unit tests, 22 combined Companion/tablebase
Node tests, five validator integrity tests, and server/protocol Clippy with
warnings denied. The required browser WASM build and preview refresh succeeded.

## Fixture provenance and permission

The fixture set contains **44 files, 15,614,720 bytes**, covering selected
materials and their capture/promotion dependencies. Files stay in ignored
`tmp/companion-tablebase-validation/tables`; no tablebase binary is committed,
added to the application shell, or copied by the current release recipes.
`.rtbw` and `.rtbz` files are also ignored throughout the repository.

[The generator author's explicit dataset terms](https://github.com/syzygy1/tb/blob/0bb8aeee525f364bb750f96df312a1a7c9b54398/README.md#terms-of-use)
allow redistribution of generated tablebase files. This was checked independently
of the generator and probing-library code licenses. The reviewed README's hash
and pinned source commit are in [the fixture manifest](../validation/syzygy-fixtures.json).
The generator executable is not integrated or distributed.

Files were obtained over HTTPS from Lichess's standard-table mirror. Each file's
embedded checksum matched the generator author's pinned checksum list. The
manifest additionally pins full-file SHA-256 and byte size, which are enforced
on every preparation and validation run. The initial footer comparison is not
represented as a recomputation of the generator's embedded checksum algorithm.

The reference is the GPL-3.0-or-later `chess` 1.11.2 Python package. Its upstream
[license](https://github.com/niklasf/python-chess/blob/master/LICENSE.txt) and
[PyPI release metadata](https://pypi.org/pypi/chess/1.11.2/json) were checked; the
source archive SHA-256 is pinned in `server/validation/reference-requirements.txt`.
It is a development tool in an isolated virtual environment, not a Companion
runtime dependency or an engine bundled with Ironwood.

## Reproduce from a repository checkout

Windows PowerShell, with Python available for development:

```powershell
cargo build --locked -p ironwood-server
python -m venv tmp/companion-tablebase-validation/venv
$tbPython = "tmp/companion-tablebase-validation/venv/Scripts/python.exe"
& $tbPython -m pip install --require-hashes -r server/validation/reference-requirements.txt
python scripts/prepare-companion-tablebase-fixtures.py
& $tbPython scripts/test_companion_tablebase_validation.py
& $tbPython scripts/validate-companion-tablebases.py
```

Only the explicit fixture preparation command downloads tablebase files. The
validator itself is offline: it starts disposable Companion processes on
loopback-selected ports, uses random in-memory owner credentials, and compares
local probes with python-chess reading the same hashed files. It does not send
positions to a public API or connect to your running Companion instance.
Default reports go to ignored `tmp/companion-tablebase-validation/report.json`.
Use `--binary`, `--tables` and `--report` to select paths. The tables must match
the reviewed fixture manifest; this command does not certify arbitrary owner
datasets. `--samples 0..100` controls additional deterministic samples (default
12 per material). Run without Python `-O`, since validation assertions are
mandatory. A failed preflight overwrites any previous passing report with a
failure record and exits nonzero.

Linux uses the same commands with `venv/bin/python` and the native Linux binary;
that platform run remains pending. Re-run the gate against each release binary
and retain its report; this development report is not a release-binary approval.

## Remaining Phase 6 work

Clean-machine Windows, Linux Docker, real UCI engine binaries, six/seven-piece
fixtures, large archives/datasets, simultaneous analysis/probes, shutdown under
load, source/notices packaging, and published download/image checks remain open.
No complete Phase 6 or production release claim is made by this validation gate.

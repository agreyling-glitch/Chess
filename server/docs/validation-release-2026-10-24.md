# Release validation — 2026-10-24

Internal record. Companion remains private, disabled by default behind the shared Version diagnostics checkbox. Public Features, Help and changelog additions describe app features only; existing 3D companion-window terminology is unrelated and preserved. No native package, release credential, private signing key, user archive or test output is committed or uploaded.

## Preparation

- `npm run release:prepare` succeeded before release checks. Opening data was refreshed/validated at upstream commit `a6189a30dc273ccb21fc2536a9a2fefd5592a67a`: 3,865 legal lines, verified CC0-1.0 license.
- Release WASM, engine assets, 3D assets and public site rebuilt. The production `build:cloudflare` pipeline also succeeded, including position-editor build and preparation of fresh Worker assets.
- Prepared app version: `8a4da3717491a50fbfd5`; package version: `5c152798eec949b7`.

## Checks

- Locked Rust workspace: 307 application tests, 63 server tests and both native binary tests passed; four opt-in benchmarks ignored.
- Complete JavaScript application/script suite: 314 tests passed.
- Fresh native `cargo build --locked -p ironwood-server --bins` succeeded. Then 75 integration/UI/tablebase tests passed against that exact native binary. Four older fixture imports were lengthened to meet the new two-full-move policy; the real HTTP archive test now explicitly verifies short-game discard counts. Complete-record transfers and backup/recovery fixtures still preserve existing short games.
- Private-preview signing checks passed: encrypted disposable keys, public-only pins, manifest/package verification, tamper/wrong-key rejection and downgrade rejection, including Windows PowerShell 5.1 verification.
- Updater checks passed: rollback/data recovery, HTTPS/publisher policy, archive traversal rejection, private credential isolation and signature validation.
- Wrangler deployment dry run passed with fresh assets, including the FICS Durable Object binding/migration. No direct deployment was performed during validation.
- Shared-flag tests cover default-off deferred tools, diagnostics, connections and engine discovery. Rust tests cover hiding/preserving private engine profiles and blocking private engine use when disabled.
- Staging audit excludes credentials, protected key material, output packages and user/test data; public-content additions contain no Companion/Maia/automatic-import announcement. Whitespace check passed.

## Artifacts

Detailed logs are ignored local files under `tmp/release-oct24-*`, including preparation, locked Rust tests, final JavaScript suite, native integration, signing/updater checks and final Worker dry run.

The previously documented FICS plaintext downstream transport limitation remains unresolved; registered sign-in remains available. See `validation-security-mediums.md` for scope and evidence. This release does not publish a new private Companion download or update manifest.

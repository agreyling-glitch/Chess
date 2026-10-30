# Windows public release 0.1.8

Owner-authorized corrective release, October 30, 2026. Supersedes the immutable 0.1.7 ZIP after the live legacy-upgrade test exposed a restart filename bug. The public feed was temporarily restored to its verified 0.1.4 manifest, then advanced to the validated 0.1.8 package.

## Published package

- Windows x64, static MSVC runtime; version 0.1.8.
- ZIP: `41,272,863` bytes.
- URL: `https://companion-releases.ironwoodchess.com/public/windows-x64/0.1.8/ironwood-companion-windows-x64.zip`.
- SHA-256: `50adabf0bbb999cbe5b4cdc65b6630d8c44548162d2d5826ea4224b355ba6225`.
- Public signed feed: `https://companion-releases.ironwoodchess.com/public/windows-x64/manifest.json`.
- Feed expiry: `2027-01-28T19:11:15.3723959+00:00`. Renew the feed before expiry without replacing a published ZIP.
- The existing protected RSA release key and verification pin are retained. No private key or owner credential is packaged. Executables have no Windows publisher signature; release-key verification is mandatory and separate from publisher trust.
- Previously published 0.1.4 and 0.1.7 ZIPs remain unchanged. Only Windows x64 is advertised publicly.

## Corrected restart and legacy limitation

The native update request now serializes its executable name as a JSON string, never a platform-specific OsStr object. Only `ironwood-server.exe` and `ironwood-companion.exe` are accepted restart targets. The regression checks exercise serialization for both entry points and reject encoded objects, traversal, and unrelated executables.

Version 0.1.4's actual automatic update attempt verified the package but failed when its old helper interpreted the encoded filename object. Rollback restored 0.1.4. A newly downloaded ZIP cannot replace that already-running old helper. Therefore **0.1.4 and 0.1.7 installations need one manual update**, preserving their data directory. Download, setup, troubleshooting and release notes state this explicitly. Future 0.1.8 update requests use the corrected handoff.

The corrected signed helper was exercised with the exact 0.1.8 ZIP, pinned manifest and a plain-string request against an isolated 0.1.4 archive. It verified signatures and checksum, snapshotted data, replaced the application, passed startup health checks, and preserved the game document, owner token and custom configuration. This tests the corrected replacement/restart path; it does not claim the broken legacy automatic handoff can bootstrap itself.

## Validation evidence

- Exact ZIP passed isolated Windows validation and the existing clean Windows 11 x64 evaluation VM, build 26300.9711, under the normal `ironwoodtest` account with restricted PATH and no developer-tool dependency.
- Checks: checksum/static CRT, private data setup, auth/origin denial, capabilities, PGN round trip and duplicates, collection membership and indexed filtering, configurable backup folder persistence, server snapshot creation and empty-archive restore with games/collections, opening explorer, local tablebase draw, UCI search/close, restart persistence, token rotation and graceful shutdown.
- Updater regression suite passed rollback/data recovery, HTTPS host policy, traversal/publisher rejection, manifest/tamper rejection, credential isolation and restart target validation.
- Companion Rust suite: 95 library tests and seven tests per Windows binary passed; four opt-in tests ignored. Application/browser baseline remains 346 Rust and 388 browser/integration tests passed, with three application opt-in tests ignored.
- Required `npm run release:prepare` refreshed the CC0 opening source and rebuilt final application/site artifacts.
- Package credential/path scan and the existing release-key tests passed. Matching source and license/NOTICE files are included.
- Publisher validated version/pin and uploaded the new immutable ZIP before the signed feed. Live verification checked anonymous public download, pinned signature, clean-tested ZIP checksum, protected private feed, platform isolation and exact-origin CORS.

Local reports: `output/companion/windows-validation-0.1.8.json`, `windows-clean-validation-0.1.8.json`, `corrected-install-0.1.8.json`, `updater-0.1.8.log`, `tests-0.1.8-final.log`, and `public-delivery-validation.json`.

The local Companion was gracefully stopped for packaging and restarted with its existing data/configuration directory using 0.1.8. No user data directory was removed.

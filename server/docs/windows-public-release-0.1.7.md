# Windows public release 0.1.7

Published with owner authorization October 30, 2026; superseded by 0.1.8 after live-upgrade validation. The existing protected release key signs the new public feed; the immutable 0.1.4 ZIP remains unchanged.

## Package and trust

- Version: 0.1.7, Windows x64, static MSVC runtime.
- Public ZIP: `https://companion-releases.ironwoodchess.com/public/windows-x64/0.1.7/ironwood-companion-windows-x64.zip`.
- ZIP bytes: 41,257,016.
- SHA-256: `6d628f5d4363c538815651e563d3cd82b88bd4b95235c8a0f24ec5a0cc32c49b`.
- Feed: `https://companion-releases.ironwoodchess.com/public/windows-x64/manifest.json`.
- Signed feed expiry: January 28, 2027, 19:00:11 UTC. Renew the feed before expiry without replacing this ZIP.
- Release-key signatures and pinned SHA-256 are mandatory. Windows executables have no Windows publisher signature. The legacy internal `private-preview-key` trust-mode name is retained for compatibility.
- Source, dependency license/NOTICE texts, and offline rebuild dependencies are included. No engines or models are bundled.

## Validation

The exact ZIP passed `scripts/validate-companion-windows.ps1` on the development machine and the existing clean Windows 11 x64 evaluation VM, build 26300.9711, under the normal `ironwoodtest` account. No host games, owner token, or release private key were copied to the VM. Validation used a restricted PATH without developer tools.

Checks covered static CRT/package checksums, private data provisioning, health, authentication, exact-origin denial, service capabilities, PGN import/export and duplicate policy, collection membership and indexed filtering, configurable backup folder persistence, server-side snapshot creation, restore into an empty archive with games and collection membership retained, opening explorer, local tablebase draw, UCI handshake/search/close, restart persistence, token rotation, and graceful shutdown.

The PowerShell 5.1 harness uses the server-returned canonical backup path and .NET file APIs for extended Windows paths. No package change was needed for that harness correction.

Updater tests passed for rollback/data recovery, trusted HTTPS origins, traversal/publisher rejection, signature/tamper rejection, and credential isolation. Release-key tests passed under PowerShell 7 and Windows PowerShell 5.1. The package scan found no owner credential, release-access credential, or protected private-key file.

The final application checks before publication passed: 388 browser/integration tests, 346 application Rust tests, and 108 Companion tests. Seven opt-in Rust tests remained ignored. `npm run release:prepare` refreshed the CC0 opening source and rebuilt the application.

Evidence is retained locally under `output/companion/`: `windows-validation-0.1.7.json`, `windows-clean-validation-0.1.7.json`, `updater-0.1.7.log`, `signing-0.1.7.log`, and `public-delivery-validation.json`.

## Live delivery

The publisher verified package version and public signing pin, refused version replacement, uploaded the new ZIP first, and uploaded the signed manifest second. Live verification checked the pinned signature, clean-tested ZIP SHA-256, anonymous public access, private-channel authentication, unsupported-platform isolation, and exact-origin CORS. All passed. Public download metadata matches the verified package.

The current development installation was replaced only after graceful shutdown and restarted using its original data/configuration directory. No user data folder was removed. QNAP remains developer-supported and is not advertised as a public Windows download.

## Withdrawal from the update feed

The post-publication live 0.1.4-to-0.1.7 update test failed during restart. The request serialized an OsStr as a Windows object instead of a filename string; PowerShell could not locate the executable. Rollback restored 0.1.4 and retained the test game, token and configuration. The public feed was temporarily restored to its verified 0.1.4 manifest, while the 0.1.7 ZIP stayed immutable. The owner authorized a corrected 0.1.8 release. Older 0.1.4/0.1.7 installs require a one-time manual update; the corrected native request and helper protect future updates. See windows-public-release-0.1.8.md.

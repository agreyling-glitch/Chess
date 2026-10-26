# Windows public release 0.1.4

Prepared October 25 and published with owner authorization October 26, 2026. The immutable ZIP and signed public feed were uploaded, the release Worker and website deployed, and public setup enabled. No Git commit or push was performed.

## Candidate and trust

- Windows x64 portable ZIP: `output/companion/ironwood-companion-windows-x64.zip`.
- SHA-256 sidecar: the same path with `.sha256` appended.
- Signed manifest: `output/companion/update-manifest.json`.
- ZIP SHA-256: `0f0b6ac8f96662cf44cb890d4787bec41efdd7c40d5347341ef234fe649c3b6b`.
- Manifest expires: `2027-01-24T08:36:30.2369825+00:00`. Renew the signed feed before expiry.
- The owner-approved existing DPAPI-protected RSA release key was reused. Only
  its public verification identity is embedded. The existing signature domain,
  format 2 and `private-preview-key` identifier are preserved for compatibility.
- Executables have no paid Windows publisher signature. `signed=false` in
  release metadata describes Authenticode; `manifest_signed=true` describes
  the release-key verification. These are separate properties.
- Public feed: `https://companion-releases.ironwoodchess.com/public/windows-x64/manifest.json`.
  This path is live. No release-access token is required by the
  public updater. The local Companion owner token remains mandatory.
- Public Windows distribution is enabled by `PUBLIC_WINDOWS_RELEASES=1`. Private
  download authentication is retained. Browser Companion availability is public;
  its setup and connections entries appear directly in Help.
- Current Windows build was replaced at `output/companion/current` and restarted
  with the existing data directory. Archive, engines and owner token are retained.

## Validation

- `npm run release:prepare`: passed; opening dataset refreshed and verified at
  `a6189a30dc273ccb21fc2536a9a2fefd5592a67a`, 3,865 legal lines, CC0-1.0.
- Locked native Rust tests: 99 passed (89 library, five per native binary), four
  opt-in tests ignored.
- Download/channel tests: ten passed, including public opt-in, private isolation,
  path/method bounds, signatures and expiry.
- Release-key tests: protected-key round trip, wrong key/mode rejection, manifest
  tamper rejection, private-material rejection and trust-downgrade rejection;
  PowerShell 7 and Windows PowerShell 5.1 verification passed.
- Updater tests: rollback/data recovery, HTTPS and exact-origin credential policy,
  traversal and publisher rejection, signatures and tampering passed.
- Dependency inventory regenerated: 162 resolved versions; existing clear license
  paths retained; matching source and dependency license/NOTICE files are packaged.
- Publisher dry run verified the actual package identity, signing pin, public
  channel, signed manifest and ZIP hash with the existing key; no uploads.
- Exact ZIP validation: passed on this development Windows x64 machine with a
  restricted PATH and isolated data. Includes health/auth/origin restrictions,
  archive PGN import and binary-storage export round trip, duplicate and short
  policy, opening explorer, local tablebase draw, UCI handshake/search/close,
  restart persistence, token rotation and graceful shutdown.
- Validation fixture corrected to satisfy the two-full-move policy and accept
  legal explicit black-move numbering in exported PGN.
- ZIP credential scan found no owner token, private signing key, or private
  release-access credential.

## Acceptance and publication

A clean Windows VM automated run passed on October 25, 2026, using the exact ZIP
above. See `output/companion/windows-clean-validation.json` and
`server/docs/windows-clean-validation-0.1.4.md`. The earlier development-machine
report remains separate. Five regular Windows updates were installed, the VM
restarted into build 26300.9711, and the automated checks passed again. Two
follow-up staged security-platform updates remain offered. The user subsequently
confirmed dashboard rendering with screenshots and authorized publication.
Complete browser-pairing/tray interaction acceptance was not recorded in the VM.

The owner approved publication on October 26. Public onboarding and delivery
were deployed and verified as recorded below. Future signature verification
must remain mandatory. Never replace the published versioned ZIP, including to
refresh historical source-document wording; use a new release for package changes.

## Live verification, October 26

- Release Worker deployment: `9732a4c3-ccf8-4e51-8795-b5a820a06f8c`.
- Main site deployment: `3702c7f5-969d-48b9-8ac2-394b7eab4fb8`.
- Browser app version: `b121583f497a0853b272`.
- `npm run release:prepare` completed, including upstream CC0 data refresh and
  WASM rebuild; 46 focused browser and release-channel tests passed.
- Live anonymous feed and complete ZIP verified against the pinned release key,
  expiry, compatibility fields and the exact clean-tested SHA-256. Private feed
  requests without credentials or with an invalid credential returned 401.
- Exact-origin CORS and unsupported-platform isolation passed.
- Live download, Features and all three Help pages returned 200 without draft
  banners. Public availability, deployed app version and full WASM checksum matched.
- Reports: `output/companion/public-delivery-validation.json` and
  `output/companion/public-site-validation.json`.
- User screenshots confirmed the native 0.1.4 dashboard displays correctly.
  Browser pairing/tray interactions in the VM were not certified by screenshots.

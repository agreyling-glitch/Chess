# Windows public-release readiness

Internal preparation audit, October 25, 2026. This is not authorization to publish
or remove the default-off Companion preview gate.

## Initial scope

Windows x64 portable package. Keep Linux, macOS and QNAP developer-only. An
installer and automatic startup are separate product choices, not prerequisites
for distributing the existing portable package.

## Work required

1. Public distribution channel: introduce a public Windows feed/download route
   separate from the authenticated private-preview channel. Public users must not
   need a release-access bearer token or the DPAPI preview provisioning script.
   Retain signature, hash, platform, version and rollback checks.
2. Signing decision (owner approved): publish unsigned Windows executables and
   use the existing protected RSA release key for update verification. A paid
   publisher certificate is not a prerequisite. Explain Windows warnings and
   possible policy blocks; the update signature does not establish publisher trust.
   Retain the current public-key pin and document future key rotation.
3. Packaging: parameterize channel metadata instead of hard-coding private-preview;
   select a public version and matching release/update metadata. Keep matching
   buildable source, dependency notices, licenses and package hashes.
4. Onboarding: provide a visible Windows download/connection entry point at launch,
   first-run instructions, masked-token/copy workflow and understandable status.
   Preserve existing experimental settings during migration. Keep loopback binding
   as the default; LAN mode, if offered, must be explicit and explain firewall,
   origin and token requirements.
5. Public help: prepare Windows requirements, install/connect/update/uninstall,
   data retention and backup, automatic-import behavior, folder locations and
   troubleshooting. Keep this content unpublished until the release is approved.
6. Release evidence: validate the exact final signed package on clean Windows
   systems without developer tools. Cover first run, reconnect, tray close/quit,
   multiple Companions, engine install/remove/search, PGN/ZIP/7z/Scid imports,
   binary archive export round trips, resume, index progress and updater rollback.
   Check malformed inputs, authentication/origin restrictions, logs and secrets.
7. Licensing: regenerate and review the resolved dependency inventory and every
   shipped notice. Reconfirm publisher-direct engine packages and separately
   licensed weights; do not assume model licenses follow engine code licenses.
8. Support: replace generic archive/import errors with actionable categories and
   provide sanitized diagnostic export. Document supported Windows/CPU scope and
   retained user data on removal.

## Release sequence

Prepare changes and review them while the preview remains hidden. Run
`npm run release:prepare` before the final release checks; resolve any opening-data
refresh failure. Package Windows with `scripts/package-companion.ps1`, validate
that exact artifact and its matching source, then publish only with authorization.
Enable public app entry points together with the approved download and help.

## Existing foundations

Native tray/dashboard, persistent owner token, archive, engine management,
publisher-direct engine downloads, signed-update verification, source packaging,
checksums and Windows validation scripts already exist. The public delivery and
onboarding paths still need to be adapted from private preview. Development-PC
test success is not evidence of a clean-Windows public-package acceptance run.

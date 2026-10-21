# Windows private-preview updates

The Windows tray and status window expose **Check for updates…** and **Update and
restart**. Checking runs outside the UI thread. Only a newer numeric version is
offered; a failed check never stops the current process. The restart action
reserves every engine slot and the engine-registration slot atomically. Finish
games/analysis and close their engine sessions first. Downloading does not stop
engines. The updater does not interrupt an active engine or validation session.

Ironwood's private Version diagnostics section reads the authenticated
`GET /api/v1/updates` status and invokes `POST /api/v1/updates/check` or
`POST /api/v1/updates/install` with an empty JSON object. All three routes use
the existing owner-token, origin, protocol and rate guards; callers cannot supply
a download URL, signing key, executable or filesystem path. The API and tray use
the same process-wide update manager, candidate and idle reservation. Status
reports native Windows, container or externally managed operation. Headless and
Docker instances do not install native desktop updates. Browser controls and
diagnostic probes are gated by the global Companion preview switch. Diagnostic
copy includes only version, connection state and update state, never the token
or endpoint. An older server without these routes retains its version display
and receives manual-update guidance. During a requested restart diagnostics
probe for reconnection without changing any game or engine state.

The authenticated private Cloudflare channel was provisioned on 2026-10-21 with
release 0.1.0 using the `private-preview-key` signing mode. No Windows publisher
certificate is configured. Channel configuration and deployment procedures are
in `deploy/companion-releases/README.md`. A default development package without
an embedded channel reports that no signed channel is configured. It
cannot install arbitrary ZIPs, unsigned builds, or a URL supplied by a web client.
This feature remains part of the unreleased Companion private preview.

The alternative `private-preview-key` mode uses a generated RSA release key
instead of Authenticode. The private key is CurrentUser-DPAPI encrypted outside
the repository and packages; only the public key is embedded. This mode does
not install certificates or establish Windows publisher trust. See the internal
channel runbook for its separate build and signing parameters.

## Release configuration

Use an RSA Windows code-signing certificate with a straightforward distribution
license, trusted by Windows. Keep its private key in the release signing service
or certificate store; never put it in the repository or package. No new runtime
libraries or third-party updater assets are used.

Private downloads additionally use a release-only bearer token. Configure it
with `scripts/configure-companion-release-access.ps1` under the Windows account
running Companion. It is stored with CurrentUser DPAPI encryption in
`.release-channel-credential` inside the configured data directory, restricted
to that user and bound to the exact HTTPS release origin. It is not compiled
into binaries, passed on command lines, included in diagnostics, or read from
the Companion owner-token. The helper sends it only to that origin and never
follows redirects. Rotating this token does not change the signing trust pin.

Build with `scripts/package-companion.ps1` and these optional release parameters:

- `UpdateFeed`: HTTPS manifest URL on the Cloudflare R2 custom download host.
- `UpdateCertificate`: Base64 DER **public** signing certificate, pinned into both
  binaries at compile time. This is not an owner token or private key.
- `SigningCertificateThumbprint`: matching certificate in CurrentUser/My; signs
  both staged executables before the ZIP/checksum is generated.
- `TimestampServer`: optional Authenticode timestamp service.

The feed/certificate can also be supplied through the build environment variables
`IRONWOOD_UPDATE_FEED` and `IRONWOOD_UPDATE_CERTIFICATE`. Both are required together.
`source/update-channel.json` records the public configuration needed to reproduce
the build. Import those values into the build environment before rebuilding the
corresponding source. A key/certificate rotation needs a release signed by the
currently pinned certificate which embeds the replacement public certificate.

After packaging, run `scripts/sign-companion-update.ps1` with Package, Version,
DownloadUrl, CertificateThumbprint and Output. Upload the immutable versioned ZIP
first, then the signed JSON envelope to the configured feed. Publish the matching
source/licenses with the ZIP. Refresh the manifest before its 14-day expiry.
Publication is a separate explicit release operation; these scripts do not upload
anything or announce Companion in public pages/help/changelogs.

Manifest envelope fields `payload` and `signature` are Base64. The raw UTF-8 payload
is signed with RSA PKCS#1 v1.5/SHA-256 and includes format/product/platform/version,
protocol=1, data_schema=1, HTTPS ZIP URL, SHA-256 and expiry. Only the configured
host is allowed; redirects, unsupported compatibility versions, old versions,
invalid signatures/checksums, oversized downloads/ZIPs, traversal, symlinks and
runtime-data entries are rejected. In Authenticode mode, both executable
signatures must be valid and match the embedded certificate. In private-preview
mode, a format-2, domain-separated manifest is verified against the embedded
RSA public key before its package hash is accepted. The verified ZIP covers
both executables and all package contents. A checksum alone is never trusted.
The package must retain the same private signing mode and public key; changing
the pin requires an explicit manual bootstrap. Check and Install both verify
the manifest and ZIP, and Install re-extracts the verified ZIP rather than using
mutable files staged during Check. Downloaded manifests cannot choose the mode.

## Installation and recovery

The bundled PowerShell helper runs hidden, with process-local execution policy
only (no machine/user policy changes), using the Windows/.NET APIs. It is written
from the running binary into a unique sibling staging folder, outside both the
application and data directories. No owner token is passed on command lines or
sent to the release host. Downloads are bounded and use TLS 1.2.

After re-verification the helper waits for graceful server exit, snapshots the
archive through the previous executable's SQLite backup command, and saves the
configuration/token under a private data-directory recovery folder. Custom config
and archive paths are preserved. The default and custom data directories must be
outside the application directory. Engines/tablebases are never moved or replaced.

The whole application folder is renamed to `previous` in the staging directory;
the verified candidate replaces it. The updated process uses the same canonical
config path. Client access remains paused with `--update-pending` until `/readyz`
and process-liveness checks pass. On startup failure the helper stops the failed
candidate, restores the previous folder, config/token and SQLite snapshot, then
restarts the old version. Move failures also roll back. Previous builds, failed
candidates and data snapshots are retained for recovery, not silently deleted.
The last result is shown in the status window from `last-update.txt`.

Data schema 1 releases must remain backward compatible. Do not publish a schema
migration as schema 1. Future incompatible migrations require a new explicitly
supported upgrade/recovery protocol. A power loss or disk/filesystem failure can
require manual recovery from the retained previous folder and database snapshot;
never delete the staging/recovery folders before confirming the new version.

## Validation

Run `scripts/test-companion-updater.ps1` under PowerShell 7. It exercises real
folder replacement, healthy startup callback, startup/move failure rollback, data
recovery callback, HTTPS-host restrictions, ZIP traversal rejection, untrusted
publisher rejection, a locally generated test-only signed manifest and tampering.
The test certificate is ephemeral and is never installed in a certificate store.
Rust tests exercise the atomic idle reservation, including an active engine and
in-progress registration. Platform lifecycle checks cover the pending marker and
custom-config archive snapshot/restore. Production Authenticode installation and
Cloudflare delivery require the real signed release channel and must be validated
before enabling it; unsigned local builds cannot substitute for that check.

# Companion release channels

Public Windows x64 release **0.1.4** was published with owner authorization on October 26, 2026.
The public feed and versioned ZIP are available without a release-access token. The website download, Features and Help pages are live, and Companion setup no longer requires an About checkbox.

The remaining private-preview instructions below describe the separately protected historical channel, provisioned on 2026-10-21. Its token requirements remain in place.

Live validation confirmed anonymous and invalid-token requests return 401,
authenticated manifest and package downloads return 200, and the Windows
PowerShell updater verifies the signature, package hash and embedded key policy.
The R2 development URL is disabled and the bucket has no public custom domains.
At validation time this PC's resolver retained a negative DNS response; the
HTTPS tests used public DNS resolution with TLS hostname verification intact.

The dedicated Worker is `ironwood-companion-releases` at
`https://companion-releases.ironwoodchess.com`. It reads only the
`ironwood-companion-private` R2 bucket. The feed is
`/private-preview/windows-x64/manifest.json`; ZIPs use versioned paths under it.
No listing, public bucket domain, workers.dev route, preview URL or anonymous
downloads are enabled. Browser CORS access is restricted to the exact Ironwood
production origins and localhost development on port 8788. Every object request
still requires release authentication; preflight never reads an object.
Requests require the `RELEASE_READ_TOKEN`
Worker secret, including HEAD requests. No secret means a closed channel.

The private download section in Version diagnostics is hidden behind the shared
Companion flag. Preview administrators distribute the release-access token
privately, separately from each Companion owner/connection token. The browser
uses it only for the current download and never persists it. It verifies the
domain-separated manifest signature against the pinned public key, expiry,
platform, compatibility and ZIP hash before saving the archive. Initial browser
download does not configure the native updater's DPAPI credential; use the
internal release-access configuration script on each Windows account for that.

## Provisioning

1. Authenticate Wrangler with `npx wrangler login`, or set a scoped Cloudflare
   API token in the local environment. Set `CLOUDFLARE_ACCOUNT_ID` if the login
   has multiple accounts. Do not put credentials in source or chat.
2. Create the dedicated bucket with
   `npx wrangler r2 bucket create ironwood-companion-private`. Leave R2 public
   access disabled, including the r2.dev development URL; do not attach a public
   custom domain directly to the bucket.
3. Set a randomly generated release-only token (at least 32 characters) through
   `npx wrangler secret put RELEASE_READ_TOKEN --config deploy/companion-releases/wrangler.jsonc`.
   Use its secure prompt. This is neither the Cloudflare API token nor the
   Companion owner token. Before adding a secret to a new Worker, deploy the
   configuration first if Wrangler requires an existing Worker: it returns 503
   until the secret is configured.
4. Deploy only this Worker with
   `npx wrangler deploy --config deploy/companion-releases/wrangler.jsonc`.
   The main Ironwood website does not need a deployment for channel setup.
5. Under the account running Companion, run PowerShell 7:
   `pwsh -File scripts/configure-companion-release-access.ps1`.
   Enter the same release-only token at its masked prompt. Supply
   `-DataDirectory` when using a custom directory. Run this on each preview
   machine; do not distribute the encrypted file between Windows accounts.

Test anonymous requests receive 401, and authenticated requests for missing
objects receive 404, before uploading a release. Check both the manifest and a
versioned ZIP path. Verify the bucket has no alternate public download route.
Never paste the Authorization header into diagnostics or logs.

## Private-preview signing (no purchased certificate)

Use `pwsh -File scripts/new-companion-release-key.ps1` once on the signing
machine. It creates a 3072-bit RSA key under `%LOCALAPPDATA%/Ironwood/ReleaseSigning`,
encrypted with Windows CurrentUser DPAPI and protected by a user-only folder
ACL. It refuses replacement and repository-local key storage. No certificates
are installed in Windows trust stores. Only `release-public-key.json` may be
copied into source; the encrypted private file must never be packaged or committed.
The project's public pin is `deploy/companion-releases/private-preview-public-key.json`.

The key can only be unlocked in its Windows DPAPI account context. An encrypted
file backup alone is not a portable signing key. Preserve the account's Windows
recovery/backup material. Losing signing access requires a new key and manual
bootstrap; preview releases cannot silently rotate their own pinned key.

Build with the default `scripts/package-companion.ps1` output and pass
`-UpdateSigningMode private-preview-key`, the feed URL as `-UpdateFeed`, and the
public configuration's `public_key` value as `-UpdatePublicKey`. No certificate
or private key is passed to the build. The package records the public mode and
pin for reproducibility, and includes the corresponding source/licenses.

Sign and validate using `scripts/publish-companion-release.ps1 -Version <version>
-ReleaseKeyFile <protected-key-file>`. Without `-Publish`, no uploads occur.
The signer emits a format-2 manifest with a domain-separated RSA/SHA-256
signature, expiry and exact ZIP hash. Updates verify the signature and full ZIP
before execution, and re-verify/re-extract before installation. A package with
a different pin/mode is rejected. The executable itself is not Authenticode
signed in this mode, so Windows publisher warnings may remain.

`pwsh -File scripts/test-companion-private-signing.ps1` checks DPAPI key handling,
verification under PS7 and the actual Windows PowerShell 5.1 updater runtime,
tampering, wrong keys, wrong modes, public-only pins and policy downgrades.

## Optional Authenticode signing and common publication

A production Authenticode build instead requires a trusted RSA signing certificate with its private key in
CurrentUser/My is required by the existing packaging scripts. A signing service
would need a separate integration. Do not install an untrusted test certificate
or disable publisher verification to work around missing production signing.

Export only the public certificate as Base64 DER. Pass that value as
`-UpdateCertificate`, the feed URL above as `-UpdateFeed`, and the certificate's
thumbprint as `-SigningCertificateThumbprint` to
`scripts/package-companion.ps1`. Use the default output directory and stop a
running build before replacing `output/companion/current`; preserve the data
directory. Use an appropriate Authenticode timestamp service. The package
includes corresponding source, dependency licenses and public update-channel
configuration. Do not include private keys or release-access credentials.

Validate the ZIP with `scripts/publish-companion-release.ps1 -Version <version>
-CertificateThumbprint <thumbprint>`. Without `-Publish` it performs no uploads.
With `-Publish` it first checks authenticated access and refuses an existing
version, then uploads the ZIP before the signed feed. Do not run concurrent
publishers or reuse version numbers. A failed/ambiguous manifest upload requires
verification before retrying. The manifest expires after 14 days; renew it by
signing and publishing the same unchanged package version to the feed.
The publish script refuses an existing version; for expiry-only feed renewal,
use the signer and upload only the validated feed, never replace the ZIP.

Before enabling the channel in an installed build, validate real signed
Cloudflare delivery, publisher pinning and update/restart recovery. Initial
development builds without an embedded feed require one manual bootstrap
installation. Older or equal numeric versions are never offered as updates.
Signing does not enable the Companion flag or advertise the preview publicly.

## Validation and references

`node --test tests/companion-release-channel.test.mjs` checks authentication,
read-only routing, missing secrets, cache policy and response isolation.
`pwsh -File scripts/test-companion-updater.ps1` checks signatures, tampering,
rollback, DPAPI access and credential origin binding. A Wrangler dry run checks
the deployment bundle. These do not substitute for live channel validation.

Cloudflare's [R2 Workers API](https://developers.cloudflare.com/r2/api/workers/workers-api-reference/)
documents the private bucket binding; [Worker secrets](https://developers.cloudflare.com/workers/configuration/secrets/)
documents secret provisioning. No new third-party runtime library is introduced.

## Public Windows channel

Owner decision, 2026-10-25: prepare Windows x64 without paid Authenticode signing.
The existing DPAPI-protected RSA key signs manifest format 2 and the complete ZIP
is verified against its signed SHA-256 before extraction. The internal trust-mode
identifier and signature domain remain `private-preview-key` for compatibility;
this name does not require private downloads. No private key enters the package.

The new allowlisted prefix is `/public/windows-x64/`. A separate
`PUBLIC_WINDOWS_RELEASES=1` Worker setting is required to serve it anonymously.
The checked-in value is `1`, deployed for the public Windows release.
Disabled public routes return 503 so the publisher cannot mistake disabled
access for a missing immutable version. Private-preview paths retain bearer-token
protection even when public Windows delivery is enabled. Linux/macOS paths and
bucket listing are not exposed.

Prepare locally using the existing public pin and protected key:

```powershell
$releasePublic = Get-Content "$env:LOCALAPPDATA/Ironwood/ReleaseSigning/release-public-key.json" -Raw | ConvertFrom-Json
.\scripts\package-companion.ps1 -Channel public -UpdateSigningMode private-preview-key -UpdatePublicKey $releasePublic.public_key -UpdateFeed https://companion-releases.ironwoodchess.com/public/windows-x64/manifest.json
.\scripts\publish-companion-release.ps1 -Channel public -Version 0.1.4 -ReleaseKeyFile "$env:LOCALAPPDATA/Ironwood/ReleaseSigning/release-key.dpapi.json"
```

Stop the current Windows Companion before packaging and preserve its data folder.
The second command verifies package identity and prepares a signed manifest without
uploads. Stable local output: `output/companion/update-manifest.json`.
Public manifests are valid for 90 days; renew the signed feed before expiry.
The native public-feed path does not load private release-access credentials.
Companion's separate local owner token is still mandatory for archive/engine use.

After explicit publication approval: validate the exact ZIP on clean Windows,
choose the public app/help entry points, enable and deploy the public Worker route,
then run the publisher with `-Publish`. Check anonymous signed-manifest/package
delivery and confirm private routes remain protected. Keep versioned ZIPs immutable.
Publish the matching `.sha256` sidecar separately if offering a checksum link; the
signed manifest is the updater's authoritative checksum. Public delivery was verified on October 26, 2026; see `output/companion/public-delivery-validation.json`.

param([string]$OutputDirectory = "output/companion",
      [string]$UpdateFeed = $env:IRONWOOD_UPDATE_FEED,
      [string]$UpdateCertificate = $env:IRONWOOD_UPDATE_CERTIFICATE,
      [string]$SigningCertificateThumbprint,
      [ValidateSet('authenticode','private-preview-key')][string]$UpdateSigningMode = 'authenticode',
      [string]$UpdatePublicKey = $env:IRONWOOD_UPDATE_PUBLIC_KEY,
      [string]$TimestampServer)
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
Push-Location $repo
$previousFlags = $env:CARGO_ENCODED_RUSTFLAGS
$previousFeed = $env:IRONWOOD_UPDATE_FEED
$previousCertificate = $env:IRONWOOD_UPDATE_CERTIFICATE
$previousPublicKey = $env:IRONWOOD_UPDATE_PUBLIC_KEY
$previousMode = $env:IRONWOOD_UPDATE_SIGNING_MODE
$stagingRoot = $null
$preserveStaging = $false
try {
    if ($UpdateSigningMode -eq 'private-preview-key') {
        if (-not $UpdateFeed -or -not $UpdatePublicKey -or $UpdateCertificate -or $SigningCertificateThumbprint) { throw 'Private-preview mode requires Feed and PublicKey only; do not supply a certificate.' }
        . (Join-Path $PSScriptRoot 'companion-release-key.ps1')
        Assert-CompanionPublicKey $UpdatePublicKey
    } elseif ($UpdatePublicKey -or [bool]$UpdateFeed -ne [bool]$UpdateCertificate) { throw 'Authenticode mode requires Feed and Certificate together, without a private-preview public key.' }
    if ($UpdateFeed -and ([uri]$UpdateFeed).Scheme -ne 'https') { throw 'UpdateFeed must use HTTPS.' }
    $env:IRONWOOD_UPDATE_FEED = $UpdateFeed
    $env:IRONWOOD_UPDATE_CERTIFICATE = $UpdateCertificate
    $env:IRONWOOD_UPDATE_PUBLIC_KEY = $UpdatePublicKey
    $env:IRONWOOD_UPDATE_SIGNING_MODE = $UpdateSigningMode
    $outputRoot = [IO.Path]::GetFullPath((Join-Path $repo 'output'))
    $destination = [IO.Path]::GetFullPath((Join-Path $repo $OutputDirectory))
    if ($destination -ne $outputRoot -and -not $destination.StartsWith($outputRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Companion packages must be written inside the repository output directory'
    }
    $current = Join-Path $destination 'current'
    if (Test-Path -LiteralPath $current) {
        $runtimeFiles = Get-ChildItem -LiteralPath $current -File | Where-Object { $_.Name -in @('config.json', 'owner-token', '.lock', 'companion.lock') -or $_.Name -like '*.sqlite3*' }
        if ($runtimeFiles) { throw 'The current build folder contains runtime data. Move that data to a separate data directory before replacing the build.' }
    }
    foreach ($binaryName in @('ironwood-server.exe', 'ironwood-companion.exe')) {
        $currentBinary = Join-Path $current $binaryName
        if (Test-Path -LiteralPath $currentBinary) {
            try { $probe = [IO.File]::Open($currentBinary, 'Open', 'ReadWrite', 'None'); $probe.Dispose() }
            catch { throw 'Stop the running Companion before updating the current build. Its files have not been changed.' }
        }
    }
    $compiler = & rustc -vV
    if ($LASTEXITCODE -ne 0 -or -not ($compiler -match '^host: x86_64-pc-windows-msvc$')) {
        throw "Packaging requires a native Windows x64 MSVC Rust toolchain"
    }
    # A portable executable must not depend on a separately installed VC runtime.
    $env:CARGO_ENCODED_RUSTFLAGS = "-C" + [char]31 + "target-feature=+crt-static"
    & cargo build --locked --release --target x86_64-pc-windows-msvc -p ironwood-server
    if ($LASTEXITCODE -ne 0) { throw "Companion release build failed" }
    $metadataText = & cargo metadata --locked --format-version 1 --filter-platform x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw "Dependency metadata failed" }
    $metadata = $metadataText | ConvertFrom-Json
    $package = $metadata.packages | Where-Object name -eq "ironwood-server"
    $stagingRoot = Join-Path $destination ('.staging-' + [Guid]::NewGuid().ToString('N'))
    $stage = Join-Path $stagingRoot 'current'
    $zip = Join-Path $stagingRoot 'ironwood-companion-windows-x64.zip'
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    Copy-Item -LiteralPath "target/x86_64-pc-windows-msvc/release/ironwood-server.exe" -Destination $stage
    Copy-Item -LiteralPath "target/x86_64-pc-windows-msvc/release/ironwood-companion.exe" -Destination $stage
    if ($SigningCertificateThumbprint) {
        $certificate = Get-Item -LiteralPath ("Cert:\CurrentUser\My\" + $SigningCertificateThumbprint)
        if (-not $certificate.HasPrivateKey -or [Convert]::ToBase64String($certificate.RawData) -cne $UpdateCertificate) { throw 'The signing certificate must match the pinned update certificate and contain its private key.' }
        foreach ($name in @('ironwood-server.exe','ironwood-companion.exe')) {
            $options = @{ FilePath = (Join-Path $stage $name); Certificate = $certificate; HashAlgorithm = 'SHA256' }
            if ($TimestampServer) { $options.TimestampServer = $TimestampServer }
            $signature = Set-AuthenticodeSignature @options
            if ($signature.Status -ne 'Valid') { throw "Signing failed for $name" }
        }
    }
    Copy-Item -LiteralPath "LICENSE" -Destination $stage
    Copy-Item -LiteralPath "server/README.md" -Destination (Join-Path $stage "README.md")
    Copy-Item -LiteralPath "server/docs" -Destination (Join-Path $stage "docs") -Recurse
    Copy-Item -LiteralPath "deploy/ironwood-server/start-companion.cmd" -Destination $stage
    $source = Join-Path $stage "source"
    New-Item -ItemType Directory -Path $source | Out-Null
    foreach ($entry in @("Cargo.lock", "LICENSE", "crates", "server", "deploy")) {
        Copy-Item -LiteralPath $entry -Destination $source -Recurse
    }
    # A standalone workspace keeps the browser's unrelated dependencies out of
    # the server source bundle. Preserve the release profile used for this build.
    @'
[workspace]
members = ["crates/ironwood-protocol", "server"]
resolver = "3"
[profile.release]
lto = "thin"
opt-level = 2
'@ | Set-Content -LiteralPath (Join-Path $source "Cargo.toml") -Encoding utf8
    & cargo metadata --offline --format-version 1 --manifest-path (Join-Path $source "Cargo.toml") > $null
    if ($LASTEXITCODE -ne 0) { throw "Standalone source lockfile resolution failed" }
    # Include exact dependency source and license/NOTICE files for a self-contained
    # corresponding-source build. This also avoids relying on a dirty Git commit.
    $vendor = Join-Path $source "vendor"
    & cargo vendor --locked --versioned-dirs --manifest-path (Join-Path $source "Cargo.toml") $vendor > $null
    if ($LASTEXITCODE -ne 0) { throw "Dependency source vendoring failed" }
    New-Item -ItemType Directory -Path (Join-Path $source ".cargo") | Out-Null
    @'
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = "vendor"
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
'@ | Set-Content -LiteralPath (Join-Path $source ".cargo/config.toml") -Encoding utf8
    $instructions = @'
Ironwood Companion development package

Run start-companion.cmd or ironwood-companion.exe for the Windows system tray.
Use ironwood-server.exe for command-line/headless operation (--tray is optional).
The tray offers status, Open Ironwood, folders, Copy token and a masked Show token
window. Click Reveal explicitly to see the credential; closing masks it again.
Quit Companion from the tray to stop the server and its engine sessions.
On Windows the default data
directory is %LOCALAPPDATA%\Ironwood\Companion. Use --help for commands.
Read docs/phase-1-operations.md for credentials, rotation and shutdown.

Matching source and third-party license/NOTICE texts are included under source/.
To rebuild offline, install Rust 1.95+ and an MSVC C++ build environment, then:
  cd source
  cargo build --offline --locked --release --target x86_64-pc-windows-msvc -p ironwood-server

Check for updates is available in the tray and status window. Development builds
without an embedded signed release channel cannot download or install updates.
Updates never install automatically: choose Update and restart when engines are idle.
'@
    $instructions | Set-Content -LiteralPath (Join-Path $stage "START-HERE.txt") -Encoding utf8
    @{ product = "Ironwood Companion"; version = $package.version; platform = "windows-x64";
       crt = "static"; protocol_version = "1"; channel = "private-preview"; signed = [bool]$SigningCertificateThumbprint; signing_mode = $UpdateSigningMode } |
       ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stage "release.json") -Encoding utf8
    @{ feed = $UpdateFeed; certificate = $UpdateCertificate; signing_mode = $UpdateSigningMode; public_key = $UpdatePublicKey } | ConvertTo-Json |
       Set-Content -LiteralPath (Join-Path $source 'update-channel.json') -Encoding utf8
    # Cargo may preserve pre-1980 timestamps in vendored sources. ZIP cannot
    # represent those dates; normalize only staged copies, not source files.
    Get-ChildItem -LiteralPath $stage -Recurse -Force | ForEach-Object {
        if ($_.LastWriteTime.Year -lt 1980 -or $_.LastWriteTime.Year -gt 2107) {
            $_.LastWriteTime = [datetime]::new(1980, 1, 2)
        }
    }
    Compress-Archive -LiteralPath $stage -DestinationPath $zip
    $hash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText("$zip.sha256", "$hash  ironwood-companion-windows-x64.zip`n", [Text.Encoding]::ASCII)
    $previous = Join-Path $stagingRoot 'previous'
    $hadCurrent = Test-Path -LiteralPath $current
    if ($hadCurrent) { Move-Item -LiteralPath $current -Destination $previous }
    try { Move-Item -LiteralPath $stage -Destination $current }
    catch {
        if ($hadCurrent) {
            try { Move-Item -LiteralPath $previous -Destination $current }
            catch { $preserveStaging = $true; throw "Previous build preserved at $previous; restore it manually" }
        }
        throw
    }
    Move-Item -LiteralPath $zip -Destination (Join-Path $destination 'ironwood-companion-windows-x64.zip') -Force
    Move-Item -LiteralPath "$zip.sha256" -Destination (Join-Path $destination 'ironwood-companion-windows-x64.zip.sha256') -Force
    Write-Output "Current build: $current"
} finally {
    if (-not $preserveStaging -and $stagingRoot -and (Test-Path -LiteralPath $stagingRoot)) {
        $resolvedStaging = [IO.Path]::GetFullPath($stagingRoot)
        if (-not $resolvedStaging.StartsWith($outputRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe staging cleanup path' }
        Remove-Item -LiteralPath $resolvedStaging -Recurse -Force
    }
    $env:CARGO_ENCODED_RUSTFLAGS = $previousFlags
    $env:IRONWOOD_UPDATE_FEED = $previousFeed
    $env:IRONWOOD_UPDATE_CERTIFICATE = $previousCertificate
    $env:IRONWOOD_UPDATE_PUBLIC_KEY = $previousPublicKey
    $env:IRONWOOD_UPDATE_SIGNING_MODE = $previousMode
    Pop-Location
}

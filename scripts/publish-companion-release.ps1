param([Parameter(Mandatory)][string]$Version,
      [ValidateSet('private-preview','public')][string]$Channel = 'private-preview',
      [string]$CertificateThumbprint,
      [string]$ReleaseKeyFile,
      [string]$Package = 'output/companion/ironwood-companion-windows-x64.zip',
      [string]$DataDirectory = (Join-Path $env:LOCALAPPDATA 'Ironwood/Companion'),
      [switch]$Publish)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$hostUrl = 'https://companion-releases.ironwoodchess.com'
$prefix = if ($Channel -eq 'public') { 'public/windows-x64' } else { 'private-preview/windows-x64' }
$bucket = 'ironwood-companion-private'
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Use a numeric release version.' }
$publishWork = Join-Path $repo ('output/companion/.publish-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $publishWork -Force | Out-Null
Push-Location $repo
try {
    . (Join-Path $repo 'server/windows/update-companion.ps1') -Mode Library
    if ($Channel -eq 'public' -and -not $ReleaseKeyFile) { throw 'Public unsigned-executable releases require ReleaseKeyFile.' }
    $privatePreview = [bool]$ReleaseKeyFile
    if ($privatePreview -eq [bool]$CertificateThumbprint) { throw 'Choose exactly one signing method.' }
    $pin = ''; $publicPin = ''; $signingMode = 'authenticode'
    if ($privatePreview) {
        . (Join-Path $repo 'scripts/companion-release-key.ps1')
        $releaseKey = Read-CompanionReleaseKey $ReleaseKeyFile
        try { $publicPin = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($releaseKey.ToXmlString($false))) } finally { $releaseKey.Dispose() }
        $signingMode = 'private-preview-key'
    } else {
        $cert = Get-Item -LiteralPath ('Cert:\CurrentUser\My\' + $CertificateThumbprint)
        if (-not $cert.HasPrivateKey) { throw 'The signing certificate private key is required.' }
        $pin = [Convert]::ToBase64String($cert.RawData)
    }
    $stage = Join-Path $publishWork 'verified'; Expand-Release ([IO.Path]::GetFullPath($Package)) $stage
    Assert-ReleaseContent $stage @{version=$Version} $signingMode $pin $publicPin
    $release = Get-Content -LiteralPath (Join-Path $stage 'release.json') -Raw | ConvertFrom-Json
    $packageChannel = Get-Content -LiteralPath (Join-Path $stage 'source/update-channel.json') -Raw | ConvertFrom-Json
    if ($release.channel -ne $Channel -or $release.version -ne $Version -or $packageChannel.feed -cne "$hostUrl/$prefix/manifest.json" -or ($privatePreview -and $packageChannel.public_key -cne $publicPin) -or (-not $privatePreview -and ($packageChannel.certificate -cne $pin -or -not $release.signed))) { throw 'Package version or pinned release channel does not match.' }
    $manifest = Join-Path $publishWork 'manifest.json'
    $signingOptions = if ($privatePreview) { @{ReleaseKeyFile=$ReleaseKeyFile} } else { @{CertificateThumbprint=$CertificateThumbprint} }
    $manifestValidity = if ($Channel -eq 'public') { 90 } else { 14 }
    & (Join-Path $repo 'scripts/sign-companion-update.ps1') -ValidityDays $manifestValidity -Package $Package -Version $Version -DownloadUrl "$hostUrl/$prefix/$Version/ironwood-companion-windows-x64.zip" @signingOptions -Output $manifest
    Read-VerifiedManifest $publishWork $pin $signingMode $publicPin | Out-Null
    Copy-Item -LiteralPath $manifest -Destination (Join-Path $repo 'output/companion/update-manifest.json') -Force
    if (-not $Publish) { Write-Output "Release validated; no uploads performed. Prepared manifest: $manifest"; return }
    $credentialPath = Join-Path $DataDirectory '.release-channel-credential'
    if ($Channel -ne 'public' -and -not (Test-Path -LiteralPath $credentialPath)) { throw 'Configure private release access before publishing.' }
    $packageUrl = "$hostUrl/$prefix/$Version/ironwood-companion-windows-x64.zip"
    $probe = [Net.HttpWebRequest]::Create($packageUrl)
    $probe.Method = 'HEAD'; $probe.AllowAutoRedirect = $false; $probe.Timeout = 30000
    if ($Channel -ne 'public') { $probe.Headers['Authorization'] = Get-ReleaseAuthorization $packageUrl $credentialPath }
    try {
        $existing = $probe.GetResponse(); $existing.Dispose()
        throw 'This version already exists. Publish a new version instead of replacing it.'
    } catch [Net.WebException] {
        if (-not $_.Exception.Response -or [int]$_.Exception.Response.StatusCode -ne 404) { throw 'Cannot confirm release availability and private access. No uploads were performed.' }
        $_.Exception.Response.Dispose()
    }
    # Do not run concurrent publishers. Upload the versioned package before the feed.

    & npx wrangler r2 object put "$bucket/$prefix/$Version/ironwood-companion-windows-x64.zip" --file $Package --remote --content-type application/zip
    if ($LASTEXITCODE -ne 0) { throw 'Package upload failed; feed was not changed.' }
    & npx wrangler r2 object put "$bucket/$prefix/manifest.json" --file $manifest --remote --content-type application/json
    if ($LASTEXITCODE -ne 0) { throw 'Manifest upload was not confirmed. Verify the signed feed before retrying.' }
    Write-Output "$Channel release $Version uploaded. Verify delivery before installing it."
} finally { Pop-Location }

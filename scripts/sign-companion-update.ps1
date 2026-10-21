param([Parameter(Mandatory)][string]$Package,
      [Parameter(Mandatory)][string]$Version,
      [Parameter(Mandatory)][string]$DownloadUrl,
      [string]$CertificateThumbprint,
      [string]$ReleaseKeyFile,
      [string]$Output = 'output/companion/update-manifest.json')
$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^\d+\.\d+\.\d+$' -or ([uri]$DownloadUrl).Scheme -ne 'https') { throw 'Use a numeric release version and an HTTPS download URL.' }
$cert = $null; $key = $null
$privatePreview = [bool]$ReleaseKeyFile
if ($privatePreview -eq [bool]$CertificateThumbprint) { throw 'Choose exactly one signing method: ReleaseKeyFile or CertificateThumbprint.' }
if ($privatePreview) {
    . (Join-Path $PSScriptRoot 'companion-release-key.ps1')
    $key = Read-CompanionReleaseKey $ReleaseKeyFile
} else {
    $cert = Get-Item -LiteralPath ("Cert:\CurrentUser\My\" + $CertificateThumbprint)
    if (-not $cert.HasPrivateKey) { throw 'A release signing certificate with its private key is required.' }
    $key = [Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPrivateKey($cert)
}
$payload = [ordered]@{
    format = $(if ($privatePreview) { 2 } else { 1 }); product = 'Ironwood Companion'; platform = 'windows-x64'
    protocol = 1; data_schema = 1; version = $Version; url = $DownloadUrl
    sha256 = (Get-FileHash -LiteralPath $Package -Algorithm SHA256).Hash.ToLowerInvariant()
    expires = [DateTimeOffset]::UtcNow.AddDays(14).ToString('o')
}
if ($privatePreview) { $payload.signing_mode = 'private-preview-key' }
$payload = $payload | ConvertTo-Json -Compress
$payloadBytes = [Text.Encoding]::UTF8.GetBytes($payload)
$bytes = [Text.Encoding]::UTF8.GetBytes($(if ($privatePreview) { "Ironwood Companion private-preview release v1`n" + $payload } else { $payload }))
try {
    $signature = $key.SignData($bytes, [Security.Cryptography.HashAlgorithmName]::SHA256, [Security.Cryptography.RSASignaturePadding]::Pkcs1)
    @{ payload = [Convert]::ToBase64String($payloadBytes); signature = [Convert]::ToBase64String($signature) } |
        ConvertTo-Json | Set-Content -LiteralPath $Output -Encoding utf8
} finally { $key.Dispose(); if ($cert) { $cert.Dispose() } }
Write-Output "Signed release manifest: $Output"

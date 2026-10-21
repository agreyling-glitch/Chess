$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Security
$repo = Split-Path -Parent $PSScriptRoot
. (Join-Path $repo 'server/windows/update-companion.ps1') -Mode Library
. (Join-Path $repo 'scripts/companion-release-key.ps1')
$root = Join-Path $repo ('tmp/private-signing-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
function Assert([bool]$Value,[string]$Message) { if (-not $Value) { throw $Message } }
function Reject([scriptblock]$Action,[string]$Message) {
    $rejected = $false; try { & $Action | Out-Null } catch { $rejected = $true }
    Assert $rejected $Message
}
$rsa = [Security.Cryptography.RSA]::Create(3072)
try {
    $public = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($rsa.ToXmlString($false)))
    $plain = [Text.Encoding]::UTF8.GetBytes($rsa.ToXmlString($true))
    $encrypted = [Security.Cryptography.ProtectedData]::Protect($plain,$null,[Security.Cryptography.DataProtectionScope]::CurrentUser)
    [Array]::Clear($plain,0,$plain.Length)
    $keyFile = Join-Path $root 'key.json'
    @{format=1;protection='windows-dpapi-current-user';public_key=$public;protected_key=[Convert]::ToBase64String($encrypted)} | ConvertTo-Json | Set-Content -LiteralPath $keyFile
    Assert (-not ((Get-Content -LiteralPath $keyFile -Raw).Contains('<D>'))) 'Private key appeared in plaintext'
    Assert-CompanionPublicKey $public
    $unlocked = Read-CompanionReleaseKey $keyFile
    Assert ($unlocked.ToXmlString($false) -ceq $rsa.ToXmlString($false)) 'Encrypted key failed to unlock'
    $unlocked.Dispose()
    $current = Join-Path $root 'current'; New-Item -ItemType Directory -Path (Join-Path $current 'source') -Force | Out-Null
    foreach ($file in @('ironwood-server.exe','ironwood-companion.exe','start-companion.cmd','LICENSE','source/Cargo.toml')) { Set-Content -LiteralPath (Join-Path $current $file) -Value 'Non-executed test fixture' }
    @{version='0.2.0';signing_mode='private-preview-key'} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $current 'release.json')
    @{signing_mode='private-preview-key';public_key=$public;certificate=''} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $current 'source/update-channel.json')
    $zip = Join-Path $root 'release.zip'; Compress-Archive -LiteralPath $current -DestinationPath $zip
    $manifest = Join-Path $root 'manifest.json'
    & (Join-Path $repo 'scripts/sign-companion-update.ps1') -Package $zip -Version '0.2.0' -DownloadUrl 'https://releases.example/0.2.0/release.zip' -ReleaseKeyFile $keyFile -Output $manifest
    $release = Read-VerifiedManifest $root '' 'private-preview-key' $public
    Assert ($release.format -eq 2) 'Wrong private manifest format'
    Assert-ReleaseArchive $zip $release
    $stage = Join-Path $root 'stage'; Expand-Release $zip $stage
    Assert-ReleaseContent $stage $release 'private-preview-key' '' $public
    # Exercise the actual Windows PowerShell 5.1 runtime used by the native updater.
    $public | Set-Content -LiteralPath (Join-Path $root 'public.txt')
    $checkScript = Join-Path $root 'verify.ps1'
    @'
param($Repo,$Folder)
$ErrorActionPreference='Stop'
. (Join-Path $Repo 'server/windows/update-companion.ps1') -Mode Library
$pin = (Get-Content -LiteralPath (Join-Path $Folder 'public.txt') -Raw).Trim()
$verified = Read-VerifiedManifest $Folder '' 'private-preview-key' $pin
Assert-ReleaseArchive (Join-Path $Folder 'release.zip') $verified
Assert-ReleaseContent (Join-Path $Folder 'stage') $verified 'private-preview-key' '' $pin
'@ | Set-Content -LiteralPath $checkScript
    & powershell.exe -NoLogo -NoProfile -File $checkScript -Repo $repo -Folder $root
    Assert ($LASTEXITCODE -eq 0) 'Windows PowerShell private verification failed'
    Reject { Read-VerifiedManifest $root '' 'authenticode' $public } 'Signing-mode confusion accepted'
    $other = [Security.Cryptography.RSA]::Create(3072)
    try { $wrong = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($other.ToXmlString($false))); Reject { Read-VerifiedManifest $root '' 'private-preview-key' $wrong } 'Wrong signing key accepted' } finally { $other.Dispose() }
    $privatePin = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($rsa.ToXmlString($true)))
    Reject { Assert-CompanionPublicKey $privatePin } 'Private material accepted as a public build pin'
    Reject { Read-VerifiedManifest $root '' 'private-preview-key' $privatePin } 'Updater accepted private material as public pin'
    Add-Content -LiteralPath $zip -Value 'tampered'
    Reject { Assert-ReleaseArchive $zip $release } 'Modified package accepted after check'
    $original = Get-Content -LiteralPath $manifest -Raw
    $originalEnvelope = $original | ConvertFrom-Json
    $originalPayload = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($originalEnvelope.payload))
    foreach ($invalidCase in @('expired','wrong-policy','no-domain')) {
        $modified = $originalPayload | ConvertFrom-Json
        if ($invalidCase -eq 'expired') { $modified.expires = '2000-01-01T00:00:00Z' }
        if ($invalidCase -eq 'wrong-policy') { $modified.signing_mode = 'authenticode' }
        $json = $modified | ConvertTo-Json -Compress
        $signingBytes = [Text.Encoding]::UTF8.GetBytes($(if ($invalidCase -eq 'no-domain') { $json } else { "Ironwood Companion private-preview release v1`n" + $json }))
        $signed = $rsa.SignData($signingBytes,[Security.Cryptography.HashAlgorithmName]::SHA256,[Security.Cryptography.RSASignaturePadding]::Pkcs1)
        @{payload=[Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($json));signature=[Convert]::ToBase64String($signed)} | ConvertTo-Json | Set-Content -LiteralPath $manifest
        Reject { Read-VerifiedManifest $root '' 'private-preview-key' $public } ("Invalid private manifest accepted: " + $invalidCase)
    }
    $original | Set-Content -LiteralPath $manifest
    $envelope = $original | ConvertFrom-Json
    $bytes = [Convert]::FromBase64String($envelope.payload); $bytes[0] = 0
    $envelope.payload = [Convert]::ToBase64String($bytes); $envelope | ConvertTo-Json | Set-Content -LiteralPath $manifest
    Reject { Read-VerifiedManifest $root '' 'private-preview-key' $public } 'Modified manifest accepted'
    $original | Set-Content -LiteralPath $manifest
    $channelFile = Join-Path $stage 'source/update-channel.json'
    @{signing_mode='authenticode';public_key=''} | ConvertTo-Json | Set-Content -LiteralPath $channelFile
    Reject { Assert-ReleaseContent $stage $release 'private-preview-key' '' $public } 'Package disabled its pinned private trust policy'
    Write-Output 'PASS: encrypted key round trip, public-only pins, private manifest/package verification in PS7 and Windows PS5.1, wrong-key/mode/tamper rejection and signing-policy downgrade rejection.'
} finally {
    $rsa.Dispose()
    $resolved = [IO.Path]::GetFullPath($root)
    if (-not $resolved.StartsWith([IO.Path]::GetFullPath((Join-Path $repo 'tmp')) + '\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe fixture cleanup' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}

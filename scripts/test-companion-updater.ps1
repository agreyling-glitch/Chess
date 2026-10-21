$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
. (Join-Path $repo 'server/windows/update-companion.ps1') -Mode Library
$root = Join-Path $repo ('tmp/update-tests-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null
function Assert([bool]$Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Fixture([string]$Name) {
    $path = Join-Path $root $Name
    foreach ($folder in @('current','stage')) { New-Item -ItemType Directory -Path (Join-Path $path $folder) -Force | Out-Null }
    Set-Content -LiteralPath (Join-Path $path 'current/version') -Value old
    Set-Content -LiteralPath (Join-Path $path 'stage/version') -Value new
    return $path
}
try {
    $success = Fixture 'success'
    Switch-Release (Join-Path $success 'current') (Join-Path $success 'stage') (Join-Path $success 'previous') { param($folder) Assert ((Get-Content (Join-Path $folder 'version')) -eq 'new') 'New application was not installed' } { throw 'Unexpected recovery' }
    Assert ((Get-Content (Join-Path $success 'previous/version')) -eq 'old') 'Old version was not retained'

    $failed = Fixture 'startup-failure'; $script:restored = $false
    try { Switch-Release (Join-Path $failed 'current') (Join-Path $failed 'stage') (Join-Path $failed 'previous') { throw 'Simulated unhealthy startup' } { $script:restored = $true }; throw 'Expected startup failure' }
    catch { Assert ($_.Exception.Message -eq 'Simulated unhealthy startup') 'Unexpected startup error' }
    Assert ((Get-Content (Join-Path $failed 'current/version')) -eq 'old') 'Failed update did not restore old application'
    Assert $script:restored 'Data restore was not called'
    Assert ((Get-Content (Join-Path $failed 'stage.failed/version')) -eq 'new') 'Failed candidate was not retained'

    $missing = Fixture 'missing-stage'
    try { Switch-Release (Join-Path $missing 'current') (Join-Path $missing 'absent') (Join-Path $missing 'previous') {} {}; throw 'Expected move failure' }
    catch { Assert ((Get-Content (Join-Path $missing 'current/version')) -eq 'old') 'Move failure lost old application' }

    foreach ($url in @('http://releases.example/test','https://other.example/test','https://name:password@releases.example/test')) {
        $rejected = $false
        try { Assert-Https $url 'releases.example' | Out-Null } catch { $rejected = $true }
        Assert $rejected 'Unsafe download URL accepted'
    }
    $zip = Join-Path $root 'traversal.zip'
    $archive = [IO.Compression.ZipFile]::Open($zip, [IO.Compression.ZipArchiveMode]::Create)
    $entry = $archive.CreateEntry('current/../../escape.txt'); $entry.Open().Dispose(); $archive.Dispose()
    $rejected = $false
    try { Expand-Release $zip (Join-Path $root 'unsafe') } catch { $rejected = $true }
    Assert $rejected 'ZIP traversal was accepted'
    Assert (-not (Test-Path -LiteralPath (Join-Path $root 'escape.txt'))) 'ZIP escaped staging'

    $rejected = $false
    try { Assert-Publisher (Join-Path $repo 'target/debug/ironwood-server.exe') 'not-a-publisher' } catch { $rejected = $true }
    Assert $rejected 'Unsigned/untrusted executable was accepted'

    # A signed envelope must be checked before parsing its payload.
    $key = [Security.Cryptography.RSACng]::new(2048)
    $request = [Security.Cryptography.X509Certificates.CertificateRequest]::new('CN=Updater test only', $key, [Security.Cryptography.HashAlgorithmName]::SHA256, [Security.Cryptography.RSASignaturePadding]::Pkcs1)
    $fixtureCertificate = $request.CreateSelfSigned([DateTimeOffset]::UtcNow.AddDays(-1), [DateTimeOffset]::UtcNow.AddDays(1))
    $pin = [Convert]::ToBase64String($fixtureCertificate.RawData)
    $payload = [Text.Encoding]::UTF8.GetBytes('{"format":1,"product":"Ironwood Companion","platform":"windows-x64","protocol":1,"data_schema":1,"version":"0.2.0","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","expires":"2099-01-01T00:00:00Z"}')
    $signature = $key.SignData($payload, [Security.Cryptography.HashAlgorithmName]::SHA256, [Security.Cryptography.RSASignaturePadding]::Pkcs1)
    $envelope = @{ payload=[Convert]::ToBase64String($payload); signature=[Convert]::ToBase64String($signature) }
    $envelope | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'manifest.json')
    Assert ((Read-VerifiedManifest $root $pin).version -eq '0.2.0') 'Valid signed manifest rejected'
    $payload[0] = 0
    $envelope.payload = [Convert]::ToBase64String($payload)
    $envelope | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'manifest.json')
    $rejected = $false; try { Read-VerifiedManifest $root $pin | Out-Null } catch { $rejected = $true }
    Assert $rejected 'Tampered manifest accepted'
    $accessFile = Join-Path $root 'release-access.json'
    $fixtureToken = 'test-only-private-release-token-0123456789'
    $secureToken = ConvertTo-SecureString $fixtureToken -AsPlainText -Force
    @{format=1;origin='https://releases.example';token=($secureToken | ConvertFrom-SecureString)} | ConvertTo-Json | Set-Content -LiteralPath $accessFile
    $secureToken.Dispose()
    Assert ((Get-ReleaseAuthorization 'https://releases.example/feed.json' $accessFile) -ceq ('Bearer ' + $fixtureToken)) 'Encrypted release access did not round trip'
    foreach ($address in @('https://other.example/feed.json','https://releases.example:444/feed.json','http://releases.example/feed.json')) {
        $rejected = $false
        try { Get-ReleaseAuthorization $address $accessFile | Out-Null } catch { $rejected = $true }
        Assert $rejected 'Private release credential escaped its exact HTTPS origin'
    }
    Assert (-not ((Get-Content -LiteralPath $accessFile -Raw).Contains($fixtureToken))) 'Release token was saved in plaintext'
    $fixtureCertificate.Dispose(); $key.Dispose()
    Write-Output 'PASS: rollback and data recovery, HTTPS host policy, traversal protection, publisher rejection, signed manifest/tamper rejection, encrypted private access and exact-origin credential isolation.'
} finally {
    $resolved = [IO.Path]::GetFullPath($root)
    $allowed = [IO.Path]::GetFullPath((Join-Path $repo 'tmp')) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolved.StartsWith($allowed, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe test cleanup path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}

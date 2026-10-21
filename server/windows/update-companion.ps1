param([ValidateSet('Check','Install','Library')][string]$Mode,
      [string]$Work, [string]$Feed, [string]$Certificate, [string]$CurrentVersion,
      [string]$CredentialFile,
      [ValidateSet('authenticode','private-preview-key')][string]$TrustMode = 'authenticode',
      [string]$PublicKey)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -AssemblyName System.IO.Compression

function Assert-Https([string]$Address, [string]$HostName) {
    $uri = [Uri]$Address
    if (-not $uri.IsAbsoluteUri -or $uri.Scheme -ne 'https' -or $uri.UserInfo -or $uri.Fragment -or ($HostName -and $uri.Host -ne $HostName)) { throw 'Update downloads must use HTTPS on the configured release host.' }
    return $uri
}
function Receive-File([string]$Address, [string]$Destination, [long]$Limit) {
    # No redirects: a manifest cannot send downloads to another origin.
    $request = [Net.HttpWebRequest]::Create($Address)
    $request.AllowAutoRedirect = $false; $request.Timeout = 30000; $request.ReadWriteTimeout = 30000
    if ($CredentialFile -and (Test-Path -LiteralPath $CredentialFile)) {
        $request.Headers['Authorization'] = Get-ReleaseAuthorization $Address $CredentialFile
    }
    $response = $request.GetResponse()
    try {
        if ([int]$response.StatusCode -ne 200 -or $response.ContentLength -gt $Limit) { throw 'Invalid update download response or size.' }
        $source = $response.GetResponseStream(); $target = [IO.File]::Create($Destination)
        try {
            $buffer = New-Object byte[] 65536; $total = 0L
            while (($count = $source.Read($buffer, 0, $buffer.Length)) -gt 0) {
                $total += $count; if ($total -gt $Limit) { throw 'Update download exceeds its size limit.' }
                $target.Write($buffer, 0, $count)
            }
        } finally { $target.Dispose(); $source.Dispose() }
    } finally { $response.Dispose() }
}
function Get-ReleaseAuthorization([string]$Address, [string]$File) {
    $uri = Assert-Https $Address ''
    $credential = Get-Content -LiteralPath $File -Raw | ConvertFrom-Json
    if ($credential.format -ne 1 -or $credential.origin -cne $uri.GetLeftPart([UriPartial]::Authority)) { throw 'Private update credential does not belong to this release origin.' }
    $secure = $null; $pointer = [IntPtr]::Zero
    try {
        $secure = ConvertTo-SecureString $credential.token
        $pointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
        $token = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pointer)
        if ($token.Length -lt 32 -or $token.Length -gt 512 -or $token -match '[\r\n]') { throw 'Invalid private update credential.' }
        return 'Bearer ' + $token
    } catch { throw 'Cannot read private release access. Configure it for the Windows account running Companion.' }
    finally {
        if ($pointer -ne [IntPtr]::Zero) { [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pointer) }
        if ($secure) { $secure.Dispose() }; $token = $null
    }
}
function Read-VerifiedManifest([string]$Folder, [string]$PinnedCertificate, [string]$SigningMode = 'authenticode', [string]$PinnedPublicKey = '') {
    $file = Join-Path $Folder 'manifest.json'
    if ((Get-Item -LiteralPath $file).Length -gt 65536) { throw 'Release manifest exceeds its size limit.' }
    $envelope = Get-Content -LiteralPath $file -Raw | ConvertFrom-Json
    $payload = [Convert]::FromBase64String($envelope.payload)
    $signature = [Convert]::FromBase64String($envelope.signature)
    $cert = $null; $key = $null
    try {
        if ($SigningMode -eq 'private-preview-key') {
            if ($PinnedCertificate -or -not $PinnedPublicKey) { throw 'Invalid private-preview trust configuration.' }
            $key = [Security.Cryptography.RSA]::Create()
            $xml = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($PinnedPublicKey))
            $key.FromXmlString($xml)
            if ($key.KeySize -lt 3072 -or $xml -cne $key.ToXmlString($false)) { throw 'Invalid public release key.' }
            $domain = [Text.Encoding]::UTF8.GetBytes("Ironwood Companion private-preview release v1`n")
            $verifiedBytes = New-Object byte[] ($domain.Length + $payload.Length)
            [Array]::Copy($domain,0,$verifiedBytes,0,$domain.Length); [Array]::Copy($payload,0,$verifiedBytes,$domain.Length,$payload.Length)
        } elseif ($SigningMode -eq 'authenticode' -and -not $PinnedPublicKey) {
            $cert = New-Object Security.Cryptography.X509Certificates.X509Certificate2 -ArgumentList (,[Convert]::FromBase64String($PinnedCertificate))
            $key = [Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPublicKey($cert)
            $verifiedBytes = $payload
        } else { throw 'Unsupported release signing mode.' }
        if (-not $key.VerifyData($verifiedBytes,$signature,[Security.Cryptography.HashAlgorithmName]::SHA256,[Security.Cryptography.RSASignaturePadding]::Pkcs1)) { throw 'Release manifest signature is invalid.' }
    } finally { if ($key) { $key.Dispose() }; if ($cert) { $cert.Dispose() } }
    $manifest = [Text.Encoding]::UTF8.GetString($payload) | ConvertFrom-Json
    $expectedFormat = if ($SigningMode -eq 'private-preview-key') { 2 } else { 1 }
    if ($manifest.product -ne 'Ironwood Companion' -or $manifest.platform -ne 'windows-x64' -or $manifest.format -ne $expectedFormat -or $manifest.protocol -ne 1 -or $manifest.data_schema -ne 1) { throw 'Unsupported Companion release format or data compatibility.' }
    if ($SigningMode -eq 'private-preview-key' -and $manifest.signing_mode -cne 'private-preview-key') { throw 'Release signing mode does not match the embedded trust policy.' }
    if ($manifest.version -notmatch '^\d+\.\d+\.\d+$' -or $manifest.sha256 -notmatch '^[a-fA-F0-9]{64}$') { throw 'Invalid release version or checksum.' }
    if ([DateTimeOffset]::Parse($manifest.expires) -le [DateTimeOffset]::UtcNow) { throw 'Release manifest has expired. Try again after the release feed is refreshed.' }
    return $manifest
}
function Assert-ReleaseContent([string]$Folder, $Release, [string]$SigningMode, [string]$PinnedCertificate, [string]$PinnedPublicKey) {
    if ($SigningMode -eq 'authenticode') {
        foreach ($exe in @('ironwood-server.exe','ironwood-companion.exe')) { Assert-Publisher (Join-Path $Folder $exe) $PinnedCertificate }
    } elseif ($SigningMode -eq 'private-preview-key') {
        # The entire ZIP (including both executables) is authenticated before extraction.
        $channel = Get-Content -LiteralPath (Join-Path $Folder 'source/update-channel.json') -Raw | ConvertFrom-Json
        if ($channel.signing_mode -cne $SigningMode -or $channel.public_key -cne $PinnedPublicKey -or $channel.certificate) { throw 'Private-preview release changes its pinned signing policy.' }
    } else { throw 'Unsupported release signing mode.' }
    $metadata = Get-Content -LiteralPath (Join-Path $Folder 'release.json') -Raw | ConvertFrom-Json
    if ($metadata.version -ne $Release.version) { throw 'Release package version does not match the signed manifest.' }
    if ($SigningMode -eq 'private-preview-key' -and $metadata.signing_mode -cne $SigningMode) { throw 'Release metadata changes the private-preview signing policy.' }
}
function Assert-Publisher([string]$File, [string]$PinnedCertificate) {
    $signature = Get-AuthenticodeSignature -LiteralPath $File
    if ($signature.Status -ne 'Valid' -or -not $signature.SignerCertificate -or [Convert]::ToBase64String($signature.SignerCertificate.RawData) -cne $PinnedCertificate) { throw 'Update executable does not have a valid signature from the configured publisher.' }
}
function Assert-ReleaseArchive([string]$File, $Release) {
    if ((Get-FileHash -LiteralPath $File -Algorithm SHA256).Hash -ine $Release.sha256) { throw 'Update checksum does not match the signed manifest.' }
}
function Expand-Release([string]$Zip, [string]$Destination) {
    $archive = [IO.Compression.ZipFile]::OpenRead($Zip)
    try {
        $root = [IO.Path]::GetFullPath($Destination) + [IO.Path]::DirectorySeparatorChar
        $size = 0L; $seen = @{}
        foreach ($entry in $archive.Entries) {
            # Packages have a single current/ root. Reject traversal and symlinks.
            $name = $entry.FullName.Replace('\','/')
            if (-not $name.StartsWith('current/') -or $name -match '(^|/)\.\.(/|$)|:|^/' -or (($entry.ExternalAttributes -shr 16) -band 0xF000) -eq 0xA000) { throw 'Unsafe update archive entry.' }
            $relative = $name.Substring(8)
            if (-not $relative) { continue }
            $path = [IO.Path]::GetFullPath((Join-Path $Destination $relative))
            if (-not $path.StartsWith($root, [StringComparison]::OrdinalIgnoreCase) -or $seen.ContainsKey($path)) { throw 'Invalid or duplicate update archive path.' }
            $seen[$path] = $true; $size += $entry.Length
            if ($size -gt 2147483648L -or $seen.Count -gt 50000) { throw 'Expanded update exceeds its size limit.' }
            if ($name.EndsWith('/')) { [IO.Directory]::CreateDirectory($path) | Out-Null; continue }
            [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($path)) | Out-Null
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $path, $false)
        }
    } finally { $archive.Dispose() }
    foreach ($file in @('ironwood-server.exe','ironwood-companion.exe','start-companion.cmd','release.json','LICENSE','source/Cargo.toml')) {
        if (-not [IO.File]::Exists((Join-Path $Destination $file))) { throw "Release package is missing $file" }
    }
    foreach ($name in @('config.json','owner-token','.release-channel-credential','.lock','companion.lock','engines','archive')) {
        if (Test-Path -LiteralPath (Join-Path $Destination $name)) { throw 'An update must not contain runtime data.' }
    }
}
function Test-Ready([string]$Executable, [string]$Address, $Process) {
    for ($attempt = 0; $attempt -lt 30; $attempt++) {
        if ($Process.HasExited) { return $false }
        & $Executable healthcheck --address $Address 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0 -and -not $Process.HasExited) { Start-Sleep -Milliseconds 1000; return (-not $Process.HasExited) }
        Start-Sleep -Milliseconds 500
    }
    return $false
}
function Start-Companion([string]$Folder, $Request, [string]$Pending) {
    # Original arguments are stored as data, never evaluated as shell code.
    $arguments = @($Request.args)
    if ($Pending) { $arguments += @('--update-pending', $Pending) }
    $quoted = @($arguments | ForEach-Object { '"' + ([string]$_ -replace '(\\*)"','$1$1\"' -replace '(\\+)$','$1$1') + '"' })
    $options = @{ FilePath = (Join-Path $Folder $Request.executable); WorkingDirectory = $Folder; WindowStyle = 'Hidden'; PassThru = $true }
    if ($quoted.Count) { $options.ArgumentList = $quoted }
    return Start-Process @options
}
function Resolve-DataPath([string]$Data, [string]$Value) {
    if ([IO.Path]::IsPathRooted($Value)) { return [IO.Path]::GetFullPath($Value) }
    return [IO.Path]::GetFullPath((Join-Path $Data $Value))
}
function Switch-Release([string]$Install, [string]$Stage, [string]$Previous, [scriptblock]$Start, [scriptblock]$RestoreData) {
    # The caller has verified paths and staged signatures; never delete the old build.
    Move-Item -LiteralPath $Install -Destination $Previous
    try { Move-Item -LiteralPath $Stage -Destination $Install; & $Start $Install }
    catch {
        if (Test-Path -LiteralPath $Install) { Move-Item -LiteralPath $Install -Destination ($Stage + '.failed') }
        Move-Item -LiteralPath $Previous -Destination $Install
        & $RestoreData
        throw
    }
}
if ($Mode -eq 'Library') { return }
try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    if ($Mode -eq 'Check') {
        $origin = Assert-Https $Feed ''
        Receive-File $Feed (Join-Path $Work 'manifest.json') 65536
        $release = Read-VerifiedManifest $Work $Certificate $TrustMode $PublicKey
        if ([version]$release.version -le [version]$CurrentVersion) { exit 0 }
        $url = Assert-Https $release.url $origin.Host
        if ($url.GetLeftPart([UriPartial]::Authority) -cne $origin.GetLeftPart([UriPartial]::Authority)) { throw 'Release download must use the exact configured HTTPS origin.' }
        Receive-File $url.AbsoluteUri (Join-Path $Work 'release.zip') 629145600
        Assert-ReleaseArchive (Join-Path $Work 'release.zip') $release
        $stage = Join-Path $Work 'current'; Expand-Release (Join-Path $Work 'release.zip') $stage
        Assert-ReleaseContent $stage $release $TrustMode $Certificate $PublicKey
        [IO.File]::WriteAllText((Join-Path $Work 'available.txt'), $release.version)
    } elseif ($Mode -eq 'Install') {
        $request = Get-Content -LiteralPath (Join-Path $Work 'request.json') -Raw | ConvertFrom-Json
        $release = Read-VerifiedManifest $Work $request.certificate $request.trust_mode $request.public_key
        Assert-ReleaseArchive (Join-Path $Work 'release.zip') $release
        # Re-extract the authenticated archive rather than trusting mutable staged files.
        $stage = Join-Path $Work ('install-' + [Guid]::NewGuid().ToString('N')); Expand-Release (Join-Path $Work 'release.zip') $stage
        Assert-ReleaseContent $stage $release $request.trust_mode $request.certificate $request.public_key
        $install = [IO.Path]::GetFullPath($request.install)
        $data = [IO.Path]::GetFullPath($request.data)
        $workRoot = [IO.Path]::GetFullPath($Work)
        if ([IO.Path]::GetDirectoryName($install) -ne [IO.Path]::GetDirectoryName($workRoot) -or $data -eq $install -or $data.StartsWith($install + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Keep runtime data outside the application folder before updating.' }
        foreach ($folder in @($install,$workRoot,$stage)) {
            if ((Get-Item -LiteralPath $folder).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Update folders cannot be directory links.' }
        }
        [IO.File]::WriteAllText((Join-Path $Work 'ready-to-stop'), 'Verified; waiting for graceful shutdown.')
        $old = Get-Process -Id $request.pid -ErrorAction SilentlyContinue
        if ($old -and -not $old.WaitForExit(60000)) { throw 'Companion did not stop. Application files have not been replaced.' }
        $restartOnFailure = $true
        # Snapshot database using the OLD executable after all writers have drained.
        $recovery = Join-Path $data ('.update-recovery-' + [Guid]::NewGuid().ToString('N'))
        [IO.Directory]::CreateDirectory($recovery) | Out-Null
        $config = Get-Content -LiteralPath $request.config -Raw | ConvertFrom-Json
        Copy-Item -LiteralPath $request.config -Destination (Join-Path $recovery 'config.json')
        $tokenName = if ($config.token_file) { $config.token_file } else { 'owner-token' }
        $token = Resolve-DataPath $data $tokenName
        Copy-Item -LiteralPath $token -Destination (Join-Path $recovery 'owner-token')
        $archive = $null
        if ($config.archive) {
            $archive = Resolve-DataPath $data $config.archive
            & (Join-Path $install 'ironwood-server.exe') archive-backup --config $request.config --output (Join-Path $recovery 'archive.sqlite3') 2>&1 | Out-Null
            if ($LASTEXITCODE -ne 0) { throw 'Could not snapshot the archive; update was not installed.' }
        }
        $previous = Join-Path $Work 'previous'
        $pending = Join-Path $Work 'startup-pending'
        [IO.File]::WriteAllText($pending, 'Client access is paused until the update passes startup checks.')
        $start = {
            param($folder)
            $script:newProcess = Start-Companion $folder $request $pending
            if (-not (Test-Ready (Join-Path $folder 'ironwood-server.exe') $request.address $script:newProcess)) {
                if (-not $script:newProcess.HasExited) { Stop-Process -Id $script:newProcess.Id; $script:newProcess.WaitForExit() }
                throw 'Updated Companion failed its startup health check.'
            }
            [IO.File]::Delete($pending)
        }
        $restore = {
            Copy-Item -LiteralPath (Join-Path $recovery 'config.json') -Destination $request.config -Force
            Copy-Item -LiteralPath (Join-Path $recovery 'owner-token') -Destination $token -Force
            if ($archive) {
                # archive-restore deliberately requires a NEW destination. Retain
                # the failed candidate's database and sidecars before restoring.
                $failedData = Join-Path $recovery 'failed-database'
                [IO.Directory]::CreateDirectory($failedData) | Out-Null
                foreach ($suffix in @('', '-wal', '-shm')) {
                    $source = $archive + $suffix
                    if (Test-Path -LiteralPath $source) {
                        Move-Item -LiteralPath $source -Destination (Join-Path $failedData ('archive.sqlite3' + $suffix))
                    }
                }
                & (Join-Path $install 'ironwood-server.exe') archive-restore --config $request.config --input (Join-Path $recovery 'archive.sqlite3') 2>&1 | Out-Null
                if ($LASTEXITCODE -ne 0) { throw "Archive recovery failed. Backup retained in $recovery" }
            }
        }
        try {
            Switch-Release $install $stage $previous $start $restore
            $restartOnFailure = $false
            [IO.File]::WriteAllText((Join-Path $data 'last-update.txt'), "Updated to $($release.version). Previous application retained at $previous")
        } catch {
            $failure = $_.Exception.Message
            [IO.File]::WriteAllText((Join-Path $data 'last-update.txt'), "Update failed; previous version restored. $failure")
            throw
        }
    } else { throw 'Missing updater mode.' }
} catch {
    [IO.File]::WriteAllText((Join-Path $Work 'error.txt'), $_.Exception.Message)
    if ($restartOnFailure -and (Test-Path -LiteralPath (Join-Path $install $request.executable))) {
        Start-Companion $install $request | Out-Null
    }
    exit 1
}

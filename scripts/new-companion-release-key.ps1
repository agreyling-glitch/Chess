param([string]$OutputDirectory = (Join-Path $env:LOCALAPPDATA 'Ironwood/ReleaseSigning'))
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Key creation requires Windows and PowerShell 7.' }
$repo = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$folder = [IO.Path]::GetFullPath($OutputDirectory)
if ($folder -eq $repo -or $folder.StartsWith($repo + '\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Keep the signing key outside the repository and package folders.' }
$keyPath = Join-Path $folder 'release-key.dpapi.json'
$publicPath = Join-Path $folder 'release-public-key.json'
if ((Test-Path -LiteralPath $keyPath) -or (Test-Path -LiteralPath $publicPath)) { throw 'A release key or public identity already exists. It has not been replaced.' }
New-Item -ItemType Directory -Path $folder -Force | Out-Null
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
$acl = [Security.AccessControl.DirectorySecurity]::new()
$acl.SetAccessRuleProtection($true,$false); $acl.SetOwner($sid)
$acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl','ContainerInherit,ObjectInherit','None','Allow'))
Set-Acl -LiteralPath $folder -AclObject $acl
$rsa = [Security.Cryptography.RSA]::Create(3072); $plain = $null
try {
    $public = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($rsa.ToXmlString($false)))
    $plain = [Text.Encoding]::UTF8.GetBytes($rsa.ToXmlString($true))
    $protected = [Security.Cryptography.ProtectedData]::Protect($plain,$null,[Security.Cryptography.DataProtectionScope]::CurrentUser)
    $fingerprint = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Convert]::FromBase64String($public))).ToLowerInvariant()
    $document = @{format=1;protection='windows-dpapi-current-user';public_key=$public;key_id=$fingerprint;protected_key=[Convert]::ToBase64String($protected)} | ConvertTo-Json
    # CreateNew prevents an accidental or concurrent key replacement.
    $stream = [IO.File]::Open($keyPath,'CreateNew','Write','None')
    try { $bytes = [Text.Encoding]::UTF8.GetBytes($document); $stream.Write($bytes,0,$bytes.Length) } finally { $stream.Dispose() }
    @{format=1;signing_mode='private-preview-key';public_key=$public;key_id=$fingerprint} | ConvertTo-Json | Set-Content -LiteralPath $publicPath -Encoding utf8
    Write-Output "Protected release key: $keyPath"
    Write-Output "Public build configuration: $publicPath"
    Write-Output "Public-key fingerprint: $fingerprint"
} finally { if ($plain) { [Array]::Clear($plain,0,$plain.Length) }; $rsa.Dispose() }

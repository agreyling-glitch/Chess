param([string]$DataDirectory = (Join-Path $env:LOCALAPPDATA 'Ironwood/Companion'),
      [string]$Feed = 'https://companion-releases.ironwoodchess.com/private-preview/windows-x64/manifest.json')
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Private native-updater credentials require Windows DPAPI.' }
$uri = [uri]$Feed
if ($uri.Scheme -ne 'https' -or $uri.UserInfo -or $uri.Query -or $uri.Fragment) { throw 'Use an HTTPS release URL without credentials or query parameters.' }
if (-not (Test-Path -LiteralPath $DataDirectory -PathType Container)) { throw 'Choose the existing Companion data directory.' }
$secret = Read-Host 'Private release-channel token (not the Companion owner token)' -AsSecureString
if ($secret.Length -lt 32) { throw 'Use a random release-channel token of at least 32 characters.' }
try {
    $path = Join-Path $DataDirectory '.release-channel-credential'
    $document = @{format=1;origin=$uri.GetLeftPart([UriPartial]::Authority);token=($secret | ConvertFrom-SecureString)} | ConvertTo-Json -Compress
    # CurrentUser DPAPI encryption; the secret is never an argument or build input.
    [IO.File]::WriteAllText($path,$document,[Text.Encoding]::UTF8)
    $sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $acl = [Security.AccessControl.FileSecurity]::new()
    $acl.SetAccessRuleProtection($true,$false); $acl.SetOwner($sid)
    $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl','Allow'))
    Set-Acl -LiteralPath $path -AclObject $acl
    Write-Output 'Private release access configured for this Windows account. No Companion restart is needed for this credential.'
} finally { $secret.Dispose() }

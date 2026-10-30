param(
    [ValidatePattern('^[a-zA-Z0-9.-]+$')][string]$NasHost = '192.168.0.2',
    [ValidatePattern('^[a-zA-Z0-9_-]+$')][string]$Username = 'admin',
    [switch]$PrepareOnly,
    [ValidatePattern('^/share/Public/ironwood-companion-upgrade-[a-f0-9]{32}$')][string]$ResumeRemoteDirectory
)
$ErrorActionPreference = 'Stop'
# Codex may launch Windows PowerShell with a PowerShell 7 module search path.
# Load the matching Windows modules explicitly rather than relying on discovery.
foreach ($taskModule in @('Microsoft.PowerShell.Utility','Microsoft.PowerShell.Host')) {
    Import-Module (Join-Path $PSHOME "Modules/$taskModule/$taskModule.psd1") -Force
}
$taskRepo = Split-Path -Parent $PSScriptRoot
$taskOutput = Join-Path $taskRepo 'output/companion-qnap'
$taskVersion = ((Get-Content -LiteralPath (Join-Path $taskRepo 'server/Cargo.toml') | Select-String '^version = "([^"]+)"$' | Select-Object -First 1).Matches.Groups[1].Value)
$taskBundle = Join-Path $taskOutput "ironwood-companion-qnap-$taskVersion-build.tar.gz"
$taskStatus = Join-Path $taskOutput 'upgrade-status.json'
$taskLog = Join-Path $taskOutput 'upgrade-session.log'
if (!(Test-Path -LiteralPath $taskBundle)) { throw "Prepared source bundle missing: $taskBundle" }
$taskRemoteId = [Guid]::NewGuid().ToString('N')
$taskUpload = "/share/Public/ironwood-companion-source-$taskRemoteId.tar.gz"
$taskRemote = if ($ResumeRemoteDirectory) { $ResumeRemoteDirectory } else { "/share/Public/ironwood-companion-upgrade-$taskRemoteId" }
$taskLogin = "$Username@$NasHost"
$taskHasher = [System.Security.Cryptography.SHA256]::Create()
$taskStream = [System.IO.File]::OpenRead($taskBundle)
try { $taskExpected = ([BitConverter]::ToString($taskHasher.ComputeHash($taskStream))).Replace('-', '').ToLowerInvariant() }
finally { $taskStream.Dispose(); $taskHasher.Dispose() }
function Save-UpgradeStatus([string]$State, [string]$Message) {
    @{state=$State;message=$Message;nas=$NasHost;version=$taskVersion;remoteDirectory=$taskRemote;updated=(Get-Date).ToUniversalTime().ToString('o')} | ConvertTo-Json | Set-Content -LiteralPath $taskStatus -Encoding UTF8
}
if ($PrepareOnly) { Write-Output "Prepared Companion $taskVersion for $taskLogin; source: $taskBundle"; return }
Start-Transcript -Path $taskLog -Force | Out-Null
try {
    Save-UpgradeStatus 'awaiting_sign_in' 'Enter the QNAP password at the SCP prompt.'
    Write-Host 'Ironwood QNAP upgrade' -ForegroundColor Yellow
    Write-Host 'Enter the NAS password only at the SSH/SCP prompts. It is not saved.'
    Write-Host 'The runner will inspect the existing app, build and test the image, then make a private recovery copy before replacing only its image.'
    Write-Host 'Existing volumes, Import/Reports mounts, ports, settings and token are preserved. No volume is removed.'
    if (!$ResumeRemoteDirectory) {
        & scp -O -o StrictHostKeyChecking=yes $taskBundle "${taskLogin}:$taskUpload"
        if ($LASTEXITCODE -ne 0) { throw 'Source upload failed; the NAS container has not been changed.' }
    }
    Save-UpgradeStatus 'building_and_upgrading' 'Sign in at the SSH prompt. The existing service stays running until the new image passes its checks.'
    $taskCommand = "umask 077; mkdir -m 700 '$taskRemote' && mv '$taskUpload' '$taskRemote/source.tar.gz' && cd '$taskRemote' && echo '$taskExpected  source.tar.gz' | sha256sum -c - && tar -xzf source.tar.gz && sh deploy/ironwood-server/qnap-run-upgrade.sh"
    if ($ResumeRemoteDirectory) {
        $taskPatch = Join-Path $taskOutput 'qnap-upgrade-patch.tar.gz'
        Push-Location $taskRepo
        try {
            & tar -czf $taskPatch deploy/ironwood-server/qnap-upgrade.sh deploy/ironwood-server/qnap-build.sh deploy/ironwood-server/qnap-run-upgrade.sh
            if ($LASTEXITCODE -ne 0) { throw 'Could not prepare upgrade scripts.' }
        } finally { Pop-Location }
        Write-Host 'Uploading the corrected scripts. Enter the NAS password at the SCP prompt.'
        & scp -O -o StrictHostKeyChecking=yes -o ServerAliveInterval=30 $taskPatch "${taskLogin}:$taskRemote/upgrade-patch.tar.gz"
        if ($LASTEXITCODE -ne 0) { throw 'Script upload failed. Existing Companion was not changed by this attempt.' }
        $taskCommand = 'cd ''{REMOTE}'' && { if [ -f .upgrade-running/pid ] && ! kill -0 "$(cat .upgrade-running/pid)" 2>/dev/null; then mv .upgrade-running .upgrade-stale-$(date +%Y%m%d-%H%M%S); fi; if [ ! -d .upgrade-running ] && [ ! -f upgrade-result ]; then tar -xzf upgrade-patch.tar.gz; fi; sh deploy/ironwood-server/qnap-run-upgrade.sh; }'
        $taskCommand = $taskCommand.Replace('{REMOTE}', $taskRemote)
    }
    Write-Host 'Starting or monitoring the NAS job. Enter the NAS password at the SSH prompt.'
    & ssh -tt -o StrictHostKeyChecking=yes -o ServerAliveInterval=30 -o ServerAliveCountMax=6 $taskLogin $taskCommand | Tee-Object -FilePath (Join-Path $taskOutput 'upgrade-remote.log')
    if ($LASTEXITCODE -ne 0) { throw "SSH ended before success was confirmed. A NAS job may still be running. Review $taskRemote/upgrade-run.log and upgrade-result before retrying; never remove the data volume." }
    Save-UpgradeStatus 'complete' 'Build, health, existing mounts, configuration and token checks passed.'
    Write-Host 'QNAP Companion upgrade complete.' -ForegroundColor Green
    Write-Host "Private NAS recovery files are under: $taskRemote"
} catch {
    Save-UpgradeStatus 'needs_attention' $_.Exception.Message
    Write-Host $_.Exception.Message -ForegroundColor Red
    throw
} finally {
    Stop-Transcript | Out-Null
}

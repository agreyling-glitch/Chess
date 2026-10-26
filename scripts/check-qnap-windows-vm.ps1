param(
    [ValidatePattern('^[a-zA-Z0-9.-]+$')][string]$NasHost = '192.168.0.2',
    [ValidatePattern('^[a-zA-Z0-9_-]+$')][string]$Username = 'admin'
)
$ErrorActionPreference = 'Stop'
$taskRepo = Split-Path -Parent $PSScriptRoot
$taskScript = Join-Path $taskRepo 'deploy/ironwood-server/qnap-windows-vm-preflight.sh'
$taskSource = [IO.File]::ReadAllText($taskScript).Replace("`r`n", "`n")
$taskEncoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($taskSource))
Write-Host 'This reads the NAS CPU, memory, disk space and Virtualization Station metadata. It makes no NAS changes.'
Write-Host 'Enter the QNAP password at the SSH prompt. It is not saved.'
& ssh -o StrictHostKeyChecking=yes "$Username@$NasHost" "printf '%s' '$taskEncoded' | base64 -d | sh"
if ($LASTEXITCODE -ne 0) { throw 'QNAP VM preflight did not complete.' }

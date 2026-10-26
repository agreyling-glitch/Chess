param(
    [ValidatePattern('^[a-zA-Z0-9.-]+$')][string]$NasHost = '192.168.0.2',
    [ValidatePattern('^[a-zA-Z0-9_-]+$')][string]$Username = 'admin'
)
$ErrorActionPreference = 'Stop'
$taskRepo = Split-Path -Parent $PSScriptRoot
$taskScript = Join-Path $taskRepo 'deploy/ironwood-server/qnap-repair-folder-permissions.sh'
$taskSource = [IO.File]::ReadAllText($taskScript).Replace("`r`n", "`n")
$taskEncoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($taskSource))
Write-Host 'This briefly stops only Ironwood Companion, repairs Import/Reports permissions, and restarts it.'
Write-Host 'Enter the QNAP password at the SSH prompt. It is not saved.'
& ssh -o StrictHostKeyChecking=yes "$Username@$NasHost" "printf '%s' '$taskEncoded' | base64 -d | sh"
if ($LASTEXITCODE -ne 0) { throw 'QNAP permission repair did not complete. Review the output; no files were deleted.' }

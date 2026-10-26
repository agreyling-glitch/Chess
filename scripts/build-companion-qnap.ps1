param(
    [ValidatePattern('^[a-zA-Z0-9.-]+$')][string]$NasHost = '192.168.0.2',
    [ValidatePattern('^[a-zA-Z0-9_-]+$')][string]$Username = 'admin'
)
$ErrorActionPreference = 'Stop'
$taskRepo = Split-Path -Parent $PSScriptRoot
$taskOutput = Join-Path $taskRepo 'output/companion-qnap'
$taskBundle = Join-Path $taskOutput 'ironwood-companion-qnap-build.tar.gz'
if (!(Test-Path -LiteralPath $taskBundle)) { throw 'The prepared QNAP source bundle is missing.' }
$taskRemote = '/share/Public/ironwood-companion-build-' + [Guid]::NewGuid().ToString('N')
$taskLogin = "$Username@$NasHost"
Write-Host 'Enter the QNAP password only at the SSH/SCP prompts. It is not stored by this script.'
& ssh -o StrictHostKeyChecking=yes $taskLogin "mkdir -m 700 '$taskRemote'"
if ($LASTEXITCODE -ne 0) { throw 'Could not create the dedicated NAS build folder.' }
& scp -O -o StrictHostKeyChecking=yes $taskBundle "${taskLogin}:$taskRemote/source.tar.gz"
if ($LASTEXITCODE -ne 0) { throw 'Source upload failed.' }
& ssh -o StrictHostKeyChecking=yes $taskLogin "cd '$taskRemote' && tar -xzf source.tar.gz && sh deploy/ironwood-server/qnap-build.sh"
if ($LASTEXITCODE -ne 0) { throw "NAS build failed. Build files and build.log are retained in $taskRemote" }
foreach ($taskName in @('ironwood-companion-qnap-linux-amd64.tar.gz','ironwood-companion-qnap-linux-amd64.tar.gz.sha256','image-info.json','build.log')) {
    & scp -O -o StrictHostKeyChecking=yes "${taskLogin}:$taskRemote/$taskName" (Join-Path $taskOutput $taskName)
    if ($LASTEXITCODE -ne 0) { throw "Could not retrieve $taskName. It remains in $taskRemote" }
}
$taskImage = Join-Path $taskOutput 'ironwood-companion-qnap-linux-amd64.tar.gz'
$taskExpected = ((Get-Content -LiteralPath "$taskImage.sha256" -Raw).Trim() -split '\s+')[0]
if ((Get-FileHash -LiteralPath $taskImage -Algorithm SHA256).Hash -ine $taskExpected) { throw 'Exported image checksum did not match.' }
Write-Host "Built, tested and downloaded: $taskImage"
Write-Host "NAS build folder retained: $taskRemote"

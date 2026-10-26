param([string]$ReportPath = "$PSScriptRoot\windows-update-check.json")
$ErrorActionPreference = 'Stop'
$report = [ordered]@{
    format = 'ironwood-clean-windows-update-check'
    timestamp = [DateTime]::UtcNow.ToString('o')
    status = 'running'
    os = $null
    rebootRequired = $null
    updates = @()
}
try {
    $os = Get-CimInstance Win32_OperatingSystem
    $version = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
    $report.os = [ordered]@{
        caption = $os.Caption
        version = $os.Version
        build = $os.BuildNumber
        revision = $version.UBR
        architecture = $os.OSArchitecture
    }
    $session = New-Object -ComObject Microsoft.Update.Session
    $session.ClientApplicationID = 'Ironwood clean Windows validation'
    $searcher = $session.CreateUpdateSearcher()
    $searcher.Online = $true
    Write-Output 'Checking Windows Update online; this may take several minutes.'
    $result = $searcher.Search("IsInstalled=0 and IsHidden=0")
    if ([int]$result.ResultCode -ne 2) {
        throw "Windows Update search did not fully succeed (result $([int]$result.ResultCode))."
    }
    $report.updates = @(for ($i = 0; $i -lt $result.Updates.Count; $i++) {
        $update = $result.Updates.Item($i)
        [ordered]@{
            title = $update.Title
            kb = @($update.KBArticleIDs)
            type = [int]$update.Type
            optional = [bool]$update.BrowseOnly
            autoSelected = [bool]$update.AutoSelectOnWebSites
            downloaded = [bool]$update.IsDownloaded
        }
    })
    $system = New-Object -ComObject Microsoft.Update.SystemInfo
    $report.rebootRequired = [bool]$system.RebootRequired
    $report.status = 'complete'
} catch {
    $report.status = 'failed'
    $report.error = $_.Exception.Message
    throw
} finally {
    $folder = Split-Path -Parent ([IO.Path]::GetFullPath($ReportPath))
    $null = New-Item -ItemType Directory -Force -Path $folder
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ReportPath -Encoding UTF8
    Write-Output "Update check report: $ReportPath"
}

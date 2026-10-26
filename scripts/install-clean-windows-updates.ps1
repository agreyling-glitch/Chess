param([string]$ReportPath = "$PSScriptRoot\windows-update-install.json")
$ErrorActionPreference = 'Stop'
$report = [ordered]@{ timestamp=[DateTime]::UtcNow.ToString('o'); status='running'; updates=@(); rebootRequired=$false }
try {
    $session = New-Object -ComObject Microsoft.Update.Session
    $session.ClientApplicationID = 'Ironwood clean Windows validation'
    $search = $session.CreateUpdateSearcher().Search("IsInstalled=0 and IsHidden=0 and Type='Software'")
    if ([int]$search.ResultCode -ne 2) { throw 'Windows Update search did not fully succeed.' }
    $selected = New-Object -ComObject Microsoft.Update.UpdateColl
    for ($i=0; $i -lt $search.Updates.Count; $i++) {
        $update = $search.Updates.Item($i)
        if (!$update.BrowseOnly -and $update.AutoSelectOnWebSites -and !$update.InstallationBehavior.CanRequestUserInput) {
            if (!$update.EulaAccepted) { $update.AcceptEula() }
            $null = $selected.Add($update)
        }
    }
    Write-Output "Downloading $($selected.Count) regular updates."
    if ($selected.Count) {
        $downloader=$session.CreateUpdateDownloader()
        $downloader.Updates=$selected
        $download=$downloader.Download()
        $report.updates=@(for($i=0;$i -lt $selected.Count;$i++) {
            $item=$download.GetUpdateResult($i)
            [ordered]@{title=$selected.Item($i).Title; stage='download'; resultCode=[int]$item.ResultCode; hresult=$item.HResult}
        })
        if ([int]$download.ResultCode -ne 2) { throw "Update download result: $([int]$download.ResultCode)" }
        $installer=$session.CreateUpdateInstaller()
        $installer.Updates=$selected
        Write-Output 'Installing regular updates.'
        $installed=$installer.Install()
        $report.updates=@(for($i=0;$i -lt $selected.Count;$i++) {
            $item=$installed.GetUpdateResult($i)
            [ordered]@{title=$selected.Item($i).Title; resultCode=[int]$item.ResultCode; hresult=$item.HResult}
        })
        $report.rebootRequired=[bool]$installed.RebootRequired
        if ([int]$installed.ResultCode -ne 2) { throw "Update installation result: $([int]$installed.ResultCode)" }
    }
    $report.status='complete'
} catch {
    $report.status='failed'
    $report.error=$_.Exception.Message
    throw
} finally {
    $report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $ReportPath -Encoding UTF8
    Write-Output "Update installation report: $ReportPath"
}

param([Parameter(Mandatory=$true)][string]$ZipPath,[string]$ReportPath='tmp/companion-windows-validation.json',[ValidateSet('development','clean-vm','windows-sandbox')][string]$EnvironmentKind='development',[string]$EnvironmentEvidence='',[string]$PublisherPackage='')
$ErrorActionPreference='Stop'
$checks=New-Object 'System.Collections.Generic.List[string]'
$processes=New-Object 'System.Collections.Generic.List[System.Diagnostics.Process]'
$zip=(Resolve-Path -LiteralPath $ZipPath).Path
$reportFile=[IO.Path]::GetFullPath($ReportPath)
$root=Join-Path ([IO.Path]::GetTempPath()) ('ironwood-platform-'+[guid]::NewGuid().ToString('N'))
$report=[ordered]@{
    format='ironwood-windows-platform-validation';
    version=1;
    status='running';
    environmentKind=$EnvironmentKind;
    environmentEvidence=$EnvironmentEvidence;
    osVersion=[Environment]::OSVersion.VersionString;
    architecture=$env:PROCESSOR_ARCHITECTURE;
    timestamp=[DateTime]::UtcNow.ToString('o');
    packageSha256=(Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant();
    checks=@()
}
function Require($condition,[string]$message){
    if(-not $condition){
        throw $message
    }
}
function Native([string[]]$Arguments){
    $output=& $script:binary @Arguments 2>&1;
    if($LASTEXITCODE -ne 0){
        throw "Native command failed: $($Arguments[0])"
    };
    return $output
}
function Start-Companion {
    $stdout=Join-Path $root ('stdout-'+[guid]::NewGuid().ToString('N'));
    $stderr=$stdout+'.err';
    $oldPath=$env:PATH
    try{
        $env:PATH="$env:SystemRoot\System32;$env:SystemRoot;$env:SystemRoot\System32\WindowsPowerShell\v1.0";
        $p=Start-Process -FilePath $script:binary -ArgumentList @('--data-dir',('"'+$script:data+'"')) -WindowStyle Hidden -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr;
        $null=$p.Handle;
        $processes.Add($p)
    }finally{
        $env:PATH=$oldPath
    }
    $deadline=[DateTime]::UtcNow.AddSeconds(10)
    do{
        if($p.HasExited){
            throw 'Packaged Companion exited before readiness'
        };
        $text=if(Test-Path -LiteralPath $stdout){
            Get-Content -LiteralPath $stdout -Raw
        }else{
            ''
        };
        if($text -match '"address":"(127\.0\.0\.1:\d+)"'){
            return @{
                process=$p;
                address=('http://'+$Matches[1]);
                stdout=$stdout;
                stderr=$stderr
            }
        };
        Start-Sleep -Milliseconds 50
    }while([DateTime]::UtcNow -lt $deadline)
    throw 'Startup timeout'
}
function Request($server,[string]$path,$body=$null,[string]$credential=$script:token,[string]$origin='http://localhost:8788',[string]$method=''){
    if(-not $method){
        $method=if($null -eq $body){
            'GET'
        }else{
            'POST'
        }
    }
    $parameters=@{
        Uri=($server.address+'/api/v1/'+$path);
        Method=$method;
        Headers=@{
            Origin=$origin;
            Authorization=('Bearer '+$credential);
            'X-Ironwood-Protocol'='1'
        };
        UseBasicParsing=$true;
        TimeoutSec=10
    }
    if($null -ne $body){
        $parameters.Body=($body|ConvertTo-Json -Depth 30 -Compress);
        $parameters.ContentType='application/json'
    }
    try{
        $response=Invoke-WebRequest @parameters;
        return @{
            status=[int]$response.StatusCode;
            value=($response.Content|ConvertFrom-Json)
        }
    }catch{
        if($_.Exception.Response){
            return @{
                status=[int]$_.Exception.Response.StatusCode;
                value=$null
            }
        };
        throw
    }
}
function Stop-Companion($server){
    Require ((Request $server 'shutdown' $null $script:token 'http://localhost:8788' 'POST').status -eq 202) 'Shutdown rejected'
    Require ($server.process.WaitForExit(10000)) 'Shutdown timeout';
    Require ($server.process.ExitCode -eq 0) "Shutdown failed (exit code: $($server.process.ExitCode))"
    $logs=[string](Get-Content -LiteralPath $server.stdout -Raw)+[string](Get-Content -LiteralPath $server.stderr -Raw)
    Require (-not $logs.Contains($script:token)) 'Credential leaked into logs';
    Require ($logs -notmatch 'shutdown_deadline') 'Shutdown deadline hit'
}
function Receive-Type($socket,[string]$type){
    for($i=0;
    $i -lt 30;
    $i++){
        $buffer=New-Object byte[] 65536;
        $cancel=New-Object System.Threading.CancellationTokenSource;
        $cancel.CancelAfter(10000)
        try{
            $segment=New-Object 'System.ArraySegment[byte]' -ArgumentList @(,$buffer);
            $result=$socket.ReceiveAsync($segment,$cancel.Token).GetAwaiter().GetResult();
            Require ($result.EndOfMessage -and $result.MessageType -eq [Net.WebSockets.WebSocketMessageType]::Text) 'Invalid socket response';
            $message=[Text.Encoding]::UTF8.GetString($buffer,0,$result.Count)|ConvertFrom-Json;
            if($message.type -eq 'error'){
                throw 'Engine protocol error'
            };
            if($message.type -eq $type){
                return $message
            }
        }finally{
            $cancel.Dispose()
        }
    }
    throw "Missing protocol message: $type"
}
function Send-Message($socket,$message){
    $bytes=[Text.Encoding]::UTF8.GetBytes(($message|ConvertTo-Json -Depth 15 -Compress));
    $segment=New-Object 'System.ArraySegment[byte]' -ArgumentList @(,$bytes);
    $socket.SendAsync($segment,[Net.WebSockets.WebSocketMessageType]::Text,$true,[Threading.CancellationToken]::None).GetAwaiter().GetResult()|Out-Null
}
try{
    Require ($env:OS -eq 'Windows_NT') 'Windows is required';
    Require ($EnvironmentKind -eq 'development' -or $EnvironmentEvidence.Length -gt 0) 'Clean-machine claims require VM/base-image evidence'
    New-Item -ItemType Directory -Path $root|Out-Null
    $sidecar=$zip+'.sha256';
    Require (Test-Path -LiteralPath $sidecar) 'ZIP checksum sidecar required';
    $expected=(Get-Content -LiteralPath $sidecar -Raw).Split(' ')[0].Trim().ToLowerInvariant();
    Require ($expected -eq $report.packageSha256) 'Package checksum mismatch'
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [IO.Compression.ZipFile]::ExtractToDirectory($zip,(Join-Path $root 'package'))
    $executables=@(Get-ChildItem -LiteralPath (Join-Path $root 'package') -Filter ironwood-server.exe -Recurse);
    Require ($executables.Count -eq 1) 'Expected one executable';
    $script:binary=$executables[0].FullName
    $report.binarySha256=(Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant()
    $metadata=Get-Content -LiteralPath (Join-Path $executables[0].DirectoryName 'release.json') -Raw|ConvertFrom-Json;
    Require ($metadata.crt -eq 'static') 'Static CRT package required'
    Require (@(Get-ChildItem -LiteralPath (Join-Path $root 'package') -Recurse -File|Where-Object Extension -in '.rtbw','.rtbz').Count -eq 0) 'Dataset accidentally bundled';
    $checks.Add('zip-checksum-static-crt-metadata-no-datasets')
    $script:data=Join-Path $root 'Companion data with spaces';
    Native @('init','--data-dir',$data)|Out-Null
    $script:token=(Get-Content -LiteralPath (Join-Path $data 'owner-token') -Raw).Trim();
    Require ($token -match '^[a-f0-9]{64}$') 'Invalid credential';
    Require ((Get-Acl -LiteralPath $data).AreAccessRulesProtected) 'Inherited data permissions';
    $checks.Add('private-provisioning')
    $fixture=Join-Path $data 'test-engine.ps1'
    @('while ($null -ne ($line = [Console]::ReadLine())) {','switch -Wildcard ($line) {','"uci" { [Console]::WriteLine("id name Ironwood Platform Fixture"); [Console]::WriteLine("option name SyzygyPath type string default <empty>"); [Console]::WriteLine("uciok") }','"isready" { [Console]::WriteLine("readyok") }','"go *" { [Console]::WriteLine("info depth 1 score cp 12 pv e2e4"); [Console]::WriteLine("bestmove e2e4") }','"quit" { exit 0 }','}','}')|Set-Content -LiteralPath $fixture -Encoding UTF8
    $configFile=Join-Path $data 'config.json';
    $config=Get-Content -LiteralPath $configFile -Raw|ConvertFrom-Json;
    $config.listen='127.0.0.1:0'
    $config.engines=@(@{
        id='fixture';
        label='Original platform fixture';
        executable="$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe";
        args=@('-NoProfile','-NonInteractive','-File',$fixture)
    })
    $config|Add-Member -Force NoteProperty tablebase_directories @($data);
    [IO.File]::WriteAllText($configFile,($config|ConvertTo-Json -Depth 30),(New-Object Text.UTF8Encoding($false)))
    $server=Start-Companion;
    Native @('healthcheck','--address',([uri]$server.address).Authority)|Out-Null
    Require ((Request $server 'capabilities' $null 'wrong').status -eq 401) 'Authentication failed open';
    Require ((Request $server 'capabilities' $null $token 'https://invalid.example').status -eq 403) 'Origin check failed open'
    $caps=(Request $server 'capabilities').value;
    Require ($caps.services.uci -and $caps.services.archive -and $caps.services.explorer -and $caps.services.tablebases) 'Service configuration failed';
    $checks.Add('restricted-path-health-auth-origin-capabilities')
    $pgn="[White `"Portable`"]`n[Black `"Validation`"]`n`n1. e4 e5 2. Nf3 Nc6 *";
    $imported=(Request $server 'archive/import' @{
        pgn=$pgn
    }).value;
    Require ($imported.imported -eq 1) 'Archive import failed';
    $id=$imported.ids[0]
    $exportReply=Request $server 'archive/export' @{ids=@($id);format='pgn'};
    Require ($exportReply.status -eq 200) 'Archive PGN export request failed';
    $exported=$exportReply.value.pgn;
    Require ($exported -match '2\.\s*Nf3\s+(?:2\.\.\.\s+)?Nc6') 'Binary archive PGN export lost moves';
    $duplicate=(Request $server 'archive/import' @{pgn=$exported}).value;
    Require ($duplicate.imported -eq 0 -and $duplicate.duplicates -eq 1) 'PGN round-trip duplicate detection failed';
    $short=(Request $server 'archive/import' @{pgn='1. e4 e5 *'}).value;
    Require ($short.discarded -eq 1 -and $short.imported -eq 0) 'Short-game discard policy failed';
    $checks.Add('binary-archive-pgn-export-round-trip-duplicates-short-policy');
    $group=(Request $server 'archive/collections' @{action='create';name='Release validation'}).value;
    Require ($group.id.Length -gt 0) 'Collection creation failed';
    Require ((Request $server 'archive/import' @{pgn=$exported;collection=$group.id}).value.duplicates -eq 1) 'Collection membership duplicated a game';
    $filter='archive/library?collection='+[uri]::EscapeDataString($group.id);
    Require ((Request $server $filter).value.total -eq 1) 'Collection library filter failed';
    $backupFolder=Join-Path $data 'Backups with spaces';
    $savedBackup=Request $server 'archive/backups/settings' @{folder=$backupFolder};
    Require ($savedBackup.status -eq 200) 'Backup folder save failed';
    $backupFolder=$savedBackup.value.folder;
    Require ((Request $server 'archive/backups' @{action='backup'}).status -eq 200) 'Server backup did not start';
    $deadline=[DateTime]::UtcNow.AddSeconds(30);
    do {$backup=(Request $server 'archive/backups').value;if($backup.job.state -ne 'running'){break};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline);
    Require ($backup.job.state -eq 'complete') 'Server backup did not complete';
    $backupName=$backup.job.file;
    Require ([IO.File]::Exists([IO.Path]::Combine($backupFolder,$backupName))) 'Server backup file missing';
    $checks.Add('collection-membership-indexed-filter-and-server-backup');


    $deadline=[DateTime]::UtcNow.AddSeconds(10);
    do{
        $pending=(Request $server 'archive/explorer/status').value.pending;
        if($pending -eq 0){
            break
        };
        Start-Sleep -Milliseconds 50
    }while([DateTime]::UtcNow -lt $deadline);
    Require ($pending -eq 0) 'Index timeout'
    $fen=[uri]::EscapeDataString('rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1');
    Require ((Request $server ('archive/explorer?fen='+$fen)).value.total -eq 1) 'Explorer mismatch'
    $draw=[uri]::EscapeDataString('8/8/8/8/8/2k5/8/K7 w - - 99 1');
    Require ((Request $server ('tablebases/probe?fen='+$draw)).value.category -eq 'draw') 'Local draw failed';
    $checks.Add('archive-explorer-local-probe')
    $ticket=(Request $server 'tickets' $null $token 'http://localhost:8788' 'POST').value.ticket;
    $socket=New-Object Net.WebSockets.ClientWebSocket
    try{
        $socket.Options.SetRequestHeader('Origin','http://localhost:8788');
        $socket.ConnectAsync([uri]($server.address.Replace('http:','ws:')+'/api/v1/socket'),[Threading.CancellationToken]::None).GetAwaiter().GetResult()|Out-Null;
        Send-Message $socket @{
            type='authenticate';
            ticket=$ticket
        };
        Receive-Type $socket 'ready'|Out-Null;
        Send-Message $socket @{
            type='open_engine';
            request_id='open';
            engine_id='fixture'
        };
        $opened=Receive-Type $socket 'engine_opened';
        Send-Message $socket @{
            type='search';
            session_id=$opened.session_id;
            search_id='search';
            position_id='initial';
            fen=[uri]::UnescapeDataString($fen);
            limit=@{
                kind='depth';
                plies=1
            }
        };
        Require ((Receive-Type $socket 'bestmove').bestmove -eq 'e2e4') 'UCI move mismatch';
        Send-Message $socket @{
            type='close_engine';
            session_id=$opened.session_id
        };
        Receive-Type $socket 'engine_closed'|Out-Null;
        $checks.Add('uci-handshake-search-close-inbox-powershell-fixture')
    }finally{
        $socket.Dispose()
    }
    if ($PublisherPackage) {
        $catalog=(Request $server 'engine-download-catalog').value
        $package=@($catalog.packages | Where-Object id -eq $PublisherPackage)
        Require ($package.Count -eq 1) 'Publisher package unavailable on this platform'
        $started=Request $server 'engine-downloads' @{package=$PublisherPackage}
        Require ($started.status -in 200,202) 'Publisher download did not start'
        $deadline=[DateTime]::UtcNow.AddMinutes(10)
        do {
            $jobs=(Request $server 'engine-downloads').value.jobs
            $job=@($jobs | Where-Object id -eq $PublisherPackage)[0]
            if ($job.phase -in 'ready','paused','cancelled') { break }
            Start-Sleep -Seconds 2
        } while ([DateTime]::UtcNow -lt $deadline)
        Require ($job.phase -eq 'ready' -and $job.engine_id) ('Publisher install failed: '+$job.message)
        $report.publisherEngine=[ordered]@{package=$PublisherPackage;source=$package[0].source;license=$package[0].license}
        $ticket=(Request $server 'tickets' $null $token 'http://localhost:8788' 'POST').value.ticket
        $socket=New-Object Net.WebSockets.ClientWebSocket
        try {
            $socket.Options.SetRequestHeader('Origin','http://localhost:8788')
            $socket.ConnectAsync([uri]($server.address.Replace('http:','ws:')+'/api/v1/socket'),[Threading.CancellationToken]::None).GetAwaiter().GetResult()|Out-Null
            Send-Message $socket @{type='authenticate';ticket=$ticket}
            Receive-Type $socket 'ready'|Out-Null
            Send-Message $socket @{type='open_engine';request_id='publisher-open';engine_id=$job.engine_id}
            $opened=Receive-Type $socket 'engine_opened'
            Send-Message $socket @{type='search';session_id=$opened.session_id;search_id='publisher-search';position_id='initial';fen=[uri]::UnescapeDataString($fen);limit=@{kind='depth';plies=2}}
            $best=Receive-Type $socket 'bestmove'
            Require ($best.bestmove -match '^[a-h][1-8][a-h][1-8][qrbn]?$') 'Publisher engine did not return a move'
            Send-Message $socket @{type='close_engine';session_id=$opened.session_id}
            Receive-Type $socket 'engine_closed'|Out-Null
            $checks.Add('publisher-direct-download-install-real-uci-search')
        } finally { $socket.Dispose() }
    }
    Stop-Companion $server;
    $server=Start-Companion;
    Require ((Request $server ('archive/games/'+$id)).status -eq 200) 'Persistence failed';
    Require ((Request $server 'archive/backups/settings').value.folder -eq $backupFolder) 'Backup folder did not persist';
    Stop-Companion $server;
    $restoredConfig=Get-Content -LiteralPath $configFile -Raw|ConvertFrom-Json;
    $restoredConfig.archive=Join-Path $data 'restored.sqlite3';
    [IO.File]::WriteAllText($configFile,($restoredConfig|ConvertTo-Json -Depth 30),(New-Object Text.UTF8Encoding($false)));
    $server=Start-Companion;
    Require ((Request $server 'archive/backups' @{action='restore';file=$backupName}).status -eq 200) 'Server restore did not start';
    $deadline=[DateTime]::UtcNow.AddSeconds(30);
    do {$restore=(Request $server 'archive/backups').value;if($restore.job.state -ne 'running'){break};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline);
    Require ($restore.job.state -eq 'complete') 'Server restore did not complete';
    Require ((Request $server ('archive/games/'+$id)).status -eq 200) 'Restore lost the game';
    Require ((Request $server $filter).value.total -eq 1) 'Restore lost collection membership';
    Require ((Request $server 'archive/collections').value[0].name -eq 'Release validation') 'Restore lost collection metadata';
    $checks.Add('server-restore-games-collections-and-persisted-folder');
    Stop-Companion $server
    $old=$token;
    Native @('rotate-token','--data-dir',$data)|Out-Null;
    $script:token=(Get-Content -LiteralPath (Join-Path $data 'owner-token') -Raw).Trim();
    Require ($token -ne $old) 'Rotation failed';
    $server=Start-Companion;
    Require ((Request $server 'capabilities' $null $old).status -eq 401) 'Old credential accepted';
    Stop-Companion $server;
    $checks.Add('restart-persistence-rotation-graceful-shutdown');
    $report.status='passed'
}catch{
    $report.status='failed';
    $report.error=$_.Exception.Message;
    throw
}finally{
    foreach($p in $processes){
        if(-not $p.HasExited){
            Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue;
            $p.WaitForExit(5000)|Out-Null
        }
    }
    $report.checks=@($checks);
    New-Item -ItemType Directory -Force -Path ([IO.Path]::GetDirectoryName($reportFile))|Out-Null;
    [IO.File]::WriteAllText($reportFile,($report|ConvertTo-Json -Depth 20),(New-Object Text.UTF8Encoding($false)))
    $resolvedRoot=[IO.Path]::GetFullPath($root);
    $parent=[IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')+'\'
    if($resolvedRoot.StartsWith($parent,[StringComparison]::OrdinalIgnoreCase) -and ([IO.Path]::GetFileName($resolvedRoot)).StartsWith('ironwood-platform-') -and (Test-Path -LiteralPath $resolvedRoot)){
        Remove-Item -LiteralPath $resolvedRoot -Recurse -Force
    }
}
Write-Output "Windows validation passed ($EnvironmentKind): $reportFile"

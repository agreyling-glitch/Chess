param([string]$OutputDirectory='output/companion-docker-validation')
$ErrorActionPreference='Stop'
$repo=Split-Path -Parent $PSScriptRoot
$destination=[IO.Path]::GetFullPath((Join-Path $repo $OutputDirectory))
if(Test-Path -LiteralPath $destination){throw 'Choose a fresh context output directory'}
$stage=Join-Path $destination 'context'
New-Item -ItemType Directory -Path $stage -Force|Out-Null
$files=New-Object 'System.Collections.Generic.List[string]'
foreach($name in @('Cargo.toml','Cargo.lock','build.rs','LICENSE','.dockerignore')){$files.Add((Join-Path $repo $name))}
foreach($directory in @('src','crates','server','tests','deploy/ironwood-server')){
 foreach($file in (Get-ChildItem -LiteralPath (Join-Path $repo $directory) -Recurse -File)){
  if(($file.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){continue}
  if($file.Name -eq 'owner-token' -or $file.Name -eq 'config.json' -or $file.Name -like '*.sqlite3*' -or $file.Extension -in '.rtbw','.rtbz'){continue}
  if($file.FullName -match '[\\/](node_modules|target|tmp|\.git)[\\/]'){continue}
  $files.Add($file.FullName)
 }
}
foreach($name in @('tablebase.test.mjs','run-companion-platform-checks.mjs','validate-companion-container-api.mjs','validate-companion-docker.sh')){$files.Add((Join-Path $repo ('scripts/'+$name)))}
foreach($name in @('companion-library.js','companion-feature.js','companion-connection.js','companion-engine-adapter.js','companion-archive-client.js','companion-explorer.js','game-storage.js','tablebase.js')){$files.Add((Join-Path $repo ('web/'+$name)))}
foreach($file in $files){
 $relative=$file.Substring($repo.Length+1)
 $target=Join-Path $stage $relative
 New-Item -ItemType Directory -Force -Path ([IO.Path]::GetDirectoryName($target))|Out-Null
 Copy-Item -LiteralPath $file -Destination $target
}
$archive=Join-Path $destination 'ironwood-companion-validation-context.tar.gz'
& tar.exe -czf $archive -C $stage .
if($LASTEXITCODE -ne 0){throw 'Validation context archive failed'}
$hash=(Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText(($archive+'.sha256'), "$hash  ironwood-companion-validation-context.tar.gz`n", [Text.Encoding]::ASCII)
@{format='ironwood-validation-context';version=1;sha256=$hash;files=$files.Count;scope='current working tree; source and development fixtures only; no owner credentials or tablebase files'}|ConvertTo-Json|Set-Content -LiteralPath (Join-Path $destination 'context.json') -Encoding utf8
Write-Output "Created $archive"

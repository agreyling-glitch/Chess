# Compatibility entry point: regenerate icons from the approved brand masters.
node (Join-Path $PSScriptRoot 'build-brand-assets.mjs')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

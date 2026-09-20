Add-Type -AssemblyName System.Drawing

$projectRoot = Split-Path -Parent $PSScriptRoot
$webRoot = Join-Path $projectRoot 'web'
$sourceSize = 1024
$background = [System.Drawing.ColorTranslator]::FromHtml('#101317')
$gold = [System.Drawing.ColorTranslator]::FromHtml('#d3ad62')
$ivory = [System.Drawing.ColorTranslator]::FromHtml('#f3f0e4')

function New-IconBitmap([int]$size) {
    $source = New-Object System.Drawing.Bitmap $sourceSize, $sourceSize
    $source.SetResolution(96, 96)
    $graphics = [System.Drawing.Graphics]::FromImage($source)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $graphics.Clear($background)

    $goldPen = New-Object System.Drawing.Pen $gold, 24
    $goldPen.Color = [System.Drawing.Color]::FromArgb(128, $gold)
    $graphics.DrawEllipse($goldPen, 114, 114, 796, 796)

    $ivoryPen = New-Object System.Drawing.Pen $ivory, 10
    $ivoryPen.Color = [System.Drawing.Color]::FromArgb(36, $ivory)
    $graphics.DrawEllipse($ivoryPen, 206, 206, 612, 612)

    $font = New-Object System.Drawing.Font 'Georgia', 730, ([System.Drawing.FontStyle]::Regular), ([System.Drawing.GraphicsUnit]::Pixel)
    $format = New-Object System.Drawing.StringFormat
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center
    $brush = New-Object System.Drawing.SolidBrush $ivory
    $graphics.DrawString(([char]0x265E), $font, $brush, (New-Object System.Drawing.RectangleF 0, 16, $sourceSize, $sourceSize), $format)

    $target = New-Object System.Drawing.Bitmap $size, $size
    $target.SetResolution(96, 96)
    $targetGraphics = [System.Drawing.Graphics]::FromImage($target)
    $targetGraphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $targetGraphics.DrawImage($source, 0, 0, $size, $size)

    $targetGraphics.Dispose()
    $brush.Dispose()
    $format.Dispose()
    $font.Dispose()
    $ivoryPen.Dispose()
    $goldPen.Dispose()
    $graphics.Dispose()
    $source.Dispose()
    return $target
}

$outputs = @{
    16 = 'favicon-16x16.png'
    32 = 'favicon-32x32.png'
    48 = 'favicon-48x48.png'
    150 = 'mstile-150x150.png'
    180 = 'apple-touch-icon.png'
    192 = 'android-chrome-192x192.png'
    512 = 'android-chrome-512x512.png'
}

foreach ($entry in $outputs.GetEnumerator()) {
    $bitmap = New-IconBitmap $entry.Key
    $bitmap.Save((Join-Path $webRoot $entry.Value), [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
}

$icoImages = @(16, 32, 48) | ForEach-Object {
    [System.IO.File]::ReadAllBytes((Join-Path $webRoot $outputs[$_]))
}
$icoPath = Join-Path $webRoot 'favicon.ico'
$stream = [System.IO.File]::Create($icoPath)
$writer = New-Object System.IO.BinaryWriter $stream
$writer.Write([uint16]0)
$writer.Write([uint16]1)
$writer.Write([uint16]$icoImages.Count)
$offset = 6 + (16 * $icoImages.Count)
for ($index = 0; $index -lt $icoImages.Count; $index++) {
    $dimension = @(16, 32, 48)[$index]
    $writer.Write([byte]$dimension)
    $writer.Write([byte]$dimension)
    $writer.Write([byte]0)
    $writer.Write([byte]0)
    $writer.Write([uint16]1)
    $writer.Write([uint16]32)
    $writer.Write([uint32]$icoImages[$index].Length)
    $writer.Write([uint32]$offset)
    $offset += $icoImages[$index].Length
}
foreach ($image in $icoImages) {
    $writer.Write($image)
}
$writer.Dispose()
$stream.Dispose()

Write-Output 'Generated Ironwood Chess favicon assets.'

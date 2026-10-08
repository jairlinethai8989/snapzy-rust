$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path $PSScriptRoot
$frames = @()
foreach ($size in @(16,24,32,48,64,128,256)) {
    $bitmap = [Drawing.Bitmap]::new($size,$size)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.ScaleTransform($size/256.0,$size/256.0)
    $path = [Drawing.Drawing2D.GraphicsPath]::new()
    foreach ($corner in @(@(0,0,90,180),@(166,0,90,270),@(166,166,90,0),@(0,166,90,90))) {
        $path.AddArc($corner[0],$corner[1],$corner[2],$corner[2],$corner[3],90)
    }
    $path.CloseFigure()
    $blue = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml('#1765f3'))
    $cyan = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml('#63d1f4'))
    $white = [Drawing.Pen]::new([Drawing.Color]::White,19)
    $white.StartCap = $white.EndCap = [Drawing.Drawing2D.LineCap]::Round
    $graphics.FillPath($blue,$path)
    $graphics.DrawLines($white,[Drawing.PointF[]]@([Drawing.PointF]::new(48,88),[Drawing.PointF]::new(48,52),[Drawing.PointF]::new(84,52)))
    $graphics.DrawLines($white,[Drawing.PointF[]]@([Drawing.PointF]::new(172,52),[Drawing.PointF]::new(208,52),[Drawing.PointF]::new(208,88)))
    $graphics.DrawLines($white,[Drawing.PointF[]]@([Drawing.PointF]::new(48,168),[Drawing.PointF]::new(48,204),[Drawing.PointF]::new(84,204)))
    $graphics.DrawLines($white,[Drawing.PointF[]]@([Drawing.PointF]::new(172,204),[Drawing.PointF]::new(208,204),[Drawing.PointF]::new(208,168)))
    $graphics.FillRectangle($cyan,80,93,96,72)
    $graphics.DrawRectangle($white,80,93,96,72)
    $graphics.FillRectangle([Drawing.Brushes]::White,165,167,81,79)
    $font = [Drawing.Font]::new('Segoe UI',60,[Drawing.FontStyle]::Bold,[Drawing.GraphicsUnit]::Pixel)
    $graphics.DrawString('R',$font,$blue,171,158)
    $stream = [IO.MemoryStream]::new()
    $bitmap.Save($stream,[Drawing.Imaging.ImageFormat]::Png)
    $frames += ,$stream.ToArray()
    if ($size -eq 256) { $bitmap.Save((Join-Path $root 'assets\snapzy-rust.png'),[Drawing.Imaging.ImageFormat]::Png) }
    $stream.Dispose(); $font.Dispose(); $white.Dispose(); $cyan.Dispose(); $blue.Dispose(); $path.Dispose(); $graphics.Dispose(); $bitmap.Dispose()
}
$file = [IO.File]::Create((Join-Path $root 'assets\snapzy-rust.ico'))
$writer = [IO.BinaryWriter]::new($file)
try {
    $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$frames.Count)
    $offset = 6 + 16*$frames.Count
    $sizes = @(16,24,32,48,64,128,256)
    for ($i=0;$i -lt $frames.Count;$i++) {
        $dimension = if ($sizes[$i] -eq 256) {0} else {$sizes[$i]}
        $writer.Write([byte]$dimension); $writer.Write([byte]$dimension)
        $writer.Write([byte]0); $writer.Write([byte]0)
        $writer.Write([uint16]1); $writer.Write([uint16]32)
        $writer.Write([uint32]$frames[$i].Length); $writer.Write([uint32]$offset)
        $offset += $frames[$i].Length
    }
    foreach ($frame in $frames) { $writer.Write([byte[]]$frame) }
} finally { $writer.Dispose(); $file.Dispose() }
Write-Output 'Generated SnapZy Rust Preview icon with R badge.'

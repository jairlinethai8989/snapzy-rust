$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$directory = Join-Path $PSScriptRoot '..\tests\fixtures\ocr'
New-Item -ItemType Directory -Force $directory | Out-Null
$cases = @(
    @{ Name = 'english'; Text = 'SnapZy OCR reads English text 12345'; Font = 'Segoe UI' },
    @{ Name = 'thai'; Text = [string]::Concat([char]0x0e22,[char]0x0e34,[char]0x0e19,[char]0x0e14,[char]0x0e35,[char]0x0e15,[char]0x0e49,[char]0x0e2d,[char]0x0e19,[char]0x0e23,[char]0x0e31,[char]0x0e1a,' ',[char]0x0e20,[char]0x0e32,[char]0x0e29,[char]0x0e32,[char]0x0e44,[char]0x0e17,[char]0x0e22); Font = 'Leelawadee UI' },
    @{ Name = 'mixed'; Text = [string]::Concat([char]0x0e2a,[char]0x0e27,[char]0x0e31,[char]0x0e2a,[char]0x0e14,[char]0x0e35,' SnapZy 2026'); Font = 'Leelawadee UI' },
    @{ Name = 'blank'; Text = ''; Font = 'Segoe UI' }
)
foreach ($case in $cases) {
    $bitmap = [Drawing.Bitmap]::new(1200, 180, [Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.Clear([Drawing.Color]::White)
    $graphics.TextRenderingHint = [Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    if ($case.Text) {
        $font = [Drawing.Font]::new($case.Font, 42, [Drawing.FontStyle]::Regular, [Drawing.GraphicsUnit]::Pixel)
        $brush = [Drawing.SolidBrush]::new([Drawing.Color]::Black)
        $graphics.DrawString($case.Text, $font, $brush, 24, 28)
        $brush.Dispose()
        $font.Dispose()
    }
    $bitmap.Save((Join-Path $directory ($case.Name + '.png')), [Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
}

param(
    [string]$Version = "0.14.0-alpha.1",
    [string]$OutputSuffix = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$packageRoot = Join-Path $repoRoot "dist\SnapZy-Rust-$Version-win-x64"
$stageRoot = Join-Path $repoRoot "work\installer-stage-$Version"
$outputRoot = Join-Path $repoRoot "dist"
$outputName = if ([string]::IsNullOrWhiteSpace($OutputSuffix)) {
    "SnapZy-Rust-Setup-$Version.exe"
} else {
    "SnapZy-Rust-Setup-$Version-$OutputSuffix.exe"
}
$outputPath = Join-Path $outputRoot $outputName
$sedPath = Join-Path $stageRoot "SnapZy-Rust-Setup.sed"

if (-not (Test-Path (Join-Path $packageRoot "SnapZy-Rust.exe"))) {
    throw "Packaged executable was not found: $packageRoot"
}

New-Item -ItemType Directory -Path $stageRoot -Force | Out-Null
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null

$payloadNames = @(
    "SnapZy-Rust.exe",
    "leptonica-1.82.0.dll",
    "tesseract50.dll",
    "LICENSE",
    "README.md",
    "THIRD-PARTY-NOTICES.md"
)

foreach ($name in $payloadNames) {
    $source = Join-Path $packageRoot $name
    if (-not (Test-Path $source)) {
        throw "Installer payload is missing: $source"
    }
    Copy-Item -LiteralPath $source -Destination (Join-Path $stageRoot $name) -Force
}

$installScript = @'
$ErrorActionPreference = "Stop"
$installDir = Join-Path $env:LOCALAPPDATA "Programs\SnapZyRust"
New-Item -ItemType Directory -Path $installDir -Force | Out-Null

$payloadFiles = @(
    "SnapZy-Rust.exe",
    "leptonica-1.82.0.dll",
    "tesseract50.dll",
    "LICENSE",
    "README.md",
    "THIRD-PARTY-NOTICES.md"
)

foreach ($name in $payloadFiles) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $installDir $name) -Force
}

$shell = New-Object -ComObject WScript.Shell
$desktopShortcut = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath("Desktop")) "SnapZy Rust.lnk"))
$desktopShortcut.TargetPath = Join-Path $installDir "SnapZy-Rust.exe"
$desktopShortcut.WorkingDirectory = $installDir
$desktopShortcut.Description = "SnapZy Rust Windows capture"
$desktopShortcut.Save()

$startMenuDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\SnapZy Rust"
New-Item -ItemType Directory -Path $startMenuDir -Force | Out-Null
$startMenuShortcut = $shell.CreateShortcut((Join-Path $startMenuDir "SnapZy Rust.lnk"))
$startMenuShortcut.TargetPath = Join-Path $installDir "SnapZy-Rust.exe"
$startMenuShortcut.WorkingDirectory = $installDir
$startMenuShortcut.Description = "SnapZy Rust Windows capture"
$startMenuShortcut.Save()

Start-Process -FilePath (Join-Path $installDir "SnapZy-Rust.exe") -WorkingDirectory $installDir
'@

Set-Content -LiteralPath (Join-Path $stageRoot "install.ps1") -Value $installScript -Encoding ASCII

$files = @(
    $payloadNames | ForEach-Object { Get-Item -LiteralPath (Join-Path $stageRoot $_) }
    Get-Item -LiteralPath (Join-Path $stageRoot "install.ps1")
) | Sort-Object Name
$sourceEntries = [System.Text.StringBuilder]::new()
$stringEntries = [System.Text.StringBuilder]::new()
for ($index = 0; $index -lt $files.Count; $index++) {
    [void]$sourceEntries.AppendLine("%FILE$index%=")
    [void]$stringEntries.AppendLine("FILE$index=`"$($files[$index].Name)`"")
}

$sed = @"
[Version]
Class=IEXPRESS
SEDVersion=3

[Options]
PackagePurpose=InstallApp
ShowInstallProgramWindow=1
HideExtractAnimation=1
UseLongFileName=1
InsideCompressed=1
CAB_FixedSize=0
CAB_ResvCodeSigning=0
RebootMode=N
InstallPrompt=%InstallPrompt%
DisplayLicense=%DisplayLicense%
FinishMessage=%FinishMessage%
TargetName=%TargetName%
FriendlyName=%FriendlyName%
AppLaunched=PowerShell.exe -NoProfile -ExecutionPolicy Bypass -File install.ps1
PostInstallCmd=<None>
AdminQuietInstCmd=
UserQuietInstCmd=
SourceFiles=SourceFiles

[SourceFiles]
SourceFiles0=$stageRoot

[SourceFiles0]
$sourceEntries
[Strings]
InstallPrompt="Install SnapZy Rust for this Windows account?"
DisplayLicense="LICENSE"
FinishMessage="SnapZy Rust installation completed."
TargetName="$outputPath"
FriendlyName="SnapZy Rust $Version Setup"
$stringEntries
"@

Set-Content -LiteralPath $sedPath -Value $sed -Encoding ASCII

$iexpress = Join-Path $env:WINDIR "System32\iexpress.exe"
if (-not (Test-Path $iexpress)) {
    throw "IExpress was not found: $iexpress"
}

$iexpressProcess = Start-Process -FilePath $iexpress -ArgumentList @("/N", "/Q", $sedPath) -Wait -PassThru
if ($iexpressProcess.ExitCode -ne 0) {
    throw "IExpress failed with exit code $($iexpressProcess.ExitCode)"
}

if (-not (Test-Path $outputPath)) {
    throw "Installer was not created: $outputPath"
}

Get-FileHash -Algorithm SHA256 -LiteralPath $outputPath

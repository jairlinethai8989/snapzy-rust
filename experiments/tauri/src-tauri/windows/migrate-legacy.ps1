param(
    [ValidateSet("CheckRunning", "Migrate")]
    [string]$Mode = "Migrate"
)

$ErrorActionPreference = "Stop"

# This installer source is tracked beside installer-hooks.nsh, not generated resources.

if ($Mode -eq "CheckRunning") {
    $running = Get-Process -ErrorAction SilentlyContinue | Where-Object {
        $_.ProcessName -in @("SnapZy-Rust", "snapzy-tauri", "SnapZy", "snapzy-engine")
    }
    if ($running) {
        exit 20
    }
    exit 0
}

$legacyTarget = [IO.Path]::GetFullPath(
    (Join-Path $env:LOCALAPPDATA "Programs\SnapZyRust\SnapZy-Rust.exe")
)
$shortcutPaths = @(
    (Join-Path ([Environment]::GetFolderPath("Desktop")) "SnapZy Rust.lnk"),
    (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\SnapZy Rust\SnapZy Rust.lnk")
)
$shell = New-Object -ComObject WScript.Shell

foreach ($shortcutPath in $shortcutPaths) {
    if (-not (Test-Path -LiteralPath $shortcutPath)) {
        continue
    }
    $shortcut = $shell.CreateShortcut($shortcutPath)
    if (-not [string]::IsNullOrWhiteSpace($shortcut.TargetPath)) {
        $target = [IO.Path]::GetFullPath($shortcut.TargetPath)
        if ([string]::Equals($target, $legacyTarget, [StringComparison]::OrdinalIgnoreCase)) {
            Remove-Item -LiteralPath $shortcutPath -Force
        }
    }
}

$legacyStartMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\SnapZy Rust"
if ((Test-Path -LiteralPath $legacyStartMenu) -and
    -not (Get-ChildItem -LiteralPath $legacyStartMenu -Force | Select-Object -First 1)) {
    Remove-Item -LiteralPath $legacyStartMenu -Force
}

$uninstallRoot = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall"
if (Test-Path -LiteralPath $uninstallRoot) {
    foreach ($entry in Get-ChildItem -LiteralPath $uninstallRoot -ErrorAction SilentlyContinue) {
        $properties = Get-ItemProperty -LiteralPath $entry.PSPath -ErrorAction SilentlyContinue
        if ($properties.DisplayName -ne "SnapZy Tauri Trial") {
            continue
        }
        $uninstallString = [Environment]::ExpandEnvironmentVariables(
            [string]$properties.UninstallString
        ).Trim()
        $uninstaller = if ($uninstallString -match '^"([^"]+)"') {
            $Matches[1]
        } elseif ($uninstallString -match '^([^\s]+\.exe)') {
            $Matches[1]
        } else {
            ""
        }
        if ([string]::IsNullOrWhiteSpace($uninstaller) -or
            -not (Test-Path -LiteralPath $uninstaller)) {
            Remove-Item -LiteralPath $entry.PSPath -Recurse -Force
        }
    }
}

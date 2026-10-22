# SnapZy Tauri Upgrade Installer

**Date:** 2026-10-22  
**Target:** SnapZy Rust `0.15.0-alpha.1` local tester installer  
**Scope:** Tauri + Vanilla JavaScript application and Windows installer only

## Problem

The current Tauri installer is registered as a separate product named
`SnapZy Tauri Trial 0.1.1`. The existing Desktop and Start Menu shortcuts still
launch `%LOCALAPPDATA%\Programs\SnapZyRust\SnapZy-Rust.exe`, version
`0.14.0-alpha.1`. Installing the trial therefore does not upgrade the program the
tester normally opens. A stale Tauri Trial uninstall entry can also remain when
its install directory is missing.

## Selected Approach

Promote the Tauri shell and editor to the next SnapZy Rust alpha and package it
as the real current-user SnapZy application. This replaces the normal launcher
and shortcuts while retaining the proven Rust capture engine and all user data.

Alternatives rejected:

- Keep a side-by-side `Tauri Trial`: this caused the reported old-version launch.
- Redirect only the old shortcut: Windows would still show two products and two
  unrelated version numbers.
- Delete every previous installation before copying the new app: this creates
  unnecessary risk for settings and recoverable tester files.

## Product Identity and Version

- Use the Windows product name `SnapZy Rust` and a stable SnapZy Rust
  application identifier rather than the temporary `tauri-trial` identity.
  The in-app brand remains `SnapZy`. This prevents the alpha installer from
  replacing the separate `.NET` product's `SnapZy.lnk` shortcut.
- Advance the complete tester candidate to `0.15.0-alpha.1`.
- Show `0.15.0-alpha.1` consistently in the launcher, editor, About window,
  executable metadata, Windows installed-app entry, installer filename and test
  notes.
- The bundled capture engine is rebuilt from the same source and reports the
  same version. Do not expose a second UI version.
- Keep this as a local alpha. Do not push, tag or publish a GitHub Release.

## Windows Installation

- Build one Tauri NSIS current-user installer; no administrator privilege is
  required.
- Install the new application in its own managed Tauri directory under the
  current user's Local AppData.
- Create one Desktop shortcut and one Start Menu shortcut named `SnapZy Rust`,
  both pointing to the new Tauri executable. These replace the identically named
  legacy Rust shortcuts without changing the separate `.NET` `SnapZy.lnk`.
- On successful installation, remove a remaining legacy `SnapZy Rust` shortcut
  only when it still points to
  `%LOCALAPPDATA%\Programs\SnapZyRust\SnapZy-Rust.exe`. Do not remove a
  same-named shortcut if its target is different.
- Preserve the legacy program folder during this tester migration. It is no
  longer reachable from the normal shortcuts and can be removed in a later
  approved cleanup after the new build is validated.
- If the stale `SnapZy Tauri Trial` uninstall entry points to a missing
  uninstaller, remove that orphaned registry entry. If a working uninstaller is
  present, leave it available so Windows can remove the trial normally.
- Preserve `%LOCALAPPDATA%\SnapZyRustPreview`, `.snapzy` projects, captures,
  recordings, OCR data and all unrelated SnapZy/.NET files.

## Upgrade Safety

- The installer must ask the user to close a running legacy or new SnapZy before
  replacement. It must never terminate unrelated processes.
- Migration cleanup runs only after the new application files and shortcuts are
  successfully installed.
- Uninstalling the new app removes only files and shortcuts owned by the new
  installer. It must not remove user projects, captures, recordings, settings or
  the retained legacy program folder.
- Installing the same version again repairs missing files and shortcuts.
- Installing a later version upgrades the same Windows application entry rather
  than creating another product.

## Verification

Automated checks must first fail against the existing trial identity, then pass
after implementation. They verify:

1. npm, Cargo, Tauri configuration, visible UI text and packaged engine all use
   `0.15.0-alpha.1`.
2. The product name and identifier no longer contain `Trial` or `tauri-trial`.
3. Installer configuration uses NSIS current-user mode and the approved migration
   hook.
4. The hook only removes legacy shortcuts after verifying their exact targets
   and only removes an orphaned trial registry entry.
5. Package names and Windows metadata identify the same version.

Native verification uses a controlled test install:

- install over the current legacy setup;
- confirm Windows shows one current SnapZy Rust entry at `0.15.0-alpha.1`;
- confirm Desktop and Start Menu open the Tauri launcher and editor;
- confirm no normal shortcut opens `0.14.0-alpha.1`;
- confirm the separate `.NET` `SnapZy.lnk` is unchanged;
- confirm settings and a test `.snapzy` project remain available;
- repair-install the same package and verify shortcuts again;
- uninstall the new app and confirm user data and the retained legacy folder are
  untouched.

The known unrelated Media Foundation duration-edge test remains outside this
installer change and must still be reported if the full Rust suite reproduces it.

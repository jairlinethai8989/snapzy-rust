# Tauri UI Trial 0.1.1 - 2026-10-22

## Candidate

- Installer: `dist/SnapZy-Tauri-Setup-0.1.1-context-menu-icons-20261022-test.exe`
- Installer SHA-256: `BCD36F15F2207AF75181F378882DAF0DDA5E4A745439BDC891423C037CDE506D`
- Portable ZIP: `dist/SnapZy-Tauri-UI-Trial-0.1.1-context-menu-icons-20261022-win-x64.zip`
- Portable SHA-256: `607C9C5786DF599ECDCC5F9A973CAB1171893B299182F047D9A9F803D2015F5C`
- Signing: unsigned local test candidate.

## Changes Under Test

- Tauri UI trial version is consistently `0.1.1` in npm, Cargo, Tauri, Windows
  executable metadata, installer metadata, launcher, About text and editor title.
- Duplicate Object uses the Lucide `Copy` icon.
- Duplicate Area uses the Lucide `SquareDashedMousePointer` icon.
- Right-clicking an editable object selects it and opens Duplicate Object, Layer
  Order and Delete commands.
- Right-clicking blank canvas opens Duplicate Area.
- Context menus support mouse, Context Menu key, `Shift+F10`, arrows, Home/End,
  Enter/Space and Escape with EN/TH text.

## Verification Evidence

- JavaScript unit/contract tests: 18 passed, 0 failed.
- Tauri Rust tests: 2 passed, 0 failed.
- Launcher UI smoke: passed at EN/TH, supported sizes/DPI, menus and tooltips.
- Editor UI smoke: passed for Text/Callout, Duplicate Area/Object, PNG, EN/TH,
  tooltips, DPI and High Contrast.
- Packaged engine smoke: passed open, draw, text/callout, both duplicate modes,
  PNG render and clean shutdown.
- Actual Tauri WebView test: blank and object context menus passed; duplicating the
  selected line changed the rendered PNG hash without an editor error.
- Upgrade test: installing over the existing `0.1.0` left one Windows app entry,
  preserved the install location and changed registry/executable versions to
  `0.1.1`. The installed launcher, editor title and two new icons were verified.
- Root `cargo fmt --check`: passed. `git diff --check`: passed.

## Known Unrelated Test Result

The full root Rust suite completed with 354 passed and 1 ignored, but the existing
`video_playback::tests::native_preview_loads_paused_seeks_replays_and_releases_real_mp4`
test failed because Media Foundation reported `1.8986666` seconds while the test
accepts `1.9..=2.2`. An isolated rerun reproduced the same result. This candidate
does not change video code. Tauri `cargo fmt --check` also reports pre-existing
formatting differences in `experiments/tauri/src-tauri/src/main.rs`; that file was
not reformatted to avoid unrelated churn.

## Hands-On Checks

1. Install over an older Tauri trial and confirm Windows Apps shows `0.1.1` once.
2. Open an image and confirm the dashed-box pointer icon starts Duplicate Area.
3. Right-click blank canvas and choose Duplicate Area, then drag a region.
4. Right-click an existing object and choose Duplicate Object; move the copy and
   verify the original remains unchanged.
5. Switch EN/TH, inspect Help/tooltips and retry both right-click menus.

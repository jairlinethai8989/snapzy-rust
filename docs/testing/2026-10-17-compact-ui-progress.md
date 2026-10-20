# 2026-10-17 Compact Native UI Progress

## Paused status

Work was paused on 2026-10-17 because the session quota was nearly exhausted.
The build and packaging work resumed on 2026-10-18.

## Completed today

- Added the compact native launcher/editor layout for `0.14.0-alpha.1`.
- Added the embedded Lucide-style native icon registry, states, DPI rendering,
  bounded cache and EN/TH accessible names.
- Added native launcher/editor tooltips with a 400 ms delay and no focus steal.
- Added short bilingual More and Layer menu labels.
- Added High Contrast fallback colors using Windows system colors for launcher,
  editor work buttons and editor toolbar icons.
- Added a regression test for High Contrast palette selection.
- Updated the English/Thai README and created the `0.14.0-alpha.1` tester notes.
- Rebuilt the separate portable `0.14.0-alpha.1` package after the final High
  Contrast change. The accepted `0.13.0-alpha.1` package remains unchanged.

## Verified before pause

- `cargo fmt --all -- --check`: passed.
- `cargo check --locked --target x86_64-pc-windows-msvc`: passed after the
  High Contrast changes.
- `cargo test --locked`: 320 passed, 0 failed, 1 intentionally ignored in the
  fresh 2026-10-18 verification run.
- The ignored test requires a Windows private window station and never falls
  back to the user's clipboard.

## Native verification completed on 2026-10-18

- Launched the exact packaged executable from the candidate folder.
- Launcher was visibly compact at the current Windows scale with no overlap;
  Area, Window, Scrolling and Video stayed directly visible.
- Window > Desktop opened as a separate menu without starting capture.
- Scrolling opened `Automatic` and `Manual`; Open destination opened `New tab`
  and `New window`.
- EN/TH switched the launcher and editor labels. The editor showed Thai help
  text and returned to English afterward.
- More showed Hotkey, About, Minimize to tray and Exit without clipping.
- Opened the real editor with the mixed Thai/English fixture. The compact
  toolbar exposed localized accessible names for every command, including
  OCR, More and Layer.
- Created two test shapes. The selected shape showed eight resize handles and
  Layer opened Bring to front, Bring forward, Send backward and Send to back.
- Help displayed the full English and Thai tool descriptions. A real localized
  tooltip was observed on the language toggle (`Switch to Thai`).
- Opened the OCR window from the editor, selected `Thai + English`, read the
  fixture successfully, and received `สวัสดีSnapZy2026` for review/editing.
- Closing the modified test tab displayed Save project, Discard and Cancel;
  only the test work was discarded. The launcher remained open in English.

Native limitations for this round:

- Windows DPI was not changed during the run; 100/125/150/200% geometry is
  covered by automated tests.
- Windows High Contrast was not toggled during the run; the system-color
  fallback is covered by automated tests.
- The native run did not perform a destructive or long-running capture/video
  session. Existing capture, OCR, scrolling/Esc, video and project workflows
  remain covered by the automated suite and prior accepted regression tests.
- A native startup timing comparison was attempted. The baseline executable
  reached a targetable window and `Ready` in about 1.68 seconds. The candidate
  could not be isolated for a fair second sample because the older executable
  kept its single-instance process alive in the tray and intercepted launches;
  no candidate timing claim is made from that run.

The candidate remains local and has not been committed, pushed or published.

## Package details from before the final change

The current package is `dist/SnapZy-Rust-0.14.0-alpha.1-win-x64.zip`, size
5,132,436 bytes, SHA-256
`BF41451D9451E0643E56D6980440D00D9AE736B362C8BBD240071EED170D3E2C`.
The EXE inside the archive has SHA-256
`7DCAFEC71D30B7F333D958AC37440A6D6476A27A9FA9D39C38D809FE5C0AB890`.

## Native test blocker on 2026-10-18

The user approved a 5-10 minute native-window test. Both the primary Windows
automation runtime and the fallback controller failed before opening or sending
input, with:
`failed to write kernel assets: The system cannot find the path specified`.
No mouse/keyboard input was sent to SnapZy. Retry the native test when the
automation runtime is available; the package and source remain unchanged.
The runtime was reset and retried at the start of the next session, including
the fallback controller, but returned the same error before app discovery.

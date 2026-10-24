# Tauri UI Trial 0.1.0 - 2026-10-20

## Delivered Scope

Tauri 2 launcher with HTML, CSS and vanilla JavaScript. Vite builds the assets;
Lucide provides icons. Capture selection, editor, OCR, projects, scrolling,
recording and hotkey dialogs remain in the existing Rust native engine.

The trial uses its own settings and a child-process command bridge. The engine
window is selected by its owned child PID. Requests are typed and size-limited.
Native Open dialogs use the Tauri launcher as their owner. A committed native
shutdown remains busy until the engine exits, avoiding a false cancellation
while native windows and services are being destroyed.

## Automated Checks

- Native baseline: `cargo test --locked` - 343 passed, 1 intentionally ignored
  private-window-station clipboard test. This full run preceded the final
  dialog-owner and shutdown-state adjustments.
- Post-adjustment bridge tests: 2 passed; native release build passed.
- Frontend state tests: 3 passed, covering invalid stored settings, accepted
  commands and one-shot delay success/failure behavior.
- Playwright UI smoke: EN/TH, control bounds, rendered icons/logo, tooltip,
  menu/Escape behavior, video dialog, browser-only notice, bridge request shape
  and delay consumption passed. Viewports: 424/390/360/1280 px; simulated
  scale factors 1/1.25/2. High Contrast preview also rendered.
- Production frontend and Tauri release builds passed.
- Packaging license collection passed after increasing its metadata buffer;
  Cargo metadata contains about 2.2 MB and exceeded Node's default buffer.

## Real Windows Checks

- Portable Tauri executable opened and rendered embedded assets with the Vite
  preview server stopped (`http://tauri.localhost`).
- English/Thai toggle changed visible labels in the running Tauri window.
- Open selected the repository-owned `tests/fixtures/ocr/english.png`, using
  the native chooser, and displayed a 1200 x 180 px image in the Rust editor.
- Area invoked the native Area selection overlay and hid the launcher/editor.
- Native close confirmation appeared when closing Tauri with that unsaved
  test image. Clicking Cancel dismissed the confirmation and retained both
  windows and image. Closing again and choosing Discard terminated both the
  editor engine and Tauri process after the shutdown-state adjustment.
- Only test images/windows created for this session were discarded.

## Remaining Manual Validation

The computer-use inventory did not expose the native Area overlay as a usable
window. Therefore Escape cancellation from that overlay was not automated in
this trial. That test engine was cleaned up without retaining a capture; the
test fixture had no user edits. Video, scrolling, native hotkey dialogs and
all editor workflows have not been fully revalidated through this launcher.
Simulated browser DPI/contrast checks do not establish native multi-monitor or
system High Contrast sign-off. The capture engine's existing Escape behavior
was not changed by this trial.

## Local Artifacts

- `dist/SnapZy-Tauri-UI-Trial-0.1.0-win-x64/SnapZy-Tauri.exe`
- `dist/SnapZy-Tauri-UI-Trial-0.1.0-win-x64/engine/`
- Browser preview: `http://127.0.0.1:51819/`
- Launcher screenshots: `experiments/tauri/test-results/`

Keep the engine directory beside the trial executable. The package includes
the native engine's notices/licenses plus a separate Tauri dependency license
manifest and Lucide license. This is a local portable trial, not an installer.

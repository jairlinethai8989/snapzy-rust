# Tauri Editor Review Fixes — Proposed Design and Implementation Plan

Date: 2026-10-26 (Asia/Bangkok)
Status: Implemented for Alpha 6 candidate; publication remains pending user acceptance.
Baseline: main at 43b3e8b, SnapZy 0.15.0-alpha.5.

## Purpose and confirmed evidence

Repair the four findings from the code review before distributing the next installer.

1. Closing a Tauri tab, editor window, or launcher destroys editor sessions without an unsaved-work decision. The Tauri editor's project commands are currently placeholders.
2. Cropping at a nonzero origin breaks the coordinate contract. In the reproduction, a 1200 × 180 image was cropped from (360, 54), then a line was drawn at visible coordinates (10, 10)–(40, 10). The revision advanced from 1 to 2, but exported PNG bytes did not change.
3. Global editor keyboard handling consumes Space and dispatches Delete/Undo/Redo while a text control is focused. Browser reproduction: typing `hello world` produced `helloworld`.
4. The installer filename test still expects `SnapZy-Rust-Setup-${version}-auto-fit-20261024.exe`, while packaging now produces `SnapZy-Setup-${version}-win-x64.exe`.

Observed baseline checks: core Rust 378 passed / 1 ignored; Tauri Rust 12 passed; JavaScript 32 passed / 1 failed; frontend production build succeeded. These are review results, not validation of future changes.

## Approaches

### Recommended: repair the contracts and reuse existing project storage

Fix keyboard routing and crop coordinates at their boundaries. Connect the existing Rust `.snapzy` reader/writer to the Tauri editor, then implement one revision-checked close flow for tabs, editor windows, and application exit. This adds the smallest useful persistence connection needed for a working Save / Discard / Cancel choice. It requires more integration work than a warning alone, but users can retain editable work.

### Smaller alternative: warn before discarding

Add Discard / Cancel and keep manual PNG export as the available save route. This is quicker, but PNG cannot preserve editable objects and the project commands remain disconnected. It is an interim guard, not the recommended completion of the data-loss fix.

## Phase 1 — Regression cases and test contract

- Add behavior tests that fail on the confirmed keyboard, crop, and close defects before changing those behaviors.
- Update the obsolete installer assertion to the approved filename. Validate the version and platform filename contract without embedding an old build date.
- Keep test inputs synthetic and close only test-owned sessions.
- Keep unit tests separate from checks requiring a real Windows window or file dialog.

Acceptance: the installer contract test passes; new regression cases demonstrate the existing defects before their fixes.

Primary files: `experiments/tauri/tests/version-ui-contract.test.mjs`, existing editor tests, `src/editor_service.rs` tests, and Tauri session/workspace tests.

## Phase 2 — Keyboard ownership

- Centralize the decision whether an event belongs to a text/number field, select, contenteditable area, modal dialog, menu, or canvas.
- Allow normal Space, Delete, Enter, text selection, and native text Undo/Redo in editable controls. These events must not dispatch editor mutations or change canvas pan state.
- Leave Escape available for dismissing the topmost dialog/menu. A modal dialog blocks background canvas shortcuts.
- Ignore canvas shortcuts during IME composition, including Thai input. Apply the same focus rule consistently to all shortcuts, not only Ctrl+D.
- Preserve canvas Space-drag, Delete, Undo/Redo, Duplicate, zoom, and crop confirmation when the canvas owns the event. Clear temporary Space/pointer state on focus changes as appropriate.

Acceptance:

- English with spaces, Thai, and multiline text work in both Text and Speech Bubble dialogs.
- Delete and Ctrl+Z inside text entry change text only, leaving document revision and objects unchanged.
- Canvas shortcuts retain their existing behavior outside input fields and dialogs.

Primary files: `experiments/tauri/ui/editor.js`, a small keyboard-routing helper if needed, and focused frontend behavior tests.

## Phase 3 — One coordinate contract after crop

- Keep frontend pointer coordinates relative to the visible cropped image.
- At the Rust editor-service boundary, translate Press / Move / Release and AddText points into document coordinates by adding the current crop origin exactly once.
- Return selection bounds and every handle in visible-image coordinates by subtracting the crop origin after ownership transforms are applied.
- Keep Duplicate Area's rectangle in visible-image coordinates for extracting flattened pixels; retain its existing explicit translation when placing the new image object. Do not apply the generic translation a second time.
- Validate finite values and existing geometry limits after translation. Use the current crop after every Undo/Redo.
- Do not rewrite original image pixels or rebase every stored object's geometry to implement cropping.

Acceptance:

- Reproduce the review's exact crop case; the line appears at the requested visible position and exported pixels change at that position.
- Test nonzero X only, nonzero Y only, both axes, and repeated crops.
- After crop, exercise selection, move, resize, line bend, Text, Speech Bubble/tail, Blur, Step, Duplicate Area, Duplicate Object, and another crop.
- Verify overlays agree with objects at Fit and manual zoom, before and after Undo/Redo.
- Compare exported pixels against expected document coordinates; a revision increment alone is not proof of correct rendering.

Primary files: `src/editor_service.rs`, its tests, and frontend coordinate/overlay tests. The crop origin remains owned by Rust.

## Phase 4 — Editable persistence and safe closing

### Persistence needed by Save

- Reuse `project::load` / `project::save` and `Document::mark_project_saved` / `is_project_saved`; retain the current `.snapzy` schema.
- Add bounded service operations for opening and saving a project, with file selection handled by the Tauri backend's Windows dialog rather than an HTML download link.
- Connect Open Project, Save Project, and Save As. Open a project into a new session; retain its path only after a successful open/save.
- Keep original pixels, objects, layers, crop, text/font/background, curves, opacity, and effects editable after reopening. Reuse existing project serialization and atomic replacement rather than preview-file writing.
- New images are unsaved projects. PNG export and clipboard copy do not mark a project as saved.
- Identify save completions by session, operation, and captured revision. A save must never clear later edits or mark another tab as saved. Failure leaves the previous file and current work intact.

### One close flow

All close entry points must use the same backend-controlled lifecycle:

1. Prevent the default destructive close. Identify the requested tab/window/application scope.
2. Finish or cancel transient gestures consistently, drain pending mutations, and gather the relevant session IDs and revisions. Guard that close attempt against repeat clicks and new mutations/capture adoption.
3. Ask Save / Discard / Cancel for unsaved projects using a compact EN/TH dialog with the project name. Cancel is the safe initial focus; Escape means Cancel. A title-bar X or Alt+F4 follows the same path as the tab close button.
4. Save writes a real `.snapzy` file and waits for a confirmed result. Cancelling the file picker or encountering a save failure leaves the close uncommitted.
5. For multiple tabs, gather all decisions and complete the requested saves before removing any tab. If any decision is Cancel, retain the entire original tab set. Files already saved remain saved; unsaved work is not discarded.
6. For application exit, keep Tauri sessions alive while existing native-engine/video close handling completes. If the native engine declines or fails shutdown, retain the Tauri workspace. Do not destroy Tauri sessions before asking the native engine to close.
7. Revalidate the guarded session set and revisions, then remove only the approved sessions and shut down their processes. Reject stale close/save completions.
8. On cancellation/failure, release the close guard and restore usable controls/focus. Captures finishing during a close attempt must be deferred and preserved; cancellation replays them, while exit must resolve them as part of its close scope before committing.

The close coordinator must not hold the global editor mutex while waiting for user input, file I/O, or native close prompts. Backend lifecycle operations must remain scoped to the requesting window/session. Never accept a frontend `force close` flag as permission to bypass dirty-state checks.

Acceptance:

- Test tab X, editor title-bar X, Alt+F4, and launcher/application exit.
- Test Save, Discard, Cancel, Escape, cancelled file dialog, write failure, repeated close, and a late save completion.
- With several tabs/windows, cancelling one decision keeps every original tab open, including the active tab and its Undo/Redo history.
- Cancelled native-engine shutdown preserves the Tauri tabs.
- Reopen saved `.snapzy` files and verify editable object state and composed pixels, not merely file existence.
- PNG/Copy still leave the project unsaved; Undo back to the saved revision clears dirty state correctly, and Redo restores it.
- A queued pointer command, pending preview, or completed capture cannot mutate or silently discard a different session during closing.

Primary files: `src/editor_service.rs`, `experiments/tauri/src-tauri/src/main.rs`, `editor_registry.rs`, the Tauri editor frontend/translations, and small dedicated persistence/close modules as needed. Existing core project APIs provide storage behavior.

## Phase 5 — Integration and candidate installer

1. Run focused regression tests for every changed boundary.
2. Run the core Rust, Tauri Rust, and frontend unit suites; report ignored tests explicitly.
3. Build the production frontend and test the packaged Tauri editor, because the browser preview does not cover Windows close/file dialogs.
4. Verify the confirmed defects on synthetic documents in the real app. Check EN/TH, close dialogs, nonzero-origin crop, text entry, Copy, PNG, tabs, and project reopening.
5. Run targeted capture/scroll/Esc and video smoke checks if bridge shutdown/routing changes affect those paths.
6. Prepare `0.15.0-alpha.6` consistently across manifests, lockfiles, visible version, installer, notices, and guide filenames where relevant. Update the guide's project limitations once those commands work.
7. Build a new Alpha 6 installer for user acceptance. Publish its code/download links only after acceptance and publication authorization; do not silently replace the already published Alpha 5 binary under the same version.

## Completion criteria

- Every reported defect has a behavior-based regression case that passes after the repair.
- No close entry point removes unsaved work without a valid decision for that revision.
- Nonzero-origin crop has consistent input, selection-overlay, and export coordinates.
- Editing text never dispatches canvas mutation shortcuts.
- The installer contract agrees with the actual packaged artifact.
- Actual checks, remaining limitations, and the candidate installer's version are recorded when implementation is complete.

## Proposed execution order

Regression cases and installer test → keyboard isolation → crop coordinates → project save/open and close coordinator → integration checks → Alpha 6 installer for acceptance.

The persistence/close phase is the largest piece. Its added scope is deliberate: a Save button must retain editable work before closing can be considered safe.

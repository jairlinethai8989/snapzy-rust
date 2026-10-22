# Duplicate Icons, Context Menu And Tauri Version Update

Date: 2026-10-22 (Asia/Bangkok).
Status: design approved by the user; written specification awaiting review.
Scope: SnapZy Rust Tauri Vanilla JS editor and its local test installer only.
Do not change the .NET editions, push, publish, or replace the current GitHub
Alpha release in this phase.

## Goal And Selected Design

This change makes duplication discoverable and fixes the installed editor being
reported as the previous version. The user approved visual approach A:

- use the familiar two-overlapping-squares `Copy` icon for Duplicate Object;
- use `SquareDashedMousePointer` for Duplicate Area so the icon communicates a
  drag selection rather than OCR or scanning;
- provide localized tooltips and Help entries for both commands;
- add compact right-click menus for object and blank-canvas workflows; and
- advance the Tauri trial from `0.1.0` to `0.1.1` consistently.

The approved visual mockup keeps the existing compact toolbar dimensions and
thin Lucide stroke treatment. It does not introduce text labels into the toolbar.

Alternatives considered:

- `CopyPlus` plus `ScanLine` emphasizes adding a copy, but ScanLine remains easy
  to confuse with OCR and the plus adds noise at small sizes.
- `ClipboardCopy` plus `MousePointerSquareDashed` is recognizable, but suggests
  writing to the Windows clipboard. The existing duplicate commands are internal
  document operations and must not alter clipboard contents.

## Version And Installer Identity

- Set the Tauri application version to `0.1.1` in every authoritative manifest
  and generated lock/installer metadata that exposes the product version.
- The editor title, About/version text, Windows installed-app entry, executable
  metadata, installer UI, installer filename, portable package filename and test
  documentation must all report `0.1.1`.
- Keep the existing Tauri product name, application identifier and installer
  upgrade identity. Installing `0.1.1` must upgrade the installed `0.1.0` trial
  instead of creating a second app entry.
- Do not reuse the previous installer filename. The local candidate name must
  include `0.1.1` and a clear test suffix so Windows and the tester can distinguish
  it from cached `0.1.0` packages.
- No data migration is required. Existing `.snapzy` files and current settings
  remain compatible.

## Toolbar And Help Icons

- Replace the current Duplicate Object `CopyPlus` icon with Lucide `Copy`.
- Replace the current Duplicate Area `Scan` icon with Lucide
  `SquareDashedMousePointer` in the toolbar, Help list and every menu that invokes
  the area tool.
- Use the editor's existing Lucide renderer at the current thin stroke width.
  Do not add custom SVG artwork or a second icon library.
- Keep stable button dimensions, focus rings, disabled state, high-contrast state
  and DPI behavior. Icon changes must not shift neighboring tools.
- English tooltips: `Duplicate object` and `Duplicate area`.
- Thai tooltips: `สำเนาวัตถุ` and `สำเนาพื้นที่`.
- Duplicate Area Help must explicitly say to drag a rectangle; Duplicate Object
  Help must mention the `Ctrl+D` shortcut and that the source remains unchanged.

## Right-Click Behavior

### Right-click on an editable object

1. Prevent the browser/WebView default context menu.
2. Resolve the pointer using the existing zoom/DPI coordinate conversion and the
   editor's authoritative hit-testing path.
3. Select the object under the pointer before opening the menu. Pointer press and
   release must pass through the existing serialized command queue so no gesture
   remains unfinished while the menu is visible.
4. After selection is confirmed, show a compact menu at the pointer containing:
   `Duplicate object`, the existing layer-order submenu/commands, and `Delete`.
5. Duplicate invokes the same `duplicate_selection` command as the toolbar button
   and `Ctrl+D`. It must not implement a separate cloning path.

Right-clicking the already selected object opens the menu without adding a history
entry or moving the object. If the object cannot be selected or the editor is busy,
do not show an enabled duplicate command; preserve the document and show the
existing concise localized error only when an actual command fails.

### Right-click on blank canvas

- Prevent the default context menu and show one command: `Duplicate area`.
- Selecting it activates the existing Duplicate Area tool. It does not capture a
  pixel until the user drags a valid rectangle.
- The active toolbar button, cursor, tooltip and Help state update normally.
- Right-click outside the document canvas dismisses any open editor menu and does
  not activate Duplicate Area.

### Menu interaction

- Position the menu inside the visible editor viewport, flipping left/up near an
  edge so no item is clipped.
- Dismiss on command, Escape, click outside, tab/document change, window blur or a
  new pointer gesture. Dismissal does not alter history or selection.
- Support keyboard invocation with the Context Menu key or `Shift+F10` when the
  canvas/selected object has focus. Arrow keys move between items; Enter/Space
  activates; Escape closes and returns focus.
- Use the existing popup/menu visual language, not a native Windows menu, so
  English/Thai text, theme, DPI and High Contrast remain consistent.

## State And Command Flow

- Add one focused context-menu state object containing menu kind, viewport anchor,
  owning document identity and keyboard focus origin. Do not encode menu state in
  project files or undo history.
- Reuse the current editor command queue for right-click selection and duplicate
  execution. Menu opening waits for selection completion; command failures close
  stale menus and never leave an active pointer draft.
- Before invoking a menu action, confirm that the menu still belongs to the active
  document and that the selected object still matches the intended target.
- Toolbar button, keyboard shortcut and context-menu duplication share enablement,
  error handling, Undo/Redo and dirty-state behavior.
- Duplicate Area continues producing an editable image object through the existing
  implementation. Duplicate Object continues cloning supported editable objects.
  This change does not alter duplication semantics or the `.snapzy` schema.

## Error Handling And Safety

- Context-menu interactions are no-ops during unresolved load/save, text commit,
  effects application or another conflicting operation.
- A stale target, closed tab or failed hit test must not duplicate the previously
  selected object accidentally.
- Invalid Duplicate Area drags remain non-mutating and create no history entry.
- No context-menu operation reads or writes the Windows clipboard.
- All new UI text is localized in both EN and TH and remains legible at 100%, 125%,
  150% and 200% display scaling.

## Test Strategy

Follow test-driven development, beginning with failing tests.

### State and command tests

- Verify right-click object selection is serialized as press then release before
  `duplicate_selection` can run.
- Verify an already selected object opens the menu without a document mutation.
- Verify blank-canvas right-click offers only Duplicate Area and activates its
  existing drag workflow.
- Verify stale document/selection guards, busy-state behavior, menu dismissal,
  edge positioning and no dangling gesture after every close path.
- Verify toolbar, `Ctrl+D` and context-menu duplicate produce equivalent document,
  selection, history and dirty-state results.

### UI and accessibility tests

- Assert the `Copy` and `SquareDashedMousePointer` icons appear in toolbar, menus
  and Help with correct EN/TH names and no obsolete Scan/CopyPlus use.
- Exercise mouse, Context Menu key, `Shift+F10`, arrow keys, Enter/Space and Escape.
- Check compact layout, focus visibility, tooltip placement, viewport collision,
  High Contrast and 100/125/150/200% DPI rendering.
- Recheck Text, Callout, selection, dragging, layers, Delete, Undo/Redo, Duplicate
  Area and Duplicate Object so context-menu listeners do not consume normal input.

### Version and installer tests

- Assert all authoritative manifests resolve to `0.1.1` and reject a mixed-version
  build before packaging.
- Inspect executable and installer metadata, installed-app entry and editor title.
- Install over `0.1.0`; verify one app entry, preserved settings and successful
  launch of `0.1.1`. Also test a clean install and uninstall.
- Build and test a local unsigned candidate only. Do not push or publish without
  separate user approval after hands-on testing.

## Deferred

This scope does not add editable copy/paste through the Windows clipboard,
multi-selection, cross-tab object copying, drag-and-drop menu customization,
custom icon artwork, automatic publishing, code signing, or any .NET changes.

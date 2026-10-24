# Duplicate Context Menu, Icons And Version Implementation Plan

Approved design: `../specs/2026-10-22-duplicate-context-menu-icons-design.md`.

SnapZy Rust Tauri Vanilla JS editor only. Preserve the dirty worktree and all
accepted editor behavior. Do not modify .NET, push, publish or replace the current
GitHub Alpha release. Build a distinct local `0.1.1` test candidate only after
verification.

The `writing-plans` skill is not installed in this environment, so this reviewed
checklist is the implementation plan. Every production change begins with a
failing focused test.

## Implementation Checklist

- [x] **Baseline and boundaries:** record current JavaScript and Rust test results;
  inspect popup lifecycle, serialized pointer commands, authoritative hit testing,
  keyboard routing, icon rendering and installer identity. Do not stage unrelated
  existing worktree changes.

- [x] **RED - icon and localization contract:** add failing DOM/source tests that
  require Lucide `Copy` for Duplicate Object and `SquareDashedMousePointer` for
  Duplicate Area in toolbar, Help and menus. Require EN/TH tooltips and drag-area
  Help text, and reject obsolete `copy-plus`/`scan` mappings for these commands.

- [x] **GREEN - toolbar and Help icons:** update editor markup, dynamic Help icon
  mapping and localized descriptions without changing stable button dimensions,
  thin strokes, keyboard focus, High Contrast or compact overflow behavior.

- [x] **RED - context-menu state and sequencing:** add failing unit tests for
  object versus blank-canvas menu resolution, viewport clamping, dismissal,
  document ownership, stale target rejection and serialized right-click selection
  before duplication. Include no-dangling-gesture assertions.

- [x] **GREEN - context-menu behavior:** implement a focused context-menu state
  helper and reuse the existing popup style. Wire object right-click to select then
  show Duplicate/Layer/Delete; wire blank-canvas right-click to show Duplicate Area.
  Reuse existing duplicate, reorder, delete and tool commands.

- [x] **RED/GREEN - accessibility:** add keyboard tests for Context Menu key,
  `Shift+F10`, arrows, Enter/Space, Escape and focus restoration. Implement ARIA
  roles and keyboard navigation while preserving mouse click-outside and blur/tab
  dismissal behavior.

- [x] **RED/GREEN - version consistency:** add a test that reads authoritative
  manifests and visible UI/package strings and fails on mixed versions. Bump the
  Tauri trial to `0.1.1` in npm, Cargo, Tauri config, UI/About, packaging script,
  smoke-test paths and test documentation while preserving application/upgrade ID.

- [x] **Focused verification:** run JavaScript state tests, UI smoke tests and
  packaged engine smoke tests. Reproduce the earlier quick-select-then-duplicate
  race and verify toolbar, `Ctrl+D` and right-click routes produce the same result.

- [x] **Full regression verification:** run `npm test`, editor UI/engine smoke,
  `cargo fmt --check`, `cargo test --locked`, release build and `git diff --check`.
  Recheck Text, Callout, pointer drawing, layers, Delete, Undo/Redo, Duplicate Area,
  PNG and Copy at EN/TH, High Contrast and representative DPI values.

  Verification note: all feature-focused, packaged UI/engine and Tauri Rust tests
  passed. The full root Rust suite completed with 354 passed, 1 ignored and one
  unrelated existing Media Foundation playback timing failure: a 1.8986666-second
  duration narrowly missed its 1.9-second lower assertion. The isolated rerun
  reproduced that same pre-existing video test failure; no video code was changed.

- [x] **Installer upgrade and delivery:** create a uniquely named unsigned local
  `0.1.1` installer and portable package, calculate SHA-256, inspect metadata, and
  test clean install plus upgrade over `0.1.0` when available. Do not push or
  publish until the user completes hands-on testing and explicitly approves it.

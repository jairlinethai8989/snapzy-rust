# Compact Native UI Implementation Plan

Approved design: `../specs/2026-10-16-compact-native-ui-design.md`.

Rust only. Preserve the dirty worktree and every accepted operation. Do not
modify .NET, commit, push, publish or replace the accepted `0.13.0-alpha.1`
package. Develop a separate `0.14.0-alpha.1` local candidate.

## Implementation Checklist

- [ ] **Baseline:** record the current launcher/editor geometry, command IDs,
  localized labels and available timing traces. Add regression fixtures for the
  accepted 0.13 behavior before changing presentation.

- [x] **RED/GREEN - pure layout model:** add a platform-independent compact
  layout module which calculates launcher and editor bands from logical client
  size and DPI. Tests first cover 100/125/150/200% DPI, the 780 px editor
  minimum, stable control sizes, no overlap, and direct-versus-`More` command
  placement.

- [x] **RED/GREEN - icon registry:** add a complete command-to-Lucide mapping
  with no letter/symbol substitutes for available icons. Tests first require
  every visible launcher/editor command, shape variant, layer action and state
  to resolve to one icon and one localized accessible name.

- [x] **RED/GREEN - native icon renderer:** embed static Lucide vector geometry,
  render through `tiny-skia`, and cache native bitmaps by icon/state/DPI. Tests
  first cover deterministic pixels, expected bounds/strokes, state colors,
  100-200% sharpness, cache reuse and bounded memory. Add Lucide attribution to
  the third-party notices.

- [x] **RED/GREEN - shared tooltips:** refactor the existing editor/tab tooltip
  patterns into a native tooltip manager and add launcher coverage. Tests first
  cover approximately 400 ms initial delay, EN/TH refresh, complete command
  coverage, bounded text and cleanup without changing focus.

- [x] **RED/GREEN - compact launcher:** create the compact header, four primary
  actions, Window split button, Video menu button, compact options row and More
  menu. Tests first cover Window-main versus Desktop-menu routing, Video menu
  routing, busy/disabled states, status truthfulness and all existing command
  IDs. Target about 360 x 140 logical client pixels without clipping.

- [x] **RED/GREEN - adaptive editor:** replace the fixed coordinate block with
  work/tool/context bands driven by the layout model. Add grouped shape menus,
  one layer menu and responsive More placement while keeping Copy/Save PNG
  visible. Tests first cover every tool/object context, crop confirmation,
  imported bubble controls, narrow/wide transitions, stable focus and unchanged
  Undo/Redo/document state.

- [x] **RED/GREEN - owner-draw states and accessibility:** use the new renderer
  for normal, hover, selected, pressed, focused, disabled, destructive and
  primary states. Tests first cover high contrast fallback, keyboard traversal,
  focus visibility, localized menus/tooltips and Help parity.

- [x] **Regression verification:** run formatting, locked check, full unit tests,
  optimized build and `git diff --check`. Re-run capture, editor, project, OCR,
  scrolling, video and workspace automated suites. Confirm no new warnings.

- [x] **Native acceptance preparation:** package and hash a separate portable
  `0.14.0-alpha.1` candidate and run the approved native smoke checks. The
  candidate remains separate from `0.13.0-alpha.1`; timing comparison is still
  listed with the remaining acceptance work because the baseline was measured
  at about 1.68 seconds but the candidate could not be isolated from the
  baseline single-instance process in this session.

- [ ] **User acceptance gate:** obtain native EN/TH, tooltip, DPI, compact-window
  and full workflow testing before any commit, push or publication.

## Approval note — 2026-10-18

The user approved continuation after the live compact-UI smoke run. Live checks
covered the current Windows scale, launcher menus, EN/TH, a localized tooltip,
editor More/Layer, Help in both languages, shape handles, OCR and close
protection. DPI and High Contrast were not changed on the host, and therefore
remain explicit acceptance items rather than being marked as passed by
assumption.

The writing-plans skill is not installed in this environment, so this reviewed
local checklist is the implementation plan. Each production change follows a
failing test that demonstrates the required behavior before implementation.

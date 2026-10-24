# Tauri Object Transform And Zoom Implementation Plan

Approved design:
`../specs/2026-10-22-tauri-object-transform-zoom-design.md`.

SnapZy Rust/Tauri only. Preserve unrelated dirty work, do not modify .NET, and
do not push or publish. The `writing-plans` skill is not installed in this
environment, so this file is the implementation checklist. Every production
change begins with a focused failing test.

## 1. Baseline And Boundaries

- Record focused Rust editor-service/interaction tests and JavaScript editor
  state/UI tests.
- Re-read the existing native selection, shape resize, text resize, curve and
  viewport implementations before changing their contracts.
- Keep flattened preview generation independent from UI overlay generation.

## 2. RED/GREEN: Selection Overlay Contract

Files:

- `src/editor_service.rs`
- `src/editor.rs`
- `src/interaction.rs`

Steps:

1. Add failing tests for bounded overlay geometry for image, duplicate image,
   shapes, blur, line/arrow, pen/highlighter, text/callout and step objects.
2. Add serializable overlay/handle roles to `EditorView` without exporting
   internal object data or unbounded paths.
3. Prove that creation and duplication select the new object immediately and
   that preview PNG bytes are unchanged by selection.

Verification:

- `cargo test editor_service -- --nocapture`
- `cargo test interaction -- --nocapture`

## 3. RED/GREEN: Native Transform Semantics

Files:

- `src/editor.rs`
- `src/interaction.rs`
- `src/text_frame.rs`
- `src/shape_frame.rs`
- focused geometry helper modules as required

Steps:

1. Add image side-handle tests for independent width/height changes while
   retaining corner aspect-ratio behavior.
2. Cover eight-handle resize for outlined/filled shapes and blur regions.
3. Add line/arrow endpoint editing, immediate curve handle and double-click
   straight reset while preserving arrow tangent behavior.
4. Add bounded freehand path scaling that keeps stroke style unchanged.
5. Retain semantic text/callout reflow and tail editing; add the overlay roles
   needed by Tauri.
6. Add uniform numbered-step resize.
7. For every transform, test one-step Undo, no-motion history and Escape cancel.

Verification:

- focused test filters for each transform while iterating
- `cargo test interaction -- --nocapture`
- `cargo test editor::tests -- --nocapture`

## 4. RED/GREEN: Tauri Overlay Rendering And Input

Files:

- `experiments/tauri/ui/editor-state.js`
- `experiments/tauri/ui/editor.js`
- `experiments/tauri/ui/editor.css`
- `experiments/tauri/tests/editor-state.test.mjs`
- `experiments/tauri/tests/editor-ui-smoke.mjs`

Steps:

1. Add failing validation and coordinate tests for overlays, constant
   screen-size handles and zoom-aware hit tolerance.
2. Draw the active outline and semantic handles after the flattened preview.
3. Route handle, curve, tail and body gestures to the existing ordered backend
   command queue without frontend geometry mutation.
4. Keep Duplicate Area's new image selected and visibly active after release.
5. Verify selection graphics never enter Copy or PNG output.

Verification:

- `npm test -- --test-name-pattern="overlay|handle|duplicate"`
- `npm run test:editor-ui`

## 5. RED/GREEN: Independent Zoom And Pan

Files:

- `experiments/tauri/ui/editor-state.js`
- `experiments/tauri/ui/editor-workspace.js`
- `experiments/tauri/ui/editor.html`
- `experiments/tauri/ui/editor.js`
- `experiments/tauri/ui/editor.css`
- associated JavaScript tests

Steps:

1. Add failing tests for stable zoom steps, manual presets, Fit below 25%,
   center zoom, pointer zoom and per-session viewport restoration.
2. Add compact Minus / Percentage / Plus / Fit controls with EN/TH tooltips and
   keyboard support outside editable controls.
3. Store zoom/pan per session and restore it when tabs switch.
4. Verify Space/middle-button pan and all document coordinate conversion at
   non-100% zoom.

Verification:

- `npm test`
- `npm run test:editor-ui`
- `npm run build`

## 6. Integration And Regression

- Run formatter and all root Rust tests.
- Run Tauri backend, JavaScript and packaged editor-engine tests.
- Recheck Duplicate Object/Area, Text/Callout, grouped shapes, EN/TH, tooltips,
  DPI, High Contrast, tabs, Copy and PNG.
- Run `git diff --check` and inspect the final diff for unrelated changes.

Commands:

- `cargo fmt --all`
- `cargo test`
- `cargo test --manifest-path experiments/tauri/src-tauri/Cargo.toml`
- `npm test`, `npm run build`, `npm run prepare:engine`, and
  `npm run test:editor-engine` from `experiments/tauri`

## 7. Packaged Candidate

- Build a distinct unsigned upgrade installer with `npm run desktop:build`.
- Install locally and verify the installed version/title.
- Hands-on test Duplicate Area active selection, every transform category,
  line/arrow bend/reset, zoom/pan at multiple scales, Undo/Escape and Copy/PNG.
- Calculate SHA-256 and provide the installer for user testing.
- Do not push or publish until the user explicitly approves the installed build.

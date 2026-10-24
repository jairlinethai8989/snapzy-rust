# Editor Palette, Line Style, and Font Implementation Plan

**Spec:** `docs/superpowers/specs/2026-10-23-editor-palette-line-font-design.md`
**Target:** SnapZy Rust `0.15.0-alpha.3`
**Delivery:** Local installer for approval; no push or release

## Phase 1: Font Model and Rasterization

### 1. Add failing font-domain tests

Files:

- `src/glyphs.rs`
- `src/editor.rs`
- `src/project.rs`
- `src/editor_service.rs`

Tests must first demonstrate:

- all eight approved font identifiers validate;
- Thai/English/mixed masks differ where expected and remain antialiased;
- missing bundled/system fonts fall back without losing the requested family;
- existing text can change family in one Undo/Redo step;
- a failed rerasterization leaves selection, revision, history, and saved state unchanged;
- old `.snapzy` metadata without a font field opens as Segoe UI; and
- a new project round-trip preserves the selected family.

### 2. Implement the font-family model

Add a bounded `FontFamily` value shared by text annotations, glyph generation, project serialization, and the editor service. Store the requested family on text and callout content. Keep Step numbering on its existing fixed UI font.

Update text creation, editing, resizing, rewrapping, import, duplication, combination, rendering, and project conversion paths so regenerating a mask always receives the requested family.

### 3. Load bundled fonts privately

Add Regular font files and OFL notices for:

- Kanit;
- Prompt;
- Noto Sans Thai; and
- Sarabun.

Package them beside the editor engine and register them privately for the process. Do not install fonts into Windows. Provide deterministic Segoe UI fallback and bounded localized errors.

## Phase 2: Editor Service Contracts

### 4. Add failing command/view tests

Extend service tests for:

- `set_font` on text and callouts;
- rejection on non-text objects and unknown values;
- current/default font exposure in `EditorView`;
- text-background enabled/color state; and
- Undo/Redo and project dirty-state behavior.

### 5. Implement service commands

Files:

- `src/editor_service.rs`
- `src/interaction.rs`
- `experiments/tauri/ui/editor-state.js`
- `experiments/tauri/tests/editor-state.test.mjs`

Add typed commands and normalized view state for font and text background. Preserve existing foreground-color behavior and ensure malformed frontend payloads cannot reach the document.

## Phase 3: Shared Visual Controls

### 6. Add palette and line-style UI tests

Files:

- `experiments/tauri/tests/editor-state.test.mjs`
- `experiments/tauri/tests/version-ui-contract.test.mjs`
- `experiments/tauri/tests/editor-ui-smoke.mjs`

Cover:

- one reusable palette implementation rather than separate native color inputs;
- 20 stable presets and six unique recent colors;
- active state, transparent state, custom-color cancellation, and viewport bounds;
- keyboard navigation and EN/TH accessible names;
- visual solid/dashed/dotted samples; and
- transform guides remaining fixed dashed lines.

### 7. Build the shared palette popover

Files:

- `experiments/tauri/ui/editor.html`
- `experiments/tauri/ui/editor.css`
- `experiments/tauri/ui/editor.js`
- `experiments/tauri/ui/editor-i18n.js`

Replace visible native color inputs with swatch buttons. Route tool foreground, text foreground/background, shadow, and outline through one popup controller. Store only six recent unique colors in validated local preferences. Keep a hidden/native color input solely behind the additional-color command.

### 8. Replace the line-style select

Replace the text `<select>` with a compact sample button and popup. Render the pattern with CSS/SVG-independent borders or canvas-safe CSS, preserve tooltips/accessibility, and disable or hide the control for tools that do not support patterns.

## Phase 4: Font Selection UI

### 9. Add the font selector to text workflows

Extend the text dialog with a compact family selector that previews each name in its own font. Use the same control when creating text/callouts and when editing selected text.

Changing a selected object's font sends one service command and refreshes its mask, handles, Effects state, and preview without changing geometry or other style values. The selector truncates safely at minimum width and exposes the complete name in its popup and tooltip.

### 10. Add text-background palette controls

Add an explicit transparent toggle plus palette trigger to the text/callout dialog. Default remains transparent. Editing an existing object loads its current state and applies foreground, background, and font atomically where appropriate.

## Phase 5: Compatibility and Packaging

### 11. Extend project schema safely

Update project metadata readers/writers so the new font field is optional for older schemas and required/validated for new saves. Confirm imported `.neosnap` text maps to Segoe UI without changing existing geometry compatibility.

### 12. Package resources and licenses

Update:

- `experiments/tauri/src-tauri/tauri.conf.json`
- `experiments/tauri/scripts/prepare-engine.mjs`
- `experiments/tauri/scripts/package-trial.mjs`
- `THIRD-PARTY-NOTICES.md`
- `third-party-licenses/`

Verify both portable and NSIS layouts contain exactly the expected font files and OFL notices. Add a package contract test so missing assets fail the build.

### 13. Bump all product surfaces to Alpha 3

Synchronize root/Tauri Cargo manifests and locks, npm manifests and lock, Tauri config, launcher/editor visible versions, README, smoke paths, and installer naming. The installed executable and Windows uninstall entry must both report `0.15.0-alpha.3`.

## Phase 6: Verification and Delivery

Run in order:

1. focused Rust font/editor/project/service tests;
2. focused JS state and UI-contract tests;
3. full root `cargo test`;
4. Tauri `cargo test`;
5. `npm test` and production frontend build;
6. packaged editor-engine smoke;
7. browser UI smoke at 780, 1024, 1280, and 1440 widths with DPI and High Contrast;
8. native installed-window checks for EN/TH, palette targets, line samples, every font, Undo/Redo, save/open, Copy/PNG, titlebar placement, and Effects;
9. OCR, scrolling capture with Esc/Stop, and video regression checks; and
10. NSIS install-over-Alpha-2 verification, file version, registry version, size, and SHA-256.

Create one clearly named Alpha 3 installer. Do not push or publish until the installed build passes user testing.

# Tauri Canvas Resize Design

## Goal

Restore the existing editor capability for changing the editable workspace size in the Tauri editor. This is distinct from display zoom: it changes the exported canvas dimensions while preserving the existing image and editable objects.

## Scope

- Add a `พื้นที่ทำงาน` / `Workspace` command under the editor More menu.
- Show the current width and height and accept bounded pixel dimensions.
- Apply bounded dimensions that may be larger or smaller than the current canvas. New space is added to the right and bottom; reducing the size clips content outside the new boundary.
- Keep the operation as one Undo/Redo history step and preserve the current project dirty state and save workflow.
- Ask for confirmation before reducing either dimension, with a localized explanation that Undo can restore the previous workspace.
- Keep the browser mockup behavior aligned with the desktop workflow.

## Validation

- Invalid, zero, fractional, or out-of-range dimensions are rejected before dispatch.
- The command is routed through the active editor session only.
- UI tests cover menu visibility, dialog fields, applying a larger canvas, and the updated dimensions.
- Rust service tests cover expansion and Undo/Redo without changing source pixels or object content.

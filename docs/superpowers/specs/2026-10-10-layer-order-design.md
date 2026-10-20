# Annotation layer ordering

Status: approved by the user in this conversation, 2026-10-10.

## Context and Scope

SnapZy Rust is currently a local portable 0.6.0-alpha.1 candidate. Automatic
scrolling has synthetic native test evidence but still needs actual Lark/Excel
tester feedback. This proposal does not imply that feedback was received.

The next editor increment adds ordering of existing annotation objects only.
It does not change either .NET application. No commit, push, tag, installer or
public release is authorized by this document.

## Options

1. Add annotation ordering first, then multi-image composition separately
   (recommended). Reuse the current object stack, selection, history and renderer;
   keep compatibility and performance checks focused.
2. Add ordering and multiple image layers together. More immediately useful for
   collages, but requires decoding, image ownership, resize and memory-policy
   changes in the same increment.
3. Add a full layer panel with hiding, locking and grouping. Better navigation
   for complex documents, but larger UI and document-model scope than needed now.

## Behavior

- Four compact icon commands: Bring to Front, Bring Forward, Send Backward and
  Send to Back. Use the existing native icon system with English/Thai tooltips
  and Help entries. Preserve the compact editor layout on small windows; do not
  overlap or obscure existing commands.
- Commands apply to the single selected annotation: pen, line, arrow, rectangle,
  text, speech bubble, numbered step or blur. The captured source stays fixed
  beneath all annotations. Crop remains a viewport operation, not a layer.
- Bringing forward/backward swaps with the neighboring object in stack order,
  even if it does not overlap. Front/back moves to the respective stack end.
- Keep the selected object ID, content, position, size and color unchanged.
  Disable unavailable directions at stack boundaries and all commands when no
  object is selected. A no-op creates neither history nor a new revision.
- Each actual reorder is one Undo step, clears Redo, updates the dirty state and
  triggers normal composition. Undo/Redo restores the exact stack and retains
  selection when that object still exists.
- Selection hit testing follows the current top-to-bottom stack. A fully covered
  lower object can be reached by moving the covering object backward; a dedicated
  layer list or select-through interaction is outside this increment.
- Blur keeps its existing ordered-filter meaning: it affects only pixels below
  it. Moving blur can intentionally change what is blurred. Help must explain
  this and retain the warning that blur is not secure redaction.
- Copy and Save PNG must use the latest completed composition at original source
  resolution, with no selection handles in output.

## Implementation Boundaries

- Add a small ordering enum plus availability/reorder operations to Document in
  src/editor.rs. Vec order remains the sole rendering and hit-testing order.
- Reuse begin_edit snapshots and revision guards. Do not copy source pixels or
  glyph masks for reorder operations; existing Arc-backed data stays shared.
- Add commands, icons and translated tooltips in src/editor_controls.rs and
  wire them into src/ui.rs using existing update/composition paths.
- Keep render/filter behavior unchanged except focused tests proving ordering.
  Do not refactor capture, scrolling or the startup path for this feature.
- Cancel or finish active gestures through the editor's existing gesture policy
  before an ordering command; never reorder a half-committed drag.

## Verification and Delivery

Write and observe failing behavior tests before production changes:

- All four operations, both boundaries, missing/stale selection and a single
  object; no-op revision/history behavior and stable object identity.
- One-step Undo/Redo, dirty-state restoration, crop preservation and topmost
  hit testing after reordering.
- Overlapping objects and ordered blur produce expected actual rendered pixels;
  selected text/background/tail data is unchanged.
- Native editor checks for translated tooltips/Help, enabled states, command
  behavior, small-window layout and PNG/clipboard pixel parity.
- Run the full unit suite, formatting, qualified Clippy and existing native
  capture/editor/scrolling regression checks. Compare startup and preview timing
  with the retained 0.6.0-alpha.1 baseline; report measurements and limitations
  without claiming a speedup from a small sample.

After implementation and verification, propose a separate portable
0.7.0-alpha.1 local tester build. Keep 0.6.0-alpha.1 and its artifacts intact.
Actual Lark feedback, mixed/high DPI and target-PC clipboard behavior remain
explicit tester checks. Public delivery still requires separate approval.

## Deferred

Multi-image import/layout, layer list/hide/lock/group, editable project files,
curves/dashes/highlight/shadows/text outlines, OCR, video and installer/startup
integration are separate future increments.

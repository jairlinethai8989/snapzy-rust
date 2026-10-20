# Text Background And Speech Bubble

Status: conceptual and written design approved by user on 2026-10-09.
Scope: Rust SnapZy only. Independent of the approved scrolling pilot.

## Alternatives And Choice

Use the existing annotation toolbar with context-sensitive background controls
for text and speech bubbles. This is preferred to a separate settings dialog
(more clicks) or a rich-text editor (unneeded formatting complexity).
Existing native Thai/English text entry and source-pixel rendering are retained.

## Behavior

- Plain text remains borderless with transparent background by default. An
  optional background checkbox and color swatch apply to the selected text or
  the next new text. Turning it off preserves the chosen color for reuse.
- Add a distinct speech-bubble tool with icon and EN/TH tooltip/Help. It creates
  editable text surrounded by an outlined rounded callout and a triangular tail.
  Its background also defaults to transparent, not white. Border color follows
  annotation color (initial red), thickness follows the existing stroke setting
  (initial 5 source pixels); text uses annotation color and initial font size 24.
- Background color is separate from text/border color, initially white but not
  painted until explicitly enabled. This phase offers solid fill or no fill,
  not per-object opacity, gradients, shadows or text outlines.
- Reuse side handles for wrap width and corner handles for font scaling. Keep
  the opposite edge/corner fixed, one Undo per release, Esc/capture-loss cancel.
  Bubble padding is 8 source pixels outside the existing text-mask rectangle.
- A separate tail-tip handle can be dragged to point at a location. The base
  attaches to the closest frame edge automatically. Moving the bubble moves
  its tail; resizing keeps its tip's position relative to the bubble origin.
  Tail dragging changes only the callout, not its text or font.
- Double-click inside text reopens the existing multiline text dialog; dragging
  a frame or tail handle has priority over opening text entry.
- Filled objects are selectable inside their background; transparent plain text
  retains existing text selection behavior. A bubble can be selected on its
  border, tail or text. Crop hides pixels without deleting editable content.

## Model, Rendering And Safety

Keep text content, wrap width, mask, optional fill and optional callout geometry
in document history. Glyph masks remain shared and charged to the current
budget; fill/geometry edits do not rerasterize text. Rendering paints fill and
outline before glyphs; the tail and frame form one perimeter without an internal
base line. Use the current antialiased rendering/composition worker for preview,
PNG and clipboard. No resize handles are exported.

All changes follow generation checks and existing coordinate/memory limits.
No loss of editable text on an invalid resize, no blocking heavy work on the UI
thread and no new runtime dependency. Existing text objects remain transparent.

## Acceptance

Tests first: transparent default leaves pixels outside glyphs untouched; fill
only changes the frame region; undo restores transparent pixels; changing text,
font, wrapping and fill preserves independent settings. Bubble tests cover all
tail directions, moving/resizing/cancel, selection and crop. Native tests cover
creation, fill/color toggle, Thai/English editing, drag handles, Undo/Redo and
byte-identical annotated PNG/clipboard. Inspect compact UI and mixed/high DPI.

No .NET changes, installer, push, tag or release until tester approval.

Implementation status: model, geometry, shared rendering, interactions and
native EN/TH controls are implemented for the separate 0.5.0-alpha.1 portable
candidate. See its verification report and implementation plan for the test
gates and outstanding physical tester acceptance. This is not a public release.

# SnapZy Rust Basic Editor

Date: 2026-10-09 (Asia/Bangkok)
Target milestone: 0.2.0-alpha.1
Status: written specification approved by the user on 2026-10-09

## Purpose and Scope

Extend the accepted native capture preview into a small usable image editor.
Keep startup, capture selection and first image display responsive. This work
belongs only to snapzy-rust; the SnapZy and Neo Snap .NET repositories remain
unchanged. Deliver a portable Windows x64 test ZIP, not an installer or public
release. Obtain tester approval before pushing implementation changes.

Included tools:

- Select, move and delete annotation objects.
- Crop the visible image canvas.
- Freehand pen, straight line, arrow and unfilled rectangle.
- Color and stroke-width controls, initially red (#FF0000) and 5 source pixels.
- Undo and redo for drawing, moving, deleting and cropping.
- Copy and PNG export of the current image with annotations.
- English by default, Thai toggle, localized tooltips and action feedback.

Deferred: text, step numbers, blur, callouts, ellipse, filled shapes, curved
lines/arrows, dashed/dotted strokes, opacity, shadows, text outlines, layer
reordering, image import/combination, project persistence, OCR, scrolling and
video. The approved wider roadmap still covers these future milestones.

## Approach

Retain the Win32 launcher, capture pipeline and preview window. Add a separate
editor document module and a native drawing/rendering module instead of placing
all document behavior in src/ui.rs. Do not introduce a WebView or a second UI
framework. Existing capture and clipboard modules retain their responsibilities.

Alternatives considered: adopting egui offers richer editor widgets but requires
a new rendering/startup baseline; implementing the complete editor at once delays
tester feedback and obscures performance regressions. Incremental native tools
are the selected approach for this milestone.

## Document Model

Each editor window owns its document and history independently. Share its
immutable original Frame through Arc; never modify captured source bytes.

The document contains stable annotation IDs, ordered annotation objects, an
integer crop rectangle within the original image and a document revision.
Coordinates and stroke widths use original-image pixels, independent of display
zoom and Windows DPI. Shapes remain editable objects until export. History stores
document metadata and annotation changes, not full image-frame copies. Keep at
most 64 completed undo steps; new edits after undo discard redo history. Selection
and tool switching are UI state, not undo steps.

An untouched document can display and export the existing shared Frame directly.
Render annotated documents only when their content changes. Reuse the composited
image for resize/repaint; do not encode PNG or rebuild a full-resolution image on
every paint or pointer movement. Interactive gestures use an inexpensive draft
overlay; completing a gesture invalidates the cached composition.

## Interaction

- Default to Select after capture. Drawing controls are icon buttons with
  localized tooltips, with Undo/Redo/Delete and Copy/Save clearly accessible.
- Pen records one continuous stroke per press/drag/release. Line and arrow use
  drag endpoints; rectangle accepts drags in either direction. A click without a
  meaningful drag creates no object or history entry.
- Select hit-tests annotations from front to back with screen-space tolerance;
  selecting empty space clears selection. Dragging moves the selected object;
  Delete removes it. No object resizing or layer commands in this milestone.
- New objects use the selected color and stroke width. Changing those controls
  while an object is selected updates that object as an undoable edit. Stroke
  width accepts integer values from 1 through 40 source pixels.
- Crop starts a drag rectangle inside the current image. Show a draft rectangle,
  commit with Enter, cancel with Escape. Clamp the region to the current crop,
  require nonzero dimensions and snap its edges to source pixels. Cropping keeps
  original pixels and objects; objects outside the crop are clipped, not deleted.
  Undo restores the prior viewport and its previously hidden objects.
- Escape cancels unfinished drawing or movement without recording a change.
  Handle mouse capture loss and window closure the same way.
- Ctrl+Z/Ctrl+Y support Undo/Redo. Do not consume editing shortcuts while focus
  is in a numeric control. Disable commands when unavailable.
- Fit the canvas within the editor without changing exported image resolution.
  Derive drawing, crop and hit-testing transforms from the same display rectangle.
  Reject gestures outside it. Keep toolbar dimensions stable across EN/TH, DPI
  changes and supported window sizes; enforce a minimum usable editor size.

## Rendering and Export

Use one annotation renderer for cached preview composition and exported pixels.
Render at source resolution, then scale only for display. No lossy image encoding.
Preserve exact source pixels outside annotation coverage; an unannotated crop
must match the corresponding original pixels exactly. Export dimensions equal
the current crop dimensions. Respect the existing per-frame 256 MiB pixel limit
and account for cached composition buffers in the application memory budget.

Copy exports the current flattened document through the existing clipboard path.
PNG saving remains asynchronous and atomic. Capture the document revision at the
start of Copy/Save: success marks only that revision as kept. Editing while a save
is pending must not cause newer edits to be reported as saved. A failed/cancelled
export leaves the unsaved state intact. Disable repeated Save while that window
has an export in progress. Closing an unsaved editor requires confirmation.

## Error Handling and Isolation

Invalid crop or empty gestures do not mutate the document. Report rendering,
allocation, clipboard and file failures without discarding edits or reporting
success. Do not overwrite input images automatically. Release temporary native
drawing objects and mouse capture on every exit path. Keep per-window selection,
history, save state and cached images isolated.

## Validation

Write failing tests before implementation for document changes and their undo,
redo branching, no-op gestures, hit-testing order, coordinate transforms, crop
clamping, crop restoration, revision-aware exports and window independence.
Check pixel-level renders of each tool, untouched background pixels, clipped
annotations, exact crop dimensions and PNG round trips. Validate clipboard DIB
bytes against the composed image rather than the original image.

Extend the native smoke test to draw/select/move/delete, crop/undo/redo, change
color/width, export annotated PNG, change language and cancel gestures. Retain
all existing launcher/capture/hotkey tests. Run formatting, the full test suite,
Release build and native smoke test before tester delivery. Manually inspect the
editor at multiple DPI settings and a compact window size; actual clipboard
pasting and physical hotkeys remain tester checks.

Compare the accepted alpha.1 executable and this milestone on the same machine,
using the same capture region. Repeat launcher-ready, selection-ready and first
preview-paint measurements, record sample count/median/slowest results and memory
usage. Label window-detection timing separately from first-paint timing. Investigate
repeatable regressions before delivery; do not infer language-level performance
claims or promise a fixed latency from subjective tester feedback.

## Delivery

Update version identity consistently to 0.2.0-alpha.1, document implemented tools
and limitations in English and Thai, and package source/library notices with the
portable ZIP. Provide the tester with the exact local ZIP path and a short test
checklist. Keep implementation local until approved for Git push. No changes to
existing .NET installations, settings, hotkeys or release repositories.

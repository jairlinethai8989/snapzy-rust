# Tauri Object Transform And Zoom Design

## Scope

Improve only the SnapZy Rust Tauri Vanilla JS editor. Do not modify the .NET
application, publish a release, or push implementation changes. The work covers
visible active-object controls, resizing and reshaping existing editable
objects, direct line/arrow bending, and compact independent workspace zoom.

The native Rust editor remains the source of truth for editable geometry,
selection, history and rendering. The Tauri canvas remains a presentation and
input layer; it must not duplicate the editor's geometry rules.

## Selected Visual Direction

Use a blue selection outline with white square handles outlined in blue. Images,
duplicate-area results, rectangular regions and shapes use eight handles: four
corners and four side midpoints. Handles remain a constant screen size at every
zoom and do not appear in Copy or PNG output.

The compact zoom group contains Minus, current percentage, Plus and Fit. The
percentage button opens manual presets from 25% through 400%. Fit may use a
smaller percentage, down to the existing 5% safety limit, when a large or long
image requires it.

## Active Selection Contract

`EditorView` gains a bounded, serializable selection overlay produced by the
Rust editor service. The overlay identifies the selected object's transform
kind and contains only finite document-space geometry needed by the frontend:

- the visible selection bounds or path endpoints;
- zero to eight resize handles with explicit semantic roles;
- an optional line/arrow curve handle;
- an optional callout-tail handle;
- whether the selected body can move.

No file paths, source pixels, object internals or unbounded point arrays cross
this boundary. The overlay is absent when nothing is selected or when the
selected object is outside the visible crop.

The Tauri editor draws this overlay after the flattened preview. Hit testing
uses a constant screen-space radius converted through the current zoom. Handle
hits take priority over curve/tail hits, which take priority over moving the
selected body, which takes priority over starting a new drawing gesture.

Newly created objects remain selected while preserving the active drawing tool.
Duplicate Area and Duplicate Object select the new copy and switch to Select,
so the copy is immediately ready to move or resize.

## Object Transform Behavior

### Images And Duplicate Areas

- Dragging inside the selected image moves it.
- Corner handles resize while preserving the source aspect ratio and anchor the
  opposite corner.
- Side handles change only width or height, allowing an intentional stretched
  result.
- Source pixels remain unchanged; resizing continues to be non-destructive.

### Shapes And Regions

Outlined/filled rectangles and ellipses, plus blur regions, expose all eight
handles. Corners and sides resize freely while anchoring the opposite edge or
corner. Stroke width, color, opacity and pattern do not change during geometry
resize.

### Lines And Arrows

Lines and arrows expose start and end handles for length/direction changes and a
center curve handle immediately after creation or selection. Dragging the center
handle bends the quadratic curve, and arrowheads follow the final tangent.
Double-clicking the curve handle returns the object to a straight line. Dragging
the body moves the whole object.

### Pen And Highlighter

Freehand strokes expose an eight-handle bounding box. Resizing transforms the
stored path coordinates while preserving stroke width, color, opacity and line
pattern. A degenerate one-point stroke remains visible and can move but does not
offer invalid resize handles.

### Text And Callouts

Text and callouts keep semantic text resizing rather than stretching glyph
pixels. Corner handles scale font and frame proportionally; left and right side
handles change wrap width and reflow the text. A callout tail retains its own
handle and remains editable independently. The text content, background,
outline and shadow remain editable.

### Numbered Steps

Numbered steps resize uniformly and remain circular. Their number, style and
automatic sequence do not change during resizing.

## Gesture And History Rules

A transform is previewed while dragging and committed only on pointer release.
One completed drag creates exactly one Undo step. A no-motion gesture creates no
history. Escape, tool change, tab close or pointer cancellation restores the
pre-gesture state. Invalid, non-finite or excessive geometry is rejected without
partially changing the object or history.

Undo/Redo restores geometry, selection and the correct overlay. Copy and PNG use
the native flattened document and therefore never contain the active outline or
handles.

## Zoom And Pan

Each editor session stores its own zoom and pan state in the Tauri workspace.
Switching tabs restores that session's viewport. Opening a new capture starts at
Fit.

- Minus and Plus move through stable zoom steps.
- The percentage button offers 25%, 50%, 75%, 100%, 125%, 150%, 200%, 300% and
  400%.
- Fit centers the full visible document and may go below 25% to a minimum of 5%.
- Ctrl + mouse wheel zooms around the pointer.
- Keyboard Plus/Minus zoom around the viewport center when focus is not in an
  editable control.
- Space + drag and middle-button drag pan without editing objects.
- All pointer-to-document conversion uses the session's current zoom and pan.

## Error Handling And Limits

The service validates overlay geometry before serialization and caps the handle
count. The frontend ignores an invalid overlay and keeps the document usable.
Transforms continue to use existing document coordinate, memory and history
limits. Errors are localized through the existing notice surface and leave the
last committed preview intact.

## Test Plan

Follow red-green-refactor for every production change.

1. Rust service tests prove the overlay for every supported object kind,
   immediate selection after creation/duplication and omission from PNG output.
2. Rust interaction tests cover image corner/side semantics, shape and blur
   handles, line endpoints and curve reset, path scaling, text reflow, step
   scaling, one-step Undo and Escape cancellation.
3. JavaScript state tests cover overlay validation, zoom-aware hit tolerance,
   preset/center/pointer zoom and per-session viewport restoration.
4. Browser UI tests cover the eight-handle appearance, compact zoom controls,
   EN/TH labels, High Contrast, DPI and no toolbar overflow.
5. Existing Rust, Tauri backend, editor-engine and JavaScript suites must pass.
6. Build a distinct unsigned local installer for hands-on verification of
   Duplicate Area, object transforms, line bending, zoom, Copy and PNG. Do not
   push or publish until the user approves the installed build.

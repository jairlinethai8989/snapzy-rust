# Duplicate Area And Editable Object Duplication

Date: 2026-10-21 (Asia/Bangkok).
Status: feature design approved by the user; written specification awaiting review.
Scope: SnapZy Rust and its Tauri Vanilla JS editor only. Preserve existing work,
project compatibility and the accepted Tauri editor direction. Do not change the
.NET products, publish, push or replace the current Alpha release in this phase.

## Goal And Selected Approach

The editor will support two related but distinct workflows:

1. **Duplicate Area** copies the pixels currently visible inside a dragged region
   and inserts them into the same document as a new movable, resizable image layer.
   The source canvas and every source object remain unchanged.
2. **Duplicate Object** copies any selected editable object, or a selected image
   group with all of its descendants, while preserving editability and styling.

The user selected the direct, one-step area workflow. Finishing the drag creates
the new image immediately; it does not require a separate Paste command and does
not use the Windows clipboard.

Alternatives considered:

- An internal Copy/Paste clipboard would support repeated pastes but would add a
  second action and hidden clipboard state to the requested direct workflow.
- The Windows clipboard would allow cross-application paste but would overwrite
  user clipboard contents and cannot retain SnapZy's editable object structure.
- The selected direct approach is faster, predictable and isolated to the current
  document. Existing PNG and clipboard commands remain unchanged.

## User Interface

- Add a **Duplicate Area** tool with a clear overlapping-image/crop-frame icon.
  The icon is accompanied by localized EN/TH tooltip and Help text. It follows
  the existing stable icon-button dimensions and compact overflow behavior.
- Add a separate icon-only **Duplicate** command beside the existing object
  commands. It is disabled when no duplicable object is selected.
- `Ctrl+D` invokes Duplicate Object. The shortcut must not run while a text field,
  text dialog or other editable control owns keyboard focus.
- Duplicate Area uses a crosshair cursor. Pointer drag shows a non-destructive
  selection rectangle. Escape cancels an unfinished drag with no history entry.
- Releasing a valid selection creates the image layer, switches to Select, selects
  the new layer and exposes its existing move/resize handles immediately.
- Duplicate Object selects the new object after creation. The duplicate appears
  directly above the source in the same ownership level and is offset by 16
  document pixels right and down so both copies are visible.
- Placement prefers `+16,+16`. If that placement would leave the duplicate
  difficult to select, use the corresponding inward offset and clamp placement
  so a useful part remains visible. Do not resize the canvas automatically.
- Keep the controls icon-only in the toolbar. Accessible names, keyboard focus,
  high-contrast appearance, hover/keyboard tooltips and EN/TH text are required.

## Duplicate Area Semantics

- Capture the **committed visible composition** within the selection: background,
  inserted images, annotations, blur and effects in their rendered layer order.
  Do not include selection handles, guides, cursors, tool previews or dialog UI.
- The result is intentionally a flattened `Image` object. Objects represented by
  those pixels are not duplicated as editable descendants. Users use Duplicate
  Object when they want an independently editable copy of a particular object.
- Convert the pointer rectangle from screen/zoom/DPI coordinates into document
  coordinates once, normalize reversed drags and intersect it with the canvas.
  Reject an empty or sub-pixel selection without mutating the document.
- Crop only the selected pixels from the renderer result. Add the cropped frame
  as a normal image object at the selected location plus the placement offset.
  The original background, source images and annotations are never cropped,
  deleted, hidden or rewritten.
- The created image behaves like every existing inserted image: it can move,
  resize, crop, reorder, receive opacity/effects, save in `.snapzy`, export to
  PNG and copy through existing image-output commands.
- Duplicate Area is a transient editor interaction mode, not a stored annotation
  type. Do not add a project-schema enum solely for this tool.

## Duplicate Object Semantics

The command supports all editable object categories currently handled by the
Rust editor:

- inserted images and editable image groups;
- pen and highlighter strokes;
- straight or curved lines and arrows;
- outline and filled rectangles/ellipses;
- blur regions;
- step-number objects;
- text and speech callouts.

The immutable captured background, workspace crop boundary, transient selection,
active gesture and editor UI are not editable objects and cannot be duplicated.
The Duplicate Area tool remains the way to copy pixels from the background.

For a single object:

- Clone its editable content with a new unique object identity.
- Preserve geometry, source data, colors, widths, line pattern, opacity, shadow,
  text outline, text frame/background, callout tail, crop viewport and rendering
  compatibility mode. A step-number copy retains the same displayed number,
  because duplication is an exact copy rather than creation of a new sequence step.
- A duplicated blur remains an editable blur effect at the new location and blurs
  the composition underneath that location; it is not replaced by baked pixels.
- If the selected object is a child of an image group, duplicate only that child
  into the same parent and immediately above the original sibling.

For a selected image group:

- Duplicate the complete ownership subtree as one atomic operation.
- Assign new unique identities to the group and every descendant, and remap all
  parent references within the clone. No cloned child may reference the source
  group, and no source object may be reparented.
- Preserve descendant order, local transforms, crop viewports, relative geometry,
  effects and shared immutable assets. Place the cloned subtree immediately above
  the complete source subtree without interleaving source and clone descendants.
- Continue enforcing the existing group-depth, object-count, point-count and
  project limits. Duplicate nothing if the complete subtree cannot be accepted.

## Architecture And Data Flow

### Core document operation

Add a focused document-level duplication operation that accepts the selected ID,
validates the complete proposed clone, allocates all identities, remaps ownership,
applies one deterministic offset and commits one history snapshot. It returns the
new root/selected ID. This is the authoritative path for button and shortcut use.

Immutable frames, path point storage and glyph masks may be shared through their
existing `Arc`-backed representation. Geometry, object records and identities are
independent. Existing distinct-source budget accounting must continue counting
shared assets once rather than charging duplicated references repeatedly.

### Area capture service

Keep Duplicate Area orchestration in the editor service because it needs the
current committed renderer output rather than a new persistent tool kind:

1. The Vanilla JS editor tracks the drag rectangle in UI state and sends a
   `DuplicateArea` command only on valid release.
2. The service validates the rectangle and obtains a render snapshot of the
   current document.
3. Rendering/cropping work runs through the existing bounded worker path so a
   large canvas does not freeze pointer input.
4. The worker returns a bounded cropped frame tied to the originating document
   identity/revision. A stale result is discarded.
5. The UI thread inserts the returned frame through the normal image-object path,
   applies placement, selects it and records exactly one history step.

The service also exposes a `DuplicateSelection` command that delegates to the
core document operation. Button state comes from the existing selected-object
view model; the front end must not infer object ownership by parsing pixels.

### Rendering and persistence

No new persisted content type is required. Duplicate Object produces normal
editable object records; Duplicate Area produces an existing image object.
Current `.snapzy` serialization can therefore store both results without a
schema bump, provided the cloned ownership tree passes current validation.

Preview, PNG export and clipboard output use the same renderer. The pixels shown
after duplication must match exported output at the same document revision.

## History, Selection And State

- Each successful Duplicate Area or Duplicate Object action is exactly one Undo
  step. Undo removes the complete result; Redo restores identities, ownership,
  placement, layer order and selection consistently.
- Cancellation, invalid rectangles, disabled commands, no-op requests, stale
  workers and failed validations add no history entry and do not mark the project
  dirty.
- A successful duplicate marks the project dirty. Saving, reopening and editing
  the duplicate follows existing project behavior.
- Duplication is unavailable while a gesture, text entry, effect dialog, project
  load/save, capture or conflicting operation is unfinished. It must not silently
  commit or cancel another interaction.
- Duplicating a selected object does not change the active drawing defaults.
  After the command, Select is active so the new object can be moved immediately.

## Limits, Errors And Privacy

- Validate expected object totals, ownership depth, points, finite geometry,
  canvas bounds and image memory before committing. Use checked arithmetic.
- Duplicate Area respects current maximum frame/canvas and scratch-buffer limits.
  Do not raise limits to make oversized captures succeed. Release temporary
  rendering buffers after insertion or failure.
- All operations are atomic: if allocation, rendering, validation or insertion
  fails, leave source objects, selection, layer order, history and dirty state as
  they were. Show a concise localized EN/TH message with a practical reason.
- Do not partially duplicate a group or silently flatten unsupported descendants.
- Duplicate Area copies visible pixels. It can therefore copy sensitive pixels,
  including pixels that are visually blurred in the captured result. A `.snapzy`
  project may still retain original information in underlying source objects.
  Keep the existing warning that blur and covering objects are not secure deletion.
- Do not read from or write to the Windows clipboard for either command.

## Test Strategy

Follow test-driven development, beginning with failing core and service tests.

### Core tests

- Duplicate every supported object category and verify new identity, exact style,
  independent editing, `+16,+16` placement and immediate-above layer order.
- Verify text/callout editability, callout tail/background, curved arrow control,
  line patterns, image crop, opacity/effects, step number and blur semantics.
- Duplicate child versus group root; verify complete ID/parent remapping, sibling
  order, nested groups, asset sharing and no source mutation.
- Exercise maximum depth/count/point/memory limits, invalid ownership and ID
  exhaustion; every failure must be atomic.
- Verify one-step Undo/Redo, selection, dirty revision and save/reopen parity.

### Area tests

- Crop exact visible pixels from layered scenes containing source imagery, alpha,
  annotations, Thai/English text, shadows, blur and overlapping objects.
- Verify zoom, DPI, reverse drags, canvas-edge intersection, tiny/empty regions,
  placement fallback and omission of handles/guides/previews.
- Verify stale worker rejection, close/cancel during work, bounded memory and no
  partial insertion after render/allocation failure.
- Compare editor preview, PNG and clipboard render for the resulting image object.

### Tauri Vanilla JS tests

- Tool activation, drag preview, Escape cancellation and command payload.
- Duplicate button enablement, `Ctrl+D`, input-focus suppression, selection and
  transition to Select after both workflows.
- EN/TH labels and Help, tooltip content, keyboard access, compact widths,
  high contrast and mixed-DPI coordinate mapping.

### Regression and delivery checks

- Run JS state/UI/engine smoke tests, full Rust tests, formatting and warning-free
  release builds. Recheck Text, Callout, Crop, grouped shapes, layers, Undo/Redo,
  project save/open, OCR, long capture and PNG/copy workflows.
- Test the packaged Tauri editor at normal and high DPI with both languages before
  preparing a new local candidate. Do not push or publish without separate user
  approval after hands-on testing.

## Deferred

Cross-document copy/paste, Windows clipboard storage of editable objects, repeated
paste history, multi-selection, rotation, automatic canvas expansion and copying
objects between tabs are separate features. This scope does not alter the .NET
edition or add a new project format.

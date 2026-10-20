# Editable Image Combination: Stage 2A

Date: 2026-10-12 (Asia/Bangkok)
Status: written design approved by the user on 2026-10-12; implementation started.
Baseline: user-accepted 0.9.0-alpha.14-r2. Rust only. Preserve that candidate,
all earlier work and the existing dirty checkout. No .NET changes, commit,
push, installer or public release. Target local candidate: 0.9.0-alpha.15.

## Scope And Approach

The user selected two stages: first editable combination, automatic layouts
and per-image crop; then a separate native tab-workspace stage. Stage 2A keeps
the existing separate editor windows and adds a compact Combine command.

Alternatives considered:

1. Editable image groups first, tabs afterward (selected): establishes image
   ownership, clipping, rendering and persistence before window management changes.
2. Combine and tabs together: fewer delivery steps, but couples the document
   migration to window lifetime, focus, close decisions and OCR ownership.
3. Flatten sources into PNG layers: simpler, but loses editable text, curves,
   blur and effects; does not meet the selected .NET-like behavior.

Include existing file/clipboard image objects in per-image cropping. Do not add
a layer-list panel, automatic layout of arbitrary selected annotations, new
capture modes, autosave, video, .neosnap interchange or a broad UI redesign.

## Combine Workflow

- Add one compact icon command with EN/TH tooltip and Help. Disable it during
  conflicting capture/import/project/dialog work, following existing guards.
- Show the open Rust editor documents by stable identity, project name or
  capture label, and current canvas dimensions. The initiating editor starts
  checked. Require at least two selected sources; never include another product.
- Checkbox selection, Up/Down ordering, layout options and a numeric gap input.
  Order changes apply only to this proposed combination, not source windows.
- Vertical: left-aligned, original displayed crop sizes, gap between images.
  Horizontal: top-aligned with the same rules. No automatic stretching or scaling.
  Default: Vertical, gap 16 native pixels; valid gap 0-500.
- Free: preserve original sizes, stagger origins by 24 px in both axes in list
  order; disable the gap input. The user then moves images manually.
- Calculate resulting width/height from checked bounds, including gaps but no
  trailing gap. Show dimensions before confirmation and reject oversized results.
- Confirmation snapshots the latest committed revisions of all selected sources.
  If a source has closed, is busy saving/importing, or has an unfinished gesture,
  text entry or effect change, stop with a localized message. Do not cancel or
  discard another source's edits just to combine it.
- Build a new independent editor. Existing source documents, selection, Undo/Redo,
  associated paths, dirty markers and windows remain unchanged. Subsequent changes
  to either source or combination do not propagate to the other.
- The result starts in Select, has an unsaved project marker and empty history.
  The initial combination is its starting document, not a destructive edit to a
  source. Opening the same saved project twice still gives independent documents.
- Snapshot immutable document state and share original Arc-backed frames, points
  and glyph masks. Do not import source Undo/Redo, selection, saved revisions,
  file-path associations or process/window identities into the combined content.
  No image encoding or expensive rendering on the UI input path.
  Worker results require the existing operation/window identity checks. Cancel
  before confirmation creates no editor; failed construction publishes no partial
  document, releases reservations and does not modify source work.
  If the initiating editor closes before completion, discard that pending result.

## Editable Image Ownership

Each selected source becomes one movable image group whose initial local viewport
is that source's current canvas crop. Its local content retains the source frame,
source clipping and ordered editable objects. A group is not a permanently
flattened screenshot. Original source pixels and hidden cropped content remain.

Source-local geometry plus an explicit translation/uniform scale maps content to
the combined canvas. Keep strokes, curves, text masks, callout tails/backgrounds,
steps, shadows/outlines, opacity, image layers and blur parameters in local units.
Repeated move/resize must not round or repeatedly mutate those original units.

Combining a document that already contains groups is supported. Groups form a
bounded ownership tree, not a cyclic/shared-parent graph. Unique edit identities
are remapped safely; frame/mask assets remain shared. Maximum group depth is 8.
Limit violations report an error rather than flattening or dropping content.

Selection behavior:

- Select hits the topmost visible editable child when the pointer is on a drawn
  object, or its image/group when on uncovered image content. Map hit tolerance
  through the displayed scale and respect all ancestor clips.
- Existing text double-click, shape handles and curve handles remain available
  for imported children. Child edits are local to the combined document.
- A compact Select owning image command is enabled for an owned child; it selects
  the immediate image/group parent. Repeating selects its ancestor if present.
  This is the fallback when annotations cover the whole image; no layer panel.
- Moving/resizing a selected image/group carries all descendants. Image corner
  resize keeps aspect ratio and the opposite corner fixed; no rotation, flipping
  or nonuniform scale. Escape cancels the draft without changing the document.
- Front/back commands for a group move the whole group in its sibling stack.
  Commands for a child reorder only within its parent; no implicit reparenting.
- Delete group removes its descendants in one Undo step. Delete child removes
  only that child. Empty image content does not disappear as a side effect.
- New drawing on the combined canvas is a normal top-level annotation. Existing
  imported children stay attached to their owner; drawing over an image does not
  silently attach a new annotation to that image.
- Each committed move, resize, edit, reorder, delete or crop is one existing
  Undo/Redo step. Restore complete geometry/ownership and project dirty state.

## Crop Behavior

- Choosing Crop with an image or group selected captures that image as the crop
  target. Otherwise Crop retains the existing whole-workspace behavior.
  Capture the target identity at tool entry; never retarget halfway through a drag.
- Intersect the confirmed crop with the target's current visible bounds. Round
  and clamp canvas endpoints once; map the retained bounds into source-local
  coordinates without repeated rounding. Reject an empty result and retain the
  same source-to-canvas scale. Cropping never stretches the remaining pixels.
- Per-image crop changes only that image/group viewport and placement bounds;
  canvas size, other layers, child geometry and original source bytes stay fixed.
  All descendants, their shadows and blur respect the new visible image clip.
- Resize after crop uses the cropped viewport's aspect ratio, not the full original
  asset ratio. Undo restores the old viewport and all hidden content. No external
  image file is rewritten or destroyed.
- Enter/checkmark confirms, Escape/cancel rejects. Tooltips and Help distinguish
  image crop from workspace crop. Do not expose original cropped content through
  a source-preview cache or shadow/filter scratch buffer.

## Rendering And Performance

Use the existing renderer for both previews and PNG/clipboard export. A group
composes its local background and children before applying the group's own
opacity/effects. Group opacity is applied once to the combined result, not once
per descendant. Source highlighter and blur retain source-document semantics:
local blur never samples an unrelated image underneath the group. A new top-level
blur retains current whole-composition semantics.

Identity-scale, integer-position combinations must match each source's current
flattened crop pixel-for-pixel inside its visible viewport. Inter-image space is
white. Scaling changes placement/rendering only, not stored source resolution.
Temporary raster caches are allowed, but never replace editable project content.

Keep the existing aggregate 32 MiB distinct source-frame budget across open
editors and retained history, counting shared assets once recursively. Canvas
pixel buffers stay capped at 32 MiB with edges at most 32767; composition scratch
stays within the existing 256 MiB bound. Do not increase limits to mask duplicate
allocations. Use a minimal white base asset for the combined workspace, rather
than allocating another full-sized source bitmap solely for its white background.

Share immutable group snapshots through history. Traverse nested assets for
budget accounting, cache invalidation and stale-composition guards. Bound depth,
total object/point counts and all checked coordinate/size calculations before
publishing the result. Heavy source rendering stays on workers; show movement
and resize feedback without full re-composition on every pointer move. Combine
initialization must not add scans, decoding or PNG encoding to startup/capture.

## Project Compatibility And Safety

Extend .snapzy with schema 2 for group ownership, local transforms and per-image
viewports. Read existing valid schema-1 Alpha 14 projects without changing their
appearance; save with schema 2. Alpha 14 cannot open schema-2 projects. State this
in EN/TH release notes and Help; do not claim backward reader compatibility.

Use the same bounded ZIP/JSON/PNG archive and atomic save path. Persist all
editable nested content, not only cached pixels. Continue deduplicating images
and glyph masks. Validate both schema branches explicitly; never silently ignore
ownership or unknown schemas/properties.

Existing limits apply to totals across the tree: 2000 objects including group
containers, 100000 points per stroke, 1000000 points total, 1 MiB distinct glyph
coverage, 96 MiB container/uncompressed entries and 32 MiB manifest. Validate
finite legal transforms, unique identities, asset references, depth and nonempty
viewports within local content. Corruption or excessive input fails atomically.

Saving/opening restores owner relationships, local edits, crop, effects and stack
order. Selection and Undo/Redo still reset on Open. Copy/PNG is not project save.
Retain the warning that project files contain original information under crop,
blur and covering objects. No secure-redaction or encryption claim.

## Verification And Delivery

TDD covers layout dimensions/order/gaps, no source mutations, safe remapped IDs,
nested grouping, shared-source budget accounting, resize after crop, direct child
editing, owning-image selection, deletion/reordering and exact Undo/Redo.

Pixel fixtures include alpha, white margins, source crops/expanded canvases,
curves, Thai/English text, callout tails, highlights, blur and effects. Compare
source crops against identity-scale combined output; exercise group opacity,
overlaps, transformed clips and filter leakage. Persist/reopen schema 2 with
exact pixels, then edit again; load schema 1 unchanged. Corrupt/deep/oversized
archives and stale worker completions must leave existing editors intact.

Run full unit tests, format and warning-free release build, plus bounded timing
and memory observations. After fresh input-free consent, test the actual combine
dialog, two/three source windows, nested combination, direct child editing,
per-image versus workspace crop, cancellation, close/save and EN/TH at compact
window sizes. Compare PNG and clipboard and recheck OCR/scroll Stop/Escape.
Document any remaining native verification gaps honestly.

Preserve accepted Alpha 14-r2 and produce a separate local Alpha 15 candidate
with accurate EN/TH notes and package hashes. Obtain user testing acceptance
before Stage 2B tabs. No commit or push without a separate user request.

## Next Stage

Stage 2B will specify native tabs, capture/open routing, document switching,
per-tab dirty state, individual/bulk close, OCR ownership and window lifetime.
Do not preempt that design by replacing the window model in Stage 2A.

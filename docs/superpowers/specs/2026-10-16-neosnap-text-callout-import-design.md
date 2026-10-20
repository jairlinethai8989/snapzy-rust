# Editable .neosnap Text And Speech-Bubble Import

Date: 2026-10-16 (Asia/Bangkok)

Baseline: user-accepted SnapZy Rust `0.12.0-alpha.1` candidate with strict
one-way `.neosnap` import for images, drawing tools, shapes, numbered markers
and blur. Proposed candidate: `0.13.0-alpha.1`. Scope: Rust only. Do not modify
.NET, publish, commit or push. Preserve the existing dirty checkout and earlier
candidate archives.

## Goal

Complete the remaining ordinary `.neosnap` version 1 annotation coverage by
importing text and speech bubbles as editable Rust objects. Preserve the source
text, anchor, font size, explicit line breaks, opacity, compatible effects,
bubble body dimensions, fill, independent border, tail, owner relationship and
layer order. Imported objects must support subsequent text edits, movement,
resizing, tail movement, effects, layering, Undo/Redo, Copy/PNG and `.snapzy`
round-trip.

The user accepts small differences in font antialiasing, line wrapping and
resulting glyph height because .NET renders these objects in a browser canvas
while Rust uses native Windows glyph rasterization. This exception does not
permit losing text, collapsing an explicitly sized bubble, merging independent
colors, dropping effects, flattening an object or silently importing only part
of a document.

## Chosen Approach

Extend the editable Rust text payload with explicit source-layout metadata and
an independent bubble-border style. Imported text uses a bounded `.NET`
rendering mode, while native Rust text keeps its current defaults and behavior.
This approach retains semantic editability and source geometry without storing
an opaque flattened copy of the annotation.

Alternatives rejected:

1. Mapping into the existing Rust text payload without new metadata would lose
   explicit bubble height, source padding and independent border color.
2. Flattening text or bubbles into image pixels would improve visual similarity
   but make the text, frame and tail non-editable.
3. Retaining a second hidden raster underneath an editable object would create
   duplicate rendering and ambiguous save/edit behavior.

The `.snapzy` project schema advances from 3 to 4 for the new text-layout and
bubble-border data. The reader must continue accepting schema 1, 2 and 3 with
their existing defaults. Schema 4 must round-trip all earlier objects unchanged.

## Source Text Mapping

- Accept `tool: "text"` only with a nonblank UTF-8 string of at most 4,096
  Unicode scalar values and valid `x1`,`y1`, color, opacity and effects.
- Preserve embedded newlines and remove no leading, trailing or repeated spaces
  from stored source data. Validation may reject text whose rendered mask would
  exceed the existing bounded glyph-memory budget.
- The .NET visible font size is `max(12, source size * 4)` CSS pixels with weight
  600. Retain the finite source size in `.NET` layout metadata and rasterize to
  the nearest supported device pixel only at the glyph boundary. A subpixel
  raster-size difference is within the accepted visual tolerance; source anchor
  and retained metadata must not drift on save/reopen.
- Accept the ordinary editable source-size range 0 through 64, which produces a
  visible 12 through 256 px font under the source rule. Larger stored values are
  rejected rather than allocated or reduced silently.
- Plain `.NET` text uses explicit newlines only and has no automatic width frame.
  Imported plain text therefore remains unwrapped until the user deliberately
  resizes its Rust text frame.
- Imported text has no fill panel behind it. The declared bubble preset fields
  present on ordinary .NET objects are validated but do not become an active
  text background.
- Text color remains independent from outline and shadow colors.

## Speech-Bubble Mapping

- Accept `tool: "bubble"` with valid text and anchor. Use the source defaults
  when legacy optional fields are absent: width 260 px, automatic minimum body
  height, white fill, the text color as border color, 5 px border and the legacy
  bottom tail. Match the .NET renderer's fallback semantics: a stored zero width
  resolves to 260 px and a stored zero border width resolves to 5 px.
- Retain `bubbleWidth` as the body width and `bubbleHeight` as the explicit
  minimum body height. The displayed body height remains the greater of that
  minimum and the height needed for all text; text is never clipped merely to
  reproduce a smaller stale value.
- Retain the .NET padding rule `min(14, width / 5)` and wrap at the available
  inner width. Rust may choose different word boundaries where its native font
  metrics differ, as approved by the user.
- Preserve `bubbleFill`, `bubbleBorder` and `bubbleBorderWidth` separately from
  the text color. The Rust model and compact existing controls must allow fill,
  text color, border color and border width to be edited independently.
- Preserve an explicit tail offset relative to the bubble body and retain the
  source edge-selection rule. A tip stored inside the body is presented on the
  nearest edge with the source 16 px outward offset. If a legacy source omits
  `tail`, materialize the exact legacy bottom-tail result so later movement and
  `.snapzy` save/reopen do not change it. The selected object keeps a draggable
  tail handle.
- Keep the body and tail inside the normal editable geometry model. Do not store
  a browser path or executable source content.

## Effects, Ownership And Scaling

- Allow source outline only for text and speech bubbles. Preserve enabled state,
  color and an exactly reversible width from 1 through 16 px. A disabled outline
  keeps its stored preset but does not affect pixels.
- Preserve compatible shadow enabled state, color, softness 0-32 px and offsets
  -32 through 32 px, using the existing whole-object opacity semantics.
- Preserve object opacity from 0 through 100 percent using the existing exact
  percentage conversion tolerance.
- Text and bubbles may be children of an imported image. Reverse the owner's
  uniform scale for anchor, font metrics, body size, padding, border width, tail
  and effects before validation. Keep the existing contiguous ownership,
  clipping, depth and draw-order rules.
- Reject the whole document if any active property cannot be represented after
  reversing owner scale. Never clamp or discard a property to make it fit.

## Editing And Native Behavior

- Imported text and bubbles open selected through the same editor tools as native
  objects. Double-click edits text; Select moves and resizes; bubbles expose the
  tail handle. Layer controls, Delete, Undo/Redo, opacity and Effects continue to
  operate on the complete object.
- Side resizing changes wrap width while retaining font size. Corner resizing
  scales font, body and tail around the opposite anchor. The stored explicit
  minimum bubble height remains meaningful after a width-only edit.
- The size control must represent imported .NET text through its full ordinary
  12-256 px visible range. Newly created Rust text keeps its existing default;
  widening the valid control range must not change existing objects by itself.
- Editing imported text retains its `.NET` layout mode until the user changes a
  layout-affecting value. The implementation may then regenerate native Windows
  glyph coverage while retaining frame, independent colors and all semantic
  values. It must not unexpectedly switch modes merely on selection or save.
- Copy Image, PNG export and editor preview use the same render path. Saving the
  imported document always creates or updates `.snapzy`; the source `.neosnap`
  is never associated as a writable destination and is never modified.

## Validation And Failure Policy

- Keep the existing 96 MiB source, 100-asset, 2,000-object, point, decoded-image,
  canvas and mask-memory limits. Derived text and effect bounds use checked
  arithmetic before allocation.
- Reject invalid or nonfinite anchors, sizes, body dimensions, tail coordinates,
  opacity, colors, border widths, effects, ownership or text whose bounded mask
  cannot be allocated.
- Continue rejecting duplicate or unknown JSON fields and unknown tools. Known
  bubble preset fields on plain text may be ignored only after validation because
  the .NET renderer does not apply them to plain text.
- Conversion remains all-or-nothing. Failure leaves current tabs, histories,
  paths and the source file unchanged. Diagnostics remain bounded and localized
  EN/TH and never echo source text.
- A successful import must pass normal project validation and encode as schema 4
  before publication to the editor. No partially valid project is shown.

## Persistence Compatibility

- Schema 4 stores source text-layout mode, retained source font size, optional
  explicit bubble frame/padding, independent border style, tail and glyph mask.
- Schema 1-3 decode with the exact behavior they had before this increment.
  Existing native text/callout payloads receive native layout and derive their
  old border behavior through migration defaults; their rendered pixels and
  controls must not change.
- A schema-4 project containing only earlier tools still opens and validates.
  Older Rust binaries are not required to open schema 4; the UI should continue
  reporting an unsupported newer project version rather than corrupting it.
- Save remains atomic. Failed serialization or replacement leaves the previous
  `.snapzy` file intact.

## Verification

Write failing tests before production changes and use synthetic fixtures rather
than user documents.

1. Import English, Thai and mixed multiline text at minimum, ordinary and maximum
   source sizes; verify content, anchor, color, opacity, outline and shadow.
2. Import bubbles with default and explicit width/height, each tail side, custom
   fill/border/text colors and border widths; verify editable geometry and layers.
3. Import owned text/bubbles beneath uniformly scaled image groups and verify
   global rendering, local values, clipping and exact source order.
4. Edit text, resize sides/corners, move tail, recolor each independent channel,
   change effects, Undo/Redo, Copy and export PNG.
5. Save/reopen `.snapzy` schema 4 and verify text, source layout, frame, tail,
   effects, owner, layer order and editability. Open schema 1-3 regression files.
6. Reject empty/oversized text, hostile dimensions, invalid colors/effects,
   nonreversible owner scaling, mask-budget overflow, unknown fields/tools and a
   mixed file containing any invalid object; verify no partial tab appears.
7. Regression-test prior `.neosnap` tools, native editor, combine, OCR, scrolling
   Stop/Esc and video preview. Run formatting, locked tests, optimized build and
   package/hash verification before presenting a separate local candidate.

Native focus-sensitive tests require fresh user readiness. Do not automate chat
paste or inspect private files. Do not push without a new explicit instruction.

## Acceptance Boundary

Success means ordinary .NET-created text and speech bubbles import as editable
Rust objects, retain their semantic values and source geometry, survive
`.snapzy` round-trip and render consistently across preview, Copy and PNG.
Pixel-identical browser font antialiasing and identical wrapping are explicitly
outside the promise. Bidirectional `.snapzy` to `.neosnap` export, a broad UI
redesign and .NET changes are outside this increment.

## Review Record

The user selected editable semantic conversion over pixel-identical flattening
and accepted small differences in line wrapping or glyph height. The design
therefore extends the Rust project model rather than weakening the existing
strict importer. Implementation remains gated on approval of this written
specification.

# Editable .neosnap Step And Blur Import

Date: 2026-10-15

Baseline: user-accepted SnapZy Rust 0.11.0-alpha.1 one-way `.neosnap`
import. Scope: Rust only. Do not modify .NET, publish, commit or push.

## Goal

Extend the strict one-way `.neosnap` version 1 importer to accept numbered
markers and blur regions as editable Rust objects. Preserve position, size,
strength, color, opacity, compatible shadow, owner relationship and layer order.
Small raster differences at number edges and in blur softness are acceptable;
semantic values must not be silently clamped, rounded away, dropped or flattened.

Text and speech bubbles remain unsupported in this increment. A file containing
either type, or any other unrepresentable object, still fails as a whole without
opening a partial document.

## Chosen Approach

Expand the native Step and Blur models and use the existing per-object
Native/.NET rendering marker for source-specific presentation. Imported objects
remain selectable and editable. Converting values into the old Rust ranges or
flattening them into image pixels is rejected because either loses data or
editability.

The current `.snapzy` schema 3 already stores a per-object rendering marker.
Schema 1 and 2 continue to decode as Native. Schema 3 validation is expanded to
allow DotNet rendering for Step and Blur in addition to the previously accepted
pen, highlighter and arrow objects. No format version bump is required because
the serialized value and its compatibility rule already exist in schema 3.

## Numbered Marker Mapping

- Accept `.NET` `tool: "number"` only when `number` is an integer from 1 through
  1,000,000, the source size resolves to an integer from 2 through 32 after
  reversing owner scale, and the remaining geometry/style fields are valid.
- Preserve the source center at `x1`,`y1`, color, opacity, shadow, owner and
  layer order.
- Reverse an owner image's exact uniform scale before validating source size and
  effect values, using the same serialization tolerance as other owned objects.
- Retain the source size in `style.width` and derive the visible radius with the
  .NET rule:
  `max(16, size * 3, digit_count * size * 1.6)` and store its resulting diameter.
- DotNet Step rendering draws the filled source color, white ring with width
  `max(1, radius / 16)`, and a centered bold number. The number switches to dark
  ink for a light fill and white for a dark fill using the source luminance rule.
- Use a bold native Windows digit mask sized from the .NET radius formula. Minor
  antialiasing or font-edge differences are accepted, but center, circle size,
  number, contrast choice and ring width must be retained.
- DotNet Step rendering computes the fractional source radius from retained size;
  its integral stored diameter is the ceiling used for bounds and hit testing.
  This creates at most a subpixel edge difference and never changes the center.
- A selected imported DotNet Step edits source size from 2 through 32. Existing
  Native Steps continue editing diameter from 20 through 120 and keep their
  existing 36 px default. Both paths check glyph allocation before publication.

## Blur Mapping

- Accept `.NET` `tool: "blur"` with a finite strength from 1 through 64 px and
  a nondegenerate rectangle.
- Preserve rectangle, strength, opacity, compatible shadow, owner and layer
  order. Color is retained as source metadata/style even though the blur pixels
  do not use it directly.
- Expand the Rust blur control and project validation from 2-30 to 1-64 px.
  Newly created Native blur regions keep their existing 8 px default.
- Apply object opacity to the blurred result, matching the source semantics, and
  retain the existing editable shadow behavior. A value of zero opacity remains
  a valid hidden but editable blur object.
- DotNet blur uses the source strength directly with the bounded Rust Gaussian
  implementation. Exact browser Canvas filter pixels are not promised; region,
  radius meaning, clipping, stacking and editability are.
- Increase blur halo calculations through checked arithmetic and retain bounded
  256 px processing tiles. Tests must cover 64 px without unbounded scratch
  allocation, seams, hangs or reading pixels above the blur layer.

## Ownership And Clipping

Number and blur objects may be children of a compatible imported image. Their
geometry, Step source size, blur strength and effect values are restored to local
units by reversing the owner's uniform scale. Existing contiguous ownership,
depth, draw-order and clipping rules remain unchanged. An owner-bound object is
clipped by its image; an active unrepresentable root clip rejects the file.

## Editing And Persistence

Imported Step objects support move, size, color, opacity, shadow, layer order,
delete and Undo/Redo. Imported Blur objects support move/resize, strength,
opacity, shadow, layer order, delete and Undo/Redo. Copy Image and PNG export use
the same rendered result as the editor. Save/Save As writes `.snapzy`; reopening
must retain geometry, source rendering, effects, ownership and editability.
The source `.neosnap` is never associated as a writable project path and is never
modified.

## Validation And Failure

- Keep the existing 96 MiB source, object, point, pixel, mask and document limits.
- Reject nonfinite geometry, invalid numbers, degenerate blur regions, invalid
  effects, unsafe derived diameters, mask-budget overflow and values that cannot
  be reversed from owner scaling without loss.
- Diagnostics remain bounded, localized EN/TH and do not echo source text or
  other untrusted values.
- Conversion remains all-or-nothing. No clamp, flatten, partial tab, reordered
  object or silent fallback is permitted.

## Verification

Test first and observe each new test fail before implementation.

1. Import one-digit and multi-digit markers at source minimum, ordinary and
   large sizes; verify center, diameter, fill, contrast, ring, opacity and shadow.
2. Import blur at 1, 30 and 64 px, including opacity endpoints, shadow, crop edge
   and a region above/below other layers.
3. Import owned Step/Blur objects under a uniformly scaled image and verify local
   editing plus rendered global geometry.
4. Verify move/resize/restyle, layer order, Undo/Redo, Copy, PNG and `.snapzy`
   save/reopen.
5. Verify mixed supported files import completely while text, speech bubble and
   malformed/oversized cases reject the complete file.
6. Regression-test schema 1/2/3 projects and all existing editor, OCR, scrolling,
   combine and video behavior.
7. Run formatting, locked check/tests, optimized build and package/hash checks.
   Present a separate candidate for native testing; do not push without a new
   explicit instruction.

## Acceptance Boundary

Success means numbered markers and blur regions from ordinary .NET-created
`.neosnap` files retain their semantic settings and are editable after import and
native round-trip. It does not claim pixel-identical browser font antialiasing or
Canvas blur kernels, and it does not add text or speech-bubble import yet.

## Review Record

The user approved splitting the remaining four tools into two increments. For
this increment the user accepted small number-edge and blur-softness differences
while requiring position, size, blur strength, color, shadow, layer order and
full editing. The user also approved expanding general Rust controls to cover
the source ranges rather than leaving imported objects outside editable limits.

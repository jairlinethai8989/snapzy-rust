# One-Way .NET Project Import: Compatible Subset

Date: 2026-10-15 (Asia/Bangkok).
Status: written first-candidate scope approved by the user on 2026-10-15.
Baseline: user-accepted 0.10.0-alpha.3 R3. Proposed candidate: 0.11.0-alpha.1.
Rust only. Do not modify .NET, commit, push, build an installer or publish.
Preserve the existing dirty checkout, accepted candidates and user-owned files.

## Decisions And Alternatives

The user selected one-way .neosnap import before UI parity and explicitly chose
to reject the whole file when complete conversion is unavailable.

1. Import the compatible subset as editable native objects, reject an entire
   incompatible file (recommended for this first candidate). Uses a bounded
   schema-3 rendering marker where source geometry differs and prevents silent
   data loss; not every .NET project can be opened yet.
2. Import only supported objects after confirmation (rejected by the user).
   Missing objects could change the meaning or concealment of a screenshot.
3. Reimplement .NET rendering/layout and all property ranges first (deferred).
   Broader coverage, but couples import to text layout, callout geometry, blur,
   new native properties and a project-schema migration.

There is no flattened-image fallback and no partial-import override. This is
not a promise to open every .neosnap file in the first candidate. Unsupported
types remain unsupported even when invisible or outside the current crop.

## Observed Formats

Both local .NET source trees export JSON with format `neo-snap`, version 1,
width, height, background, cropRect, PNG data-URL assets and ordered objects.
The .NET `.neosnap` is not the ZIP-based Rust `.snapzy` format.

References: .NET Assets/editor-project.js, editor.js, editor-annotations.js;
Rust src/project.rs, src/project_ui.rs, src/editor.rs, src/combine.rs.
Read these sources as references only. Do not launch .NET or inspect personal
projects to create fixtures without permission.

## First-Candidate Compatibility Boundary

Support ordinary v1 documents with the following content, provided all fields
and ownership can be represented without losing an active property:

- PNG background and additional image layers, including retained original PNG
  pixels and representable per-image source crop.
- Pen and highlighter paths; solid/dashed/dotted styles where native tools
  support them. Preserve .NET midpoint quadratic smoothing and single-point dots
  with explicit source-rendering metadata rather than changing path points.
- Straight and quadratic curved lines/arrows; retain endpoints, control point,
  color, width, opacity and source ordering. Imported arrows retain the .NET
  arrowhead length and angle while newly drawn Rust arrows remain unchanged.
- Outline/filled rectangles and ellipses.
- Native-compatible shadows on these objects; enabled, color, softness and
  X/Y offsets retained. No automatic shrinking of effect ranges.
- Workspace crop and image ownership/clipping only where the existing native
  group/viewport model preserves both displayed stacking and subsequent edits.
  For an owned annotation, the .NET renderer uses the owner image bounds instead
  of the stored `clipRect`; a valid fractional stored clip may therefore be
  ignored without changing pixels. An active root clip is never discarded.

Reject the entire document if it contains text, speech bubbles, numbered steps
or blur in this first candidate. Their font weight/metrics, explicit bubble
dimensions/separate border color, numbered-circle rendering and blur sampling
currently differ from the Rust model. Preserving only strings or bounding boxes
is not complete conversion. Add these types in a separately reviewed phase.

Reject unknown tools, unknown active style fields, unsupported format/version,
unrepresentable clips, cyclic/missing owners or interleaved ownership that
cannot retain original draw order. Reject invalid asset references even for
unused objects. Never sort objects into a different layer order to make a file
fit the native ownership tree.

This phase adds `.snapzy` schema 3 with a per-object Native/.NET path-rendering
marker. Readers continue to accept schema 1 and 2 as Native; schema 1/2 cannot
claim .NET rendering metadata. Native tool/effect ranges do not expand.
Native raster antialiasing may differ slightly; pixel-identical .NET rendering
is not promised. This exception does not permit dropping/clamping properties,
altering geometry, smoothing away path points or changing opacity/blend mode.

## Numeric And Resource Policy

- Bound the source JSON at 96 MiB before allocation; bounded reading also checks
  actual bytes, not only file metadata. Use structured JSON/base64/PNG parsers.
- Allow at most 100 assets and 2,000 source objects. After native group expansion,
  the existing 2,000-object and depth-8 validation still applies.
- Retained decoded source pixels, canvas pixels and open-document reservations
  obey existing 32 MiB BGRA budgets; glyph limits remain unchanged.
- Check PNG signatures, IHDR dimensions, checked arithmetic and aggregate decoded
  bytes before image allocation. Only embedded PNG data URLs; no URLs, paths,
  SVG, scripts, archives, network fetches or external executables.
- Require finite bounded geometry and intact required fields. Canvas/workspace
  rects need integral representable coordinates. Image source crops retain
  finite fractional viewport edges supported by the native group model; never
  round a crop to make it fit.
- Widths must resolve to native 1-40 px integer strokes; shadow softness 0-32 px,
  offsets -32 to 32 px. For a child of a scaled image, reverse the owner's exact
  uniform scale before checking these integer ranges. Reject values that do not
  return to a native integer within floating-point serialization tolerance.
  Use native defaults only for absent or inactive properties whose .NET
  rendering semantics demonstrably equal those defaults.
- Opacity must map to native integer percent within 1e-7 percentage points of
  an integer; this only absorbs floating-point parsing error. Do not approximate
  arbitrary fractional percentages. Reject a value outside the source 0-1 range.
- Bound JSON nesting at 16, reject duplicate object keys and count points across
  the document before construction (at most 100,000 per path and 1,000,000
  total). Apply checked arithmetic and a fixed diagnostic cap on hostile input.
- Unknown rendering properties fail preflight. Known .NET bookkeeping fields
  may be ignored only through an explicit allowlist verified from source.
- Reuse existing validation for .snapzy saveability. A successfully imported
  project must encode as schema 3 and reopen with the same rendering metadata.

## Open And Save Workflow

Use the existing Open image/project command, adding .neosnap to its filter and
help text. No new toolbar button or visual redesign. Use current tab/new-window
preference; never overwrite a currently open document.

Read, inspect and construct a complete candidate off the input thread. Publish
only after validation, resource checks and live operation/host identity checks.
Recheck the available budget on completion. If import fails or the origin is no
longer valid, release reservations and discard the candidate, leaving all
existing tabs/windows, histories and paths unchanged.

On success:

- Open an independent native document in Select with no selected object and
  empty Undo/Redo. Retain original images and editable supported objects.
- Treat it as an unsaved Rust project even before the first edit. Save and
  Save As ask for a .snapzy destination; never associate the save path with
  the input .neosnap. Closing triggers existing Save/Discard/Cancel behavior.
- Preserve the imported source filename as a label only. Reopening the source
  produces another independent document; do not overwrite the existing import.
- Copy/PNG remain exports, not a saved project. The usual warning applies:
  editable files retain original/cropped/covered data; share flattened PNG
  when redacting sensitive information.

## Failure Reporting

Return structured diagnostics from an isolated importer module; localize the
user-facing summary in EN/TH at the UI boundary. Include object index/tool or
asset index plus reason, not full text/image/base64 content. Show up to ten
diagnostics and a remaining count; malformed input may fail at the first error.

The summary states that nothing was imported, the original file is unchanged,
and the user may keep editing it in .NET. There is no Continue/Skip button.
Avoid exposing the input contents through logs. A failed import must not show
a success state, leave a half-created tab or dirty an existing document.

## Architecture

- Dedicated import module owns bounded parsing, typed v1 DTOs, preflight,
  explicit property mapping and all-or-nothing native Project construction.
- Existing project UI routes .snapzy to its current loader, .neosnap to the
  importer, images to the current decoder. No content execution by extension.
- Native editor, group model, validator and atomic .snapzy writer remain the
  authority. Identity routing and resource accounting use existing helpers.
- Minimal completion metadata distinguishes an imported unsaved document from
  a loaded/saved .snapzy. Do not infer saved state from having a source filename.

## Verification And Acceptance

Use test-first synthetic fixtures generated from the observed .NET schema.
Do not use user documents, clipboard contents, private chats or recordings.

1. Accepted fixtures cover background/no background, extra PNGs, every supported
   tool, curved controls, all patterns, opacity, shadows, crop and compatible
   owned annotations. Verify geometry, source pixels and ordered ownership.
2. Unsupported text/bubble/step/blur, unknown fields/tools, excessive active
   effects, fractional incompatible values and draw-order conflicts reject the
   complete file, including when the unsupported object is hidden/outside crop.
3. Malformed JSON/base64/PNG, duplicates, missing assets, owners/cycles, forged
   dimensions, unsupported version, truncation and all resource limits fail
   safely with bounded work and no partial document.
4. Imported edits, move/resize, curve bending, layers and Undo/Redo work; Save
   and Save As produce .snapzy that reopens with the same content. Hash the input
   .neosnap before/after success, failed import, canceled save and close.
5. Verify imported-unsaved close prompts, independent repeated imports, EN/TH,
   tab/new-window routing, closed-origin/stale completion and budget races.
6. Regression-check native .snapzy opening, combine, Copy/PNG, OCR, scrolling
   Stop/Esc, and video preview. Native focus-sensitive tests require fresh user
   readiness; interactive clipboard automation remains disabled.

Run formatting, locked tests, optimized build and package/hash verification
before presenting a local candidate. Report coverage and unsupported types
explicitly; do not label this full .NET project compatibility. No installer,
release, commit or push without a subsequent explicit request.

## Review Record

Context, alternatives and strict whole-file policy reviewed. Scope intentionally
excludes bidirectional export. Numeric limits,
unsupported tools, imported-unsaved state, atomic publication and source-file
preservation are explicit. Implementation is gated on user approval of this
written specification, including its initial compatibility boundary (approved).
Implementation review additionally found different native arrowhead geometry and
multi-point pen smoothing. On 2026-10-15 the user approved expanding the native
renderer and project schema in this phase to preserve those source conventions.
Schema 3 stores that decision per object; new Rust drawings remain Native.
The user completed the native candidate test on 2026-10-15 and reported that it
passed. Acceptance applies to this documented one-way compatibility boundary;
unsupported object types remain intentionally rejected as a whole file.

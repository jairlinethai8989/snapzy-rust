# Object Shadows And Text Outlines

Status: combined scope and written specification approved by the user on 2026-10-11.
Target: SnapZy Rust local alpha following accepted 0.9.0-alpha.12.
Do not change .NET, commit, push, or publish this increment without authorization.

## Context And Approach

The user accepted Alpha 12 and approved shadows for every annotation type,
including inserted images and blur regions, plus outlines on Text and Callout
glyphs. Both effects start disabled. Existing opacity, layers, Undo/Redo and
PNG/clipboard parity must remain intact; a broad UI redesign is deferred.

Alternatives considered: implement shadows then outlines in separate candidates,
or implement both over a shared effects pipeline. The user selected the combined
approach. One shared pipeline avoids divergent opacity and export behavior.

Read-only .NET reference:
`work/snapcraft-windows/src/SnapCraft/Assets/editor-effects-controls.js`,
`editor-annotations.js`, and `editor.html`, relative to the enclosing workspace.
Match observable behavior and defaults, not identical rasterizer pixels.

## Defaults And Supported Objects

- Shadow: disabled, RGB black, softness 6 px, horizontal offset 3 px, vertical
  offset 3 px. Softness accepts integers 0-32; offsets accept -32 through 32 px.
  Zero softness produces a hard shadow. Negative offsets point left/up.
- Shadow applies to Pen, Highlighter, Line, Arrow, outline/filled rectangle and
  ellipse, Text, Callout, Step, inserted Image and Blur. Select/Crop are tools,
  not paintable objects; the captured background never gains a shadow.
- Text outline: disabled, RGB black, thickness 2 px, accepts integers 1-16.
  It surrounds glyphs in Text and Callout only, not their text-box borders,
  bubble perimeter, tail, or numbered-step glyphs. Existing colors/font sizes,
  normal red 5 px strokes and yellow 24 px / 40% highlight defaults stay intact.
- Disabling an effect retains its last settings but removes its pixels.
  No effects defaults are persisted across application restarts in this phase.
- Blur opacity remains unsupported; adding a shadow does not enable opacity
  for Blur or alter blur strength or the protected rectangle.

## Editing And Compact Controls

Add one Effects command to the current editor controls with an accessible
EN/TH tooltip and Help entry. Open a compact, owner-centered native settings
panel instead of expanding the adjustment bar or changing the overall layout.
The panel contains Shadow and Text Outline sections, enable checkboxes, color
swatches/color chooser, numeric fields with steppers, and Apply/Cancel commands.
Disable the outline section for non-text objects/tools. Disable the Effects
command in Select without a selection and during Crop.

On open, populate from the selected object when applicable; otherwise use the
active drawing defaults. Work on a draft copy while the panel is open. Apply
validates all applicable values and changes the selected object in one history
entry, or changes only defaults if no object is selected. Invalid input keeps
the panel open with localized feedback and does not partially apply changes.
Cancel, Escape, or closing the panel leaves document and defaults unchanged.
No pointer-dependent live canvas editing is required while the panel is open;
the committed preview updates immediately after Apply.

Applying in a drawing mode also updates the defaults used for subsequently
created objects. Applying in Select changes only the selected object. Selecting
an old object displays its stored effects without rewriting drawing defaults.
Image imports use current shadow defaults; outline settings have no effect on
non-text objects. Existing objects do not acquire effects merely because a new
drawing default is enabled. A no-op Apply creates no history entry.

Resize, move, bend, reordering, text editing, crop and opacity changes preserve
effects. Undo/Redo restores effect settings and pixels, without unexpectedly
changing the active drawing defaults. Opening a panel cancels an unfinished
geometry gesture using the existing cancellation rules.

## Rendering Semantics

Store validated effect values on each annotation. Retain cheap immutable
snapshots and original image/glyph masks; never paint effects into source data.
Use a focused effects module for mask expansion, softness, bounds and shadow
composition; keep editor history, native controls and rendering responsibilities
separate. Reuse existing render/worker paths instead of adding an independent
export renderer or a second preview engine.

For ordinary objects, rasterize the complete object silhouette at intrinsic
alpha, including any text outline, background, callout frame/tail, or arrowhead.
Generate one shadow from that silhouette, then paint the original above it.
Combine shadow and object as one group before applying object opacity once.
An object at 0% opacity has no visible object or shadow. Image alpha determines
its shadow: fully transparent source pixels must not cast a rectangular shadow.
Transparent text/bubble backgrounds remain transparent and cast no filled-box
shadow. Filled backgrounds cast the corresponding filled silhouette.

Highlighter retains multiply blending for its ink. Compose its shadow behind
the ink at the same layer, using the object's opacity once for each contribution;
do not turn the ink into opaque paint or multiply its opacity twice. Preserve
the old no-shadow highlighter output exactly.

Blur retains its existing backdrop operation at its layer. Derive its shadow
from the region rectangle, exclude the rectangle interior from the shadow,
then retain the existing blurred interior. The shadow is decorative outside
the protected region and must not unblur, tint, expose, or move protected pixels.
Later objects still follow ordinary layer order. Clip final effects to the
existing canvas/crop; do not silently enlarge exported dimensions.

Glyph outline uses coverage expansion of the native mask with rounded joins,
drawn behind the original glyph fill. It must preserve Thai combining marks,
line breaks and wrapping. The specified thickness is the outward extent in
source pixels, independent of zoom. Glyph fill, outline and text-box background
are parts of the same object for opacity and shadow. Resizing/retyping must
regenerate masks without accumulating previous outlines.

## Performance And Failure Handling

With both effects disabled, use the existing Alpha 12 rendering path unchanged.
Do not rerasterize an entire long screenshot on each pointer move. Bound scratch
buffers to visible/affected tiles with input halos for softness, offsets, glyph
expansion, path clipping and existing backdrop blur dependencies. Include old
and new effect extents when replacing a moved/resized/bent annotation, including
negative offsets and objects partially outside the crop. Cache reusable derived
masks where it meaningfully avoids repeated work; invalidate by object content
and effects, not only native window identity.

Heavy committed rendering remains on the existing background composition worker;
generation guards reject stale results after Undo/Redo or closing a preview.
Respect existing frame and allocation limits. Surface localized rendering errors
without crashing or silently exporting an image with missing effects. Failed
composition must not allow Copy/Save of an old revision as if it were current.

Record no-effects and enabled-effects timings on the same large fixture against
Alpha 12. Investigate noticeable input stalls, tile seams, unbounded scratch
allocations, flicker or new GDI/resource growth before submitting a candidate.
Do not claim a speed improvement without measurement.

## Verification And Delivery

1. Add failing tests before implementation: defaults, validation, no-op edits,
   per-object effects, Select/default isolation, imports, cancellation and one
   Undo entry. Test effects retained by resize, bending, text edits and layers.
2. Pixel tests: effects-off exact regression, hard/soft and negative shadows,
   transparent images/text, full callout/arrow silhouettes, 0/50/100% opacity,
   highlighter multiply, text fill/outline ordering and Thai glyph masks.
3. Blur tests: protected interior identical with and without shadow, exterior
   shadow bounded/clipped, strength unchanged, correct order with other objects.
4. Large tiled fixtures: effect extents across tile boundaries, bounded
   replacement feedback against committed output, crop edges and multiple blur
   dependencies. Check source pixels remain immutable and no seams appear.
5. Guarded native tests after fresh input-free consent: Effects panel EN/TH,
   values and color selection, Apply/Cancel/Escape, visible result, history,
   PNG/clipboard parity, existing tools/OCR and synthetic scrolling Stop/Escape.
   Split testing into agreed intervals; do not infer live Lark, mixed-DPI or
   chat-app pasting from synthetic fixture results.
6. Run the full unit suite, formatting, release build, scoped native checks and
   ZIP/staging hash verification. Preserve Alpha 12 and earlier candidates;
   package a distinct local candidate with accurate EN/TH notes for user testing.
   No public release, commit or push before explicit authorization.

## Deferred

Project serialization, rich text, font-family selection, rotation, blur opacity,
video recording, layer-list redesign and complete .NET-style UI parity are not
part of this increment. No online service or new runtime requirement is added.

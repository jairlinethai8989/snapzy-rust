# Shapes, Stroke Patterns And Bendable Lines

Date: 2026-10-11 (Asia/Bangkok)
Status: combined written specification approved by the user on 2026-10-11.
Scope: SnapZy Rust only. No .NET changes, commit, push, installer or public release.

## Context And Approaches

Alpha 9 has user acceptance for Lark automatic long capture followed by Stop:
the resulting image matches the screen in the tested environment. Keep that
behavior intact. The next roadmap increment fills gaps in editor tools before
visual redesign or video work.

1. Add shapes, patterns and curves in one candidate, implemented and verified
   in internal steps. Selected by the user: one tester delivery includes all
   requested geometry tools, with separate test evidence for each behavior.
2. Deliver shapes/patterns first, then curves in another increment. Originally
   recommended for a smaller regression surface; superseded by the user's
   request to combine them.
3. Add only ellipse and fill first. Smaller, but delays the requested patterned
   drawing and requires another toolbar/control pass.

The current Rust editor already has rectangle outlines, lines and arrows. The
.NET reference supports rectangle/ellipse outlines, filled rectangle/ellipse,
and patterns for Pen, Line, Arrow and outline shapes. It uses width-relative
dash lengths and round dots. This increment follows that functional model.

## Tools And Controls

- Keep the rectangle outline tool; add ellipse outline, filled rectangle and
  filled ellipse. Use the existing native icon/control system, with EN/TH
  tooltips and Help entries for every added tool and control.
- Drawing uses the current drag-from-corner interaction. Either drag direction
  creates the same normalized shape. A zero-width or zero-height shape creates
  no object. No aspect-ratio modifier is added in this increment.
- Add a compact native pattern menu with visible solid/dashed/dotted samples.
  Default is Solid. Enable it for Pen, Line, Arrow, Rectangle outline and
  Ellipse outline, or a selected object of those kinds. Disable it for filled
  shapes, Highlighter, Text, Callout, Step, Blur and images.
- Normal annotation defaults remain red (#FF0000), 5 source pixels, Solid.
  Pattern choice is remembered within the normal drawing style for the editor
  session. Selecting an existing object loads its style without silently
  overwriting the remembered style for future objects. Highlighter keeps its
  separate yellow, 24 px, 40% defaults and always uses its existing continuous
  multiply-blended stroke.
- Filled shapes use the chosen annotation color, fully opaque, with no separate
  outline. Width and pattern controls do not affect them and are disabled while
  a filled shape is active or selected. Fill opacity is deferred.
- Shapes and patterns remain individual editable objects. Support selecting,
  moving, recoloring, deleting and the existing four layer-order commands.
  Existing general shape resizing is not added here; text/image resize remains
  unchanged. New resizing gestures can be specified separately.
- Escape, tool change or capture loss cancels an unfinished drawing. One
  completed drawing or changed property is one Undo/Redo edit; cancelled and
  unchanged operations add no history entry. Selection, deletion and layer
  changes retain the current saved/unsaved behavior.

## Bendable Lines And Arrows

- New Line and Arrow objects start straight. Select one with the Select tool
  to reveal a bend handle at the midpoint of the visible path. The handle stays
  on the curve, not at the off-path mathematical control point, matching the
  .NET reference's mouse interaction. Do not add a separate curve drawing tool.
- Dragging the midpoint handle bends a quadratic Bezier while both endpoints
  remain fixed. Show smooth bounded feedback throughout the drag. The handle
  follows the pointer without jumping on initial press; prioritize handle hit
  testing over whole-object movement. Keep its visual size/hit tolerance stable
  in display coordinates across zoom/DPI, and hide it outside the visible canvas.
- Double-click the bend handle to reset the selected curve to its original
  straight path. Reset is one Undo edit; resetting an already straight line
  does not change the revision. Add EN/TH Help describing bend and reset.
- Clamp the dragged midpoint to the document's current crop; validate finite
  control coordinates against existing coordinate limits. A mathematical
  control point may be outside the canvas even when the curve is inside it.
  Reject invalid proposals without changing committed geometry or history.
- Releasing a changed handle commits one edit. Escape, tool change, capture
  loss or an invalid proposal discards the uncommitted curve drag and restores
  the previously committed object. A press/release without movement is a no-op.
- Moving a curved object translates endpoints and its control point together.
  Color, width, pattern, layer order, Delete and Undo/Redo preserve curvature.
  No endpoint-resize handles, multi-control-point curves or keyboard curve
  nudging are added in this increment. Existing text/image/tail gestures remain
  unchanged.

## Rendering And Selection

- Store the pattern on the object; do not alter original image pixels. Use a
  closed rectangle/ellipse path and a continuous freehand path so the dash phase
  does not restart at every point, corner, crop boundary or render tile.
- Use tiny-skia, already pinned in the project. Dashed strokes follow the .NET
  4-width dash / 3-width gap pattern. Dots are round marks spaced 3 widths apart;
  their visible diameter matches stroke width. Do not assume a zero-length
  dash works without verifying the renderer; use a supported representation.
- The arrow shaft uses the selected pattern. Its head remains solid and joins
  the endpoint cleanly, including when the final shaft segment falls in a gap.
- Curved lines/arrows use the same patterns along the continuous curve. Orient
  the solid arrowhead to the endpoint tangent, with a deterministic fallback
  to the start-to-end direction for a zero-length endpoint tangent. Include
  curve extrema, head and stroke padding in selection/dirty/crop bounds. Hit
  testing follows the actual curve and head, not the straight endpoint chord;
  use a bounded geometric approximation rather than unbounded subdivision.
- Share geometry/pattern rules between bounded live feedback and final
  composition. Preview, PNG and clipboard agree at original pixel dimensions.
  Crop clips the result without changing pattern anchoring.
- Outline hit testing uses the outline, not its empty interior. Filled shape
  hit testing includes its interior; ellipse corners outside the ellipse do
  not select it. Pattern gaps may select the underlying logical stroke within
  normal tolerance, so thin/dotted objects remain ergonomic to move.
- Respect existing layer order and blur/highlighter composition. Dirty bounds
  include stroke width and arrowhead. No full long-image allocation per pointer
  move. Keep existing memory/point/history bounds and stale-export protections.
- Reject invalid or over-limit geometry without mutating document/history;
  preserve existing error reporting and cancellation paths.

## Verification And Delivery

- Test first: observe meaningful failing behavior before implementation. Run
  the full unit suite, format checks and release build before delivery.
- Cover defaults/style isolation; drag directions/degenerate shapes;
  selection including ellipse corners and outline interiors; changing style;
  layering, deletion and Undo/Redo; cancellation without a new revision.
- Cover bend-handle priority, midpoint mapping, unchanged drags, reset,
  cancellation, invalid coordinates, translation and history round trips.
  Assert curve extrema, curved hit testing, head orientation/fallback and
  continuous patterns along curves, including nearly horizontal/vertical and
  strongly bent cases. Selection handles must never appear in exported pixels.
- Assert synthetic pixels for fills, ellipse boundaries, dash gaps/round dots,
  continuous phase, arrowheads and crop/tile boundaries at multiple widths.
  Compare live feedback with final output at 100% and scaled views.
- Extend the guarded native fixture to exercise actual new tool buttons and
  pattern menu, EN/TH Help, selection/property changes, curve dragging/reset/
  cancellation, Undo/Redo, PNG and clipboard parity. Native input requires a
  user-agreed input-free interval.
- Check responsive dragging on a long source image and repeat comparable
  launch/selection/preview measurements. Report slow samples and limitations,
  not only medians. Do not claim speed improvements without measurements.
- Retest existing editor, OCR and automatic/manual scrolling, especially the
  Alpha 9 final-tail Stop/Escape, Cancel, timeout and covered-source guards.
  Synthetic fixture success does not establish every real Lark/DPI case.
- After verification, prepare a local portable test candidate, record exact
  version/hash, evidence and tester steps. Choose the version at packaging
  time. User acceptance does not itself authorize push or public release.

## Implementation Order And Deferred Work

Within one candidate: first implement/test shared geometry, shapes and patterns;
then curve data, bounds/rendering/hit testing; then native handle gestures and
Help; finally the full regression/performance checks and portable packaging.
Do not deliver separate feature candidates unless a blocking issue requires
agreeing a scope change with the user. Shadows, text outlines, projects, general
object opacity, recording and broad UI parity remain outside this specification.

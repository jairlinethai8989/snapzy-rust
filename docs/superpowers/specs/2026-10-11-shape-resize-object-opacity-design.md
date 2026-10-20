# Shape Resizing And Object Opacity

Date: 2026-10-11 (Asia/Bangkok).
Status: written specification approved by the user; implementation authorized.
Scope: SnapZy Rust only. No .NET changes, commit, push or public release.
Preserve Alpha 11, Alpha 10 and unrelated working-tree changes.

## Context And Approach

The user confirmed direct bending works and requested resizing outline and
filled shapes after creation, together with the next editor phase. They
approved eight immediate resize handles and general object opacity, excluding
blur. Shadows and text outlines remain a separate later phase.

Recommended approach: retain drawing mode, exposing handles on the newly
selected shape, and use Select to edit older shapes. Automatically switching
to Select would require choosing the drawing tool again for the next shape.
Select-only handles would retain the extra action the user just removed for
line bending. Reuse the existing document/history and rendering pipelines;
do not replace the editor or change the overall visual theme.

## Shape Resize Behavior

- Cover Rectangle, Ellipse, FilledRectangle and FilledEllipse. Completing a
  valid shape selects it and shows four corner handles and four edge-midpoint
  handles immediately, without switching out of its drawing tool.
- In drawing mode, only the current selected shape exposes resize handles.
  Dragging a handle resizes it; dragging elsewhere starts another shape.
  Select can select, move and resize older shapes as before for other objects.
  Changing tools removes direct handles; unfinished drawings have none.
- Corners adjust both axes freely; side handles adjust only their axis. Keep
  the opposite corner or edge fixed. Do not add aspect-ratio locking, Shift
  modifiers, rotation or flipping. Clamp each dimension to at least one source
  pixel if dragged past its fixed edge; reject non-finite/out-of-range geometry
  consistently with existing document limits.
- Preserve object identity, layer order, color, stroke width, stroke pattern
  and opacity. Resizing an ellipse changes its axes, not its tool type.
- Render handle size/hit tolerance consistently in screen pixels across zoom
  and DPI. Display and hit testing share eligibility and crop clipping rules.
  Where handles overlap on small shapes, use deterministic corner-first hit
  priority. Hidden or clipped handles must not capture unrelated drawing.
- Live drag updates a replacement preview, not committed geometry. One changed
  release creates one Undo step. No-op and cancelled drags create none.
  Escape, tool change and capture loss restore the pre-drag shape. Undo/Redo
  follows the existing selection rules and restores exact geometry.
- Do not change existing text/callout wrapping or image aspect-ratio resizing.
  Opacity-zero shapes remain selectable and resizable through geometry.

## Object Opacity Behavior

- Support Pen, Highlighter, Line, Arrow, the four shapes, Text, Callout, Step
  and inserted Image objects. Exclude Blur, the immutable captured background
  and workspace/crop operations. This is not a global canvas transparency.
- Use the existing compact percent control with EN/TH labels and Help. Enable
  it for an eligible selected object or eligible active drawing tool; disable
  it for Blur or Select with no eligible selection. No new floating panel.
- Accept integer percentages from 0 through 100, including fully transparent.
  Invalid, incomplete and out-of-range edits must not mutate the document or
  silently clamp to an unrelated value. Unchanged values add no history.
- New editor defaults: ordinary objects 100%; Highlighter 40%. Preserve normal
  red 5 px and highlighter yellow 24 px, with independent opacity defaults for
  ordinary drawing and highlighting for the editor window's lifetime.
- Selecting an existing object displays its own percentage. A deliberate
  percentage edit changes that object with Undo/Redo. Browsing older objects
  in Select does not overwrite future drawing defaults. Editing the active
  drawing mode's percentage updates its family default for subsequent marks;
  the next mark uses the value displayed by that active mode, including after
  Undo. Imported images start at 100% and are edited after insertion.
- Apply opacity to the complete object as a group: arrow shaft and head;
  number circle and numeral; text and its optional background; callout text,
  frame, fill and tail. Avoid darker overlap artifacts from applying opacity
  independently to overlapping components of one object.
- Preserve source image alpha and multiply it by object opacity. Highlighter
  retains its existing multiply blending and readability, applying its chosen
  percentage exactly once. Ordinary objects use normal alpha compositing.
- At 0%, do not delete or skip the object in selection/history/layer handling.
  At 100%, ordinary output must match the prior renderer. Changing opacity
  does not rerasterize text or alter pixel dimensions, colors or geometry.
- Existing background color enablement remains separate from object opacity.
  Opacity does not make blur suitable for sensitive-data redaction; no new
  privacy guarantee is introduced.

## Implementation Boundaries

- Add a shape-focused resize geometry helper with eight handles; leave the
  six-handle text-frame behavior and image aspect rules intact. Session owns
  the transient resize state and commits through a validated document method.
- Keep one authoritative opacity value per annotation, including highlights;
  remove or adapt highlight-only storage so two values cannot disagree.
  Extend document edits, history snapshots and creation paths consistently.
- Keep normal/highlighter defaults separate from selected-object properties.
  Imported image creation must explicitly retain its approved 100% default.
- Use the shared renderer for committed output and live feedback. Composite
  each translucent object's complete contribution before blending it with
  lower layers; do not allocate a full long-image surface per pointer move.
  Reuse bounded replacement feedback and source tiles for shape dragging,
  including layers above it and blur halos affecting the changed region.
- Native control updates must occur outside mutable application-state borrows
  to preserve the reentrancy fix from Alpha 11. Keep compact layout bounds,
  accessible names, pointer feedback and EN/TH descriptions consistent.

## Verification And Delivery

1. Test first: reproduce absent shape handles and highlight-only opacity in
   focused tests before production changes.
2. Geometry/session tests: all four shapes and eight handles, reversed initial
   draws, tiny shapes, minimum bounds, finite validation, crop/zoom eligibility,
   immediate resize versus new drawing, Select older shapes, cancel/no-op,
   one-step Undo/Redo, and preserved style/identity/layers.
3. Opacity tests: each supported object, 0/40/100 and invalid values, image
   alpha, family defaults, selection isolation, Undo/Redo, drawing after Undo,
   imported-image defaults, multiply highlight and blur exclusion.
4. Pixel tests: exact opaque regression output, transparent no-op pixels,
   group-opacity overlaps, patterned outlines, curved arrowheads, callout and
   number components, clipping/layers, and bounded drag replacement versus
   committed rendering. Check source pixels remain immutable.
5. Guarded native checks: visible eight handles, corner/edge resize immediately
   and via Select, continued drawing, cursor/controls, cancellation/history,
   opacity editing for shapes/text/images, Help EN/TH and PNG/clipboard parity.
   Obtain a fresh input-free interval before desktop-input tests, split runs
   within the agreed time and report partial failures without claiming a pass.
6. Recheck existing pen/highlighter, direct bending, grouped menus, text and
   callout resizing, imported images/layers, OCR-window behavior and synthetic
   scrolling Stop/Escape. Real Lark and mixed-DPI checks remain explicit user
   validation unless actually exercised.
7. Run the full unit suite, formatting, release build and package verification.
   Prepare a distinct local portable candidate after verification; retain old
   candidates. No public release, commit or push before user authorization.

## Deferred

Shadows, text outlines, rich text, rotation, aspect constraints, line endpoint
resizing, blur opacity, project-file compatibility, video, layer-list UI and
the overall .NET-style interface redesign are outside this increment.

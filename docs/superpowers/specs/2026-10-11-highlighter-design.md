# Native Highlighter Increment

Date: 2026-10-11 (Asia/Bangkok)
Status: written specification approved by the user on 2026-10-11.
Scope: SnapZy Rust only. Keep both .NET applications unchanged. No Git push,
public release, installer or broad UI redesign in this increment.

## Context And Approaches

The tester confirmed Alpha 7 can capture Lark and stop with a physical Escape
key. The approved roadmap now prioritizes reliability, missing editor tools,
then repeatable performance checks before visual parity with .NET.

Three approaches were considered:

1. Add a dedicated highlighter, verify it, then add geometry/stroke patterns
   and curves in separate increments. Recommended: each test ZIP has one clear
   new behavior and performance changes can be attributed.
2. Add all missing drawing tools together. This reaches more features in one
   build but couples geometry, blending, hit testing and gesture changes.
3. Redesign the editor before adding tools. Deferred by the user's explicit
   functional-parity-first direction.

## Interaction

- Add a highlighter icon next to Pen with localized EN/TH tooltip and Help.
  Follow the existing native icon/control system; no WebView or new framework.
- Drag to create one freehand highlight object. A click without a stroke creates
  nothing. Escape, capture loss or changing tool cancels an unfinished stroke.
- Match the existing .NET defaults: yellow `#FFF258`, width 24 source pixels,
  opacity 40%. Width uses the existing 1-40 px control. Add a compact opacity
  control from 0-100%, enabled for a highlighter or a selected highlight only.
- Regular annotation defaults stay red and 5 px. Remember highlight color,
  width and opacity independently within each editor session so switching tools
  cannot silently turn the pen into a wide yellow line. Reopening the app starts
  from the documented defaults; settings migration is out of scope.
- Select a highlight to move it, change its color/width/opacity, delete it or
  use the existing four layer commands. The selected object's values populate
  the controls. Selecting it does not overwrite the remembered normal-pen style.
- One completed stroke or property adjustment is one history edit. Undo/Redo
  restores geometry, color, width, opacity and layer order. A cancelled or
  unchanged operation adds no history entry.

## Rendering And Data

- Store shared immutable stroke points and explicit highlighter opacity on the
  annotation, without modifying the original captured or imported image bytes.
- Multiply-blend the highlight with pixels below its current layer, matching
  the .NET editor's highlighter. Black text remains black rather than being
  covered with pale paint. A stroke overlaps itself as one object; two separate
  strokes may darken their shared region, as in the .NET composition.
- Honor the existing crop and image-layer order. Blur above a highlight filters
  it; a highlight above blur blends with the already blurred composition.
- Live feedback and final export must use the same color/opacity and geometry.
  Render only the visible dirty region during dragging. Do not flatten or
  allocate an entire long screenshot for each pointer movement, and do not
  rerender source pixels simply because the pointer has not moved.
- Reuse the pinned tiny-skia renderer and current background composition worker.
  Keep Copy/Save disabled while the current revision is composing; never export
  an older revision as though it were the latest edit.
- Existing limits on points, document history and pixel buffers remain intact.
  Report a rejected operation without mutating the document or saved state.

## Verification And Acceptance

Use test-first implementation for each behavior. Record a meaningful failing
assertion before implementing the behavior, then run the full unit suite.

- Default/tool switching and selection do not leak highlight style into Pen.
- Stroke creation, cancellation, moving, style changes, deletion and ordering
  round-trip through Undo/Redo without copying original image pixels.
- Exact synthetic pixels show readable black text, correct multiply blending
  over white/colored/translucent content, self-overlap behavior and layering.
- Native fixture exercises the real toolbar and controls, Help/EN/TH, dragging,
  cancellation, selection and exported PNG/clipboard parity at original size.
- Check live feedback against finished output, no full-image per-move allocation,
  unchanged-pointer behavior and responsive controls on a long source image.
- Repeat launcher/selection/preview measurements under the same fixture as the
  Alpha 7 baseline. Report slow samples as well as median values. Instrumented
  first-paint intervals and test-observer timings must remain separate.
- Retest scrolling (Auto/Manual, frozen header/footer, Pause/Resume/Stop/Escape,
  Cancel, focus/cover safety, detector timeout recovery) before delivering a ZIP.
- User checks physical input and real Lark/chat compatibility separately; native
  fixtures are not a substitute for those acceptance checks.

## Delivery And Deferred Work

Prepare one local portable candidate after tests and performance review. Record
the exact binary hash, evidence paths, known limitations and user test steps.
Choose the next preview version at packaging time; do not bump or publish merely
for this draft. The user approves the candidate before any push.

Next independent specifications cover ellipse/filled shapes and solid/dashed/
dotted strokes, then bendable lines/arrows. General object opacity, shadows,
text outlines, projects, layer list, video and UI parity remain later work.

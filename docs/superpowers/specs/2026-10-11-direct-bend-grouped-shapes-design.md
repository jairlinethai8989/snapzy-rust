# Direct Bending And Grouped Shape Buttons

Date: 2026-10-11 (Asia/Bangkok).
Status: written spec approved by the user; implementation authorized.
Scope: SnapZy Rust only. No .NET changes, commit, push or public release.
Preserve Alpha 10 and unrelated working-tree changes.

## Context And Choice

The user tested Alpha 10 and confirmed line bending works. They requested
the bend handle immediately after drawing, without pressing Select first,
and one popup button for outline shapes plus one for filled shapes.

Recommended and approved: keep Line/Arrow drawing mode active and expose
the newest selected line/arrow's midpoint. This allows bending immediately
and drawing another line by dragging elsewhere. The alternative of switching
automatically to Select was discussed but would require reselecting the
drawing tool before the next line. Keeping separate shape buttons does not
meet the requested grouping.

## Immediate Bend Handle

- Completing a valid Line or Arrow selects that object as before, but now
  shows its midpoint handle and accepts dragging/reset without changing mode.
- A drag starting on the handle bends the selected object; a drag starting
  elsewhere draws another object using the active Line/Arrow tool. Only the
  currently selected line/arrow exposes a handle, not every object at once.
- Do not expose a handle for an unfinished new drawing or for a degenerate
  drawing that creates no object. Switching to another tool removes this
  direct-edit handle. Select still supports older line/arrow objects.
- Preserve fixed endpoints, tangent-oriented arrowheads, current stroke
  pattern, layer order, original-resolution output and bounded live feedback.
- Double-click the handle straightens; Escape/tool change/capture loss cancels
  an unfinished bend. One changed bend/reset is one Undo step; cancelled and
  unchanged actions add no history. Undo/Redo keeps the normal selection rules.
- Share handle eligibility between drawing and hit testing, including crop
  clipping and DPI/zoom-stable handle tolerance. Other tools must not gain
  accidental midpoint editing.

## Grouped Shape Buttons

- Replace four visible shape buttons with two adjacent buttons: Outline
  Shape and Filled Shape. Each has the last chosen shape icon and a dropdown
  indicator. Clicking anywhere on either button opens its two-item menu.
- Outline menu: Rectangle outline / Ellipse outline. Filled menu: Filled
  rectangle / Filled ellipse. Menu entries use shape symbols plus EN/TH labels
  and indicate the current choice. An ellipse with equal axes is a circle;
  no new circle constraint or aspect-ratio modifier is introduced.
- First choice is Rectangle in both groups. Remember outline/filled choices
  independently for each editor window for its lifetime; do not add persisted
  settings. Changing group choice does not alter already drawn objects.
- Choosing an entry activates that drawing tool and updates the button's icon,
  tooltip and active state. Cancelling the menu leaves the previous tool,
  remembered choices, document selection and history unchanged.
- Retain red 5 px / Solid defaults, supported outline stroke patterns and
  disabled Width/Pattern for filled shapes. No new renderer or shape resizing.
- Keep all other toolbar commands in their existing relative order. Update
  compact bounds, accessible labels, tooltips and EN/TH Help. This is a small
  interaction change, not the deferred overall .NET-style visual redesign.

## Verification And Delivery

1. Test first: demonstrate the current missing direct handle and ungrouped
   control behavior before implementing their fixes.
2. Unit checks: direct bend/reset/cancel/history without Select, new-line
   drawing away from the handle, hidden handle during drawing/crop, isolation
   from other tools, independent shape choices and unchanged old objects.
3. Guarded native checks: immediate visible handle, drag/reset without Select,
   actual outline/filled popup selection and cancellation, remembered icons,
   controls/labels/Help EN/TH, compact bounds, PNG/clipboard parity.
4. Repeat existing editor and long-capture Stop/Escape regressions. Native
   desktop-input checks require a newly agreed input-free interval; past
   consent does not authorize a new round.
5. Full unit suite, formatting, release build and exact package/binary checks.
   Prepare a separate local portable candidate after verification. Preserve
   Alpha 10 and report any untested real Lark, hardware or mixed-DPI behavior.

## Deferred

No endpoint resizing, multiple bend handles, object-wide automatic selection,
Shift-to-circle behavior, persistent shape preference, video or .NET changes.

# Escape During Scrolling Capture

User approved: Escape during active scrolling is Stop (preserve accepted pixels),
while Escape during region detection remains Cancel. No broad UI redesign or
change to Lark settling/stitching thresholds. No Git push before tester approval.

## Root Cause

The scrolling controller is intentionally non-activating, leaving keyboard focus
on Lark. Alpha 6 registers Escape only for selection and unregisters it before
capture. The controller's local key handler cannot receive Lark's keys and also
calls Cancel, contrary to the newly clarified Stop semantics.

## Implementation

- Register Escape with MOD_NOREPEAT only on the active controller, alongside an
  asynchronous key-state fallback on its existing polling timer if registration
  is unavailable. Do not require source focus or reactivate the controller.
- Route global Escape and controller/child local Escape to the same Stop action.
  Selection Escape still cancels. Explicit Cancel still discards the session.
- Unregister Escape on every controller teardown, including Stop and Cancel.
- Retain existing worker stop checks so no further scrolling is intentionally
  dispatched after Stop. Pending valid work may finish before the editor opens.
- If there is no stable initial frame, report that rather than fabricate an image.

## Verification

RED/GREEN native fixture: active controller reserves Escape; source retains focus;
global hotkey delivery stops and exports exact accepted manual pixels; registration
is released afterwards. Local Escape from a controller child stops a paused
automatic session and keeps a partial image. Cancel still produces no late image.
Run full unit and manual/automatic/stall regressions. Physical keyboard delivery
in Lark remains a tester check unless explicitly exercised.

## Investigation Status

Tester subsequently confirmed Lark scrolling works. Live read-only observation
also saw accepted height grow past two steps (799 to 1870 px), with responsive
controller windows. This does not establish why the earlier attempt stopped;
do not claim a speculative Lark algorithm fix or weaken overlap safety checks.

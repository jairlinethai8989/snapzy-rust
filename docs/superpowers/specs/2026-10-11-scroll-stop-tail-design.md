# Scrolling Stop: Preserve the Pending Final Step

Status: approved by the user on 2026-10-11.
Scope: SnapZy Rust only. No .NET changes, unrelated tools, push or public release.

## Evidence

The user supplied two images on 2026-10-11: the live Lark viewport shows
content below the row labeled 22 in the stitched image, which ends at row 22.
This identifies missing trailing content, not evidence of an internal skipped
row. Input mode is automatic scrolling.

In `scroll_job::run`, successful stitching can immediately request the next
wheel. The next iterations wait for the step delay and a stable frame. However,
control value 1 (Stop/Escape) breaks before sampling or accepting that frame.
`Stitcher::finish` consequently contains only previously accepted frames.
This code path explains the observed one-step lag. A delayed-render native
regression must reproduce the failure before claiming the runtime cause fixed.

## Alternatives

1. Recommended: stop input immediately and boundedly finish the pending frame.
   Preserves the visible final step without altering normal capture cadence.
   Stop-to-preview can take up to the finishing deadline plus validation/export.
2. Shorten all scroll delays: may narrow the gap but does not remove this race
   and can accept intermediate Lark rendering. Not selected.
3. Keep accepted frames only: immediate Stop, but retains the reported missing
   tail. Not selected.

## Behavior

- Stop and Escape permanently disable further wheel requests for this job.
- Finish the already-issued step using fresh, stable snapshots and the existing
  overlap/footer/memory validation. Do not append an unvalidated screenshot.
- Allow up to 2 seconds from the first stop observation for final sampling.
  Repeated Escape/Stop does not restart the deadline.
- Respect the existing post-wheel settling delay. A stable but unchanged old
  frame must not finish an outstanding step before delayed rendering arrives.
- Show localized EN/TH finishing status; the UI remains responsive.
- Do not move/activate the source or send input while finishing. Validate the
  target identity, bounds, DPI and lack of occlusion before sampling and before
  accepting. A Stop-button click must not disqualify an otherwise safe source.
- If content cannot settle/align safely, the target changes, capture fails or a
  limit is reached, retain the valid partial result and explain why the final
  step was not confirmed. Do not silently call the partial result complete.
- Cancel still discards the job immediately, including during finishing, with
  no late preview. Paused jobs may finish safe pending content without resuming
  wheel input. No-new-content automatic completion remains unchanged.
- Preserve original pixels, frozen headers/footers and PNG/clipboard parity.

## Verification

First add a native regression against Alpha 8: stop after the source receives a
wheel but before its 900 ms delayed render is accepted. Assert exact final
height and all source pixels, not only a lower-bound partial height. Record RED.
After the fix require GREEN for both Stop and Escape, no extra wheels,
unchanged-frame deduplication, repeated Stop, cancellation during finishing,
timeout/unstable content and invalid/covered source partial retention.
Run the full unit suite, existing scrolling/Escape safety regression, editor
checks, formatting and release build. Desktop input tests require an input-free
window and retain all foreground/ownership guards.

Package a separate local preview only after verification. Ask the user to repeat
the same Lark capture and compare the bottom visible row with the exported
image. Native synthetic fixtures are not live Lark acceptance.

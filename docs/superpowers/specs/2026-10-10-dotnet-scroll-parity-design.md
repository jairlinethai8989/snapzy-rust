# Scrolling behavior parity with Neo Snap 0.1.20

Approved by the user: implement functional parity first, visual UI parity later.
Supersedes click-to-detect-only selection in the earlier scrolling spec.
Scope: Rust scrolling capture only; do not change either .NET product or push.

## Interaction

- Moving the pointer over a target triggers a debounced background probe.
- Keep the overlay visible and non-activating throughout detection; do not hide
  and restore it on each probe. Never scroll during detection.
- Show a colored outline when a candidate is ready. A click inside that outline
  starts the selected Auto/Manual capture on mouse release, without a second Start.
- Pointer movement invalidates pending generations; stale results must not
  replace another target. A pressed candidate must still match on release.
- Escape cancels both selection and pending provider work even without focus.
- Keep existing buttons as alternatives for now; do not redesign the editor/UI.

## Detection

- Port the .NET sheet viewport algorithm: persistent vertical borders plus a
  broad colored header, applied to the original screenshot within the target
  client bounds. No network, image scaling or exploratory wheel input.
- Resolve native scrollbar descendants from the target HWND, not the overlay.
- Query UIA from the target HWND, not ElementFromPoint on an overlay. Prefer
  scrollable descendants; fall back to a Document, then the visible window
  client, matching .NET. Explicitly label the window fallback as a window frame,
  not verified scrollability. The user reviews and confirms the visible frame.
- Cache successful sheet bounds per selection/window; preserve original pixels.
- Keep the isolated bounded helper process and kill/reap lifecycle. Cancellation
  and a newer hover supersede obsolete work without accumulating helpers.
- Revalidate bounds/process/DPI/occlusion on confirmation and during capture.

## Verification

- Regression tests for sheet detection/rejection, clipping/negative origins,
  hover generation replacement, cancellation and release-point confirmation.
- Native fixture: detection starts from pointer movement, outline becomes ready
  without clicking, overlay never hides, click-release starts exactly one job.
- Run manual and automatic capture regressions, Escape, stale-target handling,
  exact frozen-header/footer stitching and export/clipboard parity.
- Actual Lark compatibility and physical mouse/keyboard delivery remain tester
  acceptance checks, not claims inferred from native-message tests.

## Delivery

Local Alpha 6 portable candidate, documented with verification and limitations.
No Git push until tester approval. Broad .NET UI parity is a later phase.

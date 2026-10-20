# Implementation plan: click-to-detect scrolling capture

Approved design: `../specs/2026-10-10-scroll-pane-detection-design.md`.
Implementation was authorized on 2026-10-10. Preserve the current
Automatic/Manual stitcher and all unrelated working-tree changes.

## 1. Add detection-domain tests first

- Define a small platform-independent candidate/state model for clicked point,
  scrollable bounds, target top-level window identity, process, DPI and detection
  generation. Keep Win32/UIA handles out of pure decision logic.
- Add tests for nearest eligible ancestor selection, vertical-only filtering,
  invalid/empty/clipped bounds, retry replacing a candidate, late-generation
  results, cancel invalidation, and confirmation starting at most one job.
- Verified the tests failed before implementing the state transitions.

## 2. Implement bounded Windows detection

- Verify the Windows bindings and COM lifetime/threading requirements for UI
  Automation in this repository before choosing direct COM FFI or an additional
  dependency. Keep the choice contained in a new `scroll_detect` module.
- On a dedicated COM-initialized worker, resolve the UIA element at the click
  point, walk its parent chain nearest-first and accept the first current
  ScrollPattern that reports vertical scrolling and usable bounds.
- Add the narrowly scoped Win32 scrollbar-range fallback. Do not use pixel
  heuristics or send exploratory wheel input.
- Bound UI-facing waits, permit at most one provider query worker, and use a
  generation token so late results after retry/cancel are discarded. Release
  COM interfaces on the owning worker and do not block the main UI thread.
- Validate candidate bounds against target-window client bounds, desktop and
  current visibility. Record root HWND/process/DPI for later revalidation.

## 3. Add target-preview overlay states

- Replace the Scroll mode's drag rectangle gesture with a single target click;
  keep normal Area and Window selection behavior unchanged.
- Add explicit states for awaiting click, detecting, candidate preview, no-match
  retry and cancelled. A new click invalidates the previous candidate.
- Highlight the candidate viewport and show the target title, Start long
  capture and Cancel. Clicking elsewhere starts a new detection. Escape cancels.
- Confirmation atomically consumes the current valid candidate and calls the
  existing `start_scroll` exactly once. Nothing scrolls before confirmation.
- Localize labels/status in English and Thai and ensure the overlay never
  appears in captured pixels.

## 4. Preserve scrolling safety and lifecycle

- Pass the detected viewport as the capture region and its top-level candidate
  as the existing scroll target; retain current automatic/manual setting.
- Recheck window/process identity, DPI, visibility, foreground requirements,
  occlusion and geometry at confirmation and during the existing job.
- If the candidate becomes stale before confirmation, return to click-to-detect
  with a clear message; never reuse old coordinates.
- Keep Pause/Resume/Stop/Cancel, stability, stitching, budget and partial-result
  behavior unchanged. Ensure closing the overlay/editor cancels detection
  delivery and does not leave a late preview or worker per click.

## 5. Extend native regression coverage

- Add a native test fixture containing nested scrollable panes and assert a
  click selects the nearest vertically scrollable pane, not its outer window.
- Verify native scrollbar fallback, unsupported/no-provider retry, repeated
  click, Escape, stale window, DPI/bounds changes, and no scroll before confirm.
- Verify confirmation starts exactly one job in Automatic and Manual modes.
- Run all existing capture/editor/scroll regressions, including focus loss,
  occlusion, held-input pause, target movement, Stop, Cancel and pixel-exact
  stitching checks.
- Test actual Lark Sheets and Excel manually. If Lark does not expose usable UIA
  or native scrollbar information, stop before packaging and report the gap;
  do not claim support based only on the synthetic fixture.

## 6. Package only after verification

- Update bilingual help and tester instructions for point detection, confirming
  the highlighted bounds, retry by clicking elsewhere and Escape cancellation.
- Increment the Rust preview alpha version only after a successful build and
  test pass; package an isolated local ZIP with required runtime/license files.
- Have the user test Lark Sheets and Excel. Do not push until explicit approval.

## Acceptance checks

- Click once within a vertically scrolling pane; its bounds are highlighted.
- Click **Start long capture**; the existing selected Auto/Manual mode begins
  once, with no earlier scroll movement.
- Click another pane; only the new point's candidate can be accepted.
- Unsupported point shows a retry message; another click works, Escape cancels;
  no drag-selection fallback is present.
- Existing scrolling/capture/editor regression suite remains green.
- No stuck UI, stale candidate, unbounded detection threads or stale completion
  after cancellation.
- Manual Lark and Excel tester checks pass before packaging is offered.

## Local Verification

- 120 Rust unit tests pass, including detector timeout, cancellation and
  kill-on-close helper lifecycle checks.
- Release x64 build succeeds.
- The initial tester reported Lark capture still relied on the drag path and
  Escape did not cancel the detector.
- A deterministic smoke delay reproduces the UIA progress handoff. The native
  fallback now caches the clicked HWND chain before the overlay is shown again;
  native `ScrollChecks` pass with the 350 ms handoff delay enabled.
- The smoke test now presses Escape during provider work and after candidate
  preview. Both cancel without opening the scroll controller.
- Alpha 5 native-message `ScrollChecks` pass, including confirmation, manual scroll,
  target movement, Stop/Cancel, pixel-exact stitching and clipboard parity.
- Automatic smoke sent test-owned wheel input and passed initial Pause/Resume,
  but its final no-motion dialog check was blocked after the safety guard
  detected another topmost window covering the fixture. Automatic mode needs
  hands-on validation for this candidate.
- Native fixture exercises the validated Win32 scrollbar fallback. UIA and
  actual Lark Sheets/Excel provider behavior still need user validation.
- Alpha 4 failed the new held-mouse regression: detection hid the overlay on
  button-down, before button-up. Alpha 5 defers detection until button-up.
- Native-message stall injection passes: two unresponsive helpers are killed,
  retry works, invalid confirmation stays disabled and Escape cancels.
- Physical mouse/keyboard delivery and actual Lark detection are not validated
  by these native-message smoke tests. See `docs/testing/0.9.0-alpha.5.md`.
- The local portable candidate is 0.9.0-alpha.5; no Git push until tester
  approval.

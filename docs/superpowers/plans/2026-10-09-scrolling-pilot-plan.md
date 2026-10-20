# Scrolling Pilot Implementation

Approved design: ../specs/2026-10-09-scrolling-capture-proposal.md.
The writing-plans skill is not installed; use this repository's plan format.

1. [x] Preserve alpha.2 executable. Write failing synthetic seam, stationary
   chrome, late-footer, ambiguous movement and bounded-memory tests.
2. [x] Implement pure pixel stitcher and settled-frame state machine, isolated
   from UI. Use public MIT SnapZy implementation as reference; add provenance.
3. [x] Add region selection mode, compact outside-region controller and bounded
   background capture loop with Stop/Cancel, target checks and stale-session guards.
4. [x] Extend native scrolling fixture and PNG/clipboard assertions. Run old
   editor/resize/paint suite and alternating ordinary capture benchmarks.
5. [x] Package portable tester only after verification; document limits and real
   Lark/high-DPI checks still required. No push/tag/release or .NET changes.

## Separate Text/Callout Design Tasks

- [x] Explore existing text, resize, toolbar and renderer ownership.
- [x] Choose shared contextual controls over dialogs or rich-text formatting.
- [x] Clarify transparent defaults and tail dragging; conceptual approval received.
- [x] Write design and review scope/ambiguities.
- [x] Receive written spec review before implementing this independent subsystem.
- [x] Prepare its test-first implementation plan in
  `2026-10-09-text-background-callout-plan.md`.
- [ ] Implement, verify and package the separate text/callout increment.

## Earlier Verification / Blocker

2026-10-09: all 66 unit tests pass, including ten scrolling cases. Initial stub
failed the join, footer, reverse-motion and budget cases before implementation;
the settled-frame stub also failed before implementation. Dynamic footer pixels
initially depressed a correct shift's score to 0.826; reserving the footer band
from motion evidence fixed that case without lowering acceptance thresholds.

The native ScrollChecks fixture is written but has not reached its assertions:
the preserved alpha.2 test process exited at the single-instance guard because
the user's existing SnapZy-Rust.exe (PID 10524 at inspection) is running from
dist. It was not killed or closed. User asked to save work and exit it via Tray.
No native scrolling UI or capture worker is integrated yet; no updated executable
or ZIP has been built, and no runtime feature-completion claim is made.

## Resumed Verification

The user confirmed the old app was closed. Native RED against alpha.2 then
failed at absent control 109. The region/controller/worker integration is now
implemented and the full native suite passed with all five switches against
0.4.0-alpha.1: `artifacts/smoke-138da63e4f7641e49c44b1c217a115af`.
All 69 unit tests passed; Release and Clippy (four existing allowances) passed.

Two alternating five-sample performance rounds are retained in
`docs/testing/0.4.0-alpha.1.md`, including the first round's 650 ms preview
outlier. The second round did not reproduce it; no general speed claim is made.
The extracted portable package passed the full five-switch native suite:
`artifacts/smoke-d490eb1330fb434d9168d0021ab1a95b`. Its executable hash, file
count, license checks and remaining manual cases are recorded in the tester
document. The local scrolling tester is ready for user validation; no claim is
made that real Lark/high-DPI acceptance is complete. No implementation commit,
push, tag or release yet.
The user approved the independent text/callout written spec after testing the
scrolling build, but reported that the target does not scroll. Diagnose manual
wheel handoff versus expected automatic scrolling before starting text/callout
implementation. The existing alpha.2 ZIP and baseline are untouched.

## Manual Input Bugfix Candidate

The tester clarified that even manually rotating the wheel does not scroll.
A reproducible alpha.1 regression allowed the controller to steal activation.
The alpha.2 candidate restores source foreground before destroying the overlay
and makes the controller non-activating. No synthetic input is sent by the
product to source applications and no matching thresholds were weakened.

69 unit tests and the extracted alpha.2 full five-switch native suite passed:
`artifacts/smoke-80474e0ce2de47b798bf8e5d3630ac3b/results.json`. The native
suite checks guarded launcher/controller clicks and direct source wheel
messages, not physical wheel routing. An optional OS-wheel probe failed before
capture and remains unresolved. Real Lark validation is still required; this is
a diagnostic tester candidate, not a completed Lark acceptance claim.
See `docs/testing/0.4.0-alpha.2.md` for failures, provenance, timings and steps.
The independent approved text/callout implementation remains pending so its
changes do not obscure this scrolling-input validation. No push/tag/release.

2026-10-09: user confirmed manual scrolling works with the candidate. Begin
the already-approved independent text/background/callout increment. Keep the
alpha.2 portable candidate available and retain remaining manual coverage limits.

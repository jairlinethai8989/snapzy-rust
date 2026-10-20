# Native Highlighter Implementation Plan

Approved design: `../specs/2026-10-11-highlighter-design.md`.
Scope: one native highlighter increment, no .NET changes or Git push.

1. Record Alpha 7 regression/performance baseline (completed; see
   `../../testing/2026-10-11-stability-baseline.md`).
2. RED: exercise the highlighter toolbar route through a real drawing session;
   require independent yellow/24 px defaults, a committed stroke and normal Pen
   defaults after switching. Record the observed failure before production edits.
3. Add highlighter annotation content/opacity and shared stroke points. Test
   selected style/opacity edits, range rejection, hit testing and history.
4. Test-first multiply rendering and bounded live feedback against opaque and
   translucent lower layers. Keep crop/layer/blur ordering and original pixels.
5. Wire the native icon, localized Help/tooltip and compact opacity control;
   route changes through the session and existing composition revision guard.
6. Native regression: actual controls/gestures, draft versus final pixels,
   selection, cancellation, independent Pen defaults, history, PNG/clipboard.
7. Full unit/build/format/native regressions and repeated comparable timings.
   Report all failures/outliers; hardware/Lark acceptance remains explicit.
8. Update README/testing notes; package a separate clean portable candidate,
   verify version/hash/content, then ask the user to test before any push.

## Progress

Steps 1-6 implemented and verified. Step 7: full 139-test unit suite, release
build, formatting, staged Highlighter/Editor/OCR native checks and three
comparable timing runs passed. Separate staged full-editor regression and
guarded OS-input scrolling/Escape rechecks also passed. Earlier failures remain
in the report; no safety guard was bypassed. Step 8 updates documentation and
packages a local portable candidate with release/stage/archive hash checks.
Real Lark, physical keys and mixed-DPI acceptance remain tester checks.
No public release or Git push approved.

Evidence and remaining acceptance: `../../testing/0.9.0-alpha.8.md`.

# Basic Editor Implementation Plan

Approved spec: ../specs/2026-10-09-basic-editor-design.md

1. Test document operations first: validated annotation geometry, crop viewport,
   hit-testing, movement, selected styling, bounded history, redo branching and
   revision-aware export state. Implement a UI-independent document module.
2. Test source-resolution drawing and flattened clipboard/PNG output first.
   Use tiny-skia for antialiased annotation paths; keep native UI and capture.
   Preserve original BGRA bytes outside painted coverage. Cache composition.
3. Integrate icon toolbar, tooltips, color/size controls, shared coordinate
   transform, mouse capture/cancellation, crop confirmation, keyboard commands,
   EN/TH and unsaved-state handling into native preview windows.
4. Extend real native smoke checks for editor gestures and exports, retain all
   capture/hotkey checks, inspect rendered UI at compact size and multiple DPIs.
5. Compare accepted executable and new build with repeated identical smoke
   scenarios; report measurements without claiming completed-paint timing from
   window detection. Update version/docs/notices; create portable tester ZIP.

The writing-plans skill is unavailable locally. This plan applies its requested
planning step directly. No implementation push or public release before tester
approval.

# Text Frame Resize Implementation

Approved spec: ../specs/2026-10-09-text-frame-resize-design.md.
The writing-plans skill is not installed; this file records the implementation
sequence using the repository's existing plan format.

1. Preserve the accepted 0.3.0-alpha.1 executable. Test wrapped native glyph
   layout, fixed width, long-word expansion and text editing/history before
   implementing optional source-pixel wrap width.
2. Test six resize handles, anchors, side/corner proposals, limits, cancellation
   and one history edit per drag. Integrate gestures with current Session.
3. Extend native overlay/cursors and existing capture-loss handling, preserving
   buffered paint, capture speed and source-resolution exports.
4. Extend native smoke checks for real handle dragging, wrapped text, font scaling,
   undo/cancel and PNG/clipboard parity. Run all unit/lint/Release checks and
   alternating baseline timings; inspect Thai/English and compact layout.
5. Update version to 0.3.0-alpha.2, docs and tester checklist, package portable
   ZIP and test its extracted executable. No implementation push/tag/release.
6. Prepare a separate proposed scrolling-capture spec after resize verification,
   including Lark frozen panes and stable/seam-safe frames; no implementation of
   that independent subsystem without its design review.

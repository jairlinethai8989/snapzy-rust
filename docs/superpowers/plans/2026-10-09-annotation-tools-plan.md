# Annotation Milestone Implementation

Approved specification: ../specs/2026-10-09-annotation-tools-design.md

1. Preserve accepted alpha.2 binary. Test typed text/step/blur document history,
   bounds, parameter validation and numbering before implementation.
2. Test native grayscale glyph masks and ordered Gaussian blur composition.
   Add lazy native text rasterization and a pinned filter dependency.
3. Test composition generation guards and bounded scheduling. Integrate worker
   results with current-revision export gating, leaving capture's fast path intact.
4. Extend compact native tools, multiline text dialog, selection, parameters,
   double-click editing, translated tooltips and Help.
5. Extend native smoke checks, retain paint regression tests, inspect Thai glyphs,
   run unit/lint/Release verification and compare accepted baseline performance.
6. Update version/docs/licenses and package a portable tester ZIP. Test the actual
   extracted package. No push, tag or release until delivery is approved.

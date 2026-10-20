# Shapes, Patterns And Curves Implementation Plan

Approved scope: ../specs/2026-10-11-shapes-stroke-patterns-design.md.
SnapZy Rust only; preserve the dirty workspace and Alpha 9 artifacts.
No commit, push or public release. The writing-plans skill is not installed;
this plan follows the existing local plan structure.

1. RED/GREEN model and geometry: ellipse/filled tool validation, pattern
   ownership and defaults, quadratic midpoint/control mapping, extrema,
   arrowhead direction, hit testing, translation and Undo/Redo.
2. RED/GREEN shared tiny-skia paths: fill/outline, continuous dashed/dotted
   paths and solid arrowheads. Pixel assertions cover crop anchoring and
   preview/export agreement. Retain bounded draft allocations.
3. RED/GREEN interaction: handle priority, drag/reset/cancel/no-op, stable
   normal/highlighter defaults. Add native shape controls and pattern menu,
   localized Help, curve handle feedback without retaining the old curve.
4. Extend guarded native fixture for combined geometry/pattern/curve behavior.
   Validate actual controls, history, PNG/clipboard, Help and performance.
   Desktop-input tests require a user-agreed input-free interval.
5. Full unit suite, formatting, release build, editor/OCR and scroll regression
   including Alpha 9 tail handling. Record failures and scope limitations.
6. Prepare one fresh local portable candidate only after verification; record
   version/hash, evidence and tester instructions. Do not publish.

Progress: steps 1-5 complete. 154 unit tests pass; corrected release build,
formatting and PowerShell syntax pass. Native RED exposed an ignored-ID
dispatcher bug, fixed through the declared control list and a unit guard.
Geometry rerun passed on the staged EXE (smoke-96c092032d0346639444ff7d9dbf1da4).
Broad old-tool sections passed before a stale tooltip-count fixture failure;
corrected expected count and extended bounds coverage, then remaining Editor/
OCR/scroll checks passed (smoke-8de902293c664f48ac9f0ff6e3d7d5f9). The bounded
native interval ended and desktop control was returned. Real Lark/mixed-DPI/
hardware input and user acceptance remain tester checks, not claimed passes.
Step 6 complete: fresh local Alpha 10 portable ZIP verified file-by-file
(135 files, one EXE, 4,400,468 bytes), with tested release/staged/archive EXE
hash parity. Packaging evidence: artifacts/package-alpha10-8e6fc202219f434db3447c990358e620/
verification.json. Alpha 9 ZIP hash is unchanged. Final pre-package full suite:
154 passed (14.19s). See testing record for failures and timing limitations.
Delivery is for user testing only; no push/public release or user acceptance.

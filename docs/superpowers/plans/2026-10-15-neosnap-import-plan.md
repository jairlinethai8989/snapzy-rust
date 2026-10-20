# One-Way .neosnap Import Implementation

Approved design: ../specs/2026-10-15-neosnap-import-design.md.
Rust only, preserve dirty worktree and accepted packages; no commit/push/.NET.
The writing-plans skill is not installed; this local checklist records the plan.

- [x] RED/GREEN bounded JSON, duplicate/depth checking, typed v1 validation.
- [x] RED/GREEN PNG assets, source/canvas budgets and strict native mapping.
- [x] RED/GREEN all-or-nothing diagnostics including unsupported hidden objects.
- [x] RED/GREEN image crop, ownership, layer-order preservation and round-trip.
- [x] Preserve .NET arrows, dots and smoothed pen/highlighter in schema 3.
- [x] RED/GREEN Open routing and imported-unsaved/path-independent state.
- [x] Integrate EN/TH diagnostics, source label, existing tab/window lifecycle.
- [x] Update version/docs/notices; formatting, full locked suite, optimized build.
- [x] Package a separate verified local candidate.
- [x] Run native comparison and record user acceptance on 2026-10-15.

Any new compatibility discovered during implementation must fail closed rather
than clamp/drop/reorder or flatten. Native focus/clipboard restrictions remain.

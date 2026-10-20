# Editable Image Combination: Implementation

Approved design: ../specs/2026-10-12-editable-image-combine-design.md
Rust only; preserve Alpha 14-r2. No commit, push, installer or .NET changes.
The writing-plans skill is unavailable; this is the local implementation plan.

## Sequence

- [x] Test and implement checked layouts, source snapshots and ownership identities.
- [x] Test local transforms, nested selection/editing, sibling ordering and history.
- [x] Test per-image crop, uniform resize and cancellation without source changes.
- [x] Integrate bounded group rendering; compare identity pixels and filter isolation.
- [x] Add schema-2 persistence with strict schema-1 compatibility and hostile-input tests.
- [x] Add native Combine dialog, parent selection, worker reservations and EN/TH Help.
- [x] Run full tests, format check and warning-free release build; record timings.
- [x] Request fresh input-free consent for native integration verification; first interval completed with partial evidence in test notes.
- [x] Package a separate Alpha 15 candidate with accurate notes and hashes.
- [x] Record user-reported core workflow checks: combine, PNG save, clipboard copy, OCR, scrolling capture and Esc stop.
- [ ] Pass native integration checks before offering the candidate for acceptance.

## Verification

Use test-first increments. Record observed failures and passing commands, not
source-text assertions. Keep native verification gaps explicit. Do not mark the
candidate ready until all model, renderer, persistence and native integration
parts are complete. Stage 2B tabs requires later acceptance/design.

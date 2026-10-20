# Multi-image implementation plan

The writing-plans skill is not installed; use this repository plan. Approval
covers the specification; do not commit or push without a separate request.

1. Observe failing tests for image insertion, memory/history, canvas geometry,
   aspect resizing and raster composition. Extend the existing document model.
2. Add bounded file/clipboard decoding with malformed/oversized fixtures.
3. Integrate image import worker, compact commands, canvas dialog, selection and
   corner gestures. Keep source sharing and composition identity guards.
4. Extend native smoke tests, observe missing-feature failures, then exercise
   real file import, clipboard paste, canvas dimensions and actual output pixels.
5. Run full verification and basic baseline comparison. Update bilingual docs,
   licenses and version. Package and verify the extracted portable candidate.

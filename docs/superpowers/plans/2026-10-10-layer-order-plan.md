# Annotation ordering implementation plan

Approved scope: ../specs/2026-10-10-layer-order-design.md.
The writing-plans skill is not installed; this local plan follows existing plans.
Do not commit/push or change the .NET apps. Retain alpha6 artifacts.

1. Add failing Document tests for four stack operations, no-op/invalid selection,
   object identity/sharing, crop, history/dirty state and topmost selection.
   Add failing renderer tests for overlap and ordered blur.
2. Implement a small Order enum, availability and one-edit reorder using existing
   snapshots. Run the full unit suite.
3. Add failing native preflight for four toolbar controls. Add native checks for
   exported overlap pixels, Undo/Redo, boundary states, gesture cancellation,
   EN/TH tooltips/Help, compact non-overlapping layout and clipboard parity.
4. Add four icons through the existing Segoe MDL2 system; compact the 17-button
   toolbar spacing within the existing minimum window. Wire commands through
   session cancellation, update_editor and revision-guarded composition.
5. Version local tester candidate to 0.7.0-alpha.1 consistently. Run formatting,
   full unit suite, qualified Clippy, release build and full native regression.
   Inspect desktop/compact screenshots. Measure alpha6 vs candidate using the
   existing alternating three-sample comparison, without overstating conclusions.
6. Update English/Thai README and tester evidence/checklist. Build a separate ZIP
   with notices/licenses/docs and verify all extracted files against staging.
   Deliver the local candidate for tester approval, with remaining real-app/DPI
   checks explicit. No push.

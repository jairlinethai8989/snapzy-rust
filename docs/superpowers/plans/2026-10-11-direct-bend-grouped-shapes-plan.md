# Direct Bending And Grouped Shapes Implementation

Approved scope: Rust only, no commit or push. Preserve Alpha 10.
The writing-plans skill is not installed; this follows the existing local plan format.

1. Add failing tests for immediate line/arrow handle access and two shape buttons.
2. Share handle eligibility between painting and input. Cover cancellation,
   continued drawing, crop boundaries, undo/redo, and unfinished gestures.
3. Remember outline and filled variants independently in each editor session.
   Implement checked native popups, remembered icons, dropdown markers,
   accessible labels, tooltips and Help in English and Thai.
4. Update native tests to select grouped shapes through actual popup menus.
   Check cancellation, independent choices, direct bend/reset and old tools.
5. Run formatting, unit tests, release build and script syntax checks. Request
   a fresh input-free interval before running native desktop fixtures.
6. Record results and limitations; deliver a separate local Alpha 11 candidate
   without modifying Alpha 10 or publishing anything.

## Execution Record

- Steps 1-4 complete. Behavioral RED failures observed before implementation;
  161 unit tests pass after the update. Native menu reentrancy failure was found
  and corrected by moving tooltip/text messages outside the STATE borrow.
- Step 5 complete: format, release build, script syntax, unit tests and bounded
  native checks. Geometry/highlighter/images/layers/callout/paint passed in the
  first bounded run; its deadline failure is retained. Remaining annotation,
  resizing, editor, OCR and scrolling checks passed in a separate interval.
- Step 6 complete: results/limitations recorded in docs/testing/0.9.0-alpha.11.md.
  Fresh portable ZIP validated file-by-file against staging (114 files, one EXE,
  two OCR DLLs); source release and both native evidence hashes match the staged
  EXE. Alpha 10 EXE/ZIP hashes are unchanged. Validation evidence:
  artifacts/package-alpha11-a88b62ddddaa460892de1e308366152a/verification.json.
  ZIP SHA256: ABCE601D31901ADCA7E7340F02C99AEB14474F0FB4118B3C17C4EA0F5508F666.
  No commit, push or public release performed.

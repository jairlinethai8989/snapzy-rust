# Editable Project Implementation

Approved spec: ../specs/2026-10-11-editable-project-design.md
Baseline: accepted Alpha 13. Rust only; no commit or push.
The writing-plans skill is unavailable locally; this file is the local plan.

- [x] Establish failing tests for project saved-state and reopened history/IDs.
- [x] Implement editor rehydration and independent project saved-state.
- [x] Establish failing all-object round-trip and hostile-input tests.
- [x] Add bounded ZIP/JSON/PNG project codec with shared assets and glyph masks.
- [x] Add atomic save and failure tests with isolated temporary fixtures.
- [x] Integrate asynchronous Open/Save/Save As, identity guards and close intents.
- [x] Add EN/TH commands, help and privacy/close prompts.
- [x] Run full tests, formatting, release build and bounded save/open timings.
- [x] Prepare distinct Alpha 14 candidate and notes; preserve Alpha 13.
- [ ] Obtain fresh native input-free consent and verify dialogs and regressions.
- [x] Report exact verification and remaining limitations for user acceptance.

Code implemented; native dialogs are partially verified on 2026-10-12:
Open, Unicode Save As, reopen/edit, Save-on-close, single-editor Discard,
Escape/Cancel, Undo/Redo saved markers and EN/TH prompts passed. Multi-editor
exit cancellation and remaining regressions are pending. Opening-status repair
passed regression tests and visible-app success-path verification in Alpha 14-r2.
Save As warning cancellation and clean-editor close also passed. Latest completed
full release unit run: 200 passed, 0 failed. Revised executable release build
and formatting passed. Multi-editor Exit, failure-path dialogs and remaining
native regressions still need independent verification. The user reported testing
passed on 2026-10-12 and accepted Alpha 14-r2; this does not close the listed
agent verification gaps. No commit or push authorized in this increment.
See ../../testing/0.9.0-alpha.14.md for candidate-specific evidence.

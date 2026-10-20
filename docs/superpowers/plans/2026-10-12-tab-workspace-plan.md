# Native Tab Workspace: Implementation

Approved spec: ../specs/2026-10-12-tab-workspace-design.md.
Rust only. Preserve older packages and unrelated dirty work. No commit/push.
The writing-plans skill is not installed; use this local implementation plan.

## Sequence

- [x] RED/GREEN persistent open-mode migration and validation.
- [x] RED/GREEN stable host/document routing, selection and close-set model.
- [x] Integrate native hosts, per-document child editors and compact tab strip.
- [x] Route capture/open/combine results to the captured destination.
- [x] Retain per-document commands, worker guards, dialog and OCR ownership.
- [x] Integrate transactional host/app closing and child/host resource cleanup.
- [x] Update launcher options, EN/TH Help and release notes.
- [x] Run full tests, format and warning-free release build; record actual results.
- [x] Package a separate Alpha 16 candidate, preserving Alpha 15.
- [x] Request fresh consent for native integration checks; record remaining gaps.

Native core execution was performed with fresh consent on 2026-10-13. Default
tabs, independent tools/history, keyboard switching, host Close all cancellation,
inactive-tab closing, New window routing and cancelled Save As were observed
passing. The complete native regression checklist is not finished; see
../../testing/0.9.0-alpha.16.md for evidence and the remaining cases.

A subsequent native round verified project Save, duplicate reopen as a new tab,
editable saved vectors, independent dirty markers, native OCR blank-input
completion and clean/dirty tab cleanup. It also found preview status text
clipping, recorded but not fixed. App Exit across hosts and scrolling-capture
regression checks remain pending; this is not full native acceptance.

On 2026-10-13 the user reported testing passed and approved advancing. Keep the
remaining agent verification gaps and status-clipping finding rather than
marking them passed. Next selected scope: video recording in stages, beginning
with disk-streamed recording and safe finalization; written design review and
approval are still required before implementation. No .NET edits or push.

## Verification

Checked implementation items mean code is integrated, not native acceptance.
Native overflow, advanced async/modal closing, capture restoration and DPI checks
remain pending. Additional native input requires a fresh input-free interval.

Observe failing behavior before each feature increment. Use real model state and
generated files, not source-text assertions. Run the whole suite before claiming
completion. Native tests require fresh input-free consent and agent-created work
only. Do not substitute unit tests for native focus, capture or close verification.

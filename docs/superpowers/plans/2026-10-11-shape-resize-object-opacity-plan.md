# Shape Resize And Object Opacity Implementation Plan

Approved specification: ../specs/2026-10-11-shape-resize-object-opacity-design.md.
Scope: Rust only; preserve existing candidates and dirty changes; no commit/push.
The writing-plans skill is not installed; this local plan follows the existing
repository plan format instead.

## Tasks

- [x] Add failing immediate-shape-resize and ordinary-opacity session tests.
- [x] Implement eight-handle shape geometry and validated one-step history.
- [x] Extend transient drag, overlay, pointer and replacement feedback paths.
- [x] Add failing object/group-opacity rendering tests.
- [x] Unify annotation opacity, document edits and independent session defaults.
- [x] Composite complete objects with opacity; preserve opaque output and highlight multiply.
- [x] Update controls, tooltips and Help EN/TH without overlapping controls.
- [x] Expand regression tests, run full tests/format/build and inspect results.
- [x] Request a fresh input-free interval for bounded native tests.
- [x] Execute and inspect scoped native checks after fresh consent.
- [x] Produce a distinct local candidate and report exact tested limitations.

## Acceptance

Shape resizing is available immediately and through Select, with eight handles,
cancel/no-op safety, unchanged styles and one Undo step. Ordinary opacity uses
0-100%, defaults to 100%, leaves highlighter at 40%, and treats composite objects
as groups. PNG/clipboard share committed renderer. Long-image dragging remains
bounded. Existing tools, OCR controls and Stop/Escape must not regress.

## Evidence

171 unit tests pass, no failures/ignored tests (12.52 seconds in final run).
Format, native test script parser and warning-free release build pass.
Eight-handle tests cover all four shapes, anchoring, cancellation, no-op,
minimum size, invalid coordinates, crop eligibility and older-object selection.
Opacity tests cover ordinary/highlighter defaults, selection isolation, Undo,
all annotation types, grouped components, original image alpha and blur exclusion.
1000 x 600 ellipse feedback regression passes exact replacement/final pixels.

Computer Use initialization failed with "failed to write kernel assets" /
os error 3, as in prior rounds. The existing guarded native fixture CLI is
used as fallback after fresh consent. The native run at
artifacts/smoke-b0ee9774a65a44179654a58ba1d5c90a completed resize checks for all
four shapes and opacity 50%/0% plus history, but FAILED at invisible-shape
selection/opacity. After a harness queue acknowledgement fix, the focused rerun
artifacts/smoke-67daceb4f3dd44318fd525877d460c51 PASSED opacity history,
invisible selection/restoration, exact PNG/clipboard parity and EN/TH layout.
EN text/TH bubble screenshots were inspected. Broad Alpha 12 editor/OCR/scroll
regression suites and real Lark remain unverified for this increment.
No .NET files, prior candidate binaries, commits or remotes were changed.

Local Alpha 12 package integrity passed: 113 files, one executable and two OCR
DLLs, every ZIP entry matches staging, staging executable matches release.
Executable: 1,801,728 bytes; SHA256
3C6B19B15162A009486DB03AC6A5376B50302E63381094ABD51BD8AB52F27FB3.
ZIP: 4,330,462 bytes; SHA256
7E87F59285D6C9E23FB8D029B93B03F56E8161A091D4B3B2BCA0FEC01A56F16F.
Evidence: artifacts/package-alpha12-2e6fb157cc4e46d3bcc7c88e29b08cb7/verification.json.
The original package evidence predates native results. Refreshed staged notes
and ZIP integrity are recorded in the same evidence directory's
verification-refreshed.json. This remains a local candidate, not a public release.

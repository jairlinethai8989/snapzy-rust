# Text Background And Callout Implementation

Design: ../specs/2026-10-09-text-background-callout-design.md.
Written design approved by the user on 2026-10-09. The writing-plans skill is not
installed; use the existing repository task format. Diagnose the reported manual
scroll-input failure first. Do not mix annotation changes into that bugfix build.

1. [x] Add failing document/render tests for transparent defaults, independent
   fill color, fill-off preservation and one-step Undo. Extend existing text
   content/history without copying glyph masks or changing old text defaults.
2. [x] Add pure callout geometry tests for all four tail directions, closest
   edge attachment, degenerate/inside tips, bounds and hit testing. Keep the
   rounded body/tail as one perimeter, with no internal base line.
3. [x] Implement shared antialiased rendering for preview/PNG/clipboard; test
   background extent, outlined transparent bubbles, glyph overlay, crop and
   exact export parity. Keep the current worker/revision/memory guards.
4. [x] Extend existing text resize and interaction tests for bubble movement,
   tail-tip dragging, side wrapping, corner font scaling, opposite anchors,
   one Undo per drag, no-motion, Esc/capture-loss and double-click precedence.
5. [x] Add the bubble tool and contextual background checkbox/color swatch to
   the native compact toolbar. Reuse text entry; initial fill off, saved fill
   color white, initial border/text red, stroke 5 px, font 24 px, padding 8 px.
   Update EN/TH tooltips and Help. Numeric/fill/geometry edits stay independent.
6. [x] Add native creation, Thai/English edit, fill/color, resize/tail/move/cancel,
   Undo/Redo and exact PNG/clipboard checks. Inspect compact layout and keep
   high/mixed-DPI and physical IME testing explicitly outstanding if unavailable.
7. [x] Run all unit/native regression tests and comparable capture benchmarks;
   package a separate portable candidate with licenses, changes and tester steps.
   No .NET changes, installer, push, tag or release without tester approval.

## Foundation Progress (Historical)

2026-10-09: user confirmed manual scrolling works in alpha.2. Independent
text/background/callout model, geometry, renderer and interaction foundation is
now implemented and all 78 unit tests pass. Text masks remain shared through
fill/tail edits, text edits preserve decoration, tail drags take priority over
resize, and wrap/cancel/history paths are covered. Rounded-body padding is
outside the existing text-mask resize bounds. Native handle presentation remains
pending UI integration and review.

The first crop-parity test failed on 32 byte channels because rendering a curve
with a crop-relative clip/translation changed edge coverage. Source-anchored
256-pixel tiles with stroke halos now preserve exact cropped-frame pixels while
bounding temporary allocation. This test passes without relaxing equality.

Tasks 3-4 have foundation code/tests but remain unchecked until preview/native
integration and the remaining interaction coverage are complete. Tasks 5-7 are
pending. The new `-CalloutChecks` native fixture is written, but its alpha.2
baseline run (`artifacts/smoke-77ff38e6524d4934b47fe99fcd1c1dda`) exited at the
single-instance guard because the user's dist alpha.2 process (PID 1904 when
inspected) is still running. This is not the expected missing-control RED.
The user was asked to save work and exit via Tray. Do not terminate that process.

No new release build, ZIP, version bump, installer, commit, push or .NET changes.
The previous tested alpha.2 portable candidate is untouched. No claim that the
new controls or physical tail dragging are usable yet.

## Native Integration Progress

After the user closed their running instance, the preserved alpha.2 binary
failed the missing-control check as expected. Native controls, contextual
background/border fields, tail/resize overlays, double-click priority and EN/TH
Help/tooltips are now integrated. Focused native checks passed creation,
palette/toggle memory, border/font independence, tail Undo/Redo/cancellation,
Thai/English editing and exact PNG/clipboard parity.

A reproducible native border-field regression was fixed: Undo restored pixels
but left the numeric border field stale. The field now reflects selected style.
The corner-anchor test also checks both plain text and decorated speech bubbles.
Current unit suite: 79 passed. Formatting and Clippy with the four pre-existing
allowances passed. Version metadata is 0.5.0-alpha.1 for this separate candidate.

The complete native suite, capture benchmark and package gates are still in
progress. See docs/testing/0.5.0-alpha.1.md for evidence and fixture corrections.
Do not present automated window messages as physical Thai IME, mixed-DPI,
hardware wheel or real chat-paste acceptance. No push or .NET changes.

The first full native suite and first extracted package passed all six switches.
The ordinary-capture benchmark recorded slower candidate medians, not a speed
win. A subsequent unit RED reproduced filled drag drafts obscuring cached text;
geometry drafts now draw outline/tail only. The final extracted executable is
being rechecked, including an exact native screenshot-region regression for text
visibility during tail dragging. No change to final fill/text export rendering.

## Local Candidate Verification

Final extracted E5B12011... executable passed all six native switches in
smoke-63904d53d10f4952b25fa177873306fe. All 79 unit tests, formatting, the
qualified Clippy check and Release build passed. Three alternating runs per
final/baseline build are recorded in comparison-aa327a8718bf49009105a5c1b81e257e.
Final capture-paint medians were similar to alpha.2, but a 3.26-second launch
outlier remains an explicit investigation before public release. No universal
speed claim, physical high-DPI/IME/chat acceptance, push or .NET changes.

The local portable candidate contains the tested EXE, EN/TH README, tester
checklist, reports, MIT license and 73 dependency-license files. Final ZIP
refresh is documentation-only and must retain executable hash and exact
staged/extracted file parity before delivery. Public/tester approval is separate
from completion of these implementation and automated verification tasks.

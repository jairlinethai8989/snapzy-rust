# Scrolling Capture Pilot Proposal

Status: approved by user on 2026-10-09 after reviewing the proposed next phase.
Candidate milestone: SnapZy Rust Preview 0.4.0-alpha.1, after text-resize testing.
Scope: Rust SnapZy only; no changes to .NET SnapZy or Neo Snap.

## Goal

Produce a native-resolution long image without repeated frozen headers,
horizontal scrollbars, status bars or false seams, particularly on Lark sheets
with variable row heights. Preserve current fast launch and ordinary captures.

## Approaches

1. Recommended: manual scrolling pilot. The user selects a region and scrolls
   the target application; SnapZy accepts only settled, confidently aligned
   frames. This isolates stitching and frozen-pane behavior before automation.
2. Manual and automatic scrolling together. More convenient immediately, but
   adds focus, wheel routing and stop/end detection before stitching is proven.
3. Browser/DOM export. Potentially useful for some web pages, but not a general
   Windows solution and may not represent canvas-based sheets or desktop apps.

## Proposed Workflow

Select Scrolling, choose the target region, then scroll downward in the original
application with overlap between successive views. A compact controller outside
the region shows capturing/settling/paused status, accepted height and Stop/Cancel.
Stop opens the valid accumulated image in the existing editor. Cancel discards
only the active session. A session with no accepted movement can return its first
frame, explicitly as a single capture rather than claiming a long image.

The initial pilot supports vertical downward motion only. Upward or horizontal
movement, changed target geometry, zoom, display scale or occlusion pauses
acceptance and requests recovery or stopping; no invalid strip is appended.
Automatic wheel input is a later, separately tested increment.

## Components And Data Flow

- Capture session owns target geometry, cancellation, bounded frame acquisition
  and session identity. Target changes or a closed editor invalidate late work.
- A worker checks stability, identifies moving content and proposes a vertical
  shift. It retains only bounded previous-frame/lookbehind state and accepted
  strips; no unbounded full-frame queue. New acquisition coalesces to the latest
  frame while work is busy.
- A stitcher validates overlap/confidence, separates stationary header/footer
  regions, and appends only new body pixels. Stationary rows cannot vote for
  body motion. Frozen headers appear once, footers only at the final bottom.
- The UI receives compact progress/state messages. Capture matching, composition
  and export do not run synchronously on the UI thread. Stop and Cancel remain
  responsive, and unrelated capture hotkeys cannot wait behind stitching work.

Stable-frame detection must compare changing body regions as well as stationary
chrome; a frozen header alone cannot establish that the sheet has finished
rendering. Confidence thresholds must be calibrated against regression fixtures,
not copied unchanged from .NET. Repeated rows and sparse/blank regions can be
ambiguous even when successive frames appear stable.

If alignment is ambiguous, pause without appending. Retry only from a new stable
view with usable overlap; otherwise allow Stop with the last valid partial image.
Do not substitute a guessed shift or insert blank bands to conceal uncertainty.

## Frozen Pane And Footer Strategy

Use the public SnapZy .NET source and its footer regression cases as reference:
`../snapzy-releases/source/snapzy/src/SnapCraft/ScrollStitcher.cs` relative to the
Rust repository root, not as a runtime dependency. Verify source
provenance/licensing before any literal port. Use local synthetic images rather
than private Lark cell data as checked-in fixtures.

The reference handles light scrollbars, 1-2 pixel separators, initially animated
or hidden footers, changing thumb positions and variable row heights. Bounded
lookbehind is required so late footer detection can repair the pending edge
before it becomes irreversible output. A moving grid must never be mistaken for
a stationary footer. Retain original RGBA pixels; no stitching resampling.

## Resource And Failure Policy

Keep the existing aggregate 32 MiB source-pixel budget and frame-dimension limits
for the pilot, reserving room before accepting output. At width 1920, 32 MiB of
RGBA is approximately 4369 rows before other open sources consume capacity;
this is an upper estimate, not a promised capture length or total-memory bound.
At the limit, stop safely with an explicit partial-result notice. Never silently
downsample. Very long disk-backed/tiled output needs its own allocation and editor
design before increasing this budget. No unlimited-length claim in this pilot.

Transient instability can retry within a bounded wait; persistent instability
pauses with an actionable status. Allocation/capture failures preserve already
accepted pixels where possible. Stop exports only accepted strips; Cancel frees
pending work and cannot open a late preview.

## Acceptance Tests

- Synthetic overlap with plain content, repeated rows, blank regions, variable
  row heights, partial final movement and unchanged frames.
- Frozen header/footer combinations; light scrollbar/status bars; blended
  separators; late/animated/hidden footer and moving scrollbar-thumb fixtures.
- Byte-for-byte output against known original pixels, including join boundaries;
  ambiguous shifts rejected and no duplicate frozen chrome or missing body rows.
- Cancel/Stop races, stale workers, target movement/occlusion, invalid geometry,
  resource-limit partial results and bounded allocations.
- Native scrolling fixture with actual UI messages and exact PNG/clipboard pixels;
  benchmark ordinary launch/capture against the accepted resize build.
- User tests on both previously good and problematic Lark sheets, Excel and a
  normal web page, including 125-200%/mixed DPI. No authenticated-sheet claim
  until the user has run those real-sheet checks.

## Exclusions And Approval

No browser extension, OCR, video, layers, unlimited capture, installer, release
or push is included. Text resizing remains a separate completed tester increment.
Review whether manual scrolling first and the bounded-length policy fit the next
test milestone before preparing its implementation plan.

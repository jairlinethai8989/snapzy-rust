# Click-to-detect scrolling capture

Status: approved approach; awaiting user review of this detailed specification.
Target: SnapZy Rust Preview, next local alpha increment. Do not change either
.NET application or push a release before tester approval.

## Goal

Replace manual drag-region selection as the normal entry point for scrolling
capture. Let the user identify the intended content pane with one click, show
the detected pane for review, and begin the existing scrolling workflow only
after explicit confirmation. This is intended to make spreadsheet and other
multi-pane applications easier to capture without guessing the capture bounds.

## User Flow

1. The user chooses **Scrolling** from the launcher. The existing automatic or
   manual scrolling preference remains in effect.
2. SnapZy hides its own launcher/editor windows and displays a full-screen
   targeting overlay, as it does for capture selection today.
3. The user clicks once inside the content pane they intend to capture. A drag
   is not required and does not define the capture region.
4. On a worker thread, SnapZy inspects the UI Automation element under that
   screen point and walks its parent chain, nearest first, looking for a
   currently vertically scrollable element. It may also accept a native HWND
   candidate when Win32 scrollbar state positively indicates vertical
   overflow. It must not infer a target from visual scrollbar pixels alone.
5. When a candidate is found, the overlay highlights its on-screen bounds and
   shows the target application's window title plus **Start long capture** and
   **Cancel** actions. Clicking elsewhere detects a different pane. Confirmation
   starts capture immediately; there is no second start step.
6. After confirmation, SnapZy uses the existing capture, stable-frame, stitch,
   memory-budget and safety checks. Automatic mode sends the existing guarded
   wheel messages; Manual mode waits for the user to scroll. Existing Pause,
   Resume, Stop and Cancel behavior remains unchanged.
7. If no reliable pane is found, the overlay explains that no scrollable area
   was detected and invites the user to click another point or press Escape.
   Do not fall back to drag-region selection. Escape cancels and restores the
   prior application windows.

## Detection And Target Bounds

- Search only around the point the user clicked; do not enumerate unrelated
  background applications or silently select the most likely pane.
- Prefer the nearest ancestor with a UI Automation ScrollPattern whose current
  `VerticallyScrollable` property is true. Use its bounding rectangle as the
  candidate viewport, clipped to the target window, virtual desktop and visible
  screen bounds. Reject empty, implausibly small, off-screen or self-overlapping
  rectangles.
- The Windows UI Automation tree can vary by UI framework and can change as the
  screen changes. UIA calls run on a dedicated worker and never block the main
  UI thread. Show a bounded detection-wait state, allow only one provider query
  worker at a time, and tag each click with a generation identity so late
  results cannot replace a newer point or a cancelled session. A nonresponsive
  provider must not freeze the launcher or create unbounded worker threads.
  Revalidate target window identity, process, bounds, visibility, foreground
  and unobstructed region at confirmation and throughout capture.
- The native fallback is limited to a child/window whose scroll range proves
  vertical overflow (for example, an actual native vertical scrollbar with a
  non-empty range). Scrollbar-like pixels or a generic window style alone are
  insufficient evidence.
- Do not send a test wheel, move the pointer, change focus to the pane, or alter
  its scroll position during detection. Scrolling begins only after explicit
  confirmation.
- The outline is a preview only. It is not included in the captured image.
  Detect and capture the vertically scrollable viewport; horizontal scrolling
  and stitching are out of scope for this increment.

## Safety And Failure Behavior

- Keep existing foreground, process, DPI, occlusion, held-input, stability,
  overlap, memory-limit, pause/resume and partial-result safeguards.
- A stale candidate is invalidated if the target closes, moves, resizes, is
  minimized, becomes covered or changes process/window identity. Ask the user
  to click the target again rather than using stale coordinates.
- Each retry replaces the pending candidate; a previous candidate cannot be
  accepted after a later click or detection failure.
- Escape at any point before confirmation cancels without scrolling. After
  confirmation, existing Stop keeps the valid partial result and Cancel
  discards the session.
- Accessibility failure, timeout, unsupported provider, multiple ambiguous
  ancestors, or invalid geometry yields a localized retry message. It must not
  silently capture the entire window or choose a different pane.

## Scope And Compatibility

- Windows x64 Rust preview only.
- Existing Automatic/Manual preference and vertical scrolling engine remain.
- Primary tester targets: Lark Sheets and Microsoft Excel; also test a standard
  browser page and a native Win32 scrollable control. Compatibility is best
  effort because applications expose accessibility information differently.
- The current manual drag-selection path is intentionally removed from the
  scrolling workflow per the user's choice. Retrying by clicking another point
  and Escape cancellation are the only pre-capture alternatives.
- OCR accuracy changes are a separate increment and are not included.

## Verification

- Unit tests for nearest matching ancestor selection, vertical-scrollability
  filtering, clipped/invalid bounds, candidate replacement on retry, stale
  target invalidation and no-input-before-confirmation state transitions.
- Native harness with nested scrollable panes verifies the click identifies the
  pane under the point rather than the outer window; no candidate returns a
  retry state; confirmation starts the existing scroll job exactly once.
- Verify UIA timeouts do not block the main thread and provider errors recover
  by another click or Escape.
- Run all existing scrolling/capture/editor regressions, especially Auto and
  Manual modes, focus/occlusion pauses, target movement, Stop and Cancel.
- Tester must validate the actual Lark sheet that previously required manual
  region selection and an Excel sheet. Do not claim generic app compatibility
  from synthetic tests alone.
- Deliver a local portable test candidate and bilingual test instructions.
  No Git push until the user tests and explicitly approves.

## Trade-offs Considered

1. UI Automation only: precise and semantic when an application exposes
   ScrollPattern, but misses controls whose provider omits the pattern.
2. Recommended hybrid: UI Automation first plus a narrowly validated native
   scrollbar fallback. This broadens native coverage without guessing from
   pixels or sending exploratory input.
3. Screenshot-based scrollbar detection: no accessibility dependency, but
   themes, scaling, hidden bars and grid lines make false detections likely;
   rejected for this workflow.

## References

- Microsoft UI Automation ScrollPattern reports whether a pane is currently
  vertically scrollable:
  https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nf-uiautomationclient-iuiautomationscrollpattern-get_currentverticallyscrollable
- UI Automation TreeWalker provides parent navigation, but UIA tree structure
  may change as visible UI changes:
  https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nf-uiautomationclient-iuiautomationtreewalker-getparentelement
- UI Automation element bounds and control-pattern access:
  https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nn-uiautomationclient-iuiautomationelement

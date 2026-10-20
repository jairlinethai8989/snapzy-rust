# Alpha 7 Stability Baseline

Date: 2026-10-11 (Asia/Bangkok)
Scope: existing SnapZy Rust 0.9.0-alpha.7. No product source change, version bump,
new installer or Git push is implied by this verification report.

Executable SHA-256:
`0B0A56B4D3A4BCD1ED871AA72622987D52B5E0FB9063FB1953D41BA5919D9070`.

## Confirmed Acceptance

The tester reported that long capture on Lark and physical Escape-to-Stop work.
This confirms that specific reported workflow, not every sheet or monitor setup.

## Automated Results

- Full Windows x64 suite: 127 passed, zero failed, 14.14 seconds. Optional OCR
  model/network fixtures are not implied by this total.
- Full native message-based regression passed:
  `artifacts/smoke-2c07c5928121405786a6def4f1d1d378/results.json`.
  Covers original-resolution PNG/clipboard pixels, Pen/Line/Arrow/Rectangle,
  text resizing, editable text/callout backgrounds and tails, layer ordering,
  image import/move/resize/crop/canvas expansion, Undo/Redo, EN/TH Help, compact
  controls, rendering checks, Auto/Manual scrolling, frozen header/footer,
  Pause/Resume, Stop/Escape/Cancel, focus and cover safety, and two blocked
  detector-helper timeouts with retry and cancellation.
- Native editor test ran on two monitors, both at 96 DPI. This is not coverage
  of mixed DPI or 125-200% scaling. Resize GDI delta was zero for the exercised
  sequence. Peak working set was 140,468,224 bytes; that is not a leak proof.
- Explicit image-fixture tests passed for the two user-supplied local Lark
  screenshots ending in `288e44ea-aa1e-4db5-8c44-c1c431158ffe.png` and
  `841a8c4f-687b-4261-a072-35809ebafa33.png`. Detected bounds were respectively
  `(0, 3, 1910, 746)` and `(0, 24, 1917, 716)`. This verifies image recognition,
  not a live inspection of sheet settings or live scroll input.
- Root-based Windows UI Automation provider test passed:
  `artifacts/provider-460dc81acf8f45e29a0ba7580dcd7427`.
  The test-owned WPF scroll pane was detected even under a covering test overlay;
  its viewport was `(232, 155, 436, 413)` and scroll offset remained unchanged.
  Helper duration was 2,677.43 ms. This is a WPF fixture, not a Lark UIA test.

## Repeated Startup And Capture Observations

Three independent Windows PowerShell test processes ran the same basic native
fixture against the same release executable. All passed.

| Observer Timing (ms) | Run 1 | Run 2 | Run 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Launcher ready observed | 300.54 | 413.65 | 369.73 | 369.73 |
| Area selection window observed | 180.76 | 220.65 | 186.23 | 186.23 |
| Area preview window observed | 59.03 | 62.27 | 65.32 | 62.27 |

Evidence directories under `artifacts`:

- `smoke-667b22b3cc814e76b2a0b048c4c34212`
- `smoke-e8b11791d5a64a758837148a8e6965d8`
- `smoke-b4277ba5493a40889c985f0e53f1ba15`

These values are test-observer timings, not hardware-key-to-painted-image
latency. The launcher metric includes process launch and ready-log observation.
Preview timing starts after selection release and can observe the window before
its first complete paint. They are not .NET/Rust comparison results.

Separate in-process instrumentation from those same three runs:

| Logged Interval (ms) | Run 1 | Run 2 | Run 3 | Median |
| --- | ---: | ---: | ---: | ---: |
| Launcher ready | 47.835 | 83.979 | 74.599 | 74.599 |
| Launcher first paint | 118.667 | 176.816 | 146.380 | 146.380 |
| Selection first paint | 177.959 | 161.708 | 171.854 | 171.854 |
| Preview first paint | 138.685 | 167.440 | 164.845 | 164.845 |

These are the application's existing elapsed-time markers. They do not include
all OS process-loading time or monitor presentation latency; do not subtract
one event's interval from another event with a different start point.

Earlier samples were slow: launcher-ready observation was 5,469.86 ms in the
full regression and 3,573.47 ms in `smoke-e64e170bc5a1454d8d1b189278bfa4ee`.
In-process `launcher.ready` was 77.206 and 75.142 ms respectively, so the
difference is not explained by that UI-initialization interval. Process loading,
observer scheduling and instrumentation require separate measurement before
attributing a root cause. No speculative product optimization was applied.

The first attempt to repeat the baseline in the same PowerShell process stopped
after a successful smoke because its wrapper checked a stale `$LASTEXITCODE`.
Running each smoke in a separate process supplied a valid exit code and produced
the three passing records above. This wrapper error is not an application bug.

## Remaining Coverage

- Repeat real Lark sheets with different frozen panes, real Excel/web scroll
  areas, physical mouse/wheel/hotkeys and paste into the team's chat applications.
- Mixed-monitor scaling, prolonged repeated use and sustained memory behavior.
- Explain startup outliers with boundaries that include process loading and
  separate ready/window/paint events; retain the slow samples in comparisons.
- Missing editor tools require separate approved designs and test candidates.

Do not describe this baseline as a new release or as all compatibility testing
being complete. Both .NET products and the user's existing running instance were
left unchanged.

# Native Video Recording: Stage One

Date: 2026-10-13 (Asia/Bangkok).
Status: written design approved by the user on 2026-10-13; implementation started.
Workflow corrected by the user on 2026-10-14: choose Save after recording.
See 2026-10-14-video-save-after-recording-design.md for that scoped change.
Local candidate target: SnapZy Rust 0.10.0-alpha.1.
Rust only. No .NET changes, commit, push, installer or public release.
Preserve Alpha 16 and earlier packages and the existing dirty checkout.

## Context And Approved Scope

The user accepted Alpha 16 and selected video recording next, rather than
`.neosnap` interchange. The user explicitly approved splitting delivery:

1. Disk-streamed recording and safe stop/finalization.
2. Audio, in-app preview and clip copying in a later specification.

This first stage records one selected monitor or one selected application
window into silent H.264 MP4. Target 30 fps, with a 3-2-1 countdown. Stop and
Escape stop recording and retain the accepted footage. There is no application
duration cutoff; available storage, hardware and operating-system limits still
apply. Low disk space triggers a controlled stop.

No region dragging, pause/resume, audio, in-app playback, clipboard clip export,
video editing or automatic upload in this stage. Existing screenshot output
remains native-resolution lossless PNG; MP4 video is lossy, not lossless PNG.

## Approaches And Recommendation

1. Windows Graphics Capture plus Media Foundation, initialized on demand in an
   isolated Rust worker: recommended. Uses Windows capture/encoding components,
   avoids adding an external encoder executable, and separates recording failure
   from the launcher/editor. More native API integration than a command wrapper.
2. An external FFmpeg worker: flexible encoding and later audio support, but
   adds a separately packaged runtime and distribution maintenance. Do not add
   it or silently download an encoder in this stage.
3. Repeated screenshot snapshots plus an encoder: reuses current screenshot
   code, but its repeated GDI allocation/copy and desktop synchronization do not
   match the proposed video path. Do not turn `capture::snapshot` into a 30 fps
   loop or change screenshot capture to serve video.

Use the pinned `windows` binding dependency already present in the repository;
add only the required Windows API features during implementation. Use platform
capture, encoding and MP4 muxing, not a hand-written codec/container.

## User Workflow

- Add a compact native Video command and localized EN/TH Help/tooltips using the
  existing control conventions. No broad launcher/editor visual redesign.
- Offer Screen / Window. Screen selects one attached monitor, initially the
  primary monitor, with its name and physical-pixel dimensions. Never silently
  record every monitor. Window uses the existing visible-window selection
  workflow; reject the recording instance's own windows and protected targets.
- Selecting a source starts preparation/countdown immediately. Stream to a unique
  instance-owned file under `Recordings/Pending`; test instances use their own
  data directory. Selection cancellation returns without recording.
- After Stop/Esc finalizes the clip, open a native Save video chooser defaulting
  to `Recordings`. Cancelling Save keeps the clip and displays its location.
  Save runs in the background and retains the original on failure.
- Stage one does not replace existing files. If the chosen destination already
  exists or appears before publication, ask for another name; preserve it.
- Validate source, codec availability, writable destination and free space off
  the UI thread. Display Preparing, not Recording, until initialization succeeds.
- Show 3-2-1 after preparation. Do not write video samples or count elapsed
  recording time during preparation/countdown. Revalidate the source at start.
  Escape before the first sample cancels and reports no completed recording.
- During recording display an accurate elapsed timer, state and Stop action in
  a compact controller where it can be kept out of the recorded content. Provide
  Stop recording in the instance tray menu, including when the controller hides.
- Escape during Recording requests the same stop as Stop. Follow the existing
  temporary Escape hotkey plus key-state polling pattern used by long capture,
  with a recording-owned registration/timer during preparation and recording. It
  must work while another app has focus, have one state-driven action, and be
  released on cancel/stop/failure. Do not add a keyboard hook, replace the
  configured image capture hotkey or intercept Escape while recording is inactive.
- Stop changes state immediately to Finalizing. Repeated Stop/Escape requests
  are idempotent; do not start another worker or publish duplicate clips.
- Success shows the actual path with explicit Open video / Open folder commands.
  Opening uses the associated Windows player only on user request. No automatic
  playback, embedded preview, copy button or claim of clipboard support yet.

## Capture And Media Policy

- Video requires Windows Graphics Capture support and the Media Foundation H.264
  encoder. Check capabilities at use time. Unsupported Windows/media components
  produce an actionable EN/TH error without breaking existing image capture or
  installing/changing system components automatically.
- `CreateForWindow` / monitor interop requires Windows 10 version 1903 or later.
  Retain existing screenshot behavior on unsupported systems; do not impose this
  video requirement on the whole application's startup.
- Use Windows Graphics Capture for the explicitly selected source. Do not fall
  back to a desktop crop when window capture fails: that could capture unrelated
  windows. Do not bypass capture restrictions or suppress the OS capture border.
- Keep the platform cursor capture behavior in this first stage, document what
  the supported API actually provides, and test it. No custom click indicator.
- Freeze physical source dimensions from the initial frame. Preserve pixels;
  do not stretch or downscale to make encoding succeed. Odd dimensions may be
  padded with at most one black row/column for H.264 alignment, never cropped.
- Stop/finalize with a reason when the selected window closes, minimizes or
  changes capture size, or its monitor is removed/changes dimensions. Do not
  switch source automatically. A reused HWND is not permission to record a new
  window; bind the capture item and validate source identity/lifetime.
- Target at most 30 fps. Use a monotonic clock and checked 64-bit timestamps in
  the media time base. Skip stale queued frames under load rather than building
  an unbounded queue. Preserve real elapsed duration; do not speed up the clip
  by assigning every delayed frame the same fixed artificial duration.
- Static content may reuse the last valid frame so its duration remains correct.
  Do not mistake the lack of new desktop damage for a closed source. Surface
  access/encoder failures must not become an indefinite recording of stale data.
- Default bitrate in bits/second is `clamp(width * height * 30 / 8, 8_000_000,
  32_000_000)` using checked wide arithmetic. This is a stage-one quality policy,
  not a promised file size or constant achieved bitrate. No quality settings UI
  or claim of guaranteed 30 fps on every computer in this stage.
- Prefer a supported hardware H.264 encoder at preparation; try software before
  beginning if hardware setup fails. Once samples have started, encoder failure
  stops the session; never silently splice a different backend into the file.
- Use the Windows fragmented MP4 sink and sink writer to stream encoded output
  into a seekable on-disk byte stream. Do not accumulate the recording in memory,
  write one PNG per video frame, or require a full-duration uncompressed file.

## Worker And Resource Boundaries

- Add separate recording state/policy, native capture/encoder, worker lifetime,
  and native UI adapter modules. Keep existing image/editor/scroll modules intact
  except for narrow busy-state, restore, tray and Exit integration.
- Follow the current isolated detector-worker pattern: spawn the same Rust
  executable in an internal video-worker mode, hidden/no console, only after
  explicit recording preparation. Attach it to a kill-on-parent-close Job Object
  and reap it on all outcomes. Do not spawn it on normal startup or image capture.
- Media APIs must load only in the worker on demand. In particular, avoid new
  unconditional Media Foundation DLL imports that would prevent the launcher
  starting on a machine without those optional media components. Binding types
  alone are not evidence that this requirement is met; inspect the built imports.
- Use serde-validated requests and bounded control/status messages over private
  child stdin/stdout. Assign a session generation; reject stale/mismatched worker
  events. Frame data stays inside the worker, not in the UI message queue.
- Permit one recording/preparation/finalization at a time. Frame input is a
  latest-frame queue of at most two pending frames. Limit a BGRA source frame
  to 32 MiB and explicitly managed video surfaces/readback/conversion scratch to
  256 MiB. Reject oversized input rather than silently rescaling it. OS/GPU/codec
  private allocations are additional; do not claim total process RAM is 256 MiB.
- Rate-limit status updates and diagnostics. No per-frame disk logging, queue of
  cloned full frames, preloaded encoder, or model scans on launcher/image paths.
- Bound preparation to 20 seconds and finalization to 30 seconds. The UI stays
  responsive throughout. An unresponsive worker after a deadline is terminated
  and reaped; keep available partial output and report failure, not success.
  Measure the finalization deadline from the parent's Stop request, not an
  acknowledgement that a blocked worker might never send. Timeouts limit
  setup/stop, not recording duration.

## Storage, Stop And Failure Handling

- Require at least 512 MiB available space to prepare. Check space before
  writing and at least once per second while recording. Under 256 MiB requests
  a low-space stop. Unknown free space is an error, not unlimited free space.
  These reserves reduce risk but cannot guarantee finalization if another
  process fills the disk or hardware fails.
- Create a uniquely owned sibling partial file exclusively. Keep it outside any
  IPC temporary directory whose Drop/cleanup removes its contents. Never use
  the detector JobDirectory cleanup policy for recorded media.
- Only successful sink finalization, byte-stream close/flush and no-replace
  publication can produce a completed clip. User Save follows this step. Do not
  publish a zero-frame/failed recording as a successful clip.
- After failure keep nonempty partial media with a visible location and explain
  that it may be incomplete/unplayable. Fragmentation is not a recovery guarantee
  for a crash, lost final fragment, full disk or storage failure. No automatic
  cleanup of older clips, partial files or user-selected paths in this stage.
- Cancelling before any sample may remove only the session's empty staging file
  and owned IPC files. Escape after samples have started means Stop/retain, not
  discard. Never delete another session's output or an existing destination.
- Use total-hours elapsed display beyond 24 hours. Test timestamps and storage
  accounting across 4 GiB and long durations without a hard cutoff. Report real
  multi-hour/large-file test gaps; simulated counters do not certify them.

## Interaction With Existing Work

- Refuse recording while image capture/scrolling, selection, a modal edit, an
  unfinished gesture, project open/import/save or a close transaction is active.
  Return a clear busy status; never cancel or discard those operations to start.
- While a recording session is preparing, counting down, recording or finalizing,
  block new capture, scroll, open/combine/import and another recording request.
  Do not start a competing operation through a hotkey or tray command.
- Hide this instance's launcher, editor hosts and owned OCR windows for recording
  using generation-checked restoration, preserving active tabs and minimized/
  hidden state. Do not expose all inactive child editors on restoration.
- A monitor recording must not contain the app's own controls. Use verified OS
  capture exclusion when supported; otherwise hide the controller for that mode
  and retain tray Stop/Escape. Do not accept a black rectangle or an overlay in
  the clip as a successful exclusion test. Window recording can place controls
  outside the selected target. Do not hide or modify unrelated applications.
- Main-window X retains its existing minimize-to-tray behavior, not silent Stop.
  Explicit app Exit during recording offers Stop recording and exit / Cancel.
  Cancel keeps recording. Stop finalizes first, then uses the existing all-work
  Save/Discard/Cancel flow. Cancelling project closure afterwards keeps all work
  open; an already saved clip remains saved. On finalization failure offer Stay
  or Exit keeping partial output, with Stay the default; do not silently exit.
- Restore surviving recorded windows on cancellation, failure and completion.
  The existing `.snapzy` schema, tab history, project budgets, OCR routing and
  scrolling Escape/Stop semantics do not change.

## Verification And Delivery

Write tests first for the session state machine, unique destination/publication,
bounded messages/queues, stale generations, duplicate Stop, pre-sample Cancel,
low/unknown disk space, long elapsed time/timestamps and worker crash/reaping.
Test failures without modifying user files or injecting failures into their
settings. Test valid captured frames, not assertions over source-code strings.

Native tests require fresh input-free user consent and generated visual targets.
Verify monitor/window recording, countdown, physical Escape with target focus,
on-screen/tray Stop, duplicate Stop, size/target loss, finalization and shutdown.
Decode short test MP4 files with Windows media components and inspect changing
frame content, color/orientation, dimensions and duration; file existence alone
is not proof of a valid recording. Test source changes near Stop and repeated
sessions for stale state and resource leaks. Include odd dimensions and Unicode
output paths. No private company windows or automatic audio capture in tests.

Check controller exclusion and window restoration with multiple image tabs,
minimized hosts and owned OCR. Recheck image capture, long-capture final frame,
Copy/PNG, project save/open and transactional closing. Keep Alpha 16's pending
native gaps and status-label clipping finding separate; do not mark them fixed
or bundle a broad UI cleanup into video implementation.

Measure cold/warm startup, image button/hotkey-to-selection and first-preview
latency against the preserved Alpha 16 package on the same generated targets.
Record actual timings and package/private-memory observations. A larger binding
feature set or a successful short clip does not establish a performance win or
multi-hour reliability. Run the complete Rust suite, formatting and release build
before packaging; record errors/warnings and unresolved native/long-run gates.

Package the proposed 0.10.0-alpha.1 as a separate local candidate with accurate
EN/TH notes and hashes, preserving Alpha 16. No .NET changes, installer, commit,
push or publication without a separate request. Implementation planning follows
the user's approval of this written design.

## References And Review

- [Windows window capture interop and minimum OS](https://learn.microsoft.com/en-us/windows/win32/api/windows.graphics.capture.interop/nf-windows-graphics-capture-interop-igraphicscaptureiteminterop-createforwindow)
- [Microsoft Win32 capture sample](https://github.com/microsoft/Windows.UI.Composition-Win32-Samples/tree/master/cpp/ScreenCaptureforHWND)
- [Fragmented MP4 sink](https://learn.microsoft.com/en-us/windows/win32/api/mfidl/nf-mfidl-mfcreatefmpeg4mediasink)
- [Using the sink writer](https://learn.microsoft.com/en-us/windows/win32/medfound/using-the-sink-writer)
- Existing `src/scroll_probe.rs` worker/Job Object pattern, `src/ui.rs` capture
  hide/restore and tray handling, and `src/project_ui.rs` transactional closing.
- .NET recording policy/session inspected read-only for workflow parity. Do not
  reuse its .NET runtime, managed recorder or application identity in Rust.

Self-review: approved stage boundaries retained; audio/preview/copy excluded;
setup/finalization deadlines distinguished from recording length; exclusive
publication and partial preservation separated from IPC cleanup; optional media
components kept off startup; inherited verification gaps retained. Written user
review was received on 2026-10-13. This document is local, not committed.

# Alpha 3 Native Check: Save And Exit Verified

User consent: approximately five minutes, generated windows/clips only, no chat
opening or sending. Focused testing stopped and the user was told the computer
was available again. A fresh three-minute window was requested for the remainder.
The user granted that window. The remaining no-clipboard run then passed all
six checks and exited cleanly. The user was told the computer was available.

## Confirmed

Three native runs reached a paused video preview after actual window recording
and Stop, with no automatic Save dialog. The preview surface contained the red
generated fixture pixels, not a blank surface. Playback advanced; controls were
exercised. Save Cancel preserved the preview. Application stderr was empty.

Latest successful partial-run evidence:

- work/video-preview-28831f8c28504d15a31c631477a2874d/failure.json
- work/video-preview-28831f8c28504d15a31c631477a2874d/paused-first-frame.png
- Retained generated MP4s under each test directory's Recordings/Pending.

Latest successful remaining-check run:

- work/video-preview-fcfe4772da17468998a320bd375098fa/result.json
- Application stderr was empty; test application exited with code 0.
- Stop opened preview without an automatic Save chooser.
- Cancel Save preserved the preview.
- Actual MP4 export preserved the pending original and exact file SHA-256.
- EN/TH preview title refresh and minimum-size control bounds passed.
- Exit: Keep Local on the first clip, Cancel on the next kept both previews open.
- Keep Local closed the generated previews and normal exit preserved clips.
- ClipboardChecked was false: this run neither read nor changed the clipboard.

An intervening pixel-check attempt sampled an obscured preview and was rejected,
not classified as a blank-video application failure. The remaining-check run
skipped the previously passed pixel/playback steps. The helper now verifies the
preview owns the sampled screen point before taking a new screenshot. Its
unintended obscured screenshot was removed; the failure record is preserved.

## Harness Issues, Not Claimed As Product Failures

1. First run: cleanup passed an OLE clipboard proxy back to OleSetClipboard and
   OleFlushClipboard recursed to a stack overflow. The original clipboard was
   not independently materialized; restoration cannot be confirmed. Only the
   exact test helper and its child application were terminated; user instances
   and files were not touched. The user was notified. Do not run the old bundled
   test script with -AllowClipboard.
2. Save automation assumed an immediate child filename control and then used
   GetWindowText across processes. The latter cannot read another application's
   Edit text. The repaired harness enumerates descendants and uses bounded
   WM_GETTEXT/WM_SETTEXT messages, matching the existing smoke-test helpers.
   It records failure details before cleanup. The subsequent real Save run
   passed, including retained source and exact exported file bytes.
3. An isolated clipboard test was attempted in a dedicated single-test child.
   CreateWindowStation failed with Windows error 5 (Access Denied), before any
   OpenClipboard call. It is now an explicit opt-in ignored test for suitable
   Windows hosts. No interactive clipboard fallback is allowed.

   Explicit failed test name:
   `clipboard::native_clipboard::file_clipboard_survives_export_and_owner_close_in_an_isolated_window_station`.
   It remains unverified on this host; an ignored default run is not a pass.

Reference for cross-process control text:
https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getwindowtextw

## Remaining

- Actual Copy Clip and chat paste; never send automatically.
- End-to-end seek/audio UI and Save-on-close beyond the tested explicit Save.
- Device/audio and long-duration checks beyond generated MP4 regression tests.

No Rust application behavior was changed by these harness repairs. An additional
Windows binding feature supports test-only private-station verification. No .NET
changes, commit, push or public release. Keep earlier candidates, but use the
corrected harness rather than the original packaged helper.

# Video Preview And Clip Copy

Date: 2026-10-15 (Asia/Bangkok).
Status: approved by the user; implementation authorized.
Target local candidate: 0.10.0-alpha.3. Rust only; no .NET edits, commit, push,
installer or public release. Alpha 2 audio was accepted by the user.

## Goal And Alternatives

Complete the staged recording workflow with an in-app preview, file clipboard
copy and explicit Save MP4. Preserve the accepted source selection, countdown,
four audio modes, streaming capture, Stop/Esc and disk-reserve behavior.

Recommended: native Windows Media Engine playback loaded on demand. This keeps
the Rust application independent of WebView2 and avoids initializing playback
during launcher startup or screenshot capture. An external player would require
less application code but separates playback from Copy/Save. A WebView2 preview
would resemble the .NET implementation but adds a browser runtime dependency.
Do not use the superseded MFPlay or Windows Media Player ActiveX APIs.

## Workflow And Controls

1. Select screen/window, prepare, count down and record as in Alpha 2.
2. Stop/Esc finalizes and closes the encoder before presenting a preview.
   Successful recording no longer opens Save automatically.
3. Show a compact, resizable preview with aspect-correct video, play/pause,
   elapsed/total time, seeking, mute and volume. Start paused at the first frame;
   no unexpected playback sound. At the end, Play restarts from the beginning.
4. Provide Copy Clip, Save MP4 and Show in Folder. Buttons have busy/success/error
   feedback; an audio-less clip disables volume controls. EN/TH labels and
   tooltips follow the app language. No oversized help copy inside the preview.
5. Save opens a chooser only when requested. Cancel/error keeps the preview and
   clip. Use existing collision-safe publication with bounded background I/O.
6. A failed playback engine or missing Windows media capability must not prevent
   Copy/Save: show the error and offer the retained file's location. Do not
   download a player/runtime or silently open another program.

## Stable Storage And Clipboard

Keep the finalized recording at its unique instance-owned Pending path for this
increment. Closing a preview or exiting does not automatically delete a completed
clip. Show in Folder exposes its retained location; automatic cleanup and a clip
library are outside this increment. Preserve old completed/partial recordings.

Copy Clip places a validated absolute MP4 path in Unicode CF_HDROP with a copy
drop effect, not text, image data or the full video bytes. Keep that backing path
unchanged after Copy, Save, closing the preview and normal app exit. No network
upload, recipient selection or automatic chat sending.

To guarantee this lifetime without copying the entire clip merely for Copy,
Save exports a separate file and preserves the retained original. This explicitly
supersedes Alpha 1's move-on-same-volume optimization for preview-owned clips.
The saved copy consumes additional disk space; stream it on a worker, use a
temporary destination in the target directory and publish without replacement.
Only claim Save success after durable publication. A failed destination write
never removes the retained source or replaces an existing user file.

Clipboard ownership transfers only after allocating/validating the file payload.
Report clipboard contention and allow retry without claiming success. Native
clipboard tests require a controlled test session; do not overwrite the user's
clipboard opportunistically. Copy success means Windows accepted a file entry,
not that a particular chat accepted or sent it. Validate actual chat paste with
the user. Show in Folder offers a manual file-drag fallback; direct drag from the
preview is deferred.

## Close, Exit And Concurrency

Before closing a clip that has neither been successfully copied nor saved, offer
Save / Keep Local / Cancel. Keep Local closes the preview and preserves the clip;
Cancel leaves it open. Save-dialog cancellation also cancels closing. A copied
or saved clip closes without another prompt; its retained source is not deleted.
Esc in a focused preview requests this same close flow. Recording Esc retains
its existing Stop behavior and must not be intercepted by the preview.

Allow multiple previews for separate clips with a finite limit of eight. Beyond
the limit, retain the completed clip and show its location rather than lose it.
Pause existing playback before a new countdown to avoid recording preview audio
through system loopback. A new capture must not record a preview by accident:
hide preview windows with other app windows during target selection/recording,
then restore them without resuming audio automatically.

Only one Save per preview at a time. Disable its duplicate Save/close actions
during export; do not block unrelated image editing. Explicit app Exit waits for
exports and reviews unkept previews before closing any of them. Cancel must not
partially close the preview set. Stopping a recording to exit first opens/reviews
the completed clip, rather than bypassing preview handling.

## Architecture And Error Boundaries

Keep playback, clip lifecycle and file clipboard operations in small dedicated
modules. The recording worker continues to own capture/audio/encoding only.
Use the existing Win32 window/control conventions and Media Foundation bindings;
initialize the engine only for a preview, shut it down before destroying its
render surface, and release callbacks/COM interfaces in the owning context.

Native callbacks post bounded events to the UI. Identify each preview with a
generation/session token and reject late events after closure. Do not hold a
global UI-state borrow across native playback calls, modal choosers or shutdown.
Resize preserves aspect ratio; controls fit the minimum window size and DPI.
Disk I/O and media loading must not run as whole-file UI-thread work. No decoded
whole-clip buffering, duration limit or change to encoded MP4 bytes.

## Verification And Delivery

First observe failing behavior tests for clip lifecycle, copy-versus-save states,
Save/Keep Local/Cancel, stable source retention, async identity and file-drop
payloads. Then test collision-safe export, Unicode destinations, disk/I/O errors
and bounded streaming without modifying the original. Retain Alpha 2's native
silent/AAC decode and delayed-audio regressions.

Use generated silent/audio MP4s for native playback: first frame, play/pause,
seek/end/replay, volume, resize, close during loading and error fallback. Verify
actual rendered video rather than only window creation. Test multiple previews,
app exit cancellation, capture hiding, late events and long timestamp formatting.
Native tests that take focus or replace the clipboard require a user-agreed test
window and only generated content; never send to a chat automatically.

Run the full locked suite, formatting, release and startup-import checks. Package
Alpha 3 separately, verify ZIP contents against the built files, preserve Alpha 2
and document actual native test gaps. User confirmation of chat paste is separate
from synthetic file-clipboard validation. Do not claim universal chat support.

No trimming, re-encoding, timeline editor, pause/resume recording, automatic
cleanup, direct preview drag or .neosnap interoperability in this stage.

## References

- https://learn.microsoft.com/en-us/windows/win32/api/mfmediaengine/nf-mfmediaengine-imfmediaengine-play
- https://learn.microsoft.com/en-us/windows/win32/medfound/media-engine-attributes
- https://learn.microsoft.com/en-us/windows/win32/medfound/mfp-pkey-streamrenderingresults
- https://learn.microsoft.com/en-us/windows/win32/shell/clipboard

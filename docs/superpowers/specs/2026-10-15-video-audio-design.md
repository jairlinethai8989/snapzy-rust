# Native Video Audio

Date: 2026-10-15 (Asia/Bangkok). Continue the user's approved staged video plan.
Local candidate: 0.10.0-alpha.2. Rust only; no commit, push or .NET changes.

## Scope And Workflow

- Video menu offers No audio (default), System audio, Microphone, and Both.
  Persist the explicit choice; older settings and requests default to No audio.
- System audio means the default playback device's output, including other apps.
  Microphone means the default recording endpoint. Freeze endpoints per session.
- Select the source, prepare, count down 3-2-1, record, then Stop/Esc and Save.
  Preserve the accepted save-after-recording and Cancel-retention workflow.
- No microphone/system capture starts before recording. Endpoint errors explain
  which input failed; do not silently change source or record a different mode.
- On endpoint loss after recording starts, stop and finalize the accepted clip
  with a reason. Preserve media on failure and retain the current disk reserves.

## Implementation

Use Windows WASAPI shared capture and loopback, with the audio engine converting
to 48 kHz stereo 16-bit PCM. Media Foundation encodes AAC-LC at 192 kbit/s in the
existing fragmented H.264 MP4. No downloaded encoder/runtime or startup scanning.

The worker owns audio interfaces and the encoder. Capture begins at the first
video frame. Map endpoint QPC timestamps onto that recording epoch. Use bounded
per-input sample rings, fill timestamp gaps with silence and mix both inputs at
equal gain with integer headroom. Limit queued samples to one second per input;
write 10 ms blocks with 100 ms delivery tolerance, flushing only to the video end.
No duration cutoff or unbounded full-clip audio buffer. Device discontinuities
must not restart timestamps or speed up the clip.

## Verification

Test old-settings compatibility, all four modes, bounded/gapped/overlapping sample
timelines, negative/pre-start samples, long timestamps and PCM mixing. Create and
decode a synthetic H.264/AAC MP4 to check real audio samples, timing, rate and
channels. Preserve the silent MP4 regression. Run the full suite, format/build,
and package integrity checks. Native device/driver, long-run sync and microphone
permission behavior need separate tester feedback using explicitly chosen inputs.

Preview and clip copying remain subsequent stages. Video editing is out of scope.

References:
- https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudioclient-initialize
- https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudiocaptureclient-getbuffer
- https://learn.microsoft.com/en-us/windows/win32/medfound/aac-encoder

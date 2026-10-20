# Native Video Audio Implementation

Design: ../specs/2026-10-15-video-audio-design.md.
Continue the user's staged video plan. Rust only; no commit/push or .NET edits.

- [x] RED/GREEN bounded stereo timeline, mixing, latency tolerance and long clocks.
- [x] RED/GREEN legacy-silent settings and explicit four-mode request validation.
- [x] RED/GREEN native H.264/AAC encode/decode and delayed-audio mux regression.
- [x] WASAPI shared capture/loopback initialized on demand in the worker.
- [x] Endpoint stop, source-loss reason and audio finalization integrated with video.
- [x] EN/TH audio menu, persisted choice and default No audio.
- [x] Full locked suite: 257 passed, formatting and whitespace checks.
- [x] Optimized build, native import inspection and separate Alpha 2 package.
- [x] User accepted Alpha 2: "tested successfully, proceed to the next stage."

Acceptance is user-reported; it does not establish a separate device-by-device,
driver-loss or long-run sync certification. Preserve these residual test gaps.

Audio/driver and long-run native gaps remain explicit. Preview and clip copying
follow audio acceptance. Video editing is outside scope.

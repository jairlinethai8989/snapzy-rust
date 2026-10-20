# Native Video Stage One: Implementation

Approved design: ../specs/2026-10-13-video-recording-stage-one-design.md.
Rust only; preserve Alpha 16 and dirty work. No commit/push or .NET edits.
The writing-plans skill is not installed; use this scoped local plan.

- [x] RED/GREEN session phases, deadlines, elapsed time, media geometry and publication.
- [x] RED/GREEN dynamically loaded Media Foundation encoder with synthetic MP4 decode.
- [x] Windows Graphics Capture with bounded frame resources and source lifetime checks.
- [x] RED/GREEN bounded worker protocol, isolation, Stop/crash/timeouts and partial retention.
- [x] Native Video picker/save/countdown/controller/tray/Esc and existing-work busy guards.
- [x] Recording-aware Exit and generation-checked window restoration.
- [x] Full tests, formatting, build and executable import inspection.
- [x] Separate 0.10.0-alpha.1 package, EN/TH notes and hashes; preserve older builds.
- [x] User accepted recording and corrected Save-after-Stop workflow on 2026-10-14.
- [ ] Verify remaining individual native cases and long-duration gaps.

2026-10-14 correction: save chooser moved after recording per the user's request.
See ../specs/2026-10-14-video-save-after-recording-design.md and
../../testing/0.10.0-alpha.1-save-after-recording.md. Full suite: 248 passed.
Native recording and Save-after-Stop have user acceptance. Cancel retention,
global Esc, target-loss, capture exclusion and long-duration cases remain
individually unconfirmed. Next planned feature stage: audio, preview and clip copy.

Use real state/files and platform media components, not source-string tests.
Synthetic encoder checks do not need desktop recording; live capture and UI
tests require fresh input-free consent and generated targets. No success claim
based on an existing MP4 filename alone: decode test footage and check content.

# Shadows And Text Outline Implementation Plan

Written specification approved by the user on 2026-10-11.
The writing-plans skill is unavailable locally; follow the existing plan format.
Rust only, preserve dirty work and earlier candidates; no commit or push.

- [x] RED/GREEN validated effects state, atomic document edits and session defaults.
- [x] RED/GREEN bounded masks, soft/hard shadows, text outlines and pixel composition.
- [x] Preserve preview replacement bounds, backdrop blur, highlight and image alpha.
- [x] Implement compact Effects panel, Apply/Cancel/Esc, EN/TH Help and tooltips.
- [x] Unit/release/format/script checks and local ZIP/staging hash verification.
- [x] Focused Effects native checks after fresh consent; local candidate and honest notes.
- [x] Existing editor/annotation, OCR-window and synthetic scrolling Stop/Escape regression.
- [ ] Follow-up: native color chooser, real Lark/hardware, OCR accuracy and direct Alpha 12 comparison.

185 release tests pass. Isolated 600 x 6000 effects OFF/ON medians: 54.34/370.93 ms.
Focused native Effects checks passed after fresh consent in
artifacts/smoke-a546e35d9cb94feda4f50bbcd4a66151. EN/TH panel images inspected.
First attempt exposed only a missing IsWindow harness declaration; corrected
without changing the executable. Second consented run passed editor/annotation,
OCR window and manual/automatic scrolling including guarded OS Escape injection:
artifacts/smoke-3caed251436445868cf07e5eecd3faff. User told machine is free again.
Further desktop checks require a new agreed interval. No direct Alpha 12
comparison or OCR accuracy claim yet.
Local candidate contains 113 verified files, with the
accepted Alpha 12 executable unchanged. Package evidence is recorded in
artifacts/alpha13-package-verification.json. Native acceptance remains open.

Test source immutability, opacity/history, crop/tile seams, old-tool regressions,
invalid inputs and rendering failures. Do not infer live Lark from fixtures.

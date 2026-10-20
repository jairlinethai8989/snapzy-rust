# Video Preview And Clip Copy Implementation

Approved design: ../specs/2026-10-15-video-preview-copy-design.md.
Rust only, Alpha 3 local candidate; no commit/push or .NET changes.

- [x] RED/GREEN clip review/save lifecycle, operation identity and time formatting.
- [x] RED/GREEN retained-original streaming export and Unicode file-drop payload.
- [x] Native on-demand Media Engine playback and generated-clip tests.
- [x] Implement dedicated compact preview controls and copy/save feedback EN/TH.
- [x] Implement completion routing, capture hiding/pausing and staged exit review.
- [x] Native explicit Save/Cancel, retained source, exact export, EN/TH and layout.
- [x] Native Keep Local then Cancel preserves all previews; normal exit retains clips.
- [ ] Actual clipboard/chat paste and remaining seek/audio/Save-on-close UI checks.
- [x] Latest full locked suite: 263 passed, 0 failed, 1 opt-in native test ignored.
- [x] Optimized build and native startup-import checks (no startup media DLLs).
- [x] Separate package: 206 files verified by extracted hashes; native gaps documented.
- [x] General user acceptance of Alpha 3 (2026-10-15); individual client/edge-case coverage is not implied.

Use test-first model/file tests and generated recordings. Never send messages,
capture actual microphone audio or replace the user's clipboard without an
agreed native test session. Playback/UI tests must disclose remaining gaps.

Candidate: dist/SnapZy-Rust-0.10.0-alpha.3-win-x64.zip (4,992,501 bytes).
SHA-256: CD68E59F66C5C0BFF15A47DCF745CD73A101952BBC81C86E2E3DEA8345B760FF.
Native test readiness was granted and three partial runs were attempted. Actual
recording/paused pixels/playback/Save Cancel passed. Harness cleanup and filename
inspection failed; see docs/testing/2026-10-15-alpha3-native-check.md. Save success,
bulk exit and chat paste remain pending. Interactive clipboard automation is
disabled; the private-station test was denied by this host and is not a pass.

Corrected candidate: dist/SnapZy-Rust-0.10.0-alpha.3-win-x64-r2.zip.
208 verified files, 4,996,357 bytes.
SHA-256: 20F3903B6CB31C9EE54FC8EB635C083EC6B18E2AB0BDE97D2CD53D128EBCA641.
Do not run the earlier bundled helper with -AllowClipboard. Fresh readiness was
requested for the remaining no-clipboard UI checks; no further focused UI run
was started without the user's reply.

The user granted the remaining test window. Run
work/video-preview-fcfe4772da17468998a320bd375098fa/result.json passed all six
remaining no-clipboard checks and exited with code 0. No user clipboard access.
Only harness/report files changed afterwards; no application behavior changes.

Latest candidate: dist/SnapZy-Rust-0.10.0-alpha.3-win-x64-r3.zip.
209 verified files, 4,998,415 bytes; same tested executable as R2.
SHA-256: 58B50657E0F284894825608333476EB3D5E0072ADB6C9CC6FA9C5D9DD7731577.

## User Acceptance And Next Scope

On 2026-10-15 the user reported testing passed and requested the next phase.
Alpha 3 is accepted for this development sequence. This does not convert the
ignored private-station clipboard test into a pass or certify every chat client,
long-duration audio sync or unreported edge case. Keep those coverage limits.
No new executable, version, installer, commit, push or .NET change is needed
for this acceptance record. The next proposed scope is one-way editable import
of .NET .neosnap projects, pending scope and written-design approval.

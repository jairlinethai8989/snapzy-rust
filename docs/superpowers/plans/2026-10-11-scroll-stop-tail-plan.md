# Pending Final Scroll Step Plan

Approved spec: `../specs/2026-10-11-scroll-stop-tail-design.md`.

1. Native RED against staged Alpha 8: Stop before a 900 ms delayed scroll is
   painted; require the final step's exact height and source pixels.
2. Add a bounded final-sampling phase without wheel input, retaining target,
   stable-frame, overlap, memory and cancellation guards. Repeated Stop must
   not restart its deadline. Add EN/TH finishing/partial notices.
3. Test-first additional safety cases: cancellation, timeout, coverage/target
   changes, unchanged final frames, repeated Stop and Escape.
4. Full unit suite, formatting, build and native regressions. Preserve failed
   attempts and distinguish synthetic checks from live Lark acceptance.
5. Update local documentation and package a separate Alpha 9 candidate; verify
   executable version/hash across build, stage and archive. No Git push.

Progress: native RED reproduced against staged Alpha 8 (400 px returned instead
of 460 px). Step 2 implemented. Alpha 9 build, full 139-test unit suite and
formatting check passed. Final staged native GREEN passed exact Stop/Escape
tail pixels, cancellation, repeated Stop/deadline, covered source and the
existing automatic/manual/Escape safety regression. The first incomplete
attempt and corrected unsaved-close harness flow remain in the test report.
Step 5 completed: local portable ZIP contains 134 files and one EXE; release,
stage and archive EXE hashes match the tested binary. No nested archive exists.
On 2026-10-11 the user confirmed live Lark long capture followed by Stop now
matches the screen and passed. This increment is accepted for that tested case.
Delivered executable/ZIP are unchanged. No Git push/public release authorized.

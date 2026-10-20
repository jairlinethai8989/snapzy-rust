# Automatic scrolling implementation

1. Write and run failing scheduler and settings tests.
2. Implement bounded target-specific input and latched worker pause/resume.
3. Add compact launcher mode selection and controller Pause/Resume.
4. Extend native harness: product-driven scrolling, exact pixels, safety and
   cancellation. Keep manual tests explicitly manual.
5. Run format, unit tests, qualified Clippy, release build and native regression.
6. Compare ordinary capture performance with preserved alpha5 executable.
7. Update English/Thai documentation and package a fresh local alpha6 ZIP.
8. Await tester feedback; do not push or publish.

Implementation, 85 unit checks, format/qualified Clippy and release build passed.
The full native suite passed twice before a cosmetic subtitle-version fix;
final EXE, EN/TH screenshots and same-machine benchmark follow-up are tracked in
docs/testing/0.6.0-alpha.1.md. Actual Lark/Excel acceptance remains with the tester.

Completed locally on 2026-10-10: the final extracted EXE (0CC7D9...) passed all
seven native switches, EN/TH screenshots were inspected, and all six alternating
alpha5/alpha6 performance samples passed. Packaging keeps that EXE unchanged.
No Git commit/push or public release; tester acceptance is still pending.

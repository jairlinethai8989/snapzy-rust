# Automatic scrolling capture

Approved by the tester in this conversation, 2026-10-09.

Scope: SnapZy Rust only, local portable 0.6.0-alpha.1 candidate. No commit,
push, installer or public release until tester approval. Preserve manual mode.

- Launcher has an Automatic / Manual choice, Automatic by default. Persist it
  in Rust-only settings. Existing settings without this field default to Auto.
- Send one downward wheel detent to the child under the selected region's
  interior. Use a bounded WM_MOUSEWHEEL message, not global injected input;
  do not move the cursor or steal foreground after selection handoff.
- Before every command, verify original HWND/process, foreground, geometry,
  DPI and visibility, including untitled occluding windows. Held modifiers
  or mouse buttons prevent automatic input. A failed/blocked command pauses;
  never try a global-input fallback that might scroll another application.
- Wait at least 600 ms after each command, then require the existing stable
  frame gate and conservative stitch validation before another step.
- Pause / Resume, Stop (keep accepted image), Cancel (discard). Safety,
  unstable-frame and ambiguous-overlap pauses are latched until Resume.
- Three quiet, stable no-motion observations, separated by bounded retries
  and at least 1.5 seconds after each command, finish with an explicit
  'no new content confirmed' notice. This is not proof of end of document.
- Existing original pixels, frozen header/footer handling, memory limits and
  validated partial-image behavior stay unchanged. Large page-mode wheel
  settings may produce an ambiguous overlap; stop safely, use Manual.

Verification: pure scheduler/settings tests; native test-owned window receiving
actual product wheel messages, exact frozen-region output, pause/resume,
focus/occlusion safety, stop/cancel and manual-mode regression. Actual Lark,
Excel and elevated applications remain tester checks. No claim that targeted
messages are hardware wheel delivery or universally supported.

API references: https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-mousewheel
and https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-sendmessagetimeoutw

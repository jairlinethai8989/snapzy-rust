# SnapZy Rust: Direct Text Frame Resizing

Date: 2026-10-09, Asia/Bangkok.
Target: 0.3.0-alpha.2 portable Windows x64 tester candidate.
Baseline: 0.3.0-alpha.1; tester accepted its tools on 2026-10-09.
Status: written specification approved by the user on 2026-10-09.

## Requested Behavior

Selected text can be resized directly on the image in two distinct ways:

1. Drag either horizontal side handle to change the text box width. Font size
   remains unchanged, text wraps between words and the box height fits all lines.
2. Drag any corner handle to increase/decrease the font size proportionally.
   Characters are rendered again at the new size; final pixels are never a
   stretched bitmap. Corner scaling also scales an explicit wrap width, if set.

There are six handles: four corners and the midpoint of each vertical side.
Dragging inside the frame moves it. Select activates these handles; T still
opens existing text for editing. Existing multiline input, color, size controls,
selection, crop, Delete, Copy, PNG and Undo/Redo remain available.

## Alternatives and Choice

- Recommended/selected: independent side reflow and proportional corner scaling.
  It provides both requested behaviors without adding toolbar modes.
- Reflow-only: smaller change, but does not directly adjust font size.
- Free bitmap stretching: simple geometry, but distorts glyphs and reduces final
  text quality. Not selected.

No top/bottom midpoint handles are added: the box height follows its text rather
than clipping lines or introducing vertical-only glyph distortion.

## Text Layout

Extend typed text content with optional explicit wrap width in source pixels.
New text keeps the current automatic width until a side resize is committed.
The stored string retains its original characters and explicit line breaks;
automatic wrapping never inserts permanent newlines into the string.

Continue using native Windows Unicode font shaping, Segoe UI/fallback and
grayscale coverage masks. Reuse DrawTextW with DT_WORDBREAK and DT_CALCRECT for
wrapped layout, preserving DT_NOPREFIX and tabs. Keep four-pixel inner padding.
The requested outer width is clamped to 24-32767 source pixels. Preserve it when
the text is shorter, so users can deliberately leave space in the box.

An indivisible word wider than the requested width expands the effective frame
to its measured width rather than clipping or silently dropping characters.
This rule also applies if Windows treats unspaced Thai text as an indivisible
unit. Verify Thai wrapping on Windows; do not claim arbitrary word segmentation.
Explicit line breaks remain available. Calculate full height before rasterizing,
and preserve the existing per-mask and retained-history memory limits.

Source: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-drawtextw

Font sizes remain 8-128 source pixels, initially 24. Changing font size using
the existing numeric control preserves an explicit wrap width. Editing a text
string preserves its frame width and font size. Selection bounds include empty
space inside an explicitly widened frame.

## Pointer Interaction

Draw handles only for selected text. Use stable screen-space handle sizes,
scaled for actual monitor DPI, independent of image zoom. Corner handles have
hit priority over side handles and text movement. Appropriate native horizontal
and diagonal cursors indicate the action. Handles must not appear in exports.

Side dragging fixes the opposite edge and the top position. Corner dragging
fixes the opposite corner; project the pointer displacement onto the starting
diagonal to determine uniform scaling, clamp the font size and derive final
position from the actual rendered bounds. Rounding to integer font sizes may
cause small layout changes; never mirror or invert text when crossing an anchor.
Existing cropping may hide part of a text object, but must not destroy it.

Capture the mouse for the gesture. Mouse-up commits one edit. Escape, lost
capture, switching tools or closing the editor cancels it and restores the exact
starting object. No-motion or clamp-only gestures are not dirty/history edits.
Use the initial object and pointer as the reference throughout a gesture rather
than accumulating rounding errors from consecutive mouse movements.

## Responsiveness and Rendering

Keep document state unchanged during dragging. Update a lightweight frame draft
at pointer speed. Corner dragging can preview the cached mask proportionally;
side dragging previews the proposed frame and recomputes line layout on release.
This temporary display preview must never become exported text pixels.

Rasterize and compose only a committed new layout, preserving bounded scheduling,
owner/generation checks and the shared-source capture fast path. Do not render a
new full-resolution mask or clone history on every pointer movement. Reject an
over-budget/invalid layout with a localized error and retain the starting object.
Copy/Save remain gated until the current revision is composed; Undo/Redo cannot
accept a stale worker result. Maintain the accepted single-present paint behavior
and native resource cleanup.

## Scope and Follow-On Work

This candidate modifies SnapZy Rust only. It does not modify .NET SnapZy or Neo
Snap, resize other annotation types, add rotation, shadows, speech bubbles or
layers, or publish a GitHub release. Preserve the accepted alpha.1 binary for
comparison and deliver a separate alpha.2 ZIP for testing.

After this addition is verified, prepare the next scrolling-capture specification
from the existing roadmap, including Lark Sheet frozen panes, dynamic content,
stable frame detection and seam handling. That independent subsystem needs its
own scope/design review before implementation; it is not bundled into resizing.
No implementation push, tag or release without explicit delivery approval.

## Validation

Write failing tests before implementation for:

- Optional wrap-width layout, preserved explicit line breaks and unchanged input
  strings, Thai/English masks, long-word expansion, padding and allocation caps.
- All six handle hits at native/scaled image coordinates, corner precedence,
  fixed anchors, integer-size clamps, crossing anchors and unchanged gestures.
- Side resize retaining font size; corner resize retaining glyph proportions;
  editing/numeric-size changes retaining explicit wrap width.
- One history edit per drag, exact Undo/Redo, Escape/capture-loss cancellation,
  crop restoration and export composition generation safety.

Extend native smoke tests to drag actual handles, verify both resulting layouts,
Undo/Redo/cancellation and matching PNG/Windows clipboard pixels. Inspect native
Thai/English screenshots. Retain existing capture, annotation, live-stroke,
compact-toolbar, EN/TH, clipboard and GDI-resize regression checks.

Compare alternating same-machine Release runs with accepted 0.3.0-alpha.1,
separating capture first paint from text-edit composition. State actual monitor
DPI coverage; manually check 125%, 150% and 200% scaling, Thai IME and chat paste
where automation cannot establish those workflows. Update identity/docs/licenses,
package the portable ZIP, and test the extracted binary before delivery.

## Design Review Checklist

- [x] Existing typed text, native masks, selection/history and roadmap inspected.
- [x] Both resize modes approved by the user.
- [x] Reflow-only and bitmap-stretching alternatives considered.
- [x] Scope, errors, rendering, gesture semantics and tests specified.
- [x] No unresolved placeholders or conflicting width/size rules.
- [x] User review of this written specification.
- [ ] Implementation plan and test-first implementation after written review.

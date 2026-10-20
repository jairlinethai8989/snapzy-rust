# Multi-image composition

Status: approved in conversation on 2026-10-10. Rust only; no Git push.

## Scope

Add PNG, JPEG and BMP files or clipboard images as movable image objects in
the current capture editor. The captured source remains fixed underneath.
Images share the annotation stack, Delete, four ordering commands and Undo/Redo.
Select and drag to move; drag any corner to resize with the original aspect
ratio. Keep original pixels shared through history. Resizing changes geometry,
not stored pixels. Export uses the canvas pixel dimensions, not display scale.

Recommended approach: manual placement first. Automatic horizontal/vertical
layout and a full layer panel are deferred, as approved. Project persistence,
other editor tools, OCR, video and .NET applications are not part of this change.

## Interaction and Safety

- Compact top-row Add image, Paste image and Canvas icon buttons, with EN/TH tooltips/Help.
  Add image opens file browsing; Paste image or Ctrl+V pastes an image
  when the editor owns focus, never while typing in another dialog.
- Insert at the current crop origin at native size. Expand right/bottom to fit
  if needed, in the same Undo step. User may then move or resize the image.
- Canvas dialog accepts width/height in pixels, expanding only right/bottom;
  blank added space is opaque white. Shrinking is still the existing Crop tool.
  Expansion does not move objects or restore previously cropped content.
- Corner resizing uses a fixed opposite corner; reject flipped, nonfinite,
  oversized or out-of-range geometry. During drag show the destination frame;
  update pixels on release, keeping expensive work off the mouse-move path.
- Source images including Undo/Redo are bounded by the existing aggregate
  32 MiB preview-source allowance across open editors. Count unique Arc-backed
  images, including retained history. Reject oversized inputs before decode;
  recheck after decode and before insertion. Encoded input capped at 32 MiB.
- Canvas pixel buffers capped at 32 MiB and dimensions at 32767. Existing
  renderer hard limit remains 256 MiB. Fail without changing document/history.
- Decode files on a worker, one import at a time. UI remains responsive; stale
  results cannot enter a closed/reused editor. Keep original alpha for PNG.
  Clipboard accepts registered PNG or uncompressed 24/32-bit CF_DIB; unsupported
  formats report an error without deleting current work or clipboard contents.
- No automatic layout, destructive resampling or pixel-copy per history entry.

## Verification

TDD for insertion/geometry/history/resource accounting, aspect resize, crop and
expanded-canvas pixels, alpha/order/blur/export, decoder formats/bounds and
malformed clipboard buffers. Native tests cover file dialog, paste, move,
resize/cancel, canvas dialog, ordering, Undo/Redo, translated controls, compact
layout and PNG/clipboard parity. Run prior capture/editor/scroll regression
suite, formatting, qualified Clippy and compare basic startup/preview with
retained alpha7. Preserve prior build and deliver a local 0.8.0-alpha.1 ZIP.

Actual chat clipboard interoperability and mixed/high DPI remain tester checks.

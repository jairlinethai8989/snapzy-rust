# SnapZy Rust: Text, Step Numbers and Blur

Date: 2026-10-09 (Asia/Bangkok)
Target milestone: 0.3.0-alpha.1
Status: approved by the user on 2026-10-09
Accepted baseline: 0.2.0-alpha.2, tester confirmed both rendering issues resolved.

## สรุปสำหรับตรวจสเปก

เฟสนี้เพิ่มเครื่องมือ Editor ของ SnapZy Rust เท่านั้น ไม่เปลี่ยน SnapZy หรือ
Neo Snap รุ่น .NET และไม่เปลี่ยนระบบจับภาพที่ผู้ทดสอบยอมรับแล้ว

1. **ข้อความ:** คลิกตำแหน่งแล้วพิมพ์ภาษาไทย/อังกฤษ รองรับหลายบรรทัด เลือกสี
   และขนาดอักษรได้ ค่าเริ่มต้น 24 px แก้ข้อความเดิมและย้ายตำแหน่งได้
2. **เลขลำดับ:** คลิกวางเลขในวงกลม เริ่มจาก 1 แล้วนับอัตโนมัติ ย้ายและลบได้
   เมื่อลบเลขสูงสุด เลขถัดไปนับต่อจากเลขสูงสุดที่เหลือ ไม่เกิดเลขซ้ำ
3. **เบลอ:** ใช้ไอคอนหยดน้ำ ลากพื้นที่ที่ต้องการเบลอ ปรับความแรงได้
   ขณะลากแสดงกรอบพื้นที่ เมื่อปล่อยเมาส์จึงแสดงผลเบลอ ย้ายหรือลบพื้นที่ได้
4. ทุกเครื่องมือรองรับ Undo/Redo, Crop, Copy และ PNG ที่ความละเอียดต้นฉบับ
   ใช้ภาษาอังกฤษเป็นค่าเริ่มต้น พร้อม EN/TH และคำอธิบายเครื่องมือครบ
5. รักษาการวาดแบบไม่ล้างหน้าจอก่อนแสดง และเส้นที่เรียบระหว่างลากจากรุ่นที่แก้แล้ว
   ต้องทดสอบความเร็วเทียบกับ 0.2.0-alpha.2 ก่อนส่งไฟล์ทดลอง

สีเริ่มต้นยังเป็นแดง ขนาดเส้นของเครื่องมือเดิมยังเป็น 5 px
ขนาดอักษร/วงกลมและความแรงเบลอเป็นคนละค่ากับความหนาเส้น ไม่เปลี่ยนค่าทับกัน

ยังไม่รวมเงา/ขอบอักษร กล่องคำพูด เลเยอร์ รวมหลายภาพ โครงการที่แก้ไขต่อได้
OCR จับภาพยาว และวิดีโอ สิ่งเหล่านี้อยู่ในเฟสถัดไป ไม่ได้ถูกยกเลิก
จัดส่งเป็น ZIP ทดลองแยกต่างหาก ไม่ทำตัวติดตั้งหรือเผยแพร่จนกว่าผู้ทดสอบอนุมัติ

## Scope and Alternatives

The original experiment roadmap puts incremental native editing before scrolling
capture. The accepted basic-editor milestone deliberately deferred these tools.

Recommended: implement text, numbered steps and region blur as one bounded
annotation milestone. This makes screenshots useful for explaining tasks and
concealing visual details, while keeping the accepted capture/backend unchanged.

Alternatives: complete all editor parity at once (larger rendering/input changes
and harder regression isolation), or advance directly to scrolling capture
(prioritizes long documents but requires separate virtualized/frozen-pane specs).
Neither alternative is selected in this proposed milestone.

## Document Model and History

Extend annotations with explicit typed content/parameters. Keep stable IDs,
original-image coordinates, independent window state and shared immutable source
frames. Do not encode text in point arrays or overload stroke width as font size,
step diameter or blur strength. Store text behind shared immutable ownership so
history does not duplicate strings/pixels unnecessarily. Preserve the 64-step
Undo/Redo limit, revision-aware export state and redo branching semantics.

Text defaults: Segoe UI with Windows font fallback, 24 source pixels, current
annotation color (initially red). Support font size 8-128 px and line breaks.
The standard Windows multiline edit control handles typing, selection, caret,
clipboard editing and Thai input. No custom keyboard/IME parser. Explicitly
confirm or cancel text entry; cancellation must not dirty the document. Editing
an existing text object is one history step. Empty/whitespace-only text is not
committed. Bound text input to 4096 Unicode characters and reject invalid sizes.

Step defaults: diameter 36 source pixels, current color for the circle, and
contrast-aware digits. A new step equals the maximum step value in the document
plus one, or 1 when none exist. Here document steps include those hidden by crop,
preventing duplicate numbers when crop is undone. Deleting a middle number does
not renumber other objects. Undo/Redo restores existing numbers; new steps after
undo use the current document maximum. No independent drifting step counter.
Allow diameter 20-120 px. No manual number replacement in this milestone.

Blur defaults: Gaussian strength 8 source pixels, adjustable from 2-30 px. Store
the rectangle and strength, not a destructively blurred copy of the source.
Use an established CPU image-filter implementation, with dependency features
limited to required filtering rather than unused codecs. Pin the actual version
and include its licenses during implementation.

Apply blur to the current composition beneath that object, including earlier
annotations, following existing object order. Clip strictly to its rectangle and
current crop. Moving/deleting/undoing a blur recomposes from the original source;
pixels outside the region must be unchanged. Blur is visual obfuscation, not a
guarantee that sensitive information cannot be inferred. The original source
remains in application memory; there is no project-sharing feature in this phase.

## Interaction and Native UI

Retain the current native Win32 launcher, buffered editor painting and tiny-skia
shape renderer. Add text (T), numbered-circle and droplet blur buttons with
localized tooltips, keyboard focus and Help entries. Do not change the existing
capture hotkey or introduce a WebView/framework.

Text clicks open a compact native entry dialog at an image anchor; double-click
an existing text object to edit it. Standard Select moves text, steps and blur
regions, and Delete removes the selected object. Hit-test their actual bounds,
including multiline text and the interior of a blur region. Crop preserves
objects for Undo; it must not renumber steps or discard hidden text/blur regions.

Expose tool-specific size/strength settings compactly with meaningful units and
labels. When selecting an object, settings reflect that object's parameters;
changing a selected object's color/size/strength is an undoable edit. Disable
controls not applicable to the current tool. Never steal text-field shortcuts.
Maintain stable toolbar layout at the existing compact minimum size, EN/TH and
supported DPI. Use overflow if necessary, not clipped or overlapping controls.

While dragging, existing shape tools keep antialiased live strokes. Blur displays
only a selection-region draft, not expensive filtering on every pointer move.
No idle-pointer repaint or visible background-clearing is reintroduced.

## Rendering and Responsiveness

Build antialiased grayscale text masks using native Windows font rendering, not
ClearType RGB fringes baked into PNG. Use source-resolution layout/masks for text
and numbers in the preview composition and exports. Initialize text/filtering
resources lazily: an unedited capture must retain the shared-source fast path.
Continue caching completed compositions by revision and using the shared bounded
display buffer. Do not flatten full-source pixels during each drag event.

Large composition/filter work must not block the UI/capture thread. Reuse the
existing worker/result delivery pattern; completion carries owner identity and
document revision. Discard stale or closed-window results. Account for crop and
generation changes, including Undo to an earlier revision, before accepting a
result. Limit concurrent composition work and buffer ownership to the existing
memory budget; report allocation/filter errors without losing document edits.

Copy/Save must export the current revision, not a stale displayed composition.
While composition is pending, visibly disable Copy/Save with localized working
feedback. Failed/cancelled work must not mark the document
saved. Keep existing asynchronous/atomic PNG saving and close confirmation.

## Validation and Delivery

Write failing tests before implementation for text edits/cancellation/history,
step numbering after deletion/crop/undo, blur movement/restoration/clipping,
object hit-testing and stale composition/export completion. Rendering tests must
verify unpainted pixels, crop dimensions, text/number presence, blur boundaries
and original source immutability. Validate actual Windows Thai/English glyphs
visually; a nonblank raster alone does not establish Thai layout correctness.

Extend native smoke checks for all three tools, native text entry/editing, size
controls, selection/move/delete, Copy/PNG, Undo/Redo, EN/TH and every Help/tooltip.
Retain the accepted idle/draft-stroke and resize-resource regression checks.
Inspect compact layout and large-image interactions; report actual DPI coverage.

Preserve the accepted 0.2.0-alpha.2 executable and compare repeated, alternating
Release runs on the same PC. Measure launcher, selection and first paint separately
from window polling; additionally measure annotated-image composition and input
responsiveness. Investigate repeatable regressions before delivery. Do not infer
language-level speed claims or promise fixed latency.

Update identity/docs/notices consistently to 0.3.0-alpha.1. Package a portable
Windows x64 ZIP with a tester checklist. Keep .NET repositories/settings unchanged.
No implementation push, tag or GitHub release without explicit approval for that
delivery. This specification requires user review before feature implementation.

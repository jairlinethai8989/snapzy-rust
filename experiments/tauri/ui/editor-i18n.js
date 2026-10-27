const strings = {
  en: {
    untitled: "Untitled", imageEditor: "Image editor", noImage: "No image open", ready: "Ready", working: "Working…",
    select: "Select", crop: "Crop", duplicateArea: "Duplicate area", duplicateAreaHelp: "Duplicate area: drag a rectangle to create a movable image copy.", duplicateObject: "Duplicate object", duplicateObjectHelp: "Duplicate object: press Ctrl+D to copy the selected object without changing the source.", pen: "Pen", highlighter: "Highlighter", line: "Line", arrow: "Arrow",
    rectangle: "Rectangle", ellipse: "Ellipse", filledRectangle: "Filled rectangle", filledEllipse: "Filled ellipse",
    outlineShape: "Outline shape", filledShape: "Filled shape", blur: "Blur", step: "Step number", text: "Text", callout: "Speech bubble",
    effects: "Effects", layers: "Layer order", delete: "Delete", undo: "Undo", redo: "Redo", color: "Color", size: "Size", font: "Font", background: "Background", backgroundColor: "Background color", customColor: "Additional color", recentColors: "Recent colors", paletteColors: "Color palette",
    lineStyle: "Line style", solid: "Solid", dashed: "Dashed", dotted: "Dotted", opacity: "Opacity", applyCrop: "Apply crop",
    fit: "Fit", actualSize: "Actual size", zoomIn: "Zoom in", zoomOut: "Zoom out", zoomPresets: "Zoom level", openToEdit: "Open an image to start editing", dropHint: "Choose a PNG, JPEG or BMP file",
    openImage: "Open image", canvasHint: "Ctrl + wheel to zoom · Space + drag to pan", more: "More", ocr: "Read text (OCR)",
    help: "Help", language: "Switch to Thai", copyImage: "Copy image", savePng: "Save PNG", closeDocument: "Close document",
    helpTitle: "Editor tools", close: "Close", bringFront: "Bring to front", bringForward: "Move forward", sendBackward: "Move backward",
    effectsTitle: "Object effects", shadow: "Shadow", shadowHelp: "Add a shadow behind the selected object", softness: "Softness", offsetX: "Horizontal", offsetY: "Vertical", textOutline: "Text outline", textOutlineHelp: "Outline is available for text and speech bubbles", outlineUnavailable: "Select a text object to use an outline.", effectApply: "Apply effects", effectCancel: "Cancel",
    addTextTitle: "Add text", addCalloutTitle: "Add speech bubble", textEntryLabel: "Text", textEntryPlaceholder: "Type your text",
    textEntryHint: "Supports multiple lines. The background starts transparent.", applyText: "Apply", cancelText: "Cancel",
    sendBack: "Send to back", openProject: "Open project", saveProject: "Save project", addImage: "Add image", combine: "Combine images",
    combineHint: "Select at least two open tabs. The new project keeps their images and objects editable.", combineLayout: "Layout", combineGap: "Gap (px)", vertical: "Vertical", horizontal: "Horizontal", free: "Free placement", combineSize: "New canvas", combineChooseTwo: "Select at least two images", moveUp: "Move up", moveDown: "Move down",
    closeProjectTitle: "Unsaved changes", saveBeforeClose: "This project has unsaved changes. Save before closing?", save: "Save", saveProjectAs: "Save project as…", discard: "Discard", cancel: "Cancel", projectSaved: "Project saved.",
    notAvailable: "This command will be connected in the next editor migration stage.", browserMode: "Browser preview uses a local canvas. The desktop build uses the Rust editor engine.",
    openFailed: "Could not open this image.", commandFailed: "The editor command could not be completed.", copied: "Image copied.", saved: "PNG saved.",
    copyFailed: "Could not copy the image.", saveFailed: "Could not save the PNG.", clipboardUnavailable: "Image clipboard access is unavailable.",
  },
  th: {
    untitled: "งานใหม่", imageEditor: "แก้ไขภาพ", noImage: "ยังไม่ได้เปิดภาพ", ready: "พร้อมใช้งาน", working: "กำลังทำงาน…",
    select: "เลือกวัตถุ", crop: "ครอบตัด", duplicateArea: "สำเนาพื้นที่", duplicateAreaHelp: "สำเนาพื้นที่: ลากกรอบเพื่อสร้างวัตถุภาพใหม่ที่ย้ายได้", duplicateObject: "สำเนาวัตถุ", duplicateObjectHelp: "สำเนาวัตถุ: กด Ctrl+D เพื่อคัดลอกวัตถุที่เลือกโดยต้นฉบับคงเดิม", pen: "ปากกา", highlighter: "ไฮไลต์", line: "เส้นตรง", arrow: "ลูกศร",
    rectangle: "สี่เหลี่ยมโปร่ง", ellipse: "วงรีโปร่ง", filledRectangle: "สี่เหลี่ยมทึบ", filledEllipse: "วงรีทึบ",
    outlineShape: "รูปทรงโปร่ง", filledShape: "รูปทรงทึบ", blur: "เบลอ", step: "เลขลำดับ", text: "ข้อความ", callout: "ช่องคำพูด",
    effects: "เอฟเฟกต์", layers: "ลำดับชั้น", delete: "ลบ", undo: "ย้อนกลับ", redo: "ทำซ้ำ", color: "สี", size: "ขนาด", font: "ฟอนต์", background: "พื้นหลัง", backgroundColor: "สีพื้นหลัง", customColor: "สีเพิ่มเติม", recentColors: "สีที่ใช้ล่าสุด", paletteColors: "จานสี",
    lineStyle: "รูปแบบเส้น", solid: "เส้นทึบ", dashed: "เส้นประ", dotted: "เส้นจุด", opacity: "ความโปร่งใส", applyCrop: "ใช้การครอบตัด",
    fit: "พอดี", actualSize: "ขนาดจริง", zoomIn: "ซูมเข้า", zoomOut: "ซูมออก", zoomPresets: "ระดับการซูม", openToEdit: "เปิดภาพเพื่อเริ่มแก้ไข", dropHint: "เลือกไฟล์ PNG, JPEG หรือ BMP",
    openImage: "เปิดภาพ", canvasHint: "Ctrl + ล้อเมาส์เพื่อซูม · Space + ลากเพื่อเลื่อน", more: "เมนูเพิ่มเติม", ocr: "อ่านข้อความ (OCR)",
    help: "วิธีใช้", language: "Switch to English", copyImage: "คัดลอกภาพ", savePng: "บันทึก PNG", closeDocument: "ปิดงาน",
    helpTitle: "เครื่องมือแก้ไขภาพ", close: "ปิด", bringFront: "นำขึ้นบนสุด", bringForward: "เลื่อนขึ้นหนึ่งชั้น", sendBackward: "เลื่อนลงหนึ่งชั้น",
    effectsTitle: "เอฟเฟกต์วัตถุ", shadow: "เงา", shadowHelp: "เพิ่มเงาด้านหลังวัตถุที่เลือก", softness: "ความฟุ้ง", offsetX: "แนวนอน", offsetY: "แนวตั้ง", textOutline: "ขอบอักษร", textOutlineHelp: "ใช้ขอบอักษรกับข้อความและช่องคำพูด", outlineUnavailable: "เลือกข้อความหรือช่องคำพูดเพื่อใช้ขอบอักษร", effectApply: "ใช้เอฟเฟกต์", effectCancel: "ยกเลิก",
    addTextTitle: "เพิ่มข้อความ", addCalloutTitle: "เพิ่มช่องคำพูด", textEntryLabel: "ข้อความ", textEntryPlaceholder: "พิมพ์ข้อความที่ต้องการ",
    textEntryHint: "รองรับหลายบรรทัด และพื้นหลังเริ่มต้นเป็นแบบโปร่งใส", applyText: "ยืนยัน", cancelText: "ยกเลิก",
    sendBack: "ส่งลงล่างสุด", openProject: "เปิดไฟล์งาน", saveProject: "บันทึกไฟล์งาน", addImage: "เพิ่มภาพ", combine: "รวมภาพ",
    combineHint: "เลือกอย่างน้อยสองแท็บ ภาพและวัตถุเดิมจะยังแก้ไขได้ในงานใหม่", combineLayout: "การจัดวาง", combineGap: "ระยะห่าง (px)", vertical: "แนวตั้ง", horizontal: "แนวนอน", free: "วางอิสระ", combineSize: "ขนาดงานใหม่", combineChooseTwo: "เลือกภาพอย่างน้อยสองภาพ", moveUp: "เลื่อนขึ้น", moveDown: "เลื่อนลง",
    closeProjectTitle: "มีการแก้ไขที่ยังไม่บันทึก", saveBeforeClose: "งานนี้มีการแก้ไขที่ยังไม่ได้บันทึก ต้องการบันทึกก่อนปิดหรือไม่", save: "บันทึก", saveProjectAs: "บันทึกเป็น…", discard: "ไม่บันทึก", cancel: "ยกเลิก", projectSaved: "บันทึกไฟล์งานแล้ว",
    notAvailable: "คำสั่งนี้จะเชื่อมต่อในขั้นถัดไปของการย้าย Editor", browserMode: "หน้าทดลองในเบราว์เซอร์ใช้ canvas ภายใน ส่วนโปรแกรม Desktop ใช้แกน Editor ของ Rust",
    openFailed: "ไม่สามารถเปิดภาพนี้ได้", commandFailed: "ไม่สามารถทำคำสั่งแก้ไขได้", copied: "คัดลอกภาพแล้ว", saved: "บันทึก PNG แล้ว",
    copyFailed: "ไม่สามารถคัดลอกภาพได้", saveFailed: "ไม่สามารถบันทึก PNG ได้", clipboardUnavailable: "ระบบไม่อนุญาตให้คัดลอกภาพลงคลิปบอร์ด",
  },
};

export function editorText(language, key) {
  return strings[language]?.[key] ?? strings.en[key] ?? key;
}

export const toolTextKey = Object.freeze({
  Select: "select", Crop: "crop", DuplicateArea: "duplicateArea", Pen: "pen", Highlighter: "highlighter", Line: "line", Arrow: "arrow",
  Rectangle: "rectangle", Ellipse: "ellipse", FilledRectangle: "filledRectangle", FilledEllipse: "filledEllipse",
  Blur: "blur", Step: "step", Text: "text", Callout: "callout", DuplicateObject: "duplicateObject",
});

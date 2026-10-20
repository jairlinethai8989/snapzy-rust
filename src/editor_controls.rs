use crate::{
    editor::{Order, Style, Tool},
    native_icons::{self, Command as IconCommand, VisualState},
    ui::wide,
};
use std::{
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Controls::*, HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
};

pub const SELECT: usize = 601;
pub const CROP: usize = 602;
pub const PEN: usize = 603;
pub const LINE: usize = 604;
pub const ARROW: usize = 605;
pub const RECTANGLE: usize = 606;
pub const UNDO: usize = 607;
pub const REDO: usize = 608;
pub const DELETE: usize = 609;
pub const COLOR: usize = 610;
pub const WIDTH: usize = 611;
pub const CONFIRM: usize = 612;
pub const CANCEL: usize = 613;
pub const LANG: usize = 614;
pub const HELP: usize = 615;
pub const UNIT: i32 = 616;
pub const TEXT: usize = 617;
pub const STEP: usize = 618;
pub const BLUR: usize = 619;
pub const BACKGROUND: usize = 620;
pub const BACK_COLOR: usize = 621;
pub const STROKE: usize = 622;
pub const STROKE_UNIT: i32 = 623;
pub const CALLOUT: usize = 624;
pub const FRONT: usize = 625;
pub const FORWARD: usize = 626;
pub const BACKWARD: usize = 627;
pub const BACK: usize = 628;
pub const ADD_IMAGE: usize = 629;
pub const PASTE_IMAGE: usize = 630;
pub const CANVAS: usize = 631;
pub const OCR: usize = 632;
pub const HIGHLIGHTER: usize = 633;
pub const OPACITY: usize = 634;
pub const OPACITY_UNIT: i32 = 635;
pub const ELLIPSE: usize = 636;
pub const FILLED_RECTANGLE: usize = 637;
pub const FILLED_ELLIPSE: usize = 638;
pub const PATTERN: usize = 639;
pub const EFFECTS: usize = 640;
pub const PROJECT_OPEN: usize = 641;
pub const PROJECT_SAVE: usize = 642;
pub const COMBINE: usize = 643;
pub const OWNER: usize = 644;
pub const BORDER_COLOR: usize = 645;
pub const MORE: usize = 646;
pub const LAYERS: usize = 647;
pub const ORDER_BUTTONS: [(usize, Order); 4] = [
    (FRONT, Order::Front),
    (FORWARD, Order::Forward),
    (BACKWARD, Order::Backward),
    (BACK, Order::Back),
];
pub const BUTTONS: [usize; 37] = [
    COMBINE,
    OWNER,
    PROJECT_OPEN,
    PROJECT_SAVE,
    EFFECTS,
    SELECT,
    CROP,
    PEN,
    HIGHLIGHTER,
    LINE,
    ARROW,
    RECTANGLE,
    FILLED_RECTANGLE,
    PATTERN,
    TEXT,
    STEP,
    BLUR,
    UNDO,
    REDO,
    DELETE,
    COLOR,
    CONFIRM,
    CANCEL,
    HELP,
    CALLOUT,
    BACK_COLOR,
    BORDER_COLOR,
    FRONT,
    FORWARD,
    BACKWARD,
    BACK,
    ADD_IMAGE,
    PASTE_IMAGE,
    CANVAS,
    OCR,
    MORE,
    LAYERS,
];
pub fn is_command(id: usize) -> bool {
    (BUTTONS.contains(&id) && !matches!(id, MORE | LAYERS))
        || matches!(id, WIDTH | BACKGROUND | STROKE | OPACITY)
}

pub fn icon_command(id: usize) -> Option<IconCommand> {
    Some(match id {
        COMBINE => IconCommand::Combine,
        OWNER => IconCommand::SelectOwner,
        PROJECT_OPEN => IconCommand::OpenProject,
        PROJECT_SAVE => IconCommand::SaveProject,
        EFFECTS => IconCommand::Effects,
        SELECT => IconCommand::Select,
        CROP => IconCommand::Crop,
        PEN => IconCommand::Pen,
        HIGHLIGHTER => IconCommand::Highlighter,
        LINE => IconCommand::Line,
        ARROW => IconCommand::Arrow,
        RECTANGLE => IconCommand::OutlineShape,
        FILLED_RECTANGLE => IconCommand::FilledShape,
        PATTERN => IconCommand::StrokePattern,
        TEXT => IconCommand::Text,
        STEP => IconCommand::Step,
        BLUR => IconCommand::Blur,
        UNDO => IconCommand::Undo,
        REDO => IconCommand::Redo,
        DELETE => IconCommand::Delete,
        COLOR => IconCommand::Color,
        CONFIRM => IconCommand::ApplyCrop,
        CANCEL => IconCommand::Cancel,
        HELP => IconCommand::Help,
        CALLOUT => IconCommand::Callout,
        BACK_COLOR => IconCommand::BackgroundColor,
        BORDER_COLOR => IconCommand::BorderColor,
        FRONT | FORWARD | BACKWARD | BACK => IconCommand::Layer,
        ADD_IMAGE => IconCommand::AddImage,
        PASTE_IMAGE => IconCommand::PasteImage,
        CANVAS => IconCommand::Canvas,
        OCR => IconCommand::Ocr,
        LANG => IconCommand::Language,
        MORE => IconCommand::More,
        LAYERS => IconCommand::Layer,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouped_descriptions_follow_variant_and_language() {
        for thai in [false, true] {
            let choices = [Tool::Ellipse, Tool::FilledRectangle];
            assert!(
                description(RECTANGLE, thai, choices).contains(shape_name(Tool::Ellipse, thai))
            );
            assert!(
                description(FILLED_RECTANGLE, thai, choices)
                    .contains(shape_name(Tool::FilledRectangle, thai))
            );
            assert_ne!(
                description(RECTANGLE, thai, choices),
                description(FILLED_RECTANGLE, thai, choices)
            );
        }
        assert_ne!(
            shape_name(Tool::Ellipse, false),
            shape_name(Tool::Ellipse, true)
        );
    }

    #[test]
    fn shapes_use_two_toolbar_buttons_instead_of_four() {
        assert!(BUTTONS.contains(&RECTANGLE));
        assert!(BUTTONS.contains(&FILLED_RECTANGLE));
        assert!(!BUTTONS.contains(&ELLIPSE));
        assert!(!BUTTONS.contains(&FILLED_ELLIPSE));
    }

    #[test]
    fn every_editor_control_routes_without_a_numeric_upper_bound() {
        for id in BUTTONS
            .into_iter()
            .filter(|id| !matches!(*id, MORE | LAYERS))
            .chain([WIDTH, BACKGROUND, STROKE, OPACITY])
        {
            assert!(is_command(id), "unrouted editor control {id}");
        }
        for id in [
            LANG,
            MORE,
            LAYERS,
            UNIT as usize,
            STROKE_UNIT as usize,
            OPACITY_UNIT as usize,
            105,
            999,
        ] {
            assert!(!is_command(id), "unexpected editor command {id}");
        }
    }

    #[test]
    fn every_visible_editor_button_maps_to_a_native_icon_command() {
        for id in BUTTONS {
            assert!(icon_command(id).is_some(), "missing icon mapping for {id}");
        }
        assert!(icon_command(LANG).is_some());
        assert!(icon_command(999).is_none());
    }

    #[test]
    fn compact_menu_buttons_are_visible_but_route_through_their_own_menus() {
        assert!(BUTTONS.contains(&MORE));
        assert!(BUTTONS.contains(&LAYERS));
        assert!(!is_command(MORE));
        assert!(!is_command(LAYERS));
    }

    #[test]
    fn compact_menu_labels_are_short_and_localized() {
        for id in [
            ADD_IMAGE,
            PASTE_IMAGE,
            CANVAS,
            EFFECTS,
            COMBINE,
            PROJECT_OPEN,
            PROJECT_SAVE,
            FRONT,
            FORWARD,
            BACKWARD,
            BACK,
        ] {
            let english = menu_label(id, false);
            let thai = menu_label(id, true);
            assert!(!english.is_empty());
            assert!(!thai.is_empty());
            assert_ne!(english, thai);
            assert!(english.chars().count() <= 28, "label too long: {english}");
        }
    }
}

pub fn tool(id: usize) -> Option<Tool> {
    Some(match id {
        SELECT => Tool::Select,
        CROP => Tool::Crop,
        PEN => Tool::Pen,
        HIGHLIGHTER => Tool::Highlighter,
        LINE => Tool::Line,
        ARROW => Tool::Arrow,
        RECTANGLE => Tool::Rectangle,
        ELLIPSE => Tool::Ellipse,
        FILLED_RECTANGLE => Tool::FilledRectangle,
        FILLED_ELLIPSE => Tool::FilledEllipse,
        TEXT => Tool::Text,
        CALLOUT => Tool::Callout,
        STEP => Tool::Step,
        BLUR => Tool::Blur,
        _ => return None,
    })
}
pub fn label(id: usize, thai: bool) -> &'static str {
    match (id, thai) {
        (COMBINE, false) => "Combine open images into editable work",
        (COMBINE, true) => "รวมภาพที่เปิดอยู่เป็นงานที่แก้ไขต่อได้",
        (OWNER, false) => "Select owning image / select ancestor",
        (OWNER, true) => "เลือกภาพเจ้าของ / เลือกภาพชั้นนอก",
        (PROJECT_OPEN, false) => "Open image, .snapzy or compatible .NET .neosnap project (Ctrl+O)",
        (PROJECT_OPEN, true) => "เปิดภาพ ไฟล์ .snapzy หรือ .neosnap จาก .NET ที่รองรับ (Ctrl+O)",
        (PROJECT_SAVE, false) => "Save project / Save As (Ctrl+S / Ctrl+Shift+S)",
        (PROJECT_SAVE, true) => "บันทึกงาน / บันทึกเป็น (Ctrl+S / Ctrl+Shift+S)",
        (EFFECTS, false) => "Effects / object shadow and text outline",
        (EFFECTS, true) => "เอฟเฟกต์ / เงาวัตถุและขอบอักษร",
        (ADD_IMAGE, false) => "Add image from file (PNG / JPEG / BMP)",
        (ADD_IMAGE, true) => "เพิ่มภาพจากไฟล์ (PNG / JPEG / BMP)",
        (PASTE_IMAGE, false) => "Paste image (Ctrl+V)",
        (PASTE_IMAGE, true) => "วางภาพ (Ctrl+V)",
        (CANVAS, false) => "Expand canvas (pixels)",
        (CANVAS, true) => "ขยายพื้นที่ทำงาน (พิกเซล)",
        (OCR, false) => "Read text from image (OCR)",
        (OCR, true) => "อ่านข้อความจากภาพ (OCR)",
        (SELECT, false) => "Select / move / resize shapes, text and images",
        (SELECT, true) => "เลือก / ย้าย / ปรับขนาดรูปทรง ข้อความ และภาพ",
        (CROP, false) => "Crop selected image, or crop workspace",
        (CROP, true) => "ตัดเฉพาะภาพที่เลือก หรือตัดพื้นที่งาน",
        (PEN, false) => "Pen",
        (PEN, true) => "ปากกา",
        (HIGHLIGHTER, false) => "Highlighter / translucent freehand mark",
        (HIGHLIGHTER, true) => "ไฮไลต์ / เน้นข้อความด้วยเส้นโปร่งใส",
        (OPACITY, false) => "Object opacity 0-100%",
        (OPACITY, true) => "ความเข้มวัตถุ 0-100%",
        (LINE, false) => "Line / drag midpoint to bend after drawing",
        (LINE, true) => "เส้น / ลากจุดกลางดัดโค้งได้ทันทีหลังวาด",
        (ARROW, false) => "Arrow / drag midpoint to bend after drawing",
        (ARROW, true) => "ลูกศร / ลากจุดกลางดัดโค้งได้ทันทีหลังวาด",
        (RECTANGLE, false) => "Outline shapes",
        (RECTANGLE, true) => "รูปทรงโปร่ง",
        (ELLIPSE, false) => "Ellipse outline",
        (ELLIPSE, true) => "กรอบวงรี",
        (FILLED_RECTANGLE, false) => "Filled shapes",
        (FILLED_RECTANGLE, true) => "รูปทรงทึบ",
        (FILLED_ELLIPSE, false) => "Filled ellipse",
        (FILLED_ELLIPSE, true) => "วงรีทึบ",
        (PATTERN, false) => "Stroke pattern: solid / dashed / dotted",
        (PATTERN, true) => "รูปแบบเส้น: ทึบ / ประ / จุด",
        (TEXT, false) => "Text / double-click to edit / Select to resize",
        (TEXT, true) => "ข้อความ / ดับเบิลคลิกแก้ไข / ใช้เลือกเพื่อปรับกรอบ",
        (CALLOUT, false) => "Speech bubble / Select to drag tail or resize / double-click to edit",
        (CALLOUT, true) => "ช่องคำพูด / ใช้เลือกเพื่อลากหางหรือปรับกรอบ / ดับเบิลคลิกแก้ไข",
        (BACKGROUND, false) => "Text background",
        (BACKGROUND, true) => "พื้นหลังข้อความ",
        (BACK_COLOR, false) => "Background color (kept when disabled)",
        (BACK_COLOR, true) => "สีพื้นหลัง (เก็บสีไว้เมื่อปิด)",
        (BORDER_COLOR, false) => "Speech bubble border color",
        (BORDER_COLOR, true) => "สีกรอบช่องคำพูด",
        (STROKE, false) => "Bubble border width",
        (STROKE, true) => "ความหนากรอบช่องคำพูด",
        (STEP, false) => "Numbered step",
        (STEP, true) => "เลขลำดับ",
        (BLUR, false) => "Blur region",
        (BLUR, true) => "เบลอพื้นที่",
        (UNDO, false) => "Undo (Ctrl+Z)",
        (UNDO, true) => "ย้อนกลับ (Ctrl+Z)",
        (REDO, false) => "Redo (Ctrl+Y)",
        (REDO, true) => "ทำซ้ำ (Ctrl+Y)",
        (DELETE, false) => "Delete selected object",
        (DELETE, true) => "ลบวัตถุที่เลือก",
        (FRONT, false) => "Bring to front",
        (FRONT, true) => "นำขึ้นบนสุด",
        (FORWARD, false) => "Bring forward one layer",
        (FORWARD, true) => "นำขึ้นหนึ่งชั้น",
        (BACKWARD, false) => "Send backward one layer",
        (BACKWARD, true) => "ส่งลงหนึ่งชั้น",
        (BACK, false) => "Send to back",
        (BACK, true) => "ส่งลงล่างสุด",
        (COLOR, false) => "Color",
        (COLOR, true) => "สี",
        (CONFIRM, false) => "Apply crop (Enter)",
        (CONFIRM, true) => "ยืนยันการตัดภาพ (Enter)",
        (CANCEL, false) => "Cancel (Escape)",
        (CANCEL, true) => "ยกเลิก (Escape)",
        (HELP, false) => "Editor help",
        (HELP, true) => "คำอธิบายเครื่องมือ",
        (WIDTH, false) => "Width 1-40 / Font 8-128 / Circle 20-120 / Blur 1-64 px",
        (WIDTH, true) => "เส้น 1-40 / อักษร 8-128 / วงกลม 20-120 / เบลอ 1-64 px",
        (LANG, false) => "Switch to Thai",
        (LANG, true) => "สลับเป็นภาษาอังกฤษ",
        (MORE, false) => "More editor commands",
        (MORE, true) => "คำสั่งเพิ่มเติม",
        (LAYERS, false) => "Layer order",
        (LAYERS, true) => "ลำดับชั้นวัตถุ",
        _ => "",
    }
}

pub fn menu_label(id: usize, thai: bool) -> &'static str {
    match (id, thai) {
        (ADD_IMAGE, false) => "Add image",
        (ADD_IMAGE, true) => "เพิ่มภาพ",
        (PASTE_IMAGE, false) => "Paste image",
        (PASTE_IMAGE, true) => "วางภาพ",
        (CANVAS, false) => "Resize workspace",
        (CANVAS, true) => "ปรับขนาดพื้นที่งาน",
        (EFFECTS, false) => "Effects",
        (EFFECTS, true) => "เอฟเฟกต์",
        (COMBINE, false) => "Combine images",
        (COMBINE, true) => "รวมภาพ",
        (PROJECT_OPEN, false) => "Open image or project",
        (PROJECT_OPEN, true) => "เปิดภาพหรือไฟล์งาน",
        (PROJECT_SAVE, false) => "Save project",
        (PROJECT_SAVE, true) => "บันทึกไฟล์งาน",
        (FRONT, false) => "Bring to front",
        (FRONT, true) => "นำขึ้นบนสุด",
        (FORWARD, false) => "Bring forward",
        (FORWARD, true) => "นำขึ้นหนึ่งชั้น",
        (BACKWARD, false) => "Send backward",
        (BACKWARD, true) => "ส่งลงหนึ่งชั้น",
        (BACK, false) => "Send to back",
        (BACK, true) => "ส่งลงล่างสุด",
        _ => label(id, thai),
    }
}
pub fn shape_name(tool: Tool, thai: bool) -> &'static str {
    match (tool, thai) {
        (Tool::Rectangle, false) => "Rectangle outline",
        (Tool::Rectangle, true) => "กรอบสี่เหลี่ยม",
        (Tool::Ellipse, false) => "Ellipse outline",
        (Tool::Ellipse, true) => "กรอบวงรี / วงกลม",
        (Tool::FilledRectangle, false) => "Filled rectangle",
        (Tool::FilledRectangle, true) => "สี่เหลี่ยมทึบ",
        (Tool::FilledEllipse, false) => "Filled ellipse",
        (Tool::FilledEllipse, true) => "วงรีทึบ / วงกลมทึบ",
        _ => "",
    }
}
pub fn description(id: usize, thai: bool, shapes: [Tool; 2]) -> String {
    let variant = match id {
        RECTANGLE => Some(shapes[0]),
        FILLED_RECTANGLE => Some(shapes[1]),
        _ => None,
    };
    if let Some(variant) = variant {
        format!(
            "{}: {} / {}",
            label(id, thai),
            shape_name(variant, thai),
            if thai {
                "คลิกเพื่อเลือก"
            } else {
                "Click to choose"
            }
        )
    } else {
        label(id, thai).into()
    }
}
pub struct Tips {
    window: HWND,
    texts: Vec<Vec<u16>>,
    context: (bool, [Tool; 2]),
}
impl Tips {
    pub fn new(owner: HWND, thai: bool) -> Self {
        unsafe {
            let window = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                TOOLTIPS_CLASSW,
                null(),
                WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
                0,
                0,
                0,
                0,
                owner,
                null_mut(),
                GetModuleHandleW(null()),
                null(),
            );
            SendMessageW(window, TTM_SETDELAYTIME, TTDT_INITIAL as usize, 400);
            SendMessageW(window, TTM_SETMAXTIPWIDTH, 0, 340);
            let mut result = Self {
                window,
                texts: vec![],
                context: (thai, [Tool::Rectangle, Tool::FilledRectangle]),
            };
            for id in BUTTONS
                .into_iter()
                .chain([WIDTH, LANG, BACKGROUND, STROKE, OPACITY])
            {
                result
                    .texts
                    .push(wide(&description(id, thai, result.context.1)));
                let mut info: TTTOOLINFOW = zeroed();
                info.cbSize = size_of::<TTTOOLINFOW>() as u32;
                info.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
                info.hwnd = owner;
                info.uId = GetDlgItem(owner, id as i32) as usize;
                info.lpszText = result.texts.last_mut().unwrap().as_mut_ptr();
                SendMessageW(window, TTM_ADDTOOLW, 0, &info as *const _ as isize);
            }
            result
        }
    }
    pub fn refresh(&mut self, owner: HWND, thai: bool, shapes: [Tool; 2]) {
        if self.context == (thai, shapes) {
            return;
        }
        self.context = (thai, shapes);
        for (index, id) in BUTTONS
            .into_iter()
            .chain([WIDTH, LANG, BACKGROUND, STROKE, OPACITY])
            .enumerate()
        {
            self.texts[index] = wide(&description(id, thai, shapes));
            unsafe {
                let mut info: TTTOOLINFOW = zeroed();
                info.cbSize = size_of::<TTTOOLINFOW>() as u32;
                info.hwnd = owner;
                info.uId = GetDlgItem(owner, id as i32) as usize;
                info.lpszText = self.texts[index].as_mut_ptr();
                SendMessageW(
                    self.window,
                    TTM_UPDATETIPTEXTW,
                    0,
                    &info as *const _ as isize,
                );
            }
        }
    }
}
impl Drop for Tips {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.window) != 0 {
                DestroyWindow(self.window);
            }
        }
    }
}

pub fn draw(item: &DRAWITEMSTRUCT, active: Tool, style: Style, shapes: [Tool; 2]) {
    unsafe {
        let id = item.CtlID as usize;
        let selected = match id {
            RECTANGLE => matches!(active, Tool::Rectangle | Tool::Ellipse),
            FILLED_RECTANGLE => active.filled(),
            _ => tool(id) == Some(active),
        };
        let disabled = item.itemState & ODS_DISABLED != 0;
        let pressed = item.itemState & ODS_SELECTED != 0;
        let command = icon_command(id);
        let visual_state = if disabled {
            VisualState::Disabled
        } else if pressed {
            VisualState::Pressed
        } else if selected {
            VisualState::Selected
        } else if item.itemState & ODS_HOTLIGHT != 0 {
            VisualState::Hover
        } else if item.itemState & ODS_FOCUS != 0 {
            VisualState::Focused
        } else if let Some(command) = command {
            match native_icons::role(command) {
                native_icons::Role::Destructive => VisualState::Destructive,
                native_icons::Role::Primary => VisualState::Primary,
                native_icons::Role::Normal => VisualState::Normal,
            }
        } else {
            VisualState::Normal
        };
        let contrast = native_icons::active_high_contrast_palette(visual_state);
        let bg = contrast
            .map(|colors| colors.background)
            .unwrap_or(if selected || pressed {
                native_icons::UI_SURFACE_HOVER
            } else {
                native_icons::UI_SURFACE
            });
        let foreground = contrast
            .map(|colors| colors.foreground)
            .unwrap_or(if disabled {
                0x00c0b8b0
            } else {
                native_icons::UI_TEXT
            });
        let brush = CreateSolidBrush(bg);
        FillRect(item.hDC, &item.rcItem, brush);
        DeleteObject(brush);
        let border =
            CreateSolidBrush(contrast.map(|colors| colors.border).unwrap_or(if selected {
                native_icons::UI_ACCENT
            } else {
                native_icons::UI_BORDER
            }));
        FrameRect(item.hDC, &item.rcItem, border);
        DeleteObject(border);
        if matches!(item.CtlID as usize, COLOR | BACK_COLOR | BORDER_COLOR) {
            let mut r = item.rcItem;
            let inset = 6 * GetDpiForWindow(item.hwndItem).max(96) as i32 / 96;
            r.left += inset;
            r.top += inset;
            r.right -= inset;
            r.bottom -= inset;
            let [r1, g, b] = style.rgb;
            let brush = CreateSolidBrush(r1 as u32 | ((g as u32) << 8) | ((b as u32) << 16));
            FillRect(item.hDC, &r, brush);
            DeleteObject(brush);
        } else if let Some(command) = command {
            let dpi = GetDpiForWindow(item.hwndItem).max(96) as i32;
            let mut r = item.rcItem;
            let grouped = matches!(id, RECTANGLE | FILLED_RECTANGLE);
            if grouped {
                r.right -= 12 * dpi / 96;
            }
            let icon = match id {
                RECTANGLE if shapes[0] == Tool::Ellipse => native_icons::Icon::Ellipse,
                FILLED_RECTANGLE if shapes[1] == Tool::FilledEllipse => {
                    native_icons::Icon::EllipseFilled
                }
                _ => native_icons::icon_for(command),
            };
            if contrast.is_some() {
                native_icons::draw_gdi_color(item.hDC, r, icon, foreground, dpi as u32, bg);
            } else {
                native_icons::draw_gdi(item.hDC, r, icon, visual_state, dpi as u32, bg);
            }
            if grouped {
                let marker = CreatePen(PS_SOLID, (dpi / 96).max(1), foreground);
                let previous = SelectObject(item.hDC, marker);
                let center = item.rcItem.right - 7 * dpi / 96;
                let y = (item.rcItem.top + item.rcItem.bottom) / 2;
                MoveToEx(item.hDC, center - 3 * dpi / 96, y - 1, null_mut());
                LineTo(item.hDC, center, y + 2 * dpi / 96);
                LineTo(item.hDC, center + 3 * dpi / 96, y - 1);
                SelectObject(item.hDC, previous);
                DeleteObject(marker);
            }
            // A boundary bar distinguishes stack endpoints from one-layer arrows.
            if matches!(item.CtlID as usize, FRONT | BACK) {
                let y = if item.CtlID as usize == FRONT {
                    item.rcItem.top + 6 * dpi / 96
                } else {
                    item.rcItem.bottom - 7 * dpi / 96
                };
                let pen = CreatePen(PS_SOLID, (dpi / 96).max(1), foreground);
                let previous = SelectObject(item.hDC, pen);
                MoveToEx(item.hDC, item.rcItem.left + 10 * dpi / 96, y, null_mut());
                LineTo(item.hDC, item.rcItem.right - 10 * dpi / 96, y);
                SelectObject(item.hDC, previous);
                DeleteObject(pen);
            }
        }
        if item.itemState & ODS_FOCUS != 0 {
            let mut r = item.rcItem;
            r.left += 3;
            r.right -= 3;
            r.top += 3;
            r.bottom -= 3;
            DrawFocusRect(item.hDC, &r);
        }
    }
}

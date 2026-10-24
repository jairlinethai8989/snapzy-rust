#![cfg_attr(not(test), allow(dead_code))]

use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    mem::{size_of, zeroed},
    sync::Arc,
};
use tiny_skia::{LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};
use windows_sys::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, COLOR_BTNFACE, COLOR_BTNTEXT, COLOR_GRAYTEXT,
        COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_MENUTEXT, COLOR_WINDOWFRAME, CreateDIBSection,
        DIB_RGB_COLORS, DeleteObject, GetSysColor, HBITMAP, HDC, SRCCOPY, StretchDIBits,
    },
    UI::{
        Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW},
        WindowsAndMessaging::{
            HMENU, MENUITEMINFOW, MIIM_BITMAP, SPI_GETHIGHCONTRAST, SetMenuItemInfoW,
            SystemParametersInfoW,
        },
    },
};

// COLORREF values are stored as 0x00BBGGRR. Keep the palette in one place so
// the launcher and editor use the same visual language at every DPI.
pub const UI_SURFACE: u32 = 0x00fffbf8;
pub const UI_SURFACE_ALT: u32 = 0x00fdf6f0;
pub const UI_SURFACE_HOVER: u32 = 0x00fff0e6;
pub const UI_ACCENT: u32 = 0x00cf6214;
pub const UI_ACCENT_DARK: u32 = 0x00a54b0f;
pub const UI_BORDER: u32 = 0x00eeddd0;
pub const UI_TEXT: u32 = 0x00483121;
pub const UI_MUTED: u32 = 0x008c7564;
const ICON_STROKE_WIDTH: f32 = 1.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Command {
    AreaCapture,
    WindowCapture,
    DesktopCapture,
    ScrollingCapture,
    VideoCapture,
    Delay,
    ScrollingMode,
    OpenDestination,
    OpenImage,
    Language,
    More,
    Hotkeys,
    About,
    HideToTray,
    Combine,
    SelectOwner,
    OpenProject,
    SaveProject,
    Effects,
    Select,
    Crop,
    Pen,
    Highlighter,
    Line,
    Arrow,
    OutlineShape,
    FilledShape,
    StrokePattern,
    Text,
    Step,
    Blur,
    Undo,
    Redo,
    Delete,
    Color,
    ApplyCrop,
    Cancel,
    Help,
    Callout,
    BackgroundColor,
    BorderColor,
    Layer,
    AddImage,
    PasteImage,
    Canvas,
    Ocr,
    Copy,
    SavePng,
}

pub const VISIBLE_COMMANDS: [Command; 48] = [
    Command::AreaCapture,
    Command::WindowCapture,
    Command::DesktopCapture,
    Command::ScrollingCapture,
    Command::VideoCapture,
    Command::Delay,
    Command::ScrollingMode,
    Command::OpenDestination,
    Command::OpenImage,
    Command::Language,
    Command::More,
    Command::Hotkeys,
    Command::About,
    Command::HideToTray,
    Command::Combine,
    Command::SelectOwner,
    Command::OpenProject,
    Command::SaveProject,
    Command::Effects,
    Command::Select,
    Command::Crop,
    Command::Pen,
    Command::Highlighter,
    Command::Line,
    Command::Arrow,
    Command::OutlineShape,
    Command::FilledShape,
    Command::StrokePattern,
    Command::Text,
    Command::Step,
    Command::Blur,
    Command::Undo,
    Command::Redo,
    Command::Delete,
    Command::Color,
    Command::ApplyCrop,
    Command::Cancel,
    Command::Help,
    Command::Callout,
    Command::BackgroundColor,
    Command::BorderColor,
    Command::Layer,
    Command::AddImage,
    Command::PasteImage,
    Command::Canvas,
    Command::Ocr,
    Command::Copy,
    Command::SavePng,
];

pub fn visible_commands() -> &'static [Command] {
    &VISIBLE_COMMANDS
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Scan,
    AppWindow,
    Monitor,
    GalleryVerticalEnd,
    ScrollCapture,
    Video,
    Timer,
    PanelTop,
    ImagePlus,
    Languages,
    Ellipsis,
    Keyboard,
    Info,
    Moon,
    Layers,
    MousePointer,
    Target,
    FolderOpen,
    Save,
    Sliders,
    Crop,
    Pencil,
    Highlighter,
    Slash,
    ArrowUpRight,
    ArrowUp,
    ArrowDown,
    ArrowUpToLine,
    ArrowDownToLine,
    Square,
    Blend,
    ShapeFilled,
    Ellipse,
    EllipseFilled,
    CircleOne,
    ListMinus,
    DashPattern,
    Type,
    Droplet,
    Undo,
    Redo,
    Trash,
    Palette,
    Check,
    X,
    CircleHelp,
    MessageSquare,
    Clipboard,
    Maximize,
    ScanText,
    Copy,
    Download,
}

pub fn icon_for(command: Command) -> Icon {
    match command {
        Command::AreaCapture => Icon::Scan,
        Command::WindowCapture => Icon::AppWindow,
        Command::DesktopCapture => Icon::Monitor,
        Command::ScrollingCapture => Icon::ScrollCapture,
        Command::VideoCapture => Icon::Video,
        Command::Delay => Icon::Timer,
        Command::ScrollingMode => Icon::Sliders,
        Command::OpenDestination => Icon::PanelTop,
        Command::OpenImage | Command::AddImage => Icon::ImagePlus,
        Command::Language => Icon::Languages,
        Command::More => Icon::Ellipsis,
        Command::Hotkeys => Icon::Keyboard,
        Command::About => Icon::Info,
        Command::HideToTray => Icon::Moon,
        Command::Combine | Command::Layer => Icon::Layers,
        Command::SelectOwner => Icon::Target,
        Command::Select => Icon::MousePointer,
        Command::OpenProject => Icon::FolderOpen,
        Command::SaveProject => Icon::Save,
        Command::Effects => Icon::Sliders,
        Command::Crop => Icon::Crop,
        Command::Pen => Icon::Pencil,
        Command::Highlighter => Icon::Highlighter,
        Command::Line => Icon::Slash,
        Command::Arrow => Icon::ArrowUpRight,
        Command::OutlineShape => Icon::Square,
        Command::FilledShape => Icon::ShapeFilled,
        Command::StrokePattern => Icon::DashPattern,
        Command::Text => Icon::Type,
        Command::Step => Icon::CircleOne,
        Command::Blur => Icon::Droplet,
        Command::Undo => Icon::Undo,
        Command::Redo => Icon::Redo,
        Command::Delete => Icon::Trash,
        Command::Color | Command::BackgroundColor | Command::BorderColor => Icon::Palette,
        Command::ApplyCrop => Icon::Check,
        Command::Cancel => Icon::X,
        Command::Help => Icon::CircleHelp,
        Command::Callout => Icon::MessageSquare,
        Command::PasteImage => Icon::Clipboard,
        Command::Canvas => Icon::Maximize,
        Command::Ocr => Icon::ScanText,
        Command::Copy => Icon::Copy,
        Command::SavePng => Icon::Download,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Normal,
    Primary,
    Destructive,
}

pub fn role(command: Command) -> Role {
    match command {
        Command::AreaCapture | Command::SavePng => Role::Primary,
        Command::Delete => Role::Destructive,
        _ => Role::Normal,
    }
}

pub fn accessible_name(command: Command, thai: bool) -> &'static str {
    match (command, thai) {
        (Command::AreaCapture, false) => "Capture area",
        (Command::AreaCapture, true) => "จับภาพพื้นที่",
        (Command::WindowCapture, false) => "Capture window",
        (Command::WindowCapture, true) => "จับภาพหน้าต่าง",
        (Command::DesktopCapture, false) => "Capture desktop",
        (Command::DesktopCapture, true) => "จับภาพเดสก์ท็อป",
        (Command::ScrollingCapture, false) => "Scrolling capture",
        (Command::ScrollingCapture, true) => "จับภาพยาว",
        (Command::VideoCapture, false) => "Record video",
        (Command::VideoCapture, true) => "บันทึกวิดีโอ",
        (Command::Delay, false) => "Capture delay",
        (Command::Delay, true) => "หน่วงเวลาก่อนจับภาพ",
        (Command::ScrollingMode, false) => "Scrolling capture mode",
        (Command::ScrollingMode, true) => "โหมดจับภาพยาว",
        (Command::OpenDestination, false) => "Open destination",
        (Command::OpenDestination, true) => "ตำแหน่งเปิดภาพ",
        (Command::OpenImage, false) => "Open image",
        (Command::OpenImage, true) => "เปิดภาพ",
        (Command::Language, false) => "Switch to Thai",
        (Command::Language, true) => "สลับเป็นภาษาอังกฤษ",
        (Command::More, false) => "More options",
        (Command::More, true) => "ตัวเลือกเพิ่มเติม",
        (Command::Hotkeys, false) => "Hotkey settings",
        (Command::Hotkeys, true) => "ตั้งค่าคีย์ลัด",
        (Command::About, false) => "About SnapZy",
        (Command::About, true) => "เกี่ยวกับ SnapZy",
        (Command::HideToTray, false) => "Minimize to system tray",
        (Command::HideToTray, true) => "พักใน System Tray",
        (Command::Combine, false) => "Combine images",
        (Command::Combine, true) => "รวมภาพ",
        (Command::SelectOwner, false) => "Select owning image",
        (Command::SelectOwner, true) => "เลือกภาพเจ้าของ",
        (Command::OpenProject, false) => "Open image or project",
        (Command::OpenProject, true) => "เปิดภาพหรือไฟล์งาน",
        (Command::SaveProject, false) => "Save editable project",
        (Command::SaveProject, true) => "บันทึกไฟล์งานที่แก้ไขต่อได้",
        (Command::Effects, false) => "Effects",
        (Command::Effects, true) => "เอฟเฟกต์",
        (Command::Select, false) => "Select object",
        (Command::Select, true) => "เลือกวัตถุ",
        (Command::Crop, false) => "Crop",
        (Command::Crop, true) => "ครอบตัด",
        (Command::Pen, false) => "Pen",
        (Command::Pen, true) => "ปากกา",
        (Command::Highlighter, false) => "Highlighter",
        (Command::Highlighter, true) => "ไฮไลต์",
        (Command::Line, false) => "Line",
        (Command::Line, true) => "เส้น",
        (Command::Arrow, false) => "Arrow",
        (Command::Arrow, true) => "ลูกศร",
        (Command::OutlineShape, false) => "Outline shapes",
        (Command::OutlineShape, true) => "รูปทรงโปร่ง",
        (Command::FilledShape, false) => "Filled shapes",
        (Command::FilledShape, true) => "รูปทรงทึบ",
        (Command::StrokePattern, false) => "Stroke pattern",
        (Command::StrokePattern, true) => "รูปแบบเส้น",
        (Command::Text, false) => "Text",
        (Command::Text, true) => "ข้อความ",
        (Command::Step, false) => "Numbered step",
        (Command::Step, true) => "เลขลำดับ",
        (Command::Blur, false) => "Blur region",
        (Command::Blur, true) => "เบลอพื้นที่",
        (Command::Undo, false) => "Undo",
        (Command::Undo, true) => "ย้อนกลับ",
        (Command::Redo, false) => "Redo",
        (Command::Redo, true) => "ทำซ้ำ",
        (Command::Delete, false) => "Delete selected object",
        (Command::Delete, true) => "ลบวัตถุที่เลือก",
        (Command::Color, false) => "Drawing color",
        (Command::Color, true) => "สีวาด",
        (Command::ApplyCrop, false) => "Apply crop",
        (Command::ApplyCrop, true) => "ยืนยันการครอบตัด",
        (Command::Cancel, false) => "Cancel",
        (Command::Cancel, true) => "ยกเลิก",
        (Command::Help, false) => "Editor help",
        (Command::Help, true) => "คำอธิบายเครื่องมือ",
        (Command::Callout, false) => "Speech bubble",
        (Command::Callout, true) => "ช่องคำพูด",
        (Command::BackgroundColor, false) => "Text background color",
        (Command::BackgroundColor, true) => "สีพื้นหลังข้อความ",
        (Command::BorderColor, false) => "Speech bubble border color",
        (Command::BorderColor, true) => "สีกรอบช่องคำพูด",
        (Command::Layer, false) => "Layer order",
        (Command::Layer, true) => "ลำดับชั้นวัตถุ",
        (Command::AddImage, false) => "Add image",
        (Command::AddImage, true) => "เพิ่มภาพ",
        (Command::PasteImage, false) => "Paste image",
        (Command::PasteImage, true) => "วางภาพ",
        (Command::Canvas, false) => "Resize workspace",
        (Command::Canvas, true) => "ปรับขนาดพื้นที่งาน",
        (Command::Ocr, false) => "Read text from image",
        (Command::Ocr, true) => "อ่านข้อความจากภาพ",
        (Command::Copy, false) => "Copy image",
        (Command::Copy, true) => "คัดลอกภาพ",
        (Command::SavePng, false) => "Save PNG",
        (Command::SavePng, true) => "บันทึก PNG",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VisualState {
    Normal,
    Hover,
    Selected,
    Pressed,
    Focused,
    Disabled,
    Destructive,
    Primary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemColors {
    pub button_face: u32,
    pub button_text: u32,
    pub gray_text: u32,
    pub highlight: u32,
    pub highlight_text: u32,
    pub window_frame: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonPalette {
    pub background: u32,
    pub foreground: u32,
    pub border: u32,
}

pub fn high_contrast_palette(state: VisualState, colors: SystemColors) -> ButtonPalette {
    match state {
        VisualState::Disabled => ButtonPalette {
            background: colors.button_face,
            foreground: colors.gray_text,
            border: colors.window_frame,
        },
        VisualState::Hover
        | VisualState::Selected
        | VisualState::Pressed
        | VisualState::Focused
        | VisualState::Primary => ButtonPalette {
            background: colors.highlight,
            foreground: colors.highlight_text,
            border: colors.highlight,
        },
        VisualState::Normal | VisualState::Destructive => ButtonPalette {
            background: colors.button_face,
            foreground: colors.button_text,
            border: colors.window_frame,
        },
    }
}

pub fn active_high_contrast_palette(state: VisualState) -> Option<ButtonPalette> {
    let mut contrast = HIGHCONTRASTW {
        cbSize: size_of::<HIGHCONTRASTW>() as u32,
        ..HIGHCONTRASTW::default()
    };
    let active = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            contrast.cbSize,
            (&mut contrast as *mut HIGHCONTRASTW).cast(),
            0,
        ) != 0
            && contrast.dwFlags & HCF_HIGHCONTRASTON != 0
    };
    active.then(|| {
        high_contrast_palette(
            state,
            SystemColors {
                button_face: unsafe { GetSysColor(COLOR_BTNFACE) },
                button_text: unsafe { GetSysColor(COLOR_BTNTEXT) },
                gray_text: unsafe { GetSysColor(COLOR_GRAYTEXT) },
                highlight: unsafe { GetSysColor(COLOR_HIGHLIGHT) },
                highlight_text: unsafe { GetSysColor(COLOR_HIGHLIGHTTEXT) },
                window_frame: unsafe { GetSysColor(COLOR_WINDOWFRAME) },
            },
        )
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconBitmap {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl IconBitmap {
    pub fn alpha_bounds(&self) -> Option<(u32, u32, u32, u32)> {
        let mut left = self.width;
        let mut top = self.height;
        let mut right = 0;
        let mut bottom = 0;
        let mut found = false;
        for y in 0..self.height {
            for x in 0..self.width {
                if self.pixels[((y * self.width + x) * 4 + 3) as usize] != 0 {
                    found = true;
                    left = left.min(x);
                    top = top.min(y);
                    right = right.max(x + 1);
                    bottom = bottom.max(y + 1);
                }
            }
        }
        found.then_some((left, top, right, bottom))
    }
}

fn icon_color(state: VisualState) -> (u8, u8, u8, u8) {
    match state {
        VisualState::Normal => (34, 48, 71, 255),
        VisualState::Hover | VisualState::Focused => (21, 94, 239, 255),
        VisualState::Selected | VisualState::Primary => (10, 109, 223, 255),
        VisualState::Pressed => (12, 70, 171, 255),
        VisualState::Disabled => (164, 172, 184, 170),
        VisualState::Destructive => (220, 38, 38, 255),
    }
}

fn line(builder: &mut PathBuilder, x1: f32, y1: f32, x2: f32, y2: f32) {
    builder.move_to(x1, y1);
    builder.line_to(x2, y2);
}

fn polyline(builder: &mut PathBuilder, points: &[(f32, f32)]) {
    if let Some(&(x, y)) = points.first() {
        builder.move_to(x, y);
        for &(x, y) in &points[1..] {
            builder.line_to(x, y);
        }
    }
}

fn rect(builder: &mut PathBuilder, x: f32, y: f32, width: f32, height: f32) {
    builder.move_to(x, y);
    builder.line_to(x + width, y);
    builder.line_to(x + width, y + height);
    builder.line_to(x, y + height);
    builder.close();
}

fn circle(builder: &mut PathBuilder, x: f32, y: f32, radius: f32) {
    builder.push_circle(x, y, radius);
}

fn corner_scan(builder: &mut PathBuilder) {
    polyline(builder, &[(8.0, 3.0), (5.0, 3.0), (3.0, 5.0), (3.0, 8.0)]);
    polyline(
        builder,
        &[(16.0, 3.0), (19.0, 3.0), (21.0, 5.0), (21.0, 8.0)],
    );
    polyline(
        builder,
        &[(21.0, 16.0), (21.0, 19.0), (19.0, 21.0), (16.0, 21.0)],
    );
    polyline(
        builder,
        &[(8.0, 21.0), (5.0, 21.0), (3.0, 19.0), (3.0, 16.0)],
    );
}

fn icon_path(icon: Icon) -> Path {
    let mut b = PathBuilder::new();
    match icon {
        Icon::Scan => corner_scan(&mut b),
        Icon::ScanText => {
            corner_scan(&mut b);
            line(&mut b, 7.0, 8.0, 17.0, 8.0);
            line(&mut b, 7.0, 12.0, 17.0, 12.0);
            line(&mut b, 7.0, 16.0, 14.0, 16.0);
        }
        Icon::AppWindow => {
            rect(&mut b, 2.5, 4.0, 19.0, 16.0);
            line(&mut b, 2.5, 8.0, 21.5, 8.0);
            line(&mut b, 7.0, 4.0, 7.0, 8.0);
            line(&mut b, 11.0, 4.0, 11.0, 8.0);
        }
        Icon::Monitor => {
            rect(&mut b, 2.0, 3.0, 20.0, 14.0);
            line(&mut b, 12.0, 17.0, 12.0, 21.0);
            line(&mut b, 8.0, 21.0, 16.0, 21.0);
        }
        Icon::GalleryVerticalEnd => {
            line(&mut b, 7.0, 2.5, 17.0, 2.5);
            line(&mut b, 5.0, 6.5, 19.0, 6.5);
            rect(&mut b, 3.0, 10.0, 18.0, 12.0);
        }
        Icon::ScrollCapture => {
            rect(&mut b, 3.0, 2.5, 13.0, 19.0);
            line(&mut b, 6.0, 7.0, 13.0, 7.0);
            line(&mut b, 6.0, 11.0, 13.0, 11.0);
            line(&mut b, 6.0, 15.0, 11.0, 15.0);
            line(&mut b, 19.0, 7.0, 19.0, 19.0);
            polyline(&mut b, &[(16.0, 16.0), (19.0, 19.0), (22.0, 16.0)]);
        }
        Icon::Video => {
            rect(&mut b, 2.0, 6.0, 14.0, 12.0);
            polyline(
                &mut b,
                &[(16.0, 10.0), (22.0, 7.0), (22.0, 17.0), (16.0, 14.0)],
            );
        }
        Icon::Timer => {
            circle(&mut b, 12.0, 14.0, 8.0);
            line(&mut b, 10.0, 2.0, 14.0, 2.0);
            line(&mut b, 12.0, 14.0, 15.0, 11.0);
        }
        Icon::PanelTop => {
            rect(&mut b, 3.0, 3.0, 18.0, 18.0);
            line(&mut b, 3.0, 9.0, 21.0, 9.0);
        }
        Icon::ImagePlus => {
            rect(&mut b, 3.0, 3.0, 18.0, 18.0);
            circle(&mut b, 8.0, 8.0, 1.5);
            polyline(
                &mut b,
                &[
                    (4.0, 19.0),
                    (10.0, 13.0),
                    (14.0, 17.0),
                    (17.0, 14.0),
                    (21.0, 18.0),
                ],
            );
            line(&mut b, 17.0, 2.0, 17.0, 8.0);
            line(&mut b, 14.0, 5.0, 20.0, 5.0);
        }
        Icon::Languages => {
            line(&mut b, 2.0, 5.0, 14.0, 5.0);
            line(&mut b, 7.0, 2.0, 7.0, 5.0);
            polyline(&mut b, &[(5.0, 8.0), (11.0, 14.0), (14.0, 10.0)]);
            polyline(&mut b, &[(11.0, 5.0), (8.0, 11.0), (4.0, 15.0)]);
            polyline(&mut b, &[(13.0, 22.0), (17.0, 12.0), (21.0, 22.0)]);
            line(&mut b, 15.0, 18.0, 19.0, 18.0);
        }
        Icon::Ellipsis => {
            circle(&mut b, 5.0, 12.0, 1.0);
            circle(&mut b, 12.0, 12.0, 1.0);
            circle(&mut b, 19.0, 12.0, 1.0);
        }
        Icon::Keyboard => {
            rect(&mut b, 2.0, 4.0, 20.0, 16.0);
            for x in [6.0, 10.0, 14.0, 18.0] {
                circle(&mut b, x, 9.0, 0.4);
                circle(&mut b, x, 13.0, 0.4);
            }
            line(&mut b, 7.0, 17.0, 17.0, 17.0);
        }
        Icon::Info => {
            circle(&mut b, 12.0, 12.0, 10.0);
            line(&mut b, 12.0, 11.0, 12.0, 17.0);
            circle(&mut b, 12.0, 7.5, 0.5);
        }
        Icon::Moon => {
            b.move_to(12.0, 3.0);
            b.cubic_to(8.0, 7.0, 9.0, 15.0, 16.0, 17.0);
            b.cubic_to(12.0, 22.0, 3.0, 19.0, 3.0, 11.0);
            b.cubic_to(3.0, 6.0, 7.0, 3.0, 12.0, 3.0);
            b.close();
        }
        Icon::Layers => {
            polyline(
                &mut b,
                &[
                    (2.0, 7.0),
                    (12.0, 2.5),
                    (22.0, 7.0),
                    (12.0, 11.5),
                    (2.0, 7.0),
                ],
            );
            polyline(&mut b, &[(2.0, 12.0), (12.0, 16.5), (22.0, 12.0)]);
            polyline(&mut b, &[(2.0, 17.0), (12.0, 21.5), (22.0, 17.0)]);
        }
        Icon::MousePointer => {
            polyline(
                &mut b,
                &[
                    (4.0, 3.0),
                    (20.0, 10.0),
                    (13.5, 12.0),
                    (11.5, 20.0),
                    (4.0, 3.0),
                ],
            );
        }
        Icon::Target => {
            circle(&mut b, 12.0, 12.0, 5.5);
            line(&mut b, 12.0, 2.0, 12.0, 6.0);
            line(&mut b, 12.0, 18.0, 12.0, 22.0);
            line(&mut b, 2.0, 12.0, 6.0, 12.0);
            line(&mut b, 18.0, 12.0, 22.0, 12.0);
        }
        Icon::FolderOpen => {
            polyline(
                &mut b,
                &[
                    (2.0, 18.0),
                    (2.0, 5.0),
                    (9.0, 5.0),
                    (11.0, 8.0),
                    (21.0, 8.0),
                    (19.0, 20.0),
                    (4.0, 20.0),
                    (2.0, 18.0),
                ],
            );
            line(&mut b, 5.0, 13.0, 21.0, 13.0);
        }
        Icon::Save => {
            polyline(
                &mut b,
                &[
                    (4.0, 3.0),
                    (17.0, 3.0),
                    (21.0, 7.0),
                    (21.0, 21.0),
                    (3.0, 21.0),
                    (3.0, 3.0),
                    (4.0, 3.0),
                ],
            );
            rect(&mut b, 7.0, 3.0, 9.0, 5.0);
            rect(&mut b, 7.0, 13.0, 10.0, 8.0);
        }
        Icon::Sliders => {
            for (y, x) in [(5.0, 15.0), (12.0, 9.0), (19.0, 16.0)] {
                line(&mut b, 3.0, y, 21.0, y);
                line(&mut b, x, y - 2.0, x, y + 2.0);
            }
        }
        Icon::Crop => {
            polyline(
                &mut b,
                &[(6.0, 2.0), (6.0, 16.0), (8.0, 18.0), (22.0, 18.0)],
            );
            polyline(
                &mut b,
                &[(18.0, 22.0), (18.0, 8.0), (16.0, 6.0), (2.0, 6.0)],
            );
        }
        Icon::Pencil => {
            polyline(
                &mut b,
                &[
                    (3.0, 21.0),
                    (5.0, 15.5),
                    (16.5, 4.0),
                    (20.0, 7.5),
                    (8.5, 19.0),
                    (3.0, 21.0),
                ],
            );
            line(&mut b, 14.5, 6.0, 18.0, 9.5);
        }
        Icon::Highlighter => {
            polyline(
                &mut b,
                &[
                    (9.0, 11.0),
                    (17.0, 3.0),
                    (22.0, 8.0),
                    (14.0, 16.0),
                    (9.0, 11.0),
                ],
            );
            polyline(
                &mut b,
                &[
                    (9.0, 11.0),
                    (6.0, 14.0),
                    (6.0, 17.0),
                    (9.0, 17.0),
                    (12.0, 14.0),
                ],
            );
            line(&mut b, 3.0, 21.0, 18.0, 21.0);
            line(&mut b, 6.0, 17.0, 4.0, 19.0);
        }
        Icon::Slash => line(&mut b, 20.0, 4.0, 4.0, 20.0),
        Icon::ArrowUpRight => {
            line(&mut b, 5.0, 19.0, 19.0, 5.0);
            polyline(&mut b, &[(9.0, 5.0), (19.0, 5.0), (19.0, 15.0)]);
        }
        Icon::ArrowUp | Icon::ArrowUpToLine => {
            line(&mut b, 12.0, 20.0, 12.0, 6.0);
            polyline(&mut b, &[(6.0, 12.0), (12.0, 6.0), (18.0, 12.0)]);
            if icon == Icon::ArrowUpToLine {
                line(&mut b, 5.0, 2.0, 19.0, 2.0);
            }
        }
        Icon::ArrowDown | Icon::ArrowDownToLine => {
            line(&mut b, 12.0, 4.0, 12.0, 18.0);
            polyline(&mut b, &[(6.0, 12.0), (12.0, 18.0), (18.0, 12.0)]);
            if icon == Icon::ArrowDownToLine {
                line(&mut b, 5.0, 22.0, 19.0, 22.0);
            }
        }
        Icon::Square => rect(&mut b, 3.0, 3.0, 18.0, 18.0),
        Icon::Blend => {
            circle(&mut b, 9.0, 9.0, 7.0);
            circle(&mut b, 15.0, 15.0, 7.0);
        }
        Icon::ShapeFilled => rect(&mut b, 4.0, 4.0, 16.0, 16.0),
        Icon::Ellipse | Icon::EllipseFilled => {
            b.push_oval(tiny_skia::Rect::from_xywh(3.0, 5.0, 18.0, 14.0).unwrap());
        }
        Icon::CircleOne => {
            circle(&mut b, 12.0, 12.0, 9.0);
            polyline(&mut b, &[(10.0, 9.0), (12.0, 7.0), (12.0, 17.0)]);
            line(&mut b, 10.0, 17.0, 14.0, 17.0);
        }
        Icon::ListMinus => {
            line(&mut b, 3.0, 6.0, 16.0, 6.0);
            line(&mut b, 3.0, 12.0, 12.0, 12.0);
            line(&mut b, 3.0, 18.0, 16.0, 18.0);
            line(&mut b, 15.0, 12.0, 21.0, 12.0);
        }
        Icon::DashPattern => {
            line(&mut b, 3.0, 12.0, 7.0, 12.0);
            line(&mut b, 10.0, 12.0, 14.0, 12.0);
            line(&mut b, 17.0, 12.0, 21.0, 12.0);
            circle(&mut b, 5.0, 6.0, 1.0);
            circle(&mut b, 12.0, 6.0, 1.0);
            circle(&mut b, 19.0, 6.0, 1.0);
        }
        Icon::Type => {
            line(&mut b, 4.0, 4.0, 20.0, 4.0);
            line(&mut b, 4.0, 4.0, 4.0, 7.0);
            line(&mut b, 20.0, 4.0, 20.0, 7.0);
            line(&mut b, 12.0, 4.0, 12.0, 20.0);
            line(&mut b, 9.0, 20.0, 15.0, 20.0);
        }
        Icon::Droplet => {
            b.move_to(12.0, 2.5);
            b.cubic_to(11.0, 7.0, 5.0, 10.0, 5.0, 15.0);
            b.cubic_to(5.0, 24.0, 19.0, 24.0, 19.0, 15.0);
            b.cubic_to(19.0, 10.0, 13.0, 7.0, 12.0, 2.5);
            b.close();
        }
        Icon::Undo => {
            polyline(&mut b, &[(9.0, 4.0), (4.0, 9.0), (9.0, 14.0)]);
            b.move_to(4.0, 9.0);
            b.cubic_to(14.0, 7.0, 20.0, 10.0, 20.0, 16.0);
        }
        Icon::Redo => {
            polyline(&mut b, &[(15.0, 4.0), (20.0, 9.0), (15.0, 14.0)]);
            b.move_to(20.0, 9.0);
            b.cubic_to(10.0, 7.0, 4.0, 10.0, 4.0, 16.0);
        }
        Icon::Trash => {
            line(&mut b, 3.0, 6.0, 21.0, 6.0);
            polyline(
                &mut b,
                &[(6.0, 6.0), (7.0, 21.0), (17.0, 21.0), (18.0, 6.0)],
            );
            polyline(&mut b, &[(8.0, 6.0), (9.0, 3.0), (15.0, 3.0), (16.0, 6.0)]);
            line(&mut b, 10.0, 10.0, 10.0, 17.0);
            line(&mut b, 14.0, 10.0, 14.0, 17.0);
        }
        Icon::Palette => {
            circle(&mut b, 12.0, 12.0, 9.5);
            for (x, y) in [(8.0, 8.0), (13.0, 6.5), (17.0, 10.0), (7.0, 14.0)] {
                circle(&mut b, x, y, 0.7);
            }
            circle(&mut b, 16.0, 16.0, 2.5);
        }
        Icon::Check => polyline(&mut b, &[(4.0, 12.0), (9.0, 17.0), (20.0, 6.0)]),
        Icon::X => {
            line(&mut b, 6.0, 6.0, 18.0, 18.0);
            line(&mut b, 18.0, 6.0, 6.0, 18.0);
        }
        Icon::CircleHelp => {
            circle(&mut b, 12.0, 12.0, 10.0);
            b.move_to(9.0, 9.0);
            b.cubic_to(10.0, 5.0, 16.0, 6.0, 16.0, 10.0);
            b.cubic_to(16.0, 12.0, 12.0, 12.0, 12.0, 15.0);
            circle(&mut b, 12.0, 18.0, 0.5);
        }
        Icon::MessageSquare => {
            polyline(
                &mut b,
                &[
                    (5.0, 3.0),
                    (19.0, 3.0),
                    (21.0, 5.0),
                    (21.0, 15.0),
                    (19.0, 17.0),
                    (8.0, 17.0),
                    (3.0, 21.0),
                    (3.0, 5.0),
                    (5.0, 3.0),
                ],
            );
        }
        Icon::Clipboard => {
            rect(&mut b, 4.0, 4.0, 16.0, 18.0);
            rect(&mut b, 8.0, 2.0, 8.0, 5.0);
        }
        Icon::Maximize => {
            polyline(&mut b, &[(8.0, 3.0), (3.0, 3.0), (3.0, 8.0)]);
            polyline(&mut b, &[(16.0, 3.0), (21.0, 3.0), (21.0, 8.0)]);
            polyline(&mut b, &[(3.0, 16.0), (3.0, 21.0), (8.0, 21.0)]);
            polyline(&mut b, &[(21.0, 16.0), (21.0, 21.0), (16.0, 21.0)]);
        }
        Icon::Copy => {
            rect(&mut b, 8.0, 8.0, 13.0, 13.0);
            polyline(
                &mut b,
                &[
                    (16.0, 8.0),
                    (16.0, 3.0),
                    (3.0, 3.0),
                    (3.0, 16.0),
                    (8.0, 16.0),
                ],
            );
        }
        Icon::Download => {
            polyline(
                &mut b,
                &[(3.0, 15.0), (3.0, 21.0), (21.0, 21.0), (21.0, 15.0)],
            );
            line(&mut b, 12.0, 3.0, 12.0, 16.0);
            polyline(&mut b, &[(7.0, 11.0), (12.0, 16.0), (17.0, 11.0)]);
        }
    }
    b.finish().expect("icon geometry is never empty")
}

fn render_rgba(icon: Icon, color: (u8, u8, u8, u8), dpi: u32) -> IconBitmap {
    let size = ((26_u64 * dpi.max(96) as u64 + 48) / 96) as u32;
    render_rgba_at_size(icon, color, size)
}

fn render_rgba_at_size(icon: Icon, color: (u8, u8, u8, u8), size: u32) -> IconBitmap {
    let mut pixmap = Pixmap::new(size, size).expect("supported icon size");
    let (r, g, b, a) = color;
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, a);
    let stroke = Stroke {
        width: ICON_STROKE_WIDTH,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    if matches!(icon, Icon::ShapeFilled | Icon::EllipseFilled) {
        pixmap.fill_path(
            &icon_path(icon),
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::from_scale(size as f32 / 24.0, size as f32 / 24.0),
            None,
        );
    } else {
        pixmap.stroke_path(
            &icon_path(icon),
            &paint,
            &stroke,
            Transform::from_scale(size as f32 / 24.0, size as f32 / 24.0),
            None,
        );
    }
    IconBitmap {
        width: size,
        height: size,
        pixels: pixmap.data().to_vec(),
    }
}

// Menus borrow their bitmaps. Keep this owner alive until after DestroyMenu.
#[derive(Default)]
pub struct MenuIcons(Vec<HBITMAP>);

impl MenuIcons {
    pub fn add(&mut self, menu: HMENU, command: usize, icon: Icon, dpi: u32) {
        let color = unsafe { GetSysColor(COLOR_MENUTEXT) };
        self.add_color(menu, command, icon, dpi, color);
    }

    pub fn add_color(&mut self, menu: HMENU, command: usize, icon: Icon, dpi: u32, color: u32) {
        let size = (16 * dpi.max(96) + 48) / 96;
        let bitmap = render_rgba_at_size(
            icon,
            (color as u8, (color >> 8) as u8, (color >> 16) as u8, 255),
            size,
        );
        let mut info: BITMAPINFO = unsafe { zeroed() };
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size as i32,
            biHeight: -(size as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..unsafe { zeroed() }
        };
        unsafe {
            let mut bits = std::ptr::null_mut();
            let handle = CreateDIBSection(
                std::ptr::null_mut(),
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if handle.is_null() {
                return;
            }
            let output = std::slice::from_raw_parts_mut(bits.cast::<u8>(), bitmap.pixels.len());
            for (source, dest) in bitmap
                .pixels
                .chunks_exact(4)
                .zip(output.chunks_exact_mut(4))
            {
                dest.copy_from_slice(&[source[2], source[1], source[0], source[3]]);
            }
            let mut item: MENUITEMINFOW = zeroed();
            item.cbSize = size_of::<MENUITEMINFOW>() as u32;
            item.fMask = MIIM_BITMAP;
            item.hbmpItem = handle;
            if SetMenuItemInfoW(menu, command as u32, 0, &item) == 0 {
                DeleteObject(handle);
            } else {
                self.0.push(handle);
            }
        }
    }
}

impl Drop for MenuIcons {
    fn drop(&mut self) {
        for handle in self.0.drain(..) {
            unsafe {
                DeleteObject(handle);
            }
        }
    }
}

pub fn render(icon: Icon, state: VisualState, dpi: u32) -> IconBitmap {
    render_rgba(icon, icon_color(state), dpi)
}

fn render_colorref(icon: Icon, color: u32, dpi: u32) -> IconBitmap {
    render_rgba(
        icon,
        (
            (color & 0xff) as u8,
            ((color >> 8) & 0xff) as u8,
            ((color >> 16) & 0xff) as u8,
            255,
        ),
        dpi,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct CacheKey {
    icon: Icon,
    state: VisualState,
    dpi: u32,
}

pub struct IconCache {
    capacity: usize,
    entries: HashMap<CacheKey, Arc<IconBitmap>>,
    order: VecDeque<CacheKey>,
}

impl IconCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn get(&mut self, icon: Icon, state: VisualState, dpi: u32) -> Arc<IconBitmap> {
        let key = CacheKey {
            icon,
            state,
            dpi: dpi.clamp(96, 192),
        };
        if let Some(bitmap) = self.entries.get(&key) {
            return Arc::clone(bitmap);
        }
        while self.entries.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        let bitmap = Arc::new(render(key.icon, key.state, key.dpi));
        self.entries.insert(key, Arc::clone(&bitmap));
        self.order.push_back(key);
        bitmap
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

thread_local! {
    static SHARED_CACHE: RefCell<IconCache> = RefCell::new(IconCache::new(192));
}

fn composite_bgra(bitmap: &IconBitmap, background: u32) -> Vec<u8> {
    let background_r = (background & 0xff) as u16;
    let background_g = ((background >> 8) & 0xff) as u16;
    let background_b = ((background >> 16) & 0xff) as u16;
    let mut pixels = Vec::with_capacity(bitmap.pixels.len());
    for rgba in bitmap.pixels.chunks_exact(4) {
        let alpha = rgba[3] as u16;
        let inverse = 255 - alpha;
        let r = rgba[0] as u16 + (background_r * inverse + 127) / 255;
        let g = rgba[1] as u16 + (background_g * inverse + 127) / 255;
        let b = rgba[2] as u16 + (background_b * inverse + 127) / 255;
        pixels.extend([b.min(255) as u8, g.min(255) as u8, r.min(255) as u8, 0]);
    }
    pixels
}

pub fn draw_gdi(dc: HDC, bounds: RECT, icon: Icon, state: VisualState, dpi: u32, background: u32) {
    let bitmap = SHARED_CACHE.with(|cache| cache.borrow_mut().get(icon, state, dpi));
    draw_bitmap_gdi(dc, bounds, &bitmap, background);
}

pub fn draw_gdi_color(
    dc: HDC,
    bounds: RECT,
    icon: Icon,
    foreground: u32,
    dpi: u32,
    background: u32,
) {
    let bitmap = render_colorref(icon, foreground, dpi);
    draw_bitmap_gdi(dc, bounds, &bitmap, background);
}

fn draw_bitmap_gdi(dc: HDC, bounds: RECT, bitmap: &IconBitmap, background: u32) {
    let pixels = composite_bgra(bitmap, background);
    let width = bitmap.width as i32;
    let height = bitmap.height as i32;
    let size = width
        .min(bounds.right - bounds.left)
        .min(bounds.bottom - bounds.top);
    if size <= 0 {
        return;
    }
    let x = bounds.left + (bounds.right - bounds.left - size) / 2;
    let y = bounds.top + (bounds.bottom - bounds.top - size) / 2;
    let mut info: BITMAPINFO = unsafe { zeroed() };
    info.bmiHeader = BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..unsafe { zeroed() }
    };
    unsafe {
        StretchDIBits(
            dc,
            x,
            y,
            size,
            size,
            0,
            0,
            width,
            height,
            pixels.as_ptr().cast(),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashSet, sync::Arc};

    #[test]
    fn generated_app_logo_has_no_r_badge() {
        let logo = image::load_from_memory(include_bytes!("../assets/snapzy-rust.png"))
            .unwrap()
            .to_rgba8();
        let pixel = logo.get_pixel(240, 240);
        assert!(
            pixel[2] > 220 && pixel[0] < 40,
            "badge remains at logo corner: {pixel:?}"
        );
    }

    #[test]
    fn filled_shape_has_solid_center_and_step_has_a_number() {
        for dpi in [96, 120, 144, 192] {
            let filled = render(Icon::ShapeFilled, VisualState::Normal, dpi);
            let center = ((filled.height / 2 * filled.width + filled.width / 2) * 4 + 3) as usize;
            assert!(
                filled.pixels[center] > 240,
                "filled shape must not resemble a list"
            );
            let step = render(icon_for(Command::Step), VisualState::Normal, dpi);
            let center_ink = (step.height / 3..step.height * 2 / 3)
                .flat_map(|y| (step.width / 3..step.width * 2 / 3).map(move |x| (x, y)))
                .any(|(x, y)| step.pixels[((y * step.width + x) * 4 + 3) as usize] > 128);
            assert!(center_ink, "step must show a number, not an empty circle");
        }
    }

    #[test]
    fn editor_icons_use_a_clearer_twenty_six_pixel_base_size() {
        for dpi in [96, 120, 144, 192] {
            let icon = render(Icon::MousePointer, VisualState::Normal, dpi);
            assert_eq!(icon.width, (26 * dpi + 48) / 96);
            assert_eq!(icon.height, icon.width);
        }
    }

    #[test]
    fn every_visible_command_has_an_icon_and_localized_accessible_name() {
        let commands = visible_commands();
        assert!(commands.len() >= 40, "registry omitted visible commands");

        for command in commands {
            let _icon = icon_for(*command);
            let english = accessible_name(*command, false);
            let thai = accessible_name(*command, true);
            assert!(
                !english.trim().is_empty(),
                "missing EN name for {command:?}"
            );
            assert!(!thai.trim().is_empty(), "missing TH name for {command:?}");
            assert_ne!(english, thai, "name was not localized for {command:?}");
        }
    }

    #[test]
    fn launcher_and_editor_use_dedicated_semantic_glyphs() {
        assert_ne!(
            icon_for(Command::ScrollingCapture),
            Icon::GalleryVerticalEnd,
            "scrolling capture must not use the generic gallery glyph"
        );
        assert_ne!(
            icon_for(Command::FilledShape),
            Icon::Blend,
            "filled shapes must not use the ambiguous blend glyph"
        );
        assert_ne!(
            icon_for(Command::StrokePattern),
            Icon::ListMinus,
            "stroke pattern must not use a generic list glyph"
        );
        assert_ne!(
            icon_for(Command::SelectOwner),
            icon_for(Command::Select),
            "owning-image selection must be visually distinct from ordinary selection"
        );
    }

    #[test]
    fn small_toolbar_icons_use_a_light_stroke() {
        assert!(ICON_STROKE_WIDTH < 1.9);
        assert!(ICON_STROKE_WIDTH >= 1.4);
    }

    #[test]
    fn visible_command_registry_contains_no_duplicates() {
        let commands = visible_commands();
        assert_eq!(
            commands.len(),
            commands.iter().copied().collect::<HashSet<_>>().len()
        );
    }

    #[test]
    fn destructive_and_primary_commands_have_explicit_visual_roles() {
        assert_eq!(role(Command::Delete), Role::Destructive);
        assert_eq!(role(Command::SavePng), Role::Primary);
        assert_eq!(role(Command::AreaCapture), Role::Primary);
        assert_eq!(role(Command::Undo), Role::Normal);
    }

    #[test]
    fn high_contrast_buttons_use_only_supplied_system_colors() {
        let system = SystemColors {
            button_face: 0x0001_0203,
            button_text: 0x0004_0506,
            gray_text: 0x0007_0809,
            highlight: 0x000a_0b0c,
            highlight_text: 0x000d_0e0f,
            window_frame: 0x0010_1112,
        };
        let normal = high_contrast_palette(VisualState::Normal, system);
        assert_eq!(normal.background, system.button_face);
        assert_eq!(normal.foreground, system.button_text);
        assert_eq!(normal.border, system.window_frame);

        for state in [
            VisualState::Hover,
            VisualState::Selected,
            VisualState::Pressed,
            VisualState::Focused,
            VisualState::Primary,
        ] {
            let palette = high_contrast_palette(state, system);
            assert_eq!(palette.background, system.highlight);
            assert_eq!(palette.foreground, system.highlight_text);
            assert_eq!(palette.border, system.highlight);
        }

        let disabled = high_contrast_palette(VisualState::Disabled, system);
        assert_eq!(disabled.background, system.button_face);
        assert_eq!(disabled.foreground, system.gray_text);
        assert_eq!(disabled.border, system.window_frame);
    }

    #[test]
    fn every_registered_icon_renders_non_empty_deterministic_pixels() {
        let icons = visible_commands()
            .iter()
            .map(|command| icon_for(*command))
            .collect::<HashSet<_>>();
        for icon in icons {
            let first = render(icon, VisualState::Normal, 96);
            let second = render(icon, VisualState::Normal, 96);
            assert_eq!(first, second, "non-deterministic render for {icon:?}");
            assert_eq!((first.width, first.height), (26, 26));
            assert!(first.pixels.iter().any(|channel| *channel != 0));
        }
    }

    #[test]
    fn renderer_scales_sharply_and_uses_distinct_state_colors() {
        let normal = render(Icon::ScanText, VisualState::Normal, 96);
        let disabled = render(Icon::ScanText, VisualState::Disabled, 96);
        let selected = render(Icon::ScanText, VisualState::Selected, 96);
        let high_dpi = render(Icon::ScanText, VisualState::Normal, 192);
        assert_eq!((high_dpi.width, high_dpi.height), (52, 52));
        assert_ne!(normal.pixels, disabled.pixels);
        assert_ne!(normal.pixels, selected.pixels);
        assert!(high_dpi.alpha_bounds().is_some());
    }

    #[test]
    fn cache_reuses_bitmaps_and_stays_bounded() {
        let mut cache = IconCache::new(8);
        let first = cache.get(Icon::Scan, VisualState::Normal, 96);
        let second = cache.get(Icon::Scan, VisualState::Normal, 96);
        assert!(Arc::ptr_eq(&first, &second));
        for (index, icon) in [
            Icon::AppWindow,
            Icon::Monitor,
            Icon::Video,
            Icon::Timer,
            Icon::Layers,
            Icon::Crop,
            Icon::Pencil,
            Icon::Trash,
            Icon::Download,
        ]
        .into_iter()
        .enumerate()
        {
            cache.get(icon, VisualState::Normal, 96 + index as u32 * 24);
        }
        assert!(cache.len() <= 8);
    }
}

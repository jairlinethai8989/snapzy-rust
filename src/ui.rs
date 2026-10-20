use crate::{
    capture::{self, Candidate},
    clipboard,
    compact_ui::{self, EditorAction, LauncherControl},
    composition::{Guard, Token},
    editor::{Annotation, Content, Point, Tool, View},
    editor_controls as ec,
    frame::{Frame, Rect},
    interaction::Session,
    native_icons::{self, Command as IconCommand, VisualState},
    native_tooltips::{self, Entry as TooltipEntry},
    paint_buffer::BackBuffer,
    render,
    scroll_detect::{
        self, DetectedPane, DetectionResult, Request as DetectionRequest,
        Service as DetectionService, State as DetectionState,
    },
    settings,
    settings::{HotkeyAction, HotkeyBinding, Settings},
    trace,
    video::{self, Command as VideoCommand, Event as VideoEvent, Phase as VideoPhase},
    video_service::Service as VideoService,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_void,
    fs,
    io::{BufWriter, Write},
    mem::{size_of, zeroed},
    path::PathBuf,
    ptr::{null, null_mut},
    sync::{Arc, mpsc},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
    UI::{
        Controls::{
            BST_CHECKED, BST_UNCHECKED, DRAWITEMSTRUCT, Dialogs::*, EM_SETLIMITTEXT, EM_SETSEL,
            ODS_DISABLED, ODS_FOCUS, ODS_HOTLIGHT, ODS_SELECTED,
        },
        HiDpi::*,
        Input::KeyboardAndMouse::*,
        Shell::*,
        WindowsAndMessaging::*,
    },
};
#[path = "combine_ui.rs"]
mod combine_ui;
#[path = "project_ui.rs"]
mod project_ui;
#[path = "video_ui.rs"]
mod video_ui;
#[path = "workspace_ui.rs"]
mod workspace_ui;

const MAIN: &str = "SnapZyRustPreview.Launcher";
const PREVIEW: &str = "SnapZyRustPreview.Capture";
const OVERLAY: &str = "SnapZyRustPreview.Selection";
const HOTKEYS: &str = "SnapZyRustPreview.Hotkeys";
const TEXT_ENTRY: &str = "SnapZyRustPreview.Text";
const SCROLL_PANEL: &str = "SnapZyRustPreview.Scroll";
const VIDEO_PANEL: &str = "SnapZyRustPreview.Video";
const OCR_WINDOW: &str = "SnapZyRustPreview.Ocr";
const TITLE: &str = concat!("SnapZy Rust Preview ", env!("CARGO_PKG_VERSION"));
pub(crate) const DIALOG_TITLE: &str = "SnapZy";
pub(crate) const TASK_DIALOG_WIDTH: u32 = 250;
const SUBTITLE: &str = "Rust Preview";
pub(crate) const UI_FONT_FACE: &str = "Segoe UI Variable Text";
const COMMAND: u32 = WM_APP + 10;
const COMPLETE: u32 = WM_APP + 11;
const CLOSE: u32 = WM_APP + 12;
const TRAY: u32 = WM_APP + 13;
const TASKBAR_CREATED: &str = "TaskbarCreated";
const AREA: usize = 101;
const WINDOW: usize = 102;
const DESKTOP: usize = 103;
const LANGUAGE: usize = 104;
const KEYS: usize = 105;
const HIDE: usize = 106;
const ABOUT: usize = 107;
const EXIT: usize = 108;
const SCROLL: usize = 109;
const SCROLL_AUTO: usize = 110;
const SCROLL_MANUAL: usize = 111;
const SCROLL_MODE_LABEL: i32 = 112;
const SCROLL_DETECT_STATUS: i32 = 113;
const SCROLL_STOP: usize = 801;
const SCROLL_CANCEL: usize = 802;
const SCROLL_PAUSE: usize = 803;
const SCROLL_DETECT_START: usize = 804;
const SCROLL_DETECT_CANCEL: usize = 805;
const SCROLL_ESCAPE_HOTKEY: i32 = 0x5343;
const SCROLL_TIMER: usize = 502;
const SCROLL_HOVER_TIMER: usize = 503;
const COPY: usize = 201;
const SAVE: usize = 202;
const APPLY: usize = 301;
const HOTKEY_ROWS: [(HotkeyAction, i32, i32, i32); 6] = [
    (HotkeyAction::Area, 302, 303, 304),
    (HotkeyAction::Window, 305, 306, 307),
    (HotkeyAction::Desktop, 308, 309, 310),
    (HotkeyAction::Scrolling, 311, 312, 313),
    (HotkeyAction::Video, 314, 315, 316),
    (HotkeyAction::Delay, 317, 318, 319),
];
const STATUS: i32 = 401;
const CAPTURE_TIMER: usize = 501;
const OCR_LANGUAGE: i32 = 901;
const OCR_READ: usize = 902;
const OCR_COPY: usize = 903;
const OCR_CLOSE: usize = 904;
const OCR_RESULT: i32 = 905;
const OCR_STATUS: i32 = 906;
const OCR_HELP: i32 = 907;
const OPEN_TAB: usize = 114;
const OPEN_WINDOW: usize = 115;
const VIDEO: usize = 116;
const VIDEO_MONITOR: usize = 117;
const VIDEO_WINDOW: usize = 118;
const VIDEO_AUDIO_NONE: usize = 119;
const VIDEO_AUDIO_SYSTEM: usize = 120;
const VIDEO_AUDIO_MIC: usize = 121;
const VIDEO_AUDIO_BOTH: usize = 122;
const VIDEO_STOP: usize = 806;
const VIDEO_ESCAPE_HOTKEY: i32 = 0x5652;
const VIDEO_TIMER: usize = 504;
const WINDOW_MENU: usize = 123;
const SCROLL_OPTIONS: usize = 124;
const OPEN_OPTIONS: usize = 125;
const MORE: usize = 126;
const DELAY_OPTIONS: usize = 127;
const DELAY_NONE: usize = 131;
const DELAY_THREE: usize = 132;
const DELAY_FIVE: usize = 133;
const DELAY_TEN: usize = 134;
const HOTKEY_ID_BASE: i32 = 1001;
const HOTKEY_HINT: i32 = 320;

thread_local! { static STATE: RefCell<Option<App>> = const { RefCell::new(None) }; }

#[derive(Clone, Copy, Debug)]
enum Mode {
    Area,
    Window,
    Desktop,
    Scroll,
    VideoMonitor,
    VideoWindow,
}

struct VideoRecording {
    panel: usize,
    service: VideoService,
    state: video::State,
    clock: Instant,
    elapsed_ms: u64,
    frames: u64,
}

struct PendingCapture {
    mode: Mode,
    started: Instant,
    ready_at: Instant,
}

struct Preview {
    project_path: Option<PathBuf>,
    source_label: Option<String>,
    project_operation: u64,
    close_after_save: bool,
    importing: bool,
    frame: Arc<Frame>,
    session: Session,
    cached: Option<(u64, Arc<Frame>)>,
    curve_feedback: Option<(crate::editor::Document, Annotation)>,
    guard: Guard,
    tips: Option<ec::Tips>,
    saving: bool,
    first_paint: Option<Instant>,
}

struct Selection {
    window: usize,
    detection_identity: u64,
    frame: Arc<Frame>,
    desktop: Rect,
    mode: Mode,
    windows: Vec<Candidate>,
    start: Option<(i32, i32)>,
    end: (i32, i32),
    detection: Option<DetectionState>,
    detected_pane: Option<DetectedPane>,
    hover_point: Option<(i32, i32)>,
    hover_requested: bool,
    hover_changed: Instant,
    pressed_pane: Option<scroll_detect::PaneCandidate>,
    first_paint: Option<Instant>,
}

enum Work {
    VideoSaved {
        owner: usize,
        identity: u64,
        operation: u64,
        result: Result<PathBuf, String>,
    },
    Combined {
        route: crate::workspace::Route,
        owner: usize,
        identity: u64,
        started: Instant,
        result: Result<(crate::project::Project, Arc<Frame>), String>,
    },
    ProjectOpened {
        origin: project_ui::OpenOrigin,
        path: PathBuf,
        started: Instant,
        result: Result<crate::project::Project, project_ui::OpenError>,
    },
    ProjectSaved {
        owner: usize,
        identity: u64,
        operation: u64,
        revision: u64,
        path: PathBuf,
        started: Instant,
        result: Result<(), String>,
    },
    Imported {
        owner: usize,
        identity: u64,
        result: Result<Frame, String>,
    },
    Composed {
        owner: usize,
        token: Token,
        started: Instant,
        result: Result<Arc<Frame>, String>,
    },
    Snapshot {
        mode: Mode,
        started: Instant,
        result: Result<(Frame, Rect, Vec<Candidate>), String>,
    },
    Saved {
        owner: usize,
        identity: u64,
        revision: u64,
        started: Instant,
        result: Result<(), String>,
    },
    OcrFinished {
        owner: usize,
        identity: u64,
        request: u64,
        revision: u64,
        result: Result<String, String>,
        started: Instant,
    },
}

struct OcrWindow {
    owner: usize,
    identity: u64,
    request: u64,
    source_revision: u64,
}

struct App {
    main: usize,
    settings: Settings,
    settings_window: usize,
    previews: HashMap<usize, Preview>,
    workspace: crate::workspace::Workspace,
    tab_tips: HashMap<usize, workspace_ui::Tips>,
    capture_route: Option<crate::workspace::Route>,
    selection: Option<Selection>,
    scrolling: Option<crate::scroll_job::Job>,
    recording: Option<VideoRecording>,
    video_previews: HashMap<usize, video_ui::Preview>,
    video_exit: Option<video_ui::ExitPlan>,
    exit_after_video_exports: bool,
    pending: Option<PendingCapture>,
    capture_busy: bool,
    restore: Vec<(usize, u64, i32)>,
    saving: usize,
    composing: bool,
    importing: bool,
    project_opening: bool,
    project_reserved: usize,
    combine_assets: Vec<Arc<Frame>>,
    project_modal: bool,
    project_exit: Option<project_ui::ExitPlan>,
    next_preview: u64,
    next_detection_identity: u64,
    next_video_session: u64,
    exit_after_video: bool,
    text_entry: Option<TextEntry>,
    effects_open: bool,
    ocr_windows: HashMap<usize, OcrWindow>,
    ocr_busy: bool,
    next_ocr_identity: u64,
    tx: mpsc::Sender<Work>,
    rx: mpsc::Receiver<Work>,
    detector: DetectionService,
    detection_rx: mpsc::Receiver<DetectionResult>,
    font_cache: HashMap<(u32, i32, i32), usize>,
    back_buffer: Option<BackBuffer>,
    icon: usize,
    tray_added: bool,
    active_hotkeys: Vec<(HotkeyAction, i32)>,
    taskbar_message: u32,
    first_paint: Option<Instant>,
    launcher_tips: Option<native_tooltips::Tips>,
}

pub fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}
struct TextEntry {
    canvas: bool,
    window: usize,
    owner: usize,
    point: Point,
    object: Option<u64>,
    identity: u64,
}

fn tr(key: &str) -> String {
    let thai = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .is_some_and(|a| a.settings.language == "th")
    });
    match (key, thai) {
        ("area", false) => "Area",
        ("area", true) => "พื้นที่",
        ("window", false) => "Window",
        ("window", true) => "หน้าต่าง",
        ("desktop", false) => "Desktop",
        ("desktop", true) => "เดสก์ท็อป",
        ("scroll", false) => "Scrolling",
        ("scroll", true) => "ภาพยาว",
        ("video", false) => "Video",
        ("video", true) => "วิดีโอ",
        ("video_monitor", false) => "Record screen",
        ("video_monitor", true) => "บันทึกทั้งจอ",
        ("video_window", false) => "Record window",
        ("video_window", true) => "บันทึกหน้าต่าง",
        ("video_audio_none", false) => "No audio",
        ("video_audio_none", true) => "ไม่บันทึกเสียง",
        ("video_audio_system", false) => "System audio (all apps)",
        ("video_audio_system", true) => "เสียงจากเครื่อง (ทุกแอป)",
        ("video_audio_mic", false) => "Microphone",
        ("video_audio_mic", true) => "ไมโครโฟน",
        ("video_audio_both", false) => "System audio + microphone",
        ("video_audio_both", true) => "เสียงจากเครื่อง + ไมโครโฟน",
        ("video_audio_selected", false) => "Audio: {mode}",
        ("video_audio_selected", true) => "เสียง: {mode}",
        ("video_audio_ended", false) => "Recording stopped because an audio input became unavailable.\n{detail}",
        ("video_audio_ended", true) => "หยุดอัดเนื่องจากไม่สามารถรับเสียงจากอุปกรณ์ได้\n{detail}",
        ("video_audio_error", false) => "Audio capture is unavailable. Check the selected default device and microphone access in Windows settings.\n\n{detail}",
        ("video_audio_error", true) => "ไม่สามารถรับเสียงได้ กรุณาตรวจสอบอุปกรณ์เสียงเริ่มต้นและสิทธิ์ไมโครโฟนใน Windows\n\n{detail}",
        ("video_preparing", false) => "Preparing the recorder...",
        ("video_preparing", true) => "กำลังเตรียมการบันทึก...",
        ("video_starting", false) => "Starting recording...",
        ("video_starting", true) => "กำลังเริ่มบันทึก...",
        ("video_finalizing", false) => "Finalizing the video...",
        ("video_finalizing", true) => "กำลังจัดเก็บวิดีโอ...",
        ("video_stop", false) => "Stop recording",
        ("video_stop", true) => "หยุดบันทึก",
        ("video_saved", false) => "Video saved to:\n{path}",
        ("video_saved", true) => "บันทึกวิดีโอแล้วที่:\n{path}",
        ("video_save", false) => "Save video",
        ("video_save", true) => "บันทึกวิดีโอ",
        ("video_exporting", false) => "Saving the video...",
        ("video_exporting", true) => "กำลังบันทึกไฟล์วิดีโอ...",
        ("video_retained", false) => "The recorded video is kept at:\n{path}",
        ("video_retained", true) => "เก็บคลิปที่อัดแล้วไว้ที่:\n{path}",
        ("video_cancelled", false) => "Video recording was cancelled.",
        ("video_cancelled", true) => "ยกเลิกการบันทึกวิดีโอแล้ว",
        ("video_exit", false) => "Stop recording and exit SnapZy?",
        ("video_exit", true) => "หยุดบันทึกและออกจาก SnapZy หรือไม่?",
        ("scroll_click", false) => {
            "Move over an area, then click its colored frame to capture. Esc cancels."
        }
        ("scroll_click", true) => "วางเมาส์บนพื้นที่ แล้วคลิกกรอบสีเพื่อจับภาพ กด Esc ยกเลิก",
        ("scroll_window_ready", false) => {
            "Window frame: {title}. Review the area, then click inside to capture."
        }
        ("scroll_window_ready", true) => "กรอบหน้าต่าง: {title} ตรวจพื้นที่แล้วคลิกภายในกรอบเพื่อจับภาพ",
        ("scroll_detecting", false) => "Checking the selected area...",
        ("scroll_detecting", true) => "กำลังตรวจสอบพื้นที่...",
        ("scroll_detect_timeout", false) => {
            "Area detection timed out. Move to another area or press Esc."
        }
        ("scroll_detect_timeout", true) => "ตรวจจับพื้นที่ไม่ทันเวลา ย้ายเมาส์ไปจุดอื่นหรือกด Esc",
        ("scroll_no_match", false) => {
            "No scrollable area found. Move to another area or press Esc."
        }
        ("scroll_no_match", true) => "ไม่พบพื้นที่เลื่อน ย้ายเมาส์ไปจุดอื่นหรือกด Esc",
        ("scroll_start", false) => "Start long capture",
        ("scroll_start", true) => "เริ่มจับภาพยาว",
        ("scroll_preview", false) => "Found: {title}. Review the highlighted area.",
        ("scroll_preview", true) => "พบ: {title} ตรวจสอบกรอบพื้นที่ที่ไฮไลต์",
        ("scroll_stale", false) => "The target changed. Move the pointer to detect again.",
        ("scroll_stale", true) => "พื้นที่เปลี่ยนแปลง ขยับเมาส์เพื่อตรวจหาใหม่",
        ("stop", false) => "Stop",
        ("stop", true) => "หยุด",
        ("cancel", false) => "Cancel",
        ("cancel", true) => "ยกเลิก",
        ("scroll_target", false) => "Select a scrolling region inside an application window.",
        ("scroll_target", true) => "กรุณาเลือกพื้นที่เลื่อนภายในหน้าต่างโปรแกรม",
        ("scroll_region", false) => {
            "Select a smaller region inside one application, or close an older capture."
        }
        ("scroll_region", true) => "กรุณาเลือกพื้นที่เล็กลงภายในโปรแกรมเดียว หรือปิดภาพเก่าก่อน",
        ("scroll_controls", false) => "Cannot open scrolling controls.",
        ("scroll_controls", true) => "ไม่สามารถเปิดหน้าควบคุมการจับภาพยาวได้",
        ("scroll_room", false) => "Leave room outside the selected region for scrolling controls.",
        ("scroll_room", true) => "กรุณาเว้นพื้นที่นอกกรอบจับภาพสำหรับหน้าควบคุม",
        ("keys", false) => "Hotkey",
        ("keys", true) => "คีย์ลัด",
        ("hide", false) => "Minimize to tray",
        ("hide", true) => "พักใน Tray",
        ("ready", false) => "Ready",
        ("ready", true) => "พร้อมใช้งาน",
        ("capture", false) => "Capturing...",
        ("capture", true) => "กำลังจับภาพ...",
        ("copy", false) => "Copy",
        ("copy", true) => "คัดลอก",
        ("save", false) => "Save PNG",
        ("save", true) => "บันทึก PNG",
        ("copied", false) => "Copied",
        ("copied", true) => "คัดลอกแล้ว",
        ("saving", false) => "Saving...",
        ("composing", false) => "Updating image...",
        ("composing", true) => "กำลังปรับภาพ...",
        ("ocr_downloading", false) => "Preparing OCR language data...",
        ("ocr_downloading", true) => "กำลังเตรียมข้อมูลภาษา OCR...",
        ("ocr_reading", false) => "Reading text...",
        ("ocr_reading", true) => "กำลังอ่านข้อความ...",
        ("ocr_copied", false) => "Text copied",
        ("ocr_copied", true) => "คัดลอกข้อความแล้ว",
        ("saving", true) => "กำลังบันทึก...",
        ("saved", false) => "Saved",
        ("saved", true) => "บันทึกแล้ว",
        ("hotkey_area", false) => "Custom / Area",
        ("hotkey_area", true) => "กำหนดพื้นที่ / พื้นที่",
        ("hotkey_window", false) => "Window",
        ("hotkey_window", true) => "หน้าต่าง",
        ("hotkey_desktop", false) => "Desktop / All",
        ("hotkey_desktop", true) => "เดสก์ท็อป / ทั้งหมด",
        ("hotkey_scroll", false) => "Scrolling",
        ("hotkey_scroll", true) => "ภาพยาว",
        ("hotkey_video", false) => "Video",
        ("hotkey_video", true) => "วิดีโอ",
        ("hotkey_delay", false) => "Delay selector",
        ("hotkey_delay", true) => "เลือกเวลาหน่วง",
        ("hotkey_hint", false) => "Choose a shortcut for each action.",
        ("hotkey_hint", true) => "เลือกคีย์ลัดแยกสำหรับแต่ละคำสั่ง",
        ("delay_none", false) => "Delay: None",
        ("delay_none", true) => "หน่วง: ไม่หน่วง",
        ("delay_three", false) => "Delay: 3 seconds",
        ("delay_three", true) => "หน่วง: 3 วินาที",
        ("delay_five", false) => "Delay: 5 seconds",
        ("delay_five", true) => "หน่วง: 5 วินาที",
        ("delay_ten", false) => "Delay: 10 seconds",
        ("delay_ten", true) => "หน่วง: 10 วินาที",
        ("apply", false) => "Apply",
        ("apply", true) => "นำไปใช้",
        ("success", false) => "Hotkey settings saved.",
        ("success", true) => "บันทึกการตั้งค่าคีย์ลัดแล้ว",
        ("conflict", false) => {
            "This hotkey is used by another application. Choose a different combination."
        }
        ("conflict", true) => "คีย์ลัดนี้ถูกใช้งานโดยโปรแกรมอื่น กรุณาเลือกชุดปุ่มใหม่",
        ("duplicate", false) => "Each SnapZy action must use a different shortcut.",
        ("duplicate", true) => "คีย์ลัดของแต่ละคำสั่งต้องไม่ซ้ำกัน",
        ("invalid_hotkey", false) => "Choose a modifier and key for every enabled action.",
        ("invalid_hotkey", true) => "กรุณาเลือกปุ่มเสริมและปุ่มหลักให้ครบทุกคำสั่งที่เปิดใช้",
        ("discard", false) => "Close this capture without saving or copying?",
        ("discard", true) => "ปิดภาพนี้โดยไม่บันทึกหรือคัดลอกหรือไม่?",
        ("exit", false) => "Exit",
        ("exit", true) => "ออกจากโปรแกรม",
        ("exit_unsaved", false) => "Close unsaved captures and exit?",
        ("exit_unsaved", true) => "ปิดภาพที่ยังไม่ได้บันทึกและออกจากโปรแกรมหรือไม่?",
        ("wait", false) => "A PNG is still being saved. Please wait before exiting.",
        ("wait", true) => "กำลังบันทึก PNG กรุณารอให้เสร็จก่อนออกจากโปรแกรม",
        ("about", false) => "About",
        ("about", true) => "เกี่ยวกับ",
        ("preview", false) => "Preview",
        ("preview", true) => "พรีวิว",
        ("covered", false) => {
            "Part of this window is covered. Bring it to the front and capture again."
        }
        ("covered", true) => "หน้าต่างนี้ถูกบังบางส่วน กรุณานำหน้าต่างขึ้นด้านหน้าแล้วจับภาพอีกครั้ง",
        _ => key,
    }
    .to_string()
}

fn notify(owner: HWND, text: &str, error: bool) {
    let owner = workspace_ui::native_owner(owner);
    if crate::settings::is_test_run() {
        trace::test_note(&format!("notification error={error} text={text}"));
    }
    unsafe {
        MessageBoxW(
            owner,
            wide(text).as_ptr(),
            wide(DIALOG_TITLE).as_ptr(),
            MB_OK
                | if error {
                    MB_ICONWARNING
                } else {
                    MB_ICONINFORMATION
                },
        );
    }
}
fn text_error(text: &str, thai: bool) -> &str {
    if !thai {
        return text;
    }
    match text {
        "Invalid text frame." => "ขนาดกรอบข้อความไม่ถูกต้อง",
        "Text frame is outside the supported coordinates." => "ตำแหน่งกรอบข้อความเกินขอบเขตที่รองรับ",
        "Text memory limit reached. Use shorter text or fewer edits." => {
            "หน่วยความจำข้อความถึงขีดจำกัด กรุณาลดข้อความหรือจำนวนการแก้ไข"
        }
        "Text is too large. Use shorter lines or a smaller size." => {
            "กรอบข้อความใหญ่เกินไป กรุณาลดความกว้างหรือขนาดอักษร"
        }
        _ => text,
    }
}
fn notify_text_error(owner: HWND, error: &str) {
    let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
    notify(owner, text_error(error, thai), true);
}

fn status(owner: HWND, text: &str) {
    unsafe {
        SetWindowTextW(GetDlgItem(owner, STATUS), wide(text).as_ptr());
    }
}

fn font(window: HWND, size: i32, weight: i32) -> HFONT {
    let dpi = unsafe { GetDpiForWindow(window) }.max(96);
    STATE.with(|state| {
        let mut borrow = state.borrow_mut();
        let app = borrow.as_mut().unwrap();
        *app.font_cache
            .entry((dpi, size, weight))
            .or_insert_with(|| unsafe {
                CreateFontW(
                    -(size * dpi as i32 / 96),
                    0,
                    0,
                    0,
                    weight,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    CLEARTYPE_QUALITY as u32,
                    DEFAULT_PITCH as u32,
                    wide(UI_FONT_FACE).as_ptr(),
                ) as usize
            }) as HFONT
    })
}

fn primary_label_text_flags() -> u32 {
    DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS
}

fn primary_label_font_weight() -> i32 {
    FW_SEMIBOLD as i32
}

fn scale(window: HWND, value: i32) -> i32 {
    value * unsafe { GetDpiForWindow(window) }.max(96) as i32 / 96
}

fn child(parent: HWND, class: &str, text: &str, id: i32, style: u32) -> HWND {
    unsafe {
        let control = CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            0,
            0,
            0,
            0,
            parent,
            id as usize as HMENU,
            GetModuleHandleW(null()),
            null(),
        );
        SendMessageW(
            control,
            WM_SETFONT,
            font(parent, 14, FW_NORMAL as i32) as WPARAM,
            1,
        );
        control
    }
}

fn position(parent: HWND, id: i32, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        let control = GetDlgItem(parent, id);
        MoveWindow(
            control,
            scale(parent, x),
            scale(parent, y),
            scale(parent, w),
            scale(parent, h),
            1,
        );
        SendMessageW(
            control,
            WM_SETFONT,
            font(parent, 14, FW_NORMAL as i32) as WPARAM,
            1,
        );
    }
}

fn client(window: HWND) -> RECT {
    let mut rect = unsafe { zeroed() };
    unsafe {
        GetClientRect(window, &mut rect);
    }
    rect
}

fn layout(window: HWND) {
    let class = class_name(window);
    if class == workspace_ui::HOST {
        workspace_ui::layout_host(window);
        return;
    }
    let rect = client(window);
    let logical_width = rect.right * 96 / unsafe { GetDpiForWindow(window) }.max(96) as i32;
    if class == MAIN {
        let compact = compact_ui::launcher_layout();
        for (control, id) in [
            (LauncherControl::OpenImage, ec::PROJECT_OPEN),
            (LauncherControl::Language, LANGUAGE),
            (LauncherControl::More, MORE),
            (LauncherControl::Area, AREA),
            (LauncherControl::Window, WINDOW),
            (LauncherControl::WindowMenu, WINDOW_MENU),
            (LauncherControl::Scrolling, SCROLL),
            (LauncherControl::Video, VIDEO),
            (LauncherControl::Delay, DELAY_OPTIONS),
            (LauncherControl::ScrollingMode, SCROLL_OPTIONS),
            (LauncherControl::OpenMode, OPEN_OPTIONS),
        ] {
            let rect = compact.rect(control);
            position(window, id as i32, rect.x, rect.y, rect.width, rect.height);
        }
        position(window, STATUS, 139, 6, 115, 24);
    } else if class == VIDEO_PANEL {
        position(window, STATUS, 12, 14, logical_width - 24, 48);
        position(window, VIDEO_STOP as i32, logical_width - 144, 70, 132, 34);
    } else if class == SCROLL_PANEL {
        position(window, STATUS, 12, 14, logical_width - 24, 48);
        let button_width = (logical_width - 48) / 3;
        position(window, SCROLL_PAUSE as i32, 12, 70, button_width, 34);
        position(
            window,
            SCROLL_STOP as i32,
            24 + button_width,
            70,
            button_width,
            34,
        );
        position(
            window,
            SCROLL_CANCEL as i32,
            36 + button_width * 2,
            70,
            button_width,
            34,
        );
    } else if class == PREVIEW {
        let compact = compact_ui::editor_layout(logical_width.max(780), false).unwrap();
        let wide = compact.direct_actions.contains(&EditorAction::AddImage);
        let mut right = logical_width - 6;
        for (id, width) in [
            (SAVE, 88),
            (COPY, 42),
            (ec::LANG, 34),
            (ec::HELP, 34),
            (ec::OCR, 34),
            (ec::MORE, 34),
        ] {
            right -= width;
            position(window, id as i32, right, 3, width, 32);
            right -= 4;
        }
        for id in [
            ec::PROJECT_SAVE,
            ec::PROJECT_OPEN,
            ec::COMBINE,
            ec::ADD_IMAGE,
        ] {
            unsafe {
                ShowWindow(
                    GetDlgItem(window, id as i32),
                    if wide { SW_SHOWNA } else { SW_HIDE },
                );
            }
            if wide {
                right -= 34;
                position(window, id as i32, right, 3, 34, 32);
                right -= 4;
            }
        }
        position(window, STATUS, 6, 7, (right - 12).max(10), 24);

        for (index, id) in [
            ec::SELECT,
            ec::OWNER,
            ec::CROP,
            ec::PEN,
            ec::HIGHLIGHTER,
            ec::LINE,
            ec::ARROW,
            ec::RECTANGLE,
            ec::FILLED_RECTANGLE,
            ec::BLUR,
            ec::STEP,
            ec::TEXT,
            ec::CALLOUT,
            ec::EFFECTS,
            ec::LAYERS,
            ec::DELETE,
            ec::UNDO,
            ec::REDO,
        ]
        .iter()
        .enumerate()
        {
            let cell = compact_ui::editor_tool_rect(index, 38);
            position(window, *id as i32, cell.x, cell.y, cell.width, cell.height);
        }
        for id in [
            ec::FRONT,
            ec::FORWARD,
            ec::BACKWARD,
            ec::BACK,
            ec::PASTE_IMAGE,
            ec::CANVAS,
        ] {
            unsafe {
                ShowWindow(GetDlgItem(window, id as i32), SW_HIDE);
            }
        }
        position(window, ec::COLOR as i32, 6, 75, 34, 26);
        position(window, ec::WIDTH as i32, 46, 75, 52, 26);
        position(window, ec::UNIT, 103, 77, 161, 22);
        position(window, ec::CONFIRM as i32, 270, 75, 34, 26);
        position(window, ec::CANCEL as i32, 310, 75, 34, 26);
        position(window, ec::BACKGROUND as i32, 270, 75, 138, 26);
        position(window, ec::BACK_COLOR as i32, 414, 75, 34, 26);
        position(window, ec::BORDER_COLOR as i32, 454, 75, 34, 26);
        position(window, ec::STROKE_UNIT, 494, 77, 68, 22);
        position(window, ec::STROKE as i32, 566, 75, 44, 26);
        position(window, ec::OPACITY_UNIT, 270, 77, 100, 22);
        position(window, ec::OPACITY as i32, 374, 75, 52, 26);
        position(window, ec::PATTERN as i32, 616, 75, 126, 26);
    } else if class == OCR_WINDOW {
        let height = rect.bottom * 96 / unsafe { GetDpiForWindow(window) }.max(96) as i32;
        position(window, OCR_HELP, 16, 14, logical_width - 32, 22);
        position(window, OCR_LANGUAGE, 16, 44, 210, 30);
        position(window, OCR_READ as i32, 238, 44, 130, 30);
        position(window, OCR_STATUS, 16, 82, logical_width - 32, 22);
        position(
            window,
            OCR_RESULT,
            16,
            110,
            logical_width - 32,
            height - 170,
        );
        position(
            window,
            OCR_COPY as i32,
            logical_width - 232,
            height - 48,
            100,
            32,
        );
        position(
            window,
            OCR_CLOSE as i32,
            logical_width - 120,
            height - 48,
            104,
            32,
        );
    } else if class == TEXT_ENTRY {
        let height = rect.bottom * 96 / unsafe { GetDpiForWindow(window) }.max(96) as i32;
        let canvas = STATE.with(|s| {
            s.borrow()
                .as_ref()
                .and_then(|a| a.text_entry.as_ref())
                .is_some_and(|e| e.window == window as usize && e.canvas)
        });
        if canvas {
            position(window, 711, 20, 24, 160, 24);
            position(window, 701, 190, 20, 150, 30);
            position(window, 712, 20, 68, 160, 24);
            position(window, 702, 190, 64, 150, 30);
        } else {
            position(window, 701, 16, 16, logical_width - 32, height - 78);
        }
        position(window, 1, logical_width - 224, height - 48, 96, 32);
        position(window, 2, logical_width - 116, height - 48, 100, 32);
    } else if class == HOTKEYS {
        position(window, HOTKEY_HINT, 16, 14, logical_width - 32, 24);
        for (index, (_, enable_id, modifier_id, key_id)) in HOTKEY_ROWS.iter().enumerate() {
            let y = 44 + index as i32 * 40;
            position(window, *enable_id, 16, y, 190, 30);
            position(window, *modifier_id, 214, y, 154, 30);
            position(window, *key_id, 376, y, logical_width - 392, 30);
        }
        position(window, APPLY as i32, logical_width - 126, 294, 110, 34);
    }
    unsafe {
        InvalidateRect(window, null(), 1);
    }
}

fn class_name(window: HWND) -> String {
    let mut name = [0u16; 96];
    let count = unsafe { GetClassNameW(window, name.as_mut_ptr(), name.len() as i32) };
    String::from_utf16_lossy(&name[..count.max(0) as usize])
}

fn refresh_language() {
    let videos = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .video_previews
            .keys()
            .copied()
            .collect::<Vec<_>>()
    });
    for window in videos {
        video_ui::refresh(window as HWND);
    }
    let (main, previews, ocr_windows, video_panel, thai) = STATE.with(|s| {
        let borrow = s.borrow();
        let a = borrow.as_ref().unwrap();
        (
            a.main,
            a.previews.keys().copied().collect::<Vec<_>>(),
            a.ocr_windows.keys().copied().collect::<Vec<_>>(),
            a.recording.as_ref().map(|r| r.panel),
            a.settings.language == "th",
        )
    });
    for (id, key) in [
        (AREA, "area"),
        (WINDOW, "window"),
        (DESKTOP, "desktop"),
        (SCROLL, "scroll"),
        (VIDEO, "video"),
        (KEYS, "keys"),
        (HIDE, "hide"),
    ] {
        unsafe {
            SetWindowTextW(GetDlgItem(main as HWND, id as i32), wide(&tr(key)).as_ptr());
        }
    }
    unsafe {
        SetWindowTextW(
            GetDlgItem(main as HWND, ec::PROJECT_OPEN as i32),
            wide(ec::label(ec::PROJECT_OPEN, thai)).as_ptr(),
        );
        SetWindowTextW(
            GetDlgItem(main as HWND, LANGUAGE as i32),
            wide(if thai { "TH" } else { "EN" }).as_ptr(),
        );
    }
    // Tooltip updates can synchronously notify the owner. Release App before
    // calling Win32, just as we do for the editor's tooltip refresh below.
    let launcher_tips = STATE.with(|s| {
        s.borrow_mut()
            .as_mut()
            .and_then(|app| app.launcher_tips.take())
    });
    if let Some(mut tips) = launcher_tips {
        tips.refresh(thai);
        STATE.with(|s| {
            if let Some(app) = s.borrow_mut().as_mut() {
                app.launcher_tips = Some(tips);
            }
        });
    }
    status(main as HWND, &tr("ready"));
    if let Some(panel) = video_panel {
        unsafe {
            SetWindowTextW(
                GetDlgItem(panel as HWND, VIDEO_STOP as i32),
                wide(&tr("video_stop")).as_ptr(),
            );
        }
        refresh_video_status();
    }
    refresh_scroll_mode();
    refresh_delay();
    refresh_open_mode();
    for window in previews {
        let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
        let tips = STATE.with(|s| {
            s.borrow_mut()
                .as_mut()
                .unwrap()
                .previews
                .get_mut(&window)
                .and_then(|p| p.tips.take())
        });
        if let Some(mut tips) = tips {
            let shapes = STATE.with(|s| {
                s.borrow().as_ref().unwrap().previews[&window]
                    .session
                    .shape_choices()
            });
            tips.refresh(window as HWND, thai, shapes);
            STATE.with(|s| {
                if let Some(p) = s.borrow_mut().as_mut().unwrap().previews.get_mut(&window) {
                    p.tips = Some(tips);
                }
            });
        }
        unsafe {
            for id in ec::BUTTONS {
                SetWindowTextW(
                    GetDlgItem(window as HWND, id as i32),
                    wide(ec::label(id, thai)).as_ptr(),
                );
            }
            SetWindowTextW(
                GetDlgItem(window as HWND, ec::LANG as i32),
                wide(if thai { "TH" } else { "EN" }).as_ptr(),
            );
            SetWindowTextW(
                window as HWND,
                wide(&format!(
                    "{TITLE} - {}",
                    if thai {
                        "แก้ไขภาพ"
                    } else {
                        "Image editor"
                    }
                ))
                .as_ptr(),
            );
            SetWindowTextW(
                GetDlgItem(window as HWND, COPY as i32),
                wide(&tr("copy")).as_ptr(),
            );
            SetWindowTextW(
                GetDlgItem(window as HWND, SAVE as i32),
                wide(&tr("save")).as_ptr(),
            );
            SetWindowTextW(
                GetDlgItem(window as HWND, ec::OCR as i32),
                wide(ec::label(ec::OCR, thai)).as_ptr(),
            );
        }
        update_editor(window as HWND);
    }
    for window in ocr_windows {
        unsafe {
            let h = window as HWND;
            SetWindowTextW(
                h,
                wide(if thai {
                    "SnapZy | อ่านข้อความ OCR"
                } else {
                    "SnapZy | OCR text"
                })
                .as_ptr(),
            );
            SetWindowTextW(GetDlgItem(h, OCR_HELP), wide(if thai {
                "อ่านข้อความจากภาพที่แสดงในหน้าต่างแก้ไข ผลลัพธ์แก้ไขได้ก่อนคัดลอก"
            } else {
                "Read text from the current editor image. Review and edit the result before copying."
            }).as_ptr());
            SetWindowTextW(
                GetDlgItem(h, OCR_READ as i32),
                wide(if thai {
                    "อ่านข้อความ"
                } else {
                    "Read text"
                })
                .as_ptr(),
            );
            SetWindowTextW(GetDlgItem(h, OCR_COPY as i32), wide(&tr("copy")).as_ptr());
            SetWindowTextW(
                GetDlgItem(h, OCR_CLOSE as i32),
                wide(if thai { "ปิด" } else { "Close" }).as_ptr(),
            );
            let current = read_control_text(h, OCR_STATUS);
            if current == "Ready" || current == "พร้อมใช้งาน" {
                SetWindowTextW(GetDlgItem(h, OCR_STATUS), wide(&tr("ready")).as_ptr());
            }
        }
        layout(window as HWND);
    }
}

fn create_window(
    class: &str,
    title: &str,
    style: u32,
    extended: u32,
    parent: HWND,
    width: i32,
    height: i32,
) -> HWND {
    unsafe {
        let dpi = GetDpiForSystem().max(96);
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: width * dpi as i32 / 96,
            bottom: height * dpi as i32 / 96,
        };
        AdjustWindowRectExForDpi(&mut rect, style, 0, extended, dpi);
        let window = CreateWindowExW(
            extended,
            wide(class).as_ptr(),
            wide(title).as_ptr(),
            style,
            if style & WS_CHILD != 0 {
                0
            } else {
                CW_USEDEFAULT
            },
            if style & WS_CHILD != 0 {
                0
            } else {
                CW_USEDEFAULT
            },
            rect.right - rect.left,
            rect.bottom - rect.top,
            parent,
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        );
        if !window.is_null() {
            let icon = STATE.with(|s| s.borrow().as_ref().unwrap().icon);
            SendMessageW(window, WM_SETICON, ICON_BIG as usize, icon as LPARAM);
            SendMessageW(window, WM_SETICON, ICON_SMALL as usize, icon as LPARAM);
        }
        window
    }
}

fn tray(add: bool) -> bool {
    let (main, icon) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        (a.main, a.icon)
    });
    let mut data: NOTIFYICONDATAW = unsafe { zeroed() };
    data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = main as HWND;
    data.uID = 1;
    data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    data.hIcon = icon as HICON;
    data.uCallbackMessage = TRAY;
    let title = wide(TITLE);
    data.szTip[..title.len()].copy_from_slice(&title);
    unsafe { Shell_NotifyIconW(if add { NIM_ADD } else { NIM_DELETE }, &data) != 0 }
}

fn show_launcher() {
    if STATE.with(|s| s.borrow().as_ref().unwrap().recording.is_some()) {
        return;
    }
    cancel_scroll();
    let started = Instant::now();
    let window = STATE.with(|s| s.borrow().as_ref().unwrap().main) as HWND;
    unsafe {
        ShowWindow(window, SW_RESTORE);
        SetForegroundWindow(window);
        UpdateWindow(window);
    }
    trace::record("tray.restore-through-paint", started);
}

fn start_capture(mode: Mode) {
    if STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.project_exit.is_some() || a.video_exit.is_some() || a.project_modal || a.project_opening
    }) {
        return;
    }
    cancel_scroll();
    let started = Instant::now();
    let windows = STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        let app = borrow.as_mut().unwrap();
        if app.capture_busy
            || app.recording.is_some()
            || app.previews.len() >= 16
            || app
                .previews
                .values()
                .any(|p| workspace_ui::unfinished(&p.session))
            || app.settings_window != 0
            || app.text_entry.is_some()
            || app.effects_open
        {
            return None;
        }
        app.capture_busy = true;
        app.capture_route = Some(app.workspace.route(app.settings.open_mode, None));
        let delay = if matches!(
            mode,
            Mode::Area | Mode::Window | Mode::Desktop | Mode::Scroll
        ) {
            app.settings.capture_delay_seconds
        } else {
            0
        };
        if delay != 0 {
            let mut settings = app.settings.clone();
            settings.capture_delay_seconds = 0;
            if settings.save().is_ok() {
                app.settings = settings;
            }
        }
        app.pending = Some(PendingCapture {
            mode,
            started,
            ready_at: started + Duration::from_secs(delay as u64),
        });
        let mut windows: Vec<_> = app.workspace.hosts.iter().map(|h| h.key.window).collect();
        windows.extend(app.ocr_windows.keys().copied());
        windows.extend(app.video_previews.keys().copied());
        windows.push(app.main);
        Some((app.main, windows))
    });
    if let Some((main, windows)) = windows {
        let visible: Vec<_> = windows
            .into_iter()
            .filter(|window| unsafe { IsWindowVisible(*window as HWND) } != 0)
            .collect();
        let restore = STATE.with(|s| {
            let b = s.borrow();
            let a = b.as_ref().unwrap();
            visible
                .iter()
                .map(|&w| {
                    let id = a
                        .workspace
                        .hosts
                        .iter()
                        .find(|h| h.key.window == w)
                        .map(|h| h.key.identity)
                        .or_else(|| a.ocr_windows.get(&w).map(|o| o.identity))
                        .or_else(|| a.video_previews.get(&w).map(|p| p.clip.identity))
                        .unwrap_or(0);
                    (
                        w,
                        id,
                        if unsafe { IsIconic(w as HWND) } != 0 {
                            SW_SHOWMINNOACTIVE
                        } else {
                            SW_SHOWNA
                        },
                    )
                })
                .collect()
        });
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().restore = restore);
        video_ui::pause_all();
        status(main as HWND, &tr("capture"));
        for window in visible {
            unsafe {
                ShowWindow(window as HWND, SW_HIDE);
            }
        }
        unsafe {
            SetTimer(main as HWND, CAPTURE_TIMER, 35, None);
        }
    }
}

fn perform_capture() {
    let (main, task, sender) = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        let Some(pending) = a.pending.as_ref() else {
            return (a.main, None, a.tx.clone());
        };
        if Instant::now() < pending.ready_at {
            return (a.main, None, a.tx.clone());
        }
        (a.main, a.pending.take(), a.tx.clone())
    });
    if task.is_none() {
        return;
    }
    unsafe {
        KillTimer(main as HWND, CAPTURE_TIMER);
    }
    if let Some(pending) = task {
        let mode = pending.mode;
        let started = pending.started;
        std::thread::spawn(move || {
            let clock = Instant::now();
            let result = capture::desktop_rect().and_then(|rect| {
                let windows = capture::window_candidates();
                capture::snapshot(rect).map(|frame| (frame, rect, windows))
            });
            trace::record("capture.desktop-snapshot", clock);
            let _ = sender.send(Work::Snapshot {
                mode,
                started,
                result,
            });
            unsafe {
                PostMessageW(main as HWND, COMPLETE, 0, 0);
            }
        });
    }
}

fn restore_windows() {
    let windows = STATE.with(|s| std::mem::take(&mut s.borrow_mut().as_mut().unwrap().restore));
    for (window, identity, show) in windows {
        let live = STATE.with(|s| {
            let b = s.borrow();
            let a = b.as_ref().unwrap();
            (a.main == window && identity == 0)
                || a.workspace
                    .hosts
                    .iter()
                    .any(|h| h.key.window == window && h.key.identity == identity)
                || a.ocr_windows
                    .get(&window)
                    .is_some_and(|o| o.identity == identity)
                || a.video_previews
                    .get(&window)
                    .is_some_and(|p| p.clip.identity == identity)
        });
        unsafe {
            if live && IsWindow(window as HWND) != 0 {
                ShowWindow(window as HWND, show);
            }
        }
    }
    let main = STATE.with(|s| s.borrow().as_ref().unwrap().main) as HWND;
    status(main, &tr("ready"));
}

fn show_selection(
    mode: Mode,
    started: Instant,
    frame: Frame,
    desktop: Rect,
    windows: Vec<Candidate>,
) {
    let test_video =
        settings::is_test_run() && matches!(mode, Mode::VideoMonitor | Mode::VideoWindow);
    let window = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST
                | if test_video {
                    WS_EX_APPWINDOW
                } else {
                    WS_EX_TOOLWINDOW
                }
                | if matches!(mode, Mode::Scroll) {
                    WS_EX_NOACTIVATE
                } else {
                    0
                },
            wide(OVERLAY).as_ptr(),
            wide(&tr(match mode {
                Mode::Window | Mode::VideoWindow => "window",
                Mode::VideoMonitor => "video_monitor",
                Mode::Scroll => "scroll",
                _ => "area",
            }))
            .as_ptr(),
            WS_POPUP
                | if matches!(mode, Mode::Scroll) {
                    WS_CLIPCHILDREN
                } else {
                    0
                },
            desktop.x,
            desktop.y,
            desktop.width as i32,
            desktop.height as i32,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        )
    };
    if window.is_null() {
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().capture_busy = false);
        restore_windows();
        notify(null_mut(), "Cannot open capture selection.", true);
        return;
    }
    let mut cursor = unsafe { zeroed() };
    unsafe {
        GetCursorPos(&mut cursor);
    }
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        let app = state.as_mut().unwrap();
        app.next_detection_identity = app.next_detection_identity.wrapping_add(1).max(1);
        let detection_identity = app.next_detection_identity;
        app.selection = Some(Selection {
            window: window as usize,
            detection_identity,
            frame: Arc::new(frame),
            desktop,
            mode,
            windows,
            start: None,
            end: (cursor.x - desktop.x, cursor.y - desktop.y),
            detection: matches!(mode, Mode::Scroll).then(DetectionState::default),
            detected_pane: None,
            hover_point: None,
            hover_requested: false,
            hover_changed: Instant::now(),
            pressed_pane: None,
            first_paint: Some(started),
        })
    });
    if matches!(mode, Mode::Scroll) {
        child(
            window,
            "STATIC",
            &tr("scroll_click"),
            SCROLL_DETECT_STATUS,
            0,
        );
        child(
            window,
            "BUTTON",
            &tr("scroll_start"),
            SCROLL_DETECT_START as i32,
            WS_TABSTOP,
        );
        child(
            window,
            "BUTTON",
            &tr("cancel"),
            SCROLL_DETECT_CANCEL as i32,
            WS_TABSTOP,
        );
        let dpi = unsafe { GetDpiForWindow(window) }.max(96) as i32;
        let logical_width = desktop.width as i32 * 96 / dpi;
        let logical_height = desktop.height as i32 * 96 / dpi;
        let center = logical_width / 2;
        let bottom = logical_height - 76;
        position(
            window,
            SCROLL_DETECT_STATUS,
            center - 260,
            bottom - 30,
            520,
            24,
        );
        position(
            window,
            SCROLL_DETECT_START as i32,
            center - 150,
            bottom,
            150,
            34,
        );
        position(
            window,
            SCROLL_DETECT_CANCEL as i32,
            center + 10,
            bottom,
            120,
            34,
        );
        unsafe { EnableWindow(GetDlgItem(window, SCROLL_DETECT_START as i32), 0) };
        unsafe {
            RegisterHotKey(window, SCROLL_ESCAPE_HOTKEY, MOD_NOREPEAT, VK_ESCAPE as u32);
            SetTimer(window, SCROLL_HOVER_TIMER, 60, None);
        }
    }
    unsafe {
        ShowWindow(
            window,
            if matches!(mode, Mode::Scroll) {
                SW_SHOWNOACTIVATE
            } else {
                SW_SHOW
            },
        );
        if !matches!(mode, Mode::Scroll) {
            SetForegroundWindow(window);
            SetFocus(window);
        }
        UpdateWindow(window);
    }
}

fn update_scroll_hover(point: (i32, i32)) {
    let reset = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let app = state.as_mut()?;
        let selection = app.selection.as_mut()?;
        if !matches!(selection.mode, Mode::Scroll)
            || selection.start.is_some()
            || selection.hover_point == Some(point)
        {
            return None;
        }
        selection.hover_point = Some(point);
        selection.hover_changed = Instant::now();
        let retained = selection
            .detected_pane
            .as_ref()
            .filter(|p| {
                p.pane.bounds.contains(point.0, point.1)
                    && scroll_detect::is_current(p)
                    && capture::window_at(&selection.windows, point.0, point.1)
                        .is_some_and(|(_, w)| w.window == p.target.window)
            })
            .cloned();
        // A positively identified sheet covers the whole cached grid viewport.
        if retained
            .as_ref()
            .is_some_and(|p| p.source == scroll_detect::Source::Sheet)
        {
            return None;
        }
        selection.hover_requested = false;
        selection.detected_pane = retained;
        let generation = selection.detection.as_mut()?.begin_hover()?;
        if let Some(p) = selection.detected_pane.as_ref() {
            selection
                .detection
                .as_mut()?
                .detection_finished(generation, Some(p.pane.clone()));
        }
        app.detector.cancel(selection.detection_identity as usize);
        Some((selection.window as HWND, selection.detected_pane.is_some()))
    });
    if let Some((window, retained)) = reset {
        unsafe {
            if !retained {
                SetWindowTextW(
                    GetDlgItem(window, SCROLL_DETECT_STATUS),
                    wide(&tr("scroll_click")).as_ptr(),
                );
                EnableWindow(GetDlgItem(window, SCROLL_DETECT_START as i32), 0);
                InvalidateRect(window, null(), 0);
            }
        }
    }
}

fn poll_scroll_hover() {
    if unsafe { GetAsyncKeyState(VK_ESCAPE as i32) } < 0 {
        finish_selection(true);
        return;
    }
    let point = STATE.with(|state| {
        let state = state.borrow();
        let selection = state.as_ref()?.selection.as_ref()?;
        matches!(selection.mode, Mode::Scroll).then_some((
            selection.desktop.x + selection.end.0,
            selection.desktop.y + selection.end.1,
        ))
    });
    let Some(point) = point else {
        return;
    };
    update_scroll_hover(point);
    let ready = STATE.with(|state| {
        let state = state.borrow();
        state
            .as_ref()
            .and_then(|a| a.selection.as_ref())
            .is_some_and(|s| {
                s.start.is_none()
                    && !s.hover_requested
                    && s.hover_changed.elapsed() >= std::time::Duration::from_millis(120)
            })
    });
    if ready {
        begin_scroll_detection(point);
    }
}

fn begin_scroll_detection(point: (i32, i32)) {
    let request = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let app = state.as_mut().unwrap();
        let Some(selection) = app.selection.as_mut() else {
            return None;
        };
        if !matches!(selection.mode, Mode::Scroll) {
            return None;
        }
        let generation = selection.detection.as_mut()?.begin_hover()?;
        selection.hover_requested = true;
        if let Some(p) = selection.detected_pane.as_ref() {
            selection
                .detection
                .as_mut()?
                .detection_finished(generation, Some(p.pane.clone()));
        }
        Some((
            app.detector.clone(),
            DetectionRequest {
                owner: selection.detection_identity as usize,
                notifier: app.main,
                notify_message: COMPLETE,
                generation,
                point,
                desktop: selection.desktop,
                windows: selection.windows.clone(),
                snapshot: Some(selection.frame.clone()),
            },
        ))
    });
    let Some((detector, request)) = request else {
        return;
    };
    trace::record("scroll.detect.hover-accepted", Instant::now());
    let window = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|app| app.selection.as_ref())
            .map(|selection| (selection.window as HWND, selection.detected_pane.is_some()))
    });
    if let Some((window, retained)) = window {
        unsafe {
            if !retained {
                SetWindowTextW(
                    GetDlgItem(window, SCROLL_DETECT_STATUS),
                    wide(&tr("scroll_detecting")).as_ptr(),
                );
                EnableWindow(GetDlgItem(window, SCROLL_DETECT_START as i32), 0);
            }
        }
    }
    detector.submit(request);
}

fn complete_detection(result: DetectionResult) {
    if !result.finished {
        return;
    }
    let display = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let app = state.as_mut().unwrap();
        let Some(selection) = app.selection.as_mut() else {
            return None;
        };
        if selection.detection_identity as usize != result.owner {
            return None;
        }
        let Some(detection) = selection.detection.as_mut() else {
            return None;
        };
        let candidate = result.result;
        detection.detection_finished(
            result.generation,
            candidate.as_ref().map(|detected| detected.pane.clone()),
        );
        if !matches!(detection, DetectionState::Preview { generation, .. } if *generation == result.generation)
            && !matches!(detection, DetectionState::NoMatch { generation } if *generation == result.generation)
        {
            return None;
        }
        selection.detected_pane = candidate;
        let title = selection
            .detected_pane
            .as_ref()
            .map(|detected| detected.pane.title.clone());
        let source = selection.detected_pane.as_ref().map(|p| p.source);
        Some((selection.window as HWND, title, result.timed_out, source))
    });
    if let Some((window, title, timed_out, source)) = display {
        let found = title.is_some();
        let message = title.map_or_else(
            || {
                tr(if timed_out {
                    "scroll_detect_timeout"
                } else {
                    "scroll_no_match"
                })
            },
            |title| {
                tr(if source == Some(scroll_detect::Source::Window) {
                    "scroll_window_ready"
                } else {
                    "scroll_preview"
                })
                .replace("{title}", &title)
            },
        );
        unsafe {
            SetWindowTextW(
                GetDlgItem(window, SCROLL_DETECT_STATUS),
                wide(&message).as_ptr(),
            );
            EnableWindow(GetDlgItem(window, SCROLL_DETECT_START as i32), found as i32);
            InvalidateRect(window, null(), 0);
            UpdateWindow(window);
        }
    }
}

fn confirm_scroll_selection(click: Option<((i32, i32), scroll_detect::PaneCandidate)>) {
    let ready = STATE.with(|state| {
        let mut state = state.borrow_mut();
        let app = state.as_mut().unwrap();
        let Some(selection) = app.selection.as_mut() else {
            return None;
        };
        let Some(detected) = selection.detected_pane.as_ref() else {
            return None;
        };
        if !scroll_detect::is_current(detected) {
            selection.detected_pane = None;
            if let Some(detection) = selection.detection.as_mut() {
                let generation = match detection {
                    DetectionState::Preview { generation, .. }
                    | DetectionState::NoMatch { generation }
                    | DetectionState::Detecting { generation }
                    | DetectionState::AwaitingClick { generation }
                    | DetectionState::Started { generation } => *generation,
                    DetectionState::Cancelled => 0,
                };
                *detection = DetectionState::NoMatch { generation };
            }
            return Some(Err(selection.window as HWND));
        }
        let candidate = if let Some((point, pressed)) = click.as_ref() {
            selection.detection.as_mut()?.confirm_at(*point, pressed)?
        } else {
            selection.detection.as_mut()?.confirm()?
        };
        if candidate != detected.pane {
            return None;
        }
        let detected = selection.detected_pane.take()?;
        let selection = app.selection.take()?;
        app.detector.cancel(selection.detection_identity as usize);
        app.capture_busy = false;
        Some(Ok((selection.window as HWND, detected)))
    });
    match ready {
        Some(Ok((overlay, detected))) => {
            unsafe {
                UnregisterHotKey(overlay, SCROLL_ESCAPE_HOTKEY);
                KillTimer(overlay, SCROLL_HOVER_TIMER);
                ReleaseCapture();
                SetForegroundWindow(detected.target.window as HWND);
                DestroyWindow(overlay);
            }
            start_scroll(detected.pane.bounds, detected.target);
        }
        Some(Err(overlay)) => unsafe {
            SetWindowTextW(
                GetDlgItem(overlay, SCROLL_DETECT_STATUS),
                wide(&tr("scroll_stale")).as_ptr(),
            );
            EnableWindow(GetDlgItem(overlay, SCROLL_DETECT_START as i32), 0);
            InvalidateRect(overlay, null(), 0);
        },
        None => {}
    }
}

fn selection_rect(selection: &Selection) -> Option<Rect> {
    if matches!(selection.mode, Mode::Scroll) {
        selection
            .detected_pane
            .as_ref()
            .map(|detected| detected.pane.bounds)
    } else if matches!(selection.mode, Mode::Window | Mode::VideoWindow) {
        let (x, y) = (
            selection.desktop.x + selection.end.0,
            selection.desktop.y + selection.end.1,
        );
        capture::window_at(&selection.windows, x, y)
            .map(|(_, window)| window.rect)
            .or_else(|| capture::monitor_at(x, y).ok())?
            .intersection(selection.desktop)
    } else if matches!(selection.mode, Mode::VideoMonitor) {
        let (x, y) = (
            selection.desktop.x + selection.end.0,
            selection.desktop.y + selection.end.1,
        );
        capture::monitor_at(x, y)
            .ok()?
            .intersection(selection.desktop)
    } else {
        let (x, y) = selection.start?;
        Rect::from_edges(
            selection.desktop.x + x.min(selection.end.0),
            selection.desktop.y + y.min(selection.end.1),
            selection.desktop.x + x.max(selection.end.0),
            selection.desktop.y + y.max(selection.end.1),
        )
        .ok()
    }
}

fn finish_selection(cancel: bool) {
    let started = Instant::now();
    let selection = STATE.with(|s| s.borrow_mut().as_mut().unwrap().selection.take());
    if let Some(mut selection) = selection {
        if matches!(selection.mode, Mode::Scroll) {
            if !cancel {
                STATE.with(|s| s.borrow_mut().as_mut().unwrap().selection = Some(selection));
                return;
            }
            STATE.with(|s| {
                let mut state = s.borrow_mut();
                let app = state.as_mut().unwrap();
                app.capture_busy = false;
                app.detector.cancel(selection.detection_identity as usize);
            });
            if let Some(detection) = selection.detection.as_mut() {
                detection.cancel();
            }
            unsafe {
                ReleaseCapture();
                UnregisterHotKey(selection.window as HWND, SCROLL_ESCAPE_HOTKEY);
                KillTimer(selection.window as HWND, SCROLL_HOVER_TIMER);
                DestroyWindow(selection.window as HWND);
            }
            restore_windows();
            return;
        }
        if matches!(selection.mode, Mode::VideoMonitor | Mode::VideoWindow) {
            STATE.with(|s| s.borrow_mut().as_mut().unwrap().capture_busy = false);
            let source = if cancel {
                None
            } else {
                let x = selection.desktop.x + selection.end.0;
                let y = selection.desktop.y + selection.end.1;
                Some(match selection.mode {
                    Mode::VideoMonitor => {
                        capture::monitor_handle_at(x, y).map(|handle| video::Source::Monitor {
                            handle: handle as u64,
                        })
                    }
                    Mode::VideoWindow => capture::window_at(&selection.windows, x, y)
                        .ok_or_else(|| "Select an application window to record.".to_string())
                        .and_then(|(_, target)| {
                            capture::window_pid(target.window).map(|pid| video::Source::Window {
                                handle: target.window as u64,
                                pid,
                            })
                        }),
                    _ => unreachable!(),
                })
            };
            unsafe {
                ReleaseCapture();
                DestroyWindow(selection.window as HWND);
            }
            match source {
                Some(Ok(source)) => start_video(source),
                Some(Err(error)) => {
                    restore_windows();
                    notify(null_mut(), &error, true);
                }
                None => restore_windows(),
            }
            return;
        }
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().capture_busy = false);
        let result = if cancel {
            None
        } else {
            let rect = if matches!(selection.mode, Mode::Window) {
                let (x, y) = (
                    selection.desktop.x + selection.end.0,
                    selection.desktop.y + selection.end.1,
                );
                if crate::settings::is_test_run() {
                    let hits: Vec<_> = selection
                        .windows
                        .iter()
                        .enumerate()
                        .filter(|(_, candidate)| candidate.rect.contains(x, y))
                        .collect();
                    let summary = hits
                        .iter()
                        .take(8)
                        .map(|(index, candidate)| {
                            format!(
                                "{index}:{}:{}:{}:{}:{}",
                                candidate.rect.x,
                                candidate.rect.y,
                                candidate.rect.width,
                                candidate.rect.height,
                                candidate.window
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(";");
                    crate::trace::test_note(&format!(
                        "window-click={x},{y} desktop={},{},{},{} hits={summary}",
                        selection.desktop.x,
                        selection.desktop.y,
                        selection.desktop.width,
                        selection.desktop.height
                    ));
                }
                match capture::window_at(&selection.windows, x, y) {
                    Some((index, target)) => {
                        if crate::settings::is_test_run() {
                            crate::trace::test_note(&format!(
                                "window-selected index={index} rect={},{},{},{} hwnd={}",
                                target.rect.x,
                                target.rect.y,
                                target.rect.width,
                                target.rect.height,
                                target.window
                            ));
                        }
                        Some(
                            capture::validate_window(&selection.windows, index, selection.desktop)
                                .and_then(|rect| {
                                    let frame = selection.frame.crop(selection.desktop, rect)?;
                                    if crate::settings::is_test_run() {
                                        let offset = ((frame.height as usize / 2
                                            * frame.width as usize)
                                            + (frame.width as usize / 2))
                                            * 4;
                                        crate::trace::test_note(&format!(
                                            "window-frame={}x{} center-bgra={:?}",
                                            frame.width,
                                            frame.height,
                                            &frame.bgra[offset..offset + 4]
                                        ));
                                    }
                                    Ok(frame)
                                }),
                        )
                    }
                    None => selection_rect(&selection)
                        .map(|rect| selection.frame.crop(selection.desktop, rect)),
                }
            } else {
                selection_rect(&selection).map(|rect| selection.frame.crop(selection.desktop, rect))
            };
            rect
        };
        unsafe {
            ReleaseCapture();
            DestroyWindow(selection.window as HWND);
        }
        restore_windows();
        match result {
            Some(Ok(frame)) => present(frame, started),
            Some(Err(error)) => {
                let message = if error.starts_with("Part of this window") {
                    tr("covered")
                } else {
                    error
                };
                notify(null_mut(), &message, true);
            }
            None => {}
        }
    }
}

fn default_video_path(pending: bool) -> Result<PathBuf, String> {
    let mut directory = settings::data_dir().join("Recordings");
    if pending {
        directory.push("Pending");
    }
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for suffix in 0..10_000u32 {
        let name = if suffix == 0 {
            format!("SnapZy-{stamp}.mp4")
        } else {
            format!("SnapZy-{stamp}-{suffix}.mp4")
        };
        let path = directory.join(name);
        if !path.try_exists().map_err(|e| e.to_string())? {
            return Ok(path);
        }
    }
    Err("Cannot choose a unique video filename.".into())
}

fn video_destination(owner: HWND) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::OsStringExt;
    let initial = default_video_path(false)?;
    let mut filename = [0u16; 32768];
    let value = wide(&initial.to_string_lossy());
    if value.len() > filename.len() {
        return Err("The recording path is too long.".into());
    }
    filename[..value.len()].copy_from_slice(&value);
    let filter: Vec<u16> = "MP4 video (*.mp4)\0*.mp4\0\0".encode_utf16().collect();
    let extension = wide("mp4");
    let title = wide(&tr("video_save"));
    let mut dialog: OPENFILENAMEW = unsafe { zeroed() };
    dialog.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    dialog.hwndOwner = owner;
    dialog.lpstrFile = filename.as_mut_ptr();
    dialog.nMaxFile = filename.len() as u32;
    dialog.lpstrFilter = filter.as_ptr();
    dialog.nFilterIndex = 1;
    dialog.lpstrDefExt = extension.as_ptr();
    dialog.lpstrTitle = title.as_ptr();
    dialog.Flags = OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
    if unsafe { GetSaveFileNameW(&mut dialog) } == 0 {
        let error = unsafe { CommDlgExtendedError() };
        return if error == 0 {
            Ok(None)
        } else {
            Err(format!("Save dialog failed ({error})."))
        };
    }
    let end = filename
        .iter()
        .position(|c| *c == 0)
        .unwrap_or(filename.len());
    let path = PathBuf::from(std::ffi::OsString::from_wide(&filename[..end]));
    if path.try_exists().map_err(|e| e.to_string())? {
        return Err("Choose a new filename; existing files are not replaced.".into());
    }
    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mp4"))
    {
        return Err("Choose an MP4 filename in an existing folder.".into());
    }
    Ok(Some(path))
}

fn launcher_menu_commands(control: usize) -> &'static [usize] {
    match control {
        WINDOW_MENU => &[WINDOW, DESKTOP],
        SCROLL_OPTIONS => &[SCROLL_AUTO, SCROLL_MANUAL],
        OPEN_OPTIONS => &[OPEN_TAB, OPEN_WINDOW],
        MORE => &[KEYS, ABOUT, HIDE, EXIT],
        _ => &[],
    }
}

fn editor_menu_commands(control: usize) -> &'static [usize] {
    match control {
        ec::LAYERS => &[ec::FRONT, ec::FORWARD, ec::BACKWARD, ec::BACK],
        ec::MORE => &[
            ec::ADD_IMAGE,
            ec::PASTE_IMAGE,
            ec::CANVAS,
            ec::EFFECTS,
            ec::COMBINE,
            ec::PROJECT_OPEN,
            ec::PROJECT_SAVE,
        ],
        _ => &[],
    }
}

fn show_editor_menu(owner: HWND, control: usize) {
    let commands = editor_menu_commands(control);
    if commands.is_empty() {
        return;
    }
    let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
    unsafe {
        let menu = CreatePopupMenu();
        let mut icons = native_icons::MenuIcons::default();
        for &command in commands {
            if control == ec::MORE && matches!(command, ec::CANVAS | ec::PROJECT_OPEN) {
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
            }
            AppendMenuW(
                menu,
                MF_STRING,
                command,
                wide(ec::menu_label(command, thai)).as_ptr(),
            );
            use native_icons::Icon;
            let icon = match command {
                ec::FRONT => Some(Icon::ArrowUpToLine),
                ec::FORWARD => Some(Icon::ArrowUp),
                ec::BACKWARD => Some(Icon::ArrowDown),
                ec::BACK => Some(Icon::ArrowDownToLine),
                _ => ec::icon_command(command).map(native_icons::icon_for),
            };
            if let Some(icon) = icon {
                icons.add(menu, command, icon, GetDpiForWindow(owner));
            }
        }
        let mut rect: RECT = zeroed();
        GetWindowRect(GetDlgItem(owner, control as i32), &mut rect);
        SetForegroundWindow(owner);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            rect.left,
            rect.bottom,
            0,
            owner,
            null(),
        );
        DestroyMenu(menu);
        PostMessageW(owner, WM_NULL, 0, 0);
        if command != 0 {
            PostMessageW(owner, COMMAND, command as usize, 0);
        }
    }
}

fn show_launcher_menu(owner: HWND, control: usize) {
    let commands = launcher_menu_commands(control);
    if commands.is_empty() {
        return;
    }
    unsafe {
        let menu = CreatePopupMenu();
        let mut icons = native_icons::MenuIcons::default();
        let settings = STATE.with(|s| s.borrow().as_ref().unwrap().settings.clone());
        for &command in commands {
            if control == MORE && command == EXIT {
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
            }
            let checked = match command {
                SCROLL_AUTO => settings.scroll_automatic,
                SCROLL_MANUAL => !settings.scroll_automatic,
                OPEN_TAB => settings.open_mode == settings::OpenMode::Tab,
                OPEN_WINDOW => settings.open_mode == settings::OpenMode::Window,
                _ => false,
            };
            let key = match command {
                WINDOW => "window",
                DESKTOP => "desktop",
                SCROLL_AUTO => {
                    if settings.language == "th" {
                        "อัตโนมัติ"
                    } else {
                        "Automatic"
                    }
                }
                SCROLL_MANUAL => {
                    if settings.language == "th" {
                        "เลื่อนเอง"
                    } else {
                        "Manual"
                    }
                }
                OPEN_TAB => {
                    if settings.language == "th" {
                        "เปิดเป็นแท็บใหม่"
                    } else {
                        "New tab"
                    }
                }
                OPEN_WINDOW => {
                    if settings.language == "th" {
                        "เปิดเป็นหน้าต่างใหม่"
                    } else {
                        "New window"
                    }
                }
                KEYS => "keys",
                ABOUT => "about",
                HIDE => "hide",
                EXIT => "exit",
                _ => "",
            };
            let title = if matches!(
                command,
                SCROLL_AUTO | SCROLL_MANUAL | OPEN_TAB | OPEN_WINDOW
            ) {
                key.to_string()
            } else {
                tr(key)
            };
            AppendMenuW(
                menu,
                MF_STRING | if checked { MF_CHECKED } else { 0 },
                command,
                wide(&title).as_ptr(),
            );
            let icon = match command {
                WINDOW => Some(IconCommand::WindowCapture),
                DESKTOP => Some(IconCommand::DesktopCapture),
                KEYS => Some(IconCommand::Hotkeys),
                ABOUT => Some(IconCommand::About),
                HIDE => Some(IconCommand::HideToTray),
                EXIT => Some(IconCommand::Cancel),
                _ => None,
            };
            if let Some(icon) = icon {
                icons.add(
                    menu,
                    command,
                    native_icons::icon_for(icon),
                    GetDpiForWindow(owner),
                );
            }
        }
        let mut rect: RECT = zeroed();
        GetWindowRect(GetDlgItem(owner, control as i32), &mut rect);
        SetForegroundWindow(owner);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            rect.left,
            rect.bottom,
            0,
            owner,
            null(),
        );
        DestroyMenu(menu);
        PostMessageW(owner, WM_NULL, 0, 0);
        if command != 0 {
            PostMessageW(owner, COMMAND, command as usize, 0);
        }
    }
}

fn delay_choices() -> [(usize, u32, &'static str); 4] {
    [
        (DELAY_NONE, 0, "delay_none"),
        (DELAY_THREE, 3, "delay_three"),
        (DELAY_FIVE, 5, "delay_five"),
        (DELAY_TEN, 10, "delay_ten"),
    ]
}

fn show_delay_menu(owner: HWND) {
    if STATE.with(|s| {
        let a = s.borrow();
        let app = a.as_ref().unwrap();
        app.capture_busy || app.recording.is_some() || app.selection.is_some()
    }) {
        return;
    }
    let selected = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .map(|app| app.settings.capture_delay_seconds)
            .unwrap_or(0)
    });
    unsafe {
        let menu = CreatePopupMenu();
        for (id, seconds, key) in delay_choices() {
            AppendMenuW(
                menu,
                MF_STRING | if selected == seconds { MF_CHECKED } else { 0 },
                id,
                wide(&tr(key)).as_ptr(),
            );
        }
        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(owner);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            point.x,
            point.y,
            0,
            owner,
            null(),
        );
        DestroyMenu(menu);
        PostMessageW(owner, WM_NULL, 0, 0);
        if command != 0 {
            PostMessageW(owner, COMMAND, command as usize, 0);
        }
    }
}

fn set_capture_delay(window: HWND, seconds: u32) {
    if !settings::CAPTURE_DELAY_PRESETS.contains(&seconds) {
        return;
    }
    let mut value = STATE.with(|s| s.borrow().as_ref().unwrap().settings.clone());
    value.capture_delay_seconds = seconds;
    match value.save() {
        Ok(()) => {
            STATE.with(|s| s.borrow_mut().as_mut().unwrap().settings = value);
            refresh_delay();
            status(
                window,
                &tr(match seconds {
                    0 => "delay_none",
                    3 => "delay_three",
                    5 => "delay_five",
                    10 => "delay_ten",
                    _ => "delay_none",
                }),
            );
        }
        Err(error) => notify(window, &error, true),
    }
}

fn show_video_menu(owner: HWND) {
    if STATE.with(|s| s.borrow().as_ref().unwrap().video_exit.is_some()) {
        return;
    }
    if STATE.with(|s| s.borrow().as_ref().unwrap().recording.is_some()) {
        stop_video();
        return;
    }
    unsafe {
        let menu = CreatePopupMenu();
        AppendMenuW(
            menu,
            MF_STRING,
            VIDEO_MONITOR,
            wide(&tr("video_monitor")).as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            VIDEO_WINDOW,
            wide(&tr("video_window")).as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        let selected = STATE.with(|s| s.borrow().as_ref().unwrap().settings.video_audio);
        for (id, key, mode) in video_audio_choices() {
            AppendMenuW(
                menu,
                MF_STRING | if selected == mode { MF_CHECKED } else { 0 },
                id,
                wide(&tr(key)).as_ptr(),
            );
        }
        if let Some((id, _, _)) = video_audio_choices()
            .into_iter()
            .find(|(_, _, mode)| *mode == selected)
        {
            CheckMenuRadioItem(
                menu,
                VIDEO_AUDIO_NONE as u32,
                VIDEO_AUDIO_BOTH as u32,
                id as u32,
                MF_BYCOMMAND,
            );
        }
        let mut point: POINT = zeroed();
        GetCursorPos(&mut point);
        SetForegroundWindow(owner);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            point.x,
            point.y,
            0,
            owner,
            null(),
        );
        DestroyMenu(menu);
        PostMessageW(owner, WM_NULL, 0, 0);
        if command != 0 {
            PostMessageW(owner, COMMAND, command as usize, 0);
        }
    }
}

fn video_audio_choices() -> [(usize, &'static str, crate::audio::Mode); 4] {
    use crate::audio::Mode;
    [
        (VIDEO_AUDIO_NONE, "video_audio_none", Mode::None),
        (VIDEO_AUDIO_SYSTEM, "video_audio_system", Mode::System),
        (VIDEO_AUDIO_MIC, "video_audio_mic", Mode::Microphone),
        (VIDEO_AUDIO_BOTH, "video_audio_both", Mode::Both),
    ]
}

fn start_video(source: video::Source) {
    let output = match default_video_path(true) {
        Ok(path) => path,
        Err(error) => {
            restore_windows();
            notify(null_mut(), &error, true);
            return;
        }
    };
    let panel = create_window(
        VIDEO_PANEL,
        TITLE,
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        null_mut(),
        380,
        140,
    );
    if panel.is_null() {
        restore_windows();
        notify(null_mut(), "Cannot open video recording controls.", true);
        return;
    }
    child(panel, "STATIC", &tr("video_preparing"), STATUS, 0);
    child(
        panel,
        "BUTTON",
        &tr("video_stop"),
        VIDEO_STOP as i32,
        WS_TABSTOP,
    );
    layout(panel);
    unsafe {
        let excluded = SetWindowDisplayAffinity(panel, WDA_EXCLUDEFROMCAPTURE) != 0;
        RegisterHotKey(panel, VIDEO_ESCAPE_HOTKEY, MOD_NOREPEAT, VK_ESCAPE as u32);
        if excluded {
            ShowWindow(panel, SW_SHOWNOACTIVATE);
            UpdateWindow(panel);
        }
    }
    let (session, main) = STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        let app = borrow.as_mut().unwrap();
        app.next_video_session = app.next_video_session.wrapping_add(1).max(1);
        (app.next_video_session, app.main)
    });
    let request = video::Request {
        audio: STATE.with(|s| s.borrow().as_ref().unwrap().settings.video_audio),
        session,
        source,
        output: output.clone(),
    };
    match VideoService::start(request) {
        Ok(service) => {
            debug_assert_eq!(service.session(), session);
            STATE.with(|s| {
                s.borrow_mut().as_mut().unwrap().recording = Some(VideoRecording {
                    panel: panel as usize,
                    service,
                    state: video::State::new(0),
                    clock: Instant::now(),
                    elapsed_ms: 0,
                    frames: 0,
                });
            });
            unsafe {
                SetTimer(main as HWND, VIDEO_TIMER, 100, None);
            }
            refresh_video_status();
        }
        Err(error) => {
            unsafe {
                UnregisterHotKey(panel, VIDEO_ESCAPE_HOTKEY);
                DestroyWindow(panel);
            }
            restore_windows();
            notify(null_mut(), &error, true);
        }
    }
}

fn refresh_video_status() {
    let display = STATE.with(|s| {
        let borrow = s.borrow();
        let recording = borrow.as_ref()?.recording.as_ref()?;
        let now = recording.clock.elapsed().as_millis().min(u64::MAX as u128) as u64;
        let text = match recording.state.phase {
            VideoPhase::Preparing => tr("video_preparing"),
            VideoPhase::Countdown => {
                let elapsed = now.saturating_sub(recording.state.since);
                let number = (3000u64.saturating_sub(elapsed).saturating_add(999) / 1000).max(1);
                number.to_string()
            }
            VideoPhase::Starting => tr("video_starting"),
            VideoPhase::Recording => format!(
                "REC  {}   {} frames",
                video::elapsed_text(recording.elapsed_ms),
                recording.frames
            ),
            VideoPhase::Finalizing => tr("video_finalizing"),
            VideoPhase::Complete | VideoPhase::Cancelled | VideoPhase::Failed => String::new(),
        };
        Some((recording.panel as HWND, text))
    });
    if let Some((panel, text)) = display {
        status(panel, &text);
    }
}

fn stop_video() {
    let command_sent = STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        let recording = borrow.as_mut()?.recording.as_mut()?;
        let now = recording.clock.elapsed().as_millis().min(u64::MAX as u128) as u64;
        if !recording.state.stop(now) {
            return Some(true);
        }
        Some(recording.service.command(VideoCommand::Stop))
    });
    if command_sent == Some(false) {
        finish_video(VideoEvent::Failed {
            session: 0,
            detail: "Cannot stop the video worker.".into(),
            partial: None,
        });
    } else {
        refresh_video_status();
    }
}

fn poll_video() {
    let events = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .and_then(|app| app.recording.as_ref())
            .map(|recording| recording.service.poll())
            .unwrap_or_default()
    });
    for event in events {
        let terminal = matches!(
            event,
            VideoEvent::Saved { .. } | VideoEvent::Cancelled { .. } | VideoEvent::Failed { .. }
        );
        STATE.with(|s| {
            let mut borrow = s.borrow_mut();
            let Some(recording) = borrow.as_mut().and_then(|app| app.recording.as_mut()) else {
                return;
            };
            let now = recording.clock.elapsed().as_millis().min(u64::MAX as u128) as u64;
            match &event {
                VideoEvent::Ready { .. } => {
                    recording.state.ready(now);
                }
                VideoEvent::Started { .. } => {
                    recording.state.started(now);
                }
                VideoEvent::Progress {
                    elapsed_ms, frames, ..
                } => {
                    recording.elapsed_ms = *elapsed_ms;
                    recording.frames = *frames;
                }
                VideoEvent::Finalizing { .. } => {
                    recording.state.stop(now);
                }
                _ => {}
            }
        });
        if terminal {
            finish_video(event);
            return;
        }
    }
    let action = STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        let recording = borrow.as_mut()?.recording.as_mut()?;
        let now = recording.clock.elapsed().as_millis().min(u64::MAX as u128) as u64;
        Some(recording.state.tick(now))
    });
    match action {
        Some(video::Action::Start) => {
            let sent = STATE.with(|s| {
                s.borrow()
                    .as_ref()
                    .and_then(|a| a.recording.as_ref())
                    .is_some_and(|r| r.service.command(VideoCommand::Start))
            });
            if !sent {
                finish_video(VideoEvent::Failed {
                    session: 0,
                    detail: "Cannot start the video worker.".into(),
                    partial: None,
                });
                return;
            }
        }
        Some(video::Action::Timeout) => {
            finish_video(VideoEvent::Failed {
                session: 0,
                detail: "Video recording timed out while preparing or finalizing.".into(),
                partial: None,
            });
            return;
        }
        _ => {}
    }
    refresh_video_status();
}

fn finish_video(event: VideoEvent) {
    let (recording, main, exit_after) = STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        let app = borrow.as_mut().unwrap();
        let recording = app.recording.take();
        let exit_after = std::mem::take(&mut app.exit_after_video);
        (recording, app.main, exit_after)
    });
    let Some(recording) = recording else {
        return;
    };
    let session = recording.service.session();
    unsafe {
        KillTimer(main as HWND, VIDEO_TIMER);
        UnregisterHotKey(recording.panel as HWND, VIDEO_ESCAPE_HOTKEY);
        if IsWindow(recording.panel as HWND) != 0 {
            DestroyWindow(recording.panel as HWND);
        }
    }
    drop(recording);
    restore_windows();
    match event {
        VideoEvent::Saved { path, reason, .. } => {
            video_ui::present(path, reason, session);
            if exit_after {
                exit_app();
            }
            return;
        }
        VideoEvent::Cancelled { .. } => notify(main as HWND, &tr("video_cancelled"), false),
        VideoEvent::Failed {
            detail, partial, ..
        } => {
            let detail = if detail.starts_with("Audio capture:") {
                tr("video_audio_error").replace("{detail}", &detail)
            } else {
                detail
            };
            let message = partial.map_or(detail.clone(), |path| {
                format!(
                    "{detail}\n\nPartial recording retained at:\n{}",
                    path.display()
                )
            });
            notify(main as HWND, &message, true);
        }
        _ => {}
    }
    if exit_after {
        exit_app();
    }
}

fn refresh_scroll_mode() {
    let (main, automatic, thai) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        (
            a.main as HWND,
            a.settings.scroll_automatic,
            a.settings.language == "th",
        )
    });
    unsafe {
        for (id, title, checked) in [
            (
                SCROLL_AUTO,
                if thai {
                    "อัตโนมัติ"
                } else {
                    "Automatic"
                },
                automatic,
            ),
            (
                SCROLL_MANUAL,
                if thai {
                    "เลื่อนเอง"
                } else {
                    "Manual"
                },
                !automatic,
            ),
        ] {
            SetWindowTextW(GetDlgItem(main, id as i32), wide(title).as_ptr());
            SendMessageW(
                GetDlgItem(main, id as i32),
                BM_SETCHECK,
                usize::from(checked),
                0,
            );
        }
        SetWindowTextW(
            GetDlgItem(main, SCROLL_MODE_LABEL),
            wide(if thai {
                "จับภาพยาว:"
            } else {
                "Scrolling:"
            })
            .as_ptr(),
        );
        SetWindowTextW(
            GetDlgItem(main, SCROLL_OPTIONS as i32),
            wide(scroll_mode_button_label(automatic, thai)).as_ptr(),
        );
    }
}

fn refresh_delay() {
    let (main, seconds) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        (a.main as HWND, a.settings.capture_delay_seconds)
    });
    unsafe {
        SetWindowTextW(
            GetDlgItem(main, DELAY_OPTIONS as i32),
            wide(&delay_button_label(
                seconds,
                STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th"),
            ))
            .as_ptr(),
        );
    }
}

fn refresh_open_mode() {
    let (main, mode, thai) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        (
            a.main as HWND,
            a.settings.open_mode,
            a.settings.language == "th",
        )
    });
    for (id, value, label) in [
        (
            OPEN_TAB,
            settings::OpenMode::Tab,
            if thai {
                "เปิดเป็นแท็บใหม่"
            } else {
                "New tab"
            },
        ),
        (
            OPEN_WINDOW,
            settings::OpenMode::Window,
            if thai {
                "เปิดเป็นหน้าต่างใหม่"
            } else {
                "New window"
            },
        ),
    ] {
        unsafe {
            SetWindowTextW(GetDlgItem(main, id as i32), wide(label).as_ptr());
            SendMessageW(
                GetDlgItem(main, id as i32),
                BM_SETCHECK,
                if mode == value {
                    BST_CHECKED
                } else {
                    BST_UNCHECKED
                } as usize,
                0,
            );
        }
    }
    unsafe {
        SetWindowTextW(
            GetDlgItem(main, OPEN_OPTIONS as i32),
            wide(open_mode_button_label(
                mode == settings::OpenMode::Tab,
                thai,
            ))
            .as_ptr(),
        );
    }
}
fn refresh_scroll_pause() {
    STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        if let Some(job) = &a.scrolling {
            let title = match (job.paused(), a.settings.language == "th") {
                (true, false) => "Resume",
                (false, false) => "Pause",
                (true, true) => "ทำต่อ",
                (false, true) => "พัก",
            };
            unsafe {
                SetWindowTextW(
                    GetDlgItem(job.window as HWND, SCROLL_PAUSE as i32),
                    wide(title).as_ptr(),
                );
            }
        }
    });
}
fn start_scroll(region: Rect, target: Candidate) {
    let (main, budget, full, automatic) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        (
            a.main,
            crate::image_limits::scroll_output_budget(
                crate::memory::available_physical_bytes().unwrap_or(0),
                crate::image_limits::SOURCE_LIMIT
                    .saturating_sub(a.previews.values().map(|p| p.frame.bgra.len()).sum()),
            ),
            a.previews.len() >= 16,
            a.settings.scroll_automatic,
        )
    });
    if full
        || region.width < 24
        || region.height < 80
        || region.width as usize * region.height as usize * 4 > budget
        || region.intersection(target.rect) != Some(region)
    {
        restore_windows();
        notify(null_mut(), &tr("scroll_region"), true);
        return;
    }
    let window = create_window(
        SCROLL_PANEL,
        &format!("{TITLE} - {}", tr("scroll")),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_CLIPCHILDREN,
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        null_mut(),
        360,
        120,
    );
    if window.is_null() {
        restore_windows();
        notify(null_mut(), &tr("scroll_controls"), true);
        return;
    }
    let mut bounds: RECT = unsafe { zeroed() };
    unsafe {
        GetWindowRect(window, &mut bounds);
    }
    let display = capture::monitor_at(
        region.x + region.width as i32 / 2,
        region.y + region.height as i32 / 2,
    );
    let position = display.ok().and_then(|d| {
        crate::scroll_job::controller_position(
            region,
            d,
            (bounds.right - bounds.left) as u32,
            (bounds.bottom - bounds.top) as u32,
        )
    });
    let Some((x, y)) = position else {
        unsafe {
            DestroyWindow(window);
        }
        restore_windows();
        notify(null_mut(), &tr("scroll_room"), true);
        return;
    };
    child(window, "STATIC", "", STATUS, 0);
    child(
        window,
        "BUTTON",
        if tr("ready") == "Ready" {
            "Pause"
        } else {
            "พัก"
        },
        SCROLL_PAUSE as i32,
        WS_TABSTOP,
    );
    child(
        window,
        "BUTTON",
        &tr("stop"),
        SCROLL_STOP as i32,
        WS_TABSTOP,
    );
    child(
        window,
        "BUTTON",
        &tr("cancel"),
        SCROLL_CANCEL as i32,
        WS_TABSTOP,
    );
    layout(window);
    let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
    status(
        window,
        &format!(
            "{}\r\n0 px",
            scroll_status(crate::scroll_job::Status::Settling, thai)
        ),
    );
    unsafe {
        SetWindowPos(
            window,
            HWND_TOPMOST,
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        );
        ShowWindow(window, SW_SHOWNA);
        UpdateWindow(window);
        let foreground_before = GetForegroundWindow();
        let activated = SetForegroundWindow(target.window as HWND);
        crate::trace::test_note(&format!(
            "scroll.target-activation before={foreground_before:?} target={} result={activated} after={:?}",
            target.window,
            GetForegroundWindow()
        ));
    }
    STATE.with(|s| {
        s.borrow_mut().as_mut().unwrap().scrolling = Some(crate::scroll_job::Job::start(
            window as usize,
            region,
            target,
            budget,
            automatic,
        ))
    });
    unsafe {
        RegisterHotKey(window, SCROLL_ESCAPE_HOTKEY, MOD_NOREPEAT, VK_ESCAPE as u32);
        SetTimer(main as HWND, SCROLL_TIMER, 100, None);
    }
}
fn cancel_scroll() {
    let (main, job) = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        (a.main, a.scrolling.take())
    });
    if let Some(job) = job {
        let window = job.window;
        drop(job);
        unsafe {
            KillTimer(main as HWND, SCROLL_TIMER);
            UnregisterHotKey(window as HWND, SCROLL_ESCAPE_HOTKEY);
            DestroyWindow(window as HWND);
        }
        restore_windows();
    }
}
fn scroll_status(state: crate::scroll_job::Status, thai: bool) -> &'static str {
    use crate::scroll_job::Status::*;
    match (state, thai) {
        (Settling, false) => "Waiting for stable image...",
        (Settling, true) => "กำลังรอภาพนิ่ง...",
        (Finishing, false) => "Finishing the last frame; no further scrolling...",
        (Finishing, true) => "กำลังเก็บภาพรอบสุดท้าย โดยไม่เลื่อนเพิ่ม...",
        (FinalUnconfirmed, false) => {
            "Could not confirm the final frame in time. Kept the valid partial image; the last scroll may be missing."
        }
        (FinalUnconfirmed, true) => "ยืนยันภาพรอบสุดท้ายไม่ทันเวลา เก็บภาพส่วนที่ต่อสำเร็จแล้ว ซึ่งอาจยังขาดช่วงท้าย",
        (Unstable, false) => "Paused: image keeps changing; wait or stop",
        (Unstable, true) => "พัก: ภาพยังเปลี่ยนต่อเนื่อง กรุณารอหรือหยุด",
        (Capturing, false) => "Capturing",
        (Capturing, true) => "กำลังจับภาพ",
        (Ambiguous, false) => "Paused: no confident overlap; retry or stop",
        (Ambiguous, true) => "พัก: รอยต่อไม่ชัดเจน กรุณาลองใหม่หรือหยุด",
        (TargetChanged, false) => "Paused: target inactive, moved or covered",
        (TargetChanged, true) => "พัก: สลับหน้าต่าง หน้าต่างย้าย ถูกย่อ หรือถูกบัง",
        (Limit, false) => "Image memory limit reached; kept valid partial image.",
        (Limit, true) => "ถึงขีดจำกัดหน่วยความจำ เก็บภาพส่วนที่ต่อสำเร็จแล้ว",
        (CaptureFailed, false) => "Capture failed; retry or stop with valid frames",
        (CaptureFailed, true) => "จับภาพไม่สำเร็จ ลองใหม่หรือหยุดเพื่อเก็บส่วนที่สำเร็จ",
        (Paused, false) => "Paused: resume or stop to keep the image",
        (Paused, true) => "พัก: กดทำต่อ หรือหยุดเพื่อเก็บภาพ",
        (InputBlocked, false) => "Paused: wheel blocked; release keys, focus source, Resume",
        (InputBlocked, true) => "พัก: เลื่อนไม่ได้ ปล่อยปุ่ม กลับหน้าต่างเดิม แล้วทำต่อ",
        (NoNewContent, false) => {
            "No new content confirmed after retries. Kept valid image; this may not be the end of the document."
        }
        (NoNewContent, true) => {
            "ยังยืนยันเนื้อหาใหม่ไม่ได้หลังลองซ้ำ เก็บภาพส่วนที่สำเร็จแล้ว ซึ่งอาจยังไม่ใช่ท้ายเอกสาร"
        }
    }
}
fn poll_scroll() {
    use crate::scroll_job::Event;
    // The non-activating controller leaves keyboard focus on the source app.
    // Also cover a failed hotkey registration (for example, another app owns Esc).
    if unsafe { GetAsyncKeyState(VK_ESCAPE as i32) } < 0 {
        stop_scroll();
    }
    let item = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        let job = a.scrolling.as_ref()?;
        Some((
            job.window,
            job.rx.try_recv().ok()?,
            a.settings.language == "th",
        ))
    });
    let Some((window, event, thai)) = item else {
        return;
    };
    match event {
        Event::Progress(state, height) => {
            status(
                window as HWND,
                &format!("{}\r\n{height} px", scroll_status(state, thai)),
            );
            refresh_scroll_pause();
        }
        Event::Done(image, notice) => {
            let single = STATE.with(|s| {
                s.borrow()
                    .as_ref()
                    .unwrap()
                    .scrolling
                    .as_ref()
                    .map(|j| j.region.height)
            });
            cancel_scroll();
            match image {
                Ok(frame) => {
                    let unchanged = Some(frame.height) == single;
                    present(frame, Instant::now());
                    if let Some(n) = notice {
                        notify(null_mut(), scroll_status(n, thai), false);
                    } else if unchanged {
                        notify(
                            null_mut(),
                            if thai {
                                "ยังไม่มีการเลื่อนที่ยืนยันได้ เก็บเป็นภาพหน้าจอเดียว"
                            } else {
                                "No confirmed scroll movement. Kept a single screenshot."
                            },
                            false,
                        );
                    }
                }
                Err(error) => notify(
                    null_mut(),
                    if thai {
                        "ยังจับภาพนิ่งไม่สำเร็จ กรุณาเลือกพื้นที่และลองใหม่"
                    } else {
                        &error
                    },
                    true,
                ),
            }
        }
    }
}

fn stop_scroll() {
    STATE.with(|state| {
        if let Some(job) = &state.borrow().as_ref().unwrap().scrolling {
            job.stop();
        }
    });
}

fn source_bytes(app: &App) -> usize {
    source_bytes_with(app, &[])
}
fn source_bytes_with(app: &App, incoming: &[Arc<Frame>]) -> usize {
    let mut seen = std::collections::HashSet::new();
    app.previews
        .values()
        .flat_map(|p| std::iter::once(p.frame.clone()).chain(p.session.document.image_sources()))
        .chain(app.combine_assets.iter().cloned())
        .chain(incoming.iter().cloned())
        .filter(|f| seen.insert(Arc::as_ptr(f)))
        .map(|f| f.bgra.len())
        .sum::<usize>()
        + app.project_reserved
}

fn import_image(owner: HWND, paste: bool) {
    if STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.importing || a.project_opening || a.project_exit.is_some() || a.project_modal
    }) {
        return;
    }
    let input = if paste {
        match clipboard::read(owner) {
            Ok((bytes, png)) => Ok((None, Some((bytes, png)))),
            Err(error) => Err(error),
        }
    } else {
        let mut path = [0u16; 32768];
        let filter = wide("Images (PNG, JPEG, BMP)\0*.png;*.jpg;*.jpeg;*.bmp\0\0");
        let mut dialog: OPENFILENAMEW = unsafe { zeroed() };
        dialog.lStructSize = size_of::<OPENFILENAMEW>() as u32;
        dialog.hwndOwner = workspace_ui::native_owner(owner);
        dialog.lpstrFile = path.as_mut_ptr();
        dialog.nMaxFile = path.len() as u32;
        dialog.lpstrFilter = filter.as_ptr();
        dialog.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
        if unsafe { GetOpenFileNameW(&mut dialog) } == 0 {
            return;
        }
        let n = path.iter().position(|c| *c == 0).unwrap_or(path.len());
        use std::os::windows::ffi::OsStringExt;
        Ok((
            Some(PathBuf::from(std::ffi::OsString::from_wide(&path[..n]))),
            None,
        ))
    };
    let (path, clipboard) = match input {
        Ok(input) => input,
        Err(error) => {
            notify(owner, &error, true);
            return;
        }
    };
    let job = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        let p = a.previews.get(&(owner as usize))?;
        let identity = p.guard.token.identity;
        let budget = crate::image_io::SOURCE_LIMIT.saturating_sub(source_bytes(a));
        a.importing = true;
        a.previews.get_mut(&(owner as usize)).unwrap().importing = true;
        Some((identity, budget, a.tx.clone(), a.main))
    });
    let Some((identity, budget, tx, main)) = job else {
        return;
    };
    let owner = owner as usize;
    update_editor(owner as HWND);
    std::thread::spawn(move || {
        let result = (|| {
            if let Some((bytes, png)) = clipboard {
                if png {
                    crate::image_io::decode(&bytes, budget)
                } else {
                    crate::image_io::dib(&bytes, budget)
                }
            } else {
                use std::io::Read;
                let file = fs::File::open(path.unwrap()).map_err(|e| e.to_string())?;
                if file.metadata().map_err(|e| e.to_string())?.len()
                    > crate::image_io::INPUT_FILE_LIMIT as u64
                {
                    return Err("Image file exceeds the 256 MiB input limit.".into());
                }
                let mut bytes = vec![];
                file.take(crate::image_io::INPUT_FILE_LIMIT as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|e| e.to_string())?;
                crate::image_io::decode(&bytes, budget)
            }
        })();
        let _ = tx.send(Work::Imported {
            owner,
            identity,
            result,
        });
        unsafe {
            PostMessageW(main as HWND, COMPLETE, 0, 0);
        }
    });
}

fn open_canvas_entry(owner: HWND) {
    let data = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        if a.text_entry.is_some() {
            return None;
        }
        let p = a.previews.get_mut(&(owner as usize))?;
        p.session.cancel();
        Some((
            p.session.document.crop,
            p.guard.token.identity,
            a.settings.language == "th",
        ))
    });
    let Some((crop, identity, thai)) = data else {
        return;
    };
    let window = create_window(
        TEXT_ENTRY,
        if thai {
            "ขยายพื้นที่ทำงาน"
        } else {
            "Expand canvas"
        },
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
        WS_EX_CONTROLPARENT,
        workspace_ui::native_owner(owner),
        400,
        210,
    );
    if window.is_null() {
        notify(owner, "Cannot open canvas settings.", true);
        return;
    }
    STATE.with(|s| {
        s.borrow_mut().as_mut().unwrap().text_entry = Some(TextEntry {
            canvas: true,
            window: window as usize,
            owner: owner as usize,
            point: Point::new(0., 0.),
            object: None,
            identity,
        })
    });
    child(
        window,
        "STATIC",
        if thai {
            "ความกว้าง (px)"
        } else {
            "Width (px)"
        },
        711,
        0,
    );
    child(
        window,
        "STATIC",
        if thai {
            "ความสูง (px)"
        } else {
            "Height (px)"
        },
        712,
        0,
    );
    for (id, value) in [(701, crop.width), (702, crop.height)] {
        let edit = child(
            window,
            "EDIT",
            &value.to_string(),
            id,
            WS_TABSTOP | WS_BORDER | ES_NUMBER as u32,
        );
        unsafe {
            SendMessageW(edit, EM_SETLIMITTEXT, 5, 0);
        }
    }
    child(
        window,
        "BUTTON",
        if thai { "ยืนยัน" } else { "Apply" },
        1,
        WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
    );
    child(
        window,
        "BUTTON",
        if thai { "ยกเลิก" } else { "Cancel" },
        2,
        WS_TABSTOP,
    );
    layout(window);
    unsafe {
        EnableWindow(workspace_ui::native_owner(owner), 0);
        ShowWindow(window, SW_SHOW);
        SetForegroundWindow(window);
        SetFocus(GetDlgItem(window, 701));
    }
}

fn present(frame: Frame, started: Instant) {
    let route = STATE.with(|s| s.borrow_mut().as_mut().unwrap().capture_route.take());
    if let Some(route) = route {
        present_project(Arc::new(frame), None, None, started, route);
    }
}
fn present_project(
    frame: Arc<Frame>,
    document: Option<crate::editor::Document>,
    path: Option<PathBuf>,
    started: Instant,
    route: crate::workspace::Route,
) {
    present_project_cached(frame, document, path, started, None, route, None);
}
fn present_project_cached(
    frame: Arc<Frame>,
    document: Option<crate::editor::Document>,
    path: Option<PathBuf>,
    started: Instant,
    cached: Option<(u64, Arc<Frame>)>,
    route: crate::workspace::Route,
    source_label: Option<String>,
) {
    let full = STATE.with(|s| {
        let borrow = s.borrow();
        let app = borrow.as_ref().unwrap();
        app.previews.len() + usize::from(app.project_opening) >= 16
            || source_bytes_with(
                app,
                &std::iter::once(frame.clone())
                    .chain(
                        document
                            .as_ref()
                            .map(|d| d.image_sources())
                            .unwrap_or_default(),
                    )
                    .collect::<Vec<_>>(),
            ) > crate::image_io::SOURCE_LIMIT
    });
    if full {
        notify(
            null_mut(),
            "Close an older capture before opening another preview.",
            true,
        );
        return;
    }
    let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
    let title = format!(
        "{} - {}",
        TITLE,
        if thai {
            "แก้ไขภาพ"
        } else {
            "Image editor"
        }
    );
    let create_started = Instant::now();
    let Some(host) = workspace_ui::destination(route) else {
        return;
    };
    let window = create_window(
        PREVIEW,
        &title,
        WS_CHILD | WS_CLIPCHILDREN,
        WS_EX_CONTROLPARENT,
        host,
        800,
        580,
    );
    if window.is_null() {
        workspace_ui::remove_empty(host);
        notify(null_mut(), "Cannot open the image preview.", true);
        return;
    }
    trace::record("editor.create-window", create_started);
    let size = document
        .as_ref()
        .map(|d| (d.crop.width, d.crop.height))
        .unwrap_or((frame.width, frame.height));
    let dimensions = format!("{} x {} px", size.0, size.1);
    let mut session = Session::new(frame.width, frame.height);
    if let Some(d) = document {
        session.document = d;
    }
    STATE.with(|s| {
        let mut b = s.borrow_mut();
        let app = b.as_mut().unwrap();
        app.next_preview += 1;
        app.previews.insert(
            window as usize,
            Preview {
                project_path: path,
                source_label,
                project_operation: 0,
                close_after_save: false,
                importing: false,
                session,
                frame,
                cached,
                curve_feedback: None,
                guard: Guard::new(app.next_preview),
                tips: None,
                saving: false,
                first_paint: Some(started),
            },
        )
    });
    let controls_started = Instant::now();
    child(
        window,
        "BUTTON",
        &tr("copy"),
        COPY as i32,
        WS_TABSTOP | BS_OWNERDRAW as u32,
    );
    child(
        window,
        "BUTTON",
        &tr("save"),
        SAVE as i32,
        WS_TABSTOP | BS_OWNERDRAW as u32 | BS_DEFPUSHBUTTON as u32,
    );
    child(window, "STATIC", &dimensions, STATUS, 0);
    for id in ec::BUTTONS {
        child(
            window,
            "BUTTON",
            ec::label(id, thai),
            id as i32,
            WS_TABSTOP
                | if id == ec::PATTERN {
                    0
                } else {
                    BS_OWNERDRAW as u32
                },
        );
    }
    let width = child(
        window,
        "EDIT",
        "5",
        ec::WIDTH as i32,
        WS_TABSTOP | WS_BORDER | ES_NUMBER as u32 | ES_CENTER as u32,
    );
    unsafe {
        SendMessageW(width, EM_SETLIMITTEXT, 3, 0);
    }
    child(window, "STATIC", "px", ec::UNIT, 0);
    child(window, "STATIC", "Opacity %", ec::OPACITY_UNIT, 0);
    let opacity = child(
        window,
        "EDIT",
        "100",
        ec::OPACITY as i32,
        WS_TABSTOP | WS_BORDER | ES_NUMBER as u32 | ES_CENTER as u32,
    );
    unsafe {
        SendMessageW(opacity, EM_SETLIMITTEXT, 3, 0);
    }
    child(
        window,
        "BUTTON",
        ec::label(ec::BACKGROUND, thai),
        ec::BACKGROUND as i32,
        WS_TABSTOP | BS_AUTOCHECKBOX as u32,
    );
    child(
        window,
        "STATIC",
        if thai { "กรอบ px" } else { "Border px" },
        ec::STROKE_UNIT,
        0,
    );
    let border = child(
        window,
        "EDIT",
        "5",
        ec::STROKE as i32,
        WS_TABSTOP | WS_BORDER | ES_NUMBER as u32 | ES_CENTER as u32,
    );
    unsafe {
        SendMessageW(border, EM_SETLIMITTEXT, 2, 0);
    }
    child(
        window,
        "BUTTON",
        if STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th") {
            "TH"
        } else {
            "EN"
        },
        ec::LANG as i32,
        WS_TABSTOP,
    );
    trace::record("editor.create-controls", controls_started);
    let tips_started = Instant::now();
    let tips = ec::Tips::new(
        window,
        STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th"),
    );
    STATE.with(|s| {
        s.borrow_mut()
            .as_mut()
            .unwrap()
            .previews
            .get_mut(&(window as usize))
            .unwrap()
            .tips = Some(tips)
    });
    trace::record("editor.create-tooltips", tips_started);
    let layout_started = Instant::now();
    layout(window);
    update_editor(window);
    trace::record("editor.initial-layout", layout_started);
    if !workspace_ui::attach(host, window) {
        let removed = STATE.with(|s| {
            s.borrow_mut()
                .as_mut()
                .unwrap()
                .previews
                .remove(&(window as usize))
        });
        drop(removed);
        unsafe {
            DestroyWindow(window);
        }
        workspace_ui::remove_empty(host);
        return;
    }
    let show_started = Instant::now();
    unsafe {
        ShowWindow(host, SW_SHOW);
        SetForegroundWindow(host);
        SetFocus(window);
        UpdateWindow(window);
    }
    trace::record("editor.show-window", show_started);
}

fn editor_view(window: HWND, crop: Rect) -> View {
    let bounds = client(window);
    let width = (bounds.right - scale(window, 32)).max(1);
    let height = (bounds.bottom - scale(window, 141)).max(1);
    let ratio = (width as f64 / crop.width as f64)
        .min(height as f64 / crop.height as f64)
        .min(1.);
    View {
        crop,
        x: ((bounds.right as f64 - crop.width as f64 * ratio) / 2.).round(),
        y: (scale(window, 111) as f64 + (height as f64 - crop.height as f64 * ratio) / 2.).round(),
        scale: ratio,
    }
}

fn rendered_frame(window: HWND) -> Result<(Arc<Frame>, u64), String> {
    STATE.with(|s| {
        let mut b = s.borrow_mut();
        let p = b
            .as_mut()
            .unwrap()
            .previews
            .get_mut(&(window as usize))
            .ok_or("Editor is closed.")?;
        let revision = p.session.document.revision;
        if p.importing {
            return Err("Image is still loading. Please wait.".into());
        }
        if let Some((r, f)) = &p.cached {
            if *r == revision {
                return Ok((f.clone(), revision));
            }
        }
        Err("Image is still updating. Please wait.".into())
    })
}

fn schedule_composition() {
    let job = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        if a.composing {
            return None;
        }
        let found = a.previews.iter().find(|(_, p)| {
            !p.guard.failed
                && p.cached
                    .as_ref()
                    .is_none_or(|(r, _)| *r != p.session.document.revision)
        });
        let (&owner, p) = found?;
        let job = (
            owner,
            p.guard.token,
            p.frame.clone(),
            p.session.document.render_snapshot(),
            a.tx.clone(),
            a.main,
        );
        a.composing = true;
        Some(job)
    });
    if let Some((owner, token, source, document, tx, main)) = job {
        std::thread::spawn(move || {
            let started = Instant::now();
            let result = render::flatten(&source, &document);
            let _ = tx.send(Work::Composed {
                owner,
                token,
                started,
                result,
            });
            unsafe {
                PostMessageW(main as HWND, COMPLETE, 0, 0);
            }
        });
    }
}

fn source_is_unmodified(frame: &Frame, document: &crate::editor::Document) -> bool {
    document.objects.is_empty()
        && document.crop == Rect::new(0, 0, frame.width, frame.height).unwrap()
        && document.source_clip == Some(document.crop)
}
fn update_editor(window: HWND) {
    project_ui::update_title(window);
    let context = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let app = b.as_mut().unwrap();
        let thai = app.settings.language == "th";
        app.previews
            .get_mut(&(window as usize))
            .map(|p| (thai, p.session.shape_choices(), p.tips.take()))
    });
    if let Some((thai, shapes, tips)) = context {
        // Owner-drawn controls may synchronously reenter the window procedure.
        // Release STATE before sending any native tooltip/text messages.
        if let Some(mut tips) = tips {
            tips.refresh(window, thai, shapes);
            STATE.with(|s| {
                if let Some(p) = s
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .previews
                    .get_mut(&(window as usize))
                {
                    p.tips = Some(tips);
                }
            });
        }
        for id in [ec::RECTANGLE, ec::FILLED_RECTANGLE] {
            unsafe {
                SetWindowTextW(
                    GetDlgItem(window, id as i32),
                    wide(&ec::description(id, thai, shapes)).as_ptr(),
                );
            }
        }
    }
    STATE.with(|s| {
        let mut b = s.borrow_mut();
        let app = b.as_mut().unwrap();
        if let Some(p) = app.previews.get_mut(&(window as usize)) {
            p.session.sync_selected_style();
            let revision = p.session.document.revision;
            p.guard.observe(revision);
            if source_is_unmodified(&p.frame, &p.session.document) {
                p.cached = Some((revision, p.frame.clone()));
            }
        }
    });
    let combine_state = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.previews.get(&(window as usize)).map(|p| {
            (
                a.previews.len() < 16
                    && !a.project_opening
                    && !a.project_modal
                    && !a.importing
                    && !a.capture_busy
                    && !p.saving
                    && !p.importing,
                p.session
                    .document
                    .objects
                    .iter()
                    .any(|o| Some(o.id) == p.session.document.selected && o.parent.is_some()),
            )
        })
    });
    if let Some((combine, owner)) = combine_state {
        unsafe {
            EnableWindow(GetDlgItem(window, ec::COMBINE as i32), combine as i32);
            EnableWindow(GetDlgItem(window, ec::OWNER as i32), owner as i32);
        }
    }
    let info = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .previews
            .get(&(window as usize))
            .map(|p| {
                (
                    p.session.document.can_undo(),
                    p.session.document.can_redo(),
                    p.session.document.selected.is_some(),
                    p.session.parameter(),
                    p.session.crop_draft().is_some()
                        && p.session.gesture.as_ref().is_some_and(|g| g.ready),
                    p.session.document.crop,
                    p.saving || p.importing,
                    !p.importing
                        && p.cached
                            .as_ref()
                            .is_some_and(|(r, _)| *r == p.session.document.revision),
                    p.session.effective_tool(),
                    p.session.text_background(),
                    p.session.callout_border(),
                    p.session.has_independent_callout_border(),
                    ec::ORDER_BUTTONS
                        .map(|(id, order)| (id, p.session.document.can_reorder(order))),
                    p.session.opacity(),
                    p.session.pattern(),
                )
            })
    });
    if let Some((
        undo,
        redo,
        selected,
        width,
        crop,
        rect,
        saving,
        ready,
        tool,
        background,
        border,
        independent_border,
        ordering,
        opacity,
        pattern,
    )) = info
    {
        unsafe {
            let can_layer = ordering.iter().any(|(_, enabled)| *enabled);
            for (id, enabled) in [
                (ec::UNDO, undo),
                (ec::REDO, redo),
                (ec::DELETE, selected),
                (ec::CONFIRM, crop),
                (ec::CANCEL, crop),
                (ec::PROJECT_SAVE, !saving),
                (
                    ec::EFFECTS,
                    tool != Tool::Crop && (tool != Tool::Select || selected),
                ),
                (ec::LAYERS, can_layer),
            ] {
                EnableWindow(GetDlgItem(window, id as i32), enabled as i32);
            }
            for (id, enabled) in ordering {
                EnableWindow(GetDlgItem(window, id as i32), enabled as i32);
            }
            EnableWindow(GetDlgItem(window, SAVE as i32), (ready && !saving) as i32);
            EnableWindow(GetDlgItem(window, COPY as i32), ready as i32);
            EnableWindow(
                GetDlgItem(window, ec::COLOR as i32),
                (!matches!(tool, Tool::Blur | Tool::Crop | Tool::Select | Tool::Image)) as i32,
            );
            EnableWindow(
                GetDlgItem(window, ec::WIDTH as i32),
                (!matches!(tool, Tool::Crop | Tool::Select | Tool::Image) && !tool.filled()) as i32,
            );
            EnableWindow(
                GetDlgItem(window, ec::PATTERN as i32),
                tool.patterned() as i32,
            );
            SetWindowTextW(
                GetDlgItem(window, ec::PATTERN as i32),
                wide(match pattern {
                    crate::editor::Pattern::Solid => "──── ▾",
                    crate::editor::Pattern::Dashed => "— — — ▾",
                    crate::editor::Pattern::Dotted => "· · · · ▾",
                })
                .as_ptr(),
            );
            let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
            let text = matches!(tool, Tool::Text | Tool::Callout);
            for (id, visible) in [
                (ec::BACKGROUND, text),
                (ec::BACK_COLOR, text),
                (ec::BORDER_COLOR, independent_border),
                (ec::STROKE, tool == Tool::Callout),
                (ec::STROKE_UNIT as usize, tool == Tool::Callout),
                (ec::OPACITY, true),
                (ec::OPACITY_UNIT as usize, true),
                (ec::PATTERN, !text),
                (ec::CONFIRM, tool == Tool::Crop),
                (ec::CANCEL, tool == Tool::Crop),
            ] {
                ShowWindow(
                    GetDlgItem(window, id as i32),
                    if visible { SW_SHOWNA } else { SW_HIDE },
                );
            }
            if independent_border {
                position(window, ec::BACKGROUND as i32, 270, 75, 110, 26);
                position(window, ec::BACK_COLOR as i32, 386, 75, 34, 26);
                position(window, ec::BORDER_COLOR as i32, 426, 75, 34, 26);
                position(window, ec::STROKE_UNIT, 466, 77, 78, 22);
                position(window, ec::STROKE as i32, 548, 75, 44, 26);
            } else if text {
                position(window, ec::BACKGROUND as i32, 270, 75, 138, 26);
                position(window, ec::BACK_COLOR as i32, 414, 75, 34, 26);
            }
            EnableWindow(
                GetDlgItem(window, ec::OPACITY as i32),
                tool.opacity_supported() as i32,
            );
            position(
                window,
                ec::OPACITY_UNIT,
                if independent_border {
                    630
                } else if text {
                    600
                } else {
                    390
                },
                77,
                96,
                22,
            );
            position(
                window,
                ec::OPACITY as i32,
                if independent_border {
                    728
                } else if text {
                    698
                } else {
                    490
                },
                75,
                44,
                26,
            );
            SetWindowTextW(
                GetDlgItem(window, ec::BACKGROUND as i32),
                wide(ec::label(ec::BACKGROUND, thai)).as_ptr(),
            );
            SendMessageW(
                GetDlgItem(window, ec::BACKGROUND as i32),
                BM_SETCHECK,
                if background.enabled {
                    BST_CHECKED
                } else {
                    BST_UNCHECKED
                } as usize,
                0,
            );
            SetWindowTextW(
                GetDlgItem(window, ec::STROKE_UNIT),
                wide(if thai { "กรอบ px" } else { "Border px" }).as_ptr(),
            );
            SetWindowTextW(
                GetDlgItem(window, ec::STROKE as i32),
                wide(&border.width.to_string()).as_ptr(),
            );
            SetWindowTextW(
                GetDlgItem(window, ec::OPACITY_UNIT),
                wide(if thai {
                    "ความเข้ม %"
                } else {
                    "Opacity %"
                })
                .as_ptr(),
            );
            SetWindowTextW(
                GetDlgItem(window, ec::OPACITY as i32),
                wide(&opacity.to_string()).as_ptr(),
            );
            let label = match (tool, thai) {
                (Tool::Text | Tool::Callout, false) => "Font px (8-256)",
                (Tool::Text | Tool::Callout, true) => "อักษร px (8-256)",
                (Tool::Step, false) => "Circle px (20-120)",
                (Tool::Step, true) => "วงกลม px (20-120)",
                (Tool::Blur, false) => "Blur px (1-64)",
                (Tool::Blur, true) => "เบลอ px (1-64)",
                (_, false) => "Stroke px (1-40)",
                (_, true) => "เส้น px (1-40)",
            };
            SetWindowTextW(GetDlgItem(window, ec::UNIT), wide(label).as_ptr());
            SetWindowTextW(
                GetDlgItem(window, ec::WIDTH as i32),
                wide(&width.to_string()).as_ptr(),
            );
            InvalidateRect(window, null(), 0);
            for id in ec::BUTTONS {
                InvalidateRect(GetDlgItem(window, id as i32), null(), 0);
            }
        }
        status(
            window,
            &if ready {
                format!("{} x {} px", rect.width, rect.height)
            } else {
                tr("composing")
            },
        );
    }
    schedule_composition();
}

fn open_ocr(owner: HWND) {
    if unsafe { IsWindow(owner) } == 0 {
        return;
    }
    let current = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .ocr_windows
            .iter()
            .find(|(_, item)| item.owner == owner as usize)
            .map(|(key, _)| *key)
    });
    if let Some(existing) = current {
        unsafe {
            ShowWindow(existing as HWND, SW_SHOW);
            SetForegroundWindow(existing as HWND);
        }
        return;
    }
    let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
    let title = if thai {
        "SnapZy | อ่านข้อความ OCR"
    } else {
        "SnapZy | OCR text"
    };
    let window = create_window(
        OCR_WINDOW,
        title,
        WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
        WS_EX_TOOLWINDOW,
        workspace_ui::native_owner(owner),
        560,
        400,
    );
    if window.is_null() {
        notify(
            owner,
            if thai {
                "ไม่สามารถเปิดหน้าต่าง OCR ได้"
            } else {
                "Cannot open the OCR window."
            },
            true,
        );
        return;
    }
    let identity = STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        let app = borrow.as_mut().unwrap();
        app.next_ocr_identity = app.next_ocr_identity.wrapping_add(1).max(1);
        let identity = app.next_ocr_identity;
        app.ocr_windows.insert(
            window as usize,
            OcrWindow {
                owner: owner as usize,
                identity,
                request: 0,
                source_revision: 0,
            },
        );
        identity
    });
    let _ = identity;
    child(
        window,
        "STATIC",
        if thai {
            "เลือกภาษาและอ่านข้อความจากภาพที่เปิดอยู่ในตัวแก้ไข"
        } else {
            "Choose a language and read text from the image in this editor."
        },
        OCR_HELP,
        0,
    );
    let language = child(
        window,
        "COMBOBOX",
        "",
        OCR_LANGUAGE,
        WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
    );
    unsafe {
        SendMessageW(
            language,
            CB_ADDSTRING,
            0,
            wide("Thai + English").as_ptr() as LPARAM,
        );
        SendMessageW(
            language,
            CB_ADDSTRING,
            0,
            wide("English").as_ptr() as LPARAM,
        );
        SendMessageW(language, CB_SETCURSEL, 0, 0);
    }
    child(
        window,
        "BUTTON",
        if thai {
            "อ่านข้อความ"
        } else {
            "Read text"
        },
        OCR_READ as i32,
        WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
    );
    child(
        window,
        "STATIC",
        if thai {
            "พร้อมใช้งาน"
        } else {
            "Ready"
        },
        OCR_STATUS,
        0,
    );
    let result = child(
        window,
        "EDIT",
        "",
        OCR_RESULT,
        WS_TABSTOP
            | WS_BORDER
            | ES_MULTILINE as u32
            | ES_AUTOVSCROLL as u32
            | ES_WANTRETURN as u32
            | WS_VSCROLL,
    );
    unsafe {
        SendMessageW(result, EM_SETLIMITTEXT, 1_000_000, 0);
    }
    child(
        window,
        "BUTTON",
        if thai { "คัดลอก" } else { "Copy" },
        OCR_COPY as i32,
        WS_TABSTOP,
    );
    child(
        window,
        "BUTTON",
        if thai { "ปิด" } else { "Close" },
        OCR_CLOSE as i32,
        WS_TABSTOP,
    );
    layout(window);
    unsafe {
        ShowWindow(window, SW_SHOW);
        SetForegroundWindow(window);
        SetFocus(language);
    }
}

fn read_control_text(window: HWND, id: i32) -> String {
    unsafe {
        let control = GetDlgItem(window, id);
        let length = GetWindowTextLengthW(control).max(0) as usize;
        let mut text = vec![0u16; length + 1];
        let count = GetWindowTextW(control, text.as_mut_ptr(), text.len() as i32).max(0) as usize;
        String::from_utf16_lossy(&text[..count])
    }
}

fn start_ocr(window: HWND) {
    let can_start = STATE.with(|s| {
        !s.borrow().as_ref().unwrap().ocr_busy
            && s.borrow()
                .as_ref()
                .unwrap()
                .ocr_windows
                .contains_key(&(window as usize))
    });
    if !can_start {
        return;
    }
    let (frame, revision) = match rendered_frame(
        STATE.with(|s| s.borrow().as_ref().unwrap().ocr_windows[&(window as usize)].owner as HWND),
    ) {
        Ok(value) => value,
        Err(error) => {
            set_ocr_status(window, &error);
            return;
        }
    };
    let selection = unsafe { SendMessageW(GetDlgItem(window, OCR_LANGUAGE), CB_GETCURSEL, 0, 0) };
    let language = if selection == 1 {
        crate::ocr::Language::English
    } else {
        crate::ocr::Language::ThaiEnglish
    };
    let directory = crate::settings::data_dir().join("OcrModels");
    let missing = crate::ocr::missing_or_invalid(&directory, language);
    if !missing.is_empty() {
        let bytes: usize = missing.iter().map(|model| model.length).sum();
        let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
        let prompt = if thai {
            format!(
                "ต้องดาวน์โหลดโมเดลภาษา OCR ประมาณ {:.1} MB จาก GitHub เพื่อใช้งานครั้งแรก\n\nภาพจะประมวลผลบนเครื่องนี้และไม่มีการอัปโหลด\n\nยอมรับการดาวน์โหลดหรือไม่?",
                bytes as f64 / 1_000_000.0
            )
        } else {
            format!(
                "OCR language data ({:.1} MB) must be downloaded from GitHub for first use.\n\nYour image is processed on this device and is not uploaded.\n\nAllow the download?",
                bytes as f64 / 1_000_000.0
            )
        };
        if unsafe {
            MessageBoxW(
                window,
                wide(&prompt).as_ptr(),
                wide(if thai {
                    "ติดตั้งข้อมูล OCR"
                } else {
                    "Install OCR language data"
                })
                .as_ptr(),
                MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
            )
        } != IDYES
        {
            set_ocr_status(
                window,
                if thai {
                    "ยกเลิกการดาวน์โหลดแล้ว"
                } else {
                    "Download declined."
                },
            );
            return;
        }
    }
    let (owner, identity, request, tx, main) = STATE
        .with(|s| {
            let mut borrow = s.borrow_mut();
            let app = borrow.as_mut().unwrap();
            if app.ocr_busy {
                return None;
            }
            app.ocr_busy = true;
            let entry = app.ocr_windows.get_mut(&(window as usize)).unwrap();
            entry.request = entry.request.wrapping_add(1).max(1);
            entry.source_revision = revision;
            Some((
                entry.owner,
                entry.identity,
                entry.request,
                app.tx.clone(),
                app.main,
            ))
        })
        .unwrap();
    unsafe {
        EnableWindow(GetDlgItem(window, OCR_READ as i32), 0);
    }
    set_ocr_status(
        window,
        &if missing.is_empty() {
            tr("ocr_reading")
        } else {
            tr("ocr_downloading")
        },
    );
    let started = Instant::now();
    std::thread::spawn(move || {
        let result = (|| {
            for model in missing {
                let bytes = crate::ocr::download_model(&model)?;
                crate::ocr::install_model(&directory, &model, &bytes)?;
            }
            if !crate::ocr::missing_or_invalid(&directory, language).is_empty() {
                return Err("OCR language model verification failed after installation.".into());
            }
            crate::ocr::recognize(&frame, &directory, language)
        })();
        trace::record("ocr.recognition", started);
        let _ = tx.send(Work::OcrFinished {
            owner,
            identity,
            request,
            revision,
            result,
            started,
        });
        unsafe {
            PostMessageW(main as HWND, COMPLETE, 0, 0);
        }
    });
}

fn set_ocr_status(window: HWND, value: &str) {
    if unsafe { IsWindow(window) } != 0 {
        let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
        let localized = if thai {
            if value == "Image is still loading. Please wait." {
                "ภาพกำลังโหลด กรุณารอสักครู่".to_owned()
            } else if value == "Image is still updating. Please wait." {
                "ภาพกำลังปรับปรุง กรุณารอสักครู่".to_owned()
            } else if value.starts_with("This image is too large for OCR") {
                "ภาพมีขนาดใหญ่เกินไปสำหรับ OCR กรุณาครอบตัดให้เล็กลงแล้วลองอีกครั้ง".to_owned()
            } else if let Some(name) = value.strip_prefix("OCR language data is missing: ") {
                format!("ไม่พบโมเดลภาษา {name} กดอ่านข้อความเพื่อติดตั้ง")
            } else if let Some(name) = value.strip_prefix("OCR language data is damaged: ") {
                format!("โมเดลภาษา {name} เสียหาย กดอ่านข้อความเพื่อติดตั้งใหม่")
            } else if value.contains("download")
                || value.contains("downloaded")
                || value.contains("Download")
            {
                format!("เตรียมโมเดล OCR ไม่สำเร็จ: {value}")
            } else if value.contains("OCR")
                || value.contains("recognition")
                || value.starts_with("Cannot")
                || value.starts_with("The ")
            {
                format!("OCR ทำงานไม่สำเร็จ: {value}")
            } else {
                value.to_owned()
            }
        } else {
            value.to_owned()
        };
        unsafe {
            SetWindowTextW(GetDlgItem(window, OCR_STATUS), wide(&localized).as_ptr());
        }
    }
}

fn ocr_copy(window: HWND) {
    let value = read_control_text(window, OCR_RESULT);
    match clipboard::copy_text(window, &value) {
        Ok(()) => set_ocr_status(window, &tr("ocr_copied")),
        Err(error) => set_ocr_status(window, &error),
    }
}

fn editor_command(window: HWND, id: usize) {
    if STATE.with(|s| {
        s.borrow()
            .as_ref()
            .is_some_and(|a| a.project_exit.is_some() || a.project_modal)
    }) {
        return;
    }
    if id == ec::COMBINE {
        combine_ui::begin(window);
        return;
    }
    if id == ec::OWNER {
        STATE.with(|s| {
            if let Some(p) = s
                .borrow_mut()
                .as_mut()
                .unwrap()
                .previews
                .get_mut(&(window as usize))
            {
                p.session.cancel();
                p.session.tool = Tool::Select;
                p.session.document.select_owner();
            }
        });
        update_editor(window);
        return;
    }
    if id == ec::PROJECT_OPEN {
        project_ui::open(window);
        return;
    }
    if id == ec::PROJECT_SAVE {
        project_ui::save_menu(window);
        return;
    }
    if id == ec::EFFECTS {
        let settings = STATE.with(|s| {
            let b = s.borrow();
            let a = b.as_ref().unwrap();
            let p = a.previews.get(&(window as usize))?;
            if a.effects_open {
                return None;
            }
            if p.session.tool == Tool::Crop
                || (p.session.tool == Tool::Select && p.session.document.selected.is_none())
            {
                return None;
            }
            Some((
                p.session.effects(),
                matches!(p.session.effective_tool(), Tool::Text | Tool::Callout),
                a.settings.language == "th",
            ))
        });
        if let Some((effects, text, thai)) = settings {
            cancel_editor(window);
            STATE.with(|s| s.borrow_mut().as_mut().unwrap().effects_open = true);
            let result =
                crate::effects_ui::show(workspace_ui::native_owner(window), effects, text, thai);
            STATE.with(|s| s.borrow_mut().as_mut().unwrap().effects_open = false);
            match result {
                Ok(Some(effects)) => {
                    let error = STATE.with(|s| {
                        let mut b = s.borrow_mut();
                        let p = b.as_mut().unwrap().previews.get_mut(&(window as usize))?;
                        p.session.set_effects(effects).err()
                    });
                    update_editor(window);
                    if let Some(error) = error {
                        notify_text_error(window, &error);
                    }
                }
                Ok(None) => {}
                Err(error) => notify_text_error(window, &error),
            }
        }
        return;
    }
    if matches!(id, ec::RECTANGLE | ec::FILLED_RECTANGLE) {
        choose_shape(window, id == ec::FILLED_RECTANGLE);
        return;
    }
    if matches!(id, ec::ADD_IMAGE | ec::PASTE_IMAGE) {
        import_image(window, id == ec::PASTE_IMAGE);
        return;
    }
    if id == ec::CANVAS {
        open_canvas_entry(window);
        return;
    }
    if id == ec::OCR {
        open_ocr(window);
        return;
    }
    if matches!(id, ec::COLOR | ec::BACK_COLOR | ec::BORDER_COLOR) {
        choose_color(window, id);
        return;
    }
    if id == ec::PATTERN {
        choose_pattern(window);
        return;
    }
    if id == ec::HELP {
        let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
        let text = if thai {
            "เลือก: คลิกวัตถุแล้วลากย้าย\nตัดภาพ: ลากพื้นที่แล้วกดยืนยัน หรือ Enter\nปากกา: ลากวาดเส้นอิสระ\nเส้นตรง / ลูกศร / กรอบ: ลากจากจุดเริ่มถึงจุดสิ้นสุด\nสี / ขนาด: ปรับวัตถุที่เลือกหรือเส้นที่จะวาดใหม่\nย้อนกลับ / ทำซ้ำ: Ctrl+Z / Ctrl+Y\nลบ: ลบวัตถุที่เลือก\nEscape: ยกเลิกสิ่งที่กำลังลาก\nCopy / Save PNG: ส่งออกภาพรวมที่ความละเอียดต้นฉบับ"
        } else {
            "Select: click an object and drag to move it.\nCrop: drag a region, then confirm or press Enter.\nPen: draw a freehand stroke.\nLine / Arrow / Rectangle: drag between endpoints.\nColor / Width: update the selected object or the next stroke.\nUndo / Redo: Ctrl+Z / Ctrl+Y.\nDelete: remove the selected object.\nEscape: cancel an unfinished gesture.\nCopy / Save PNG: export the composition at source resolution."
        };
        let extra = if thai {
            "\nข้อความ: คลิกพิมพ์ไทย/อังกฤษหลายบรรทัด ดับเบิลคลิกเพื่อแก้ไข\nกรอบข้อความ (เลือก): ลากขอบซ้าย/ขวาเพื่อจัดบรรทัด ลากมุมเพื่อย่อ/ขยายอักษร\nปล่อยเมาส์เพื่อจัดข้อความ กด Esc ยกเลิก คำที่ยาวกว่ากรอบจะขยายกรอบให้พอดี\nเลขลำดับ: คลิกวางเลข เมื่อลบเลขสูงสุดจะนับต่อจากเลขที่เหลือ\nเบลอ (หยดน้ำ): ลากพื้นที่ ปล่อยเมาส์เพื่อประมวลผล ลบเพื่อคืนภาพ\nเบลอเป็นการซ่อนรายละเอียดทางสายตา ไม่รับประกันการปกปิดข้อมูลอย่างปลอดภัย"
        } else {
            "\nText: click to type multiline Thai/English; double-click to edit.\nText frame (Select): drag sides to reflow; drag corners to scale the font.\nRelease to lay out text; Esc cancels. A longer unbroken word expands the frame.\nNumbered step: click to place; deleting the highest resumes numbering.\nBlur (droplet): drag a region, release to process; delete to restore.\nBlur is visual obfuscation, not guaranteed secure redaction."
        };
        let callout = if thai {
            "\nพื้นหลังข้อความ: เปิด/ปิดและเลือกสีแยกจากตัวอักษร ค่าเริ่มต้นโปร่งใส\nช่องคำพูด: คลิกพิมพ์ ใช้เลือกเพื่อลากจุดปลายหาง ลากขอบจัดบรรทัดหรือลากมุมปรับอักษร\nสีกรอบ / กรอบ px: ปรับสีและความหนากรอบแยกจากตัวอักษร"
        } else {
            "\nText background: toggle and choose its color independently; transparent by default.\nSpeech bubble: click to type; Select to drag the tail tip, sides to wrap or corners to scale text.\nBorder color / Border px: change the bubble border independently from its text."
        };
        let layers = if thai {
            "\nเลเยอร์: นำขึ้นบนสุด / ขึ้นหนึ่งชั้น / ลงหนึ่งชั้น / ส่งลงล่างสุด ใช้กับวัตถุที่เลือก\nภาพต้นฉบับอยู่ล่างสุดเสมอ ย้อนกลับ/ทำซ้ำได้\nเบลอมีผลต่อวัตถุที่อยู่ด้านล่างเท่านั้น การย้ายชั้นเบลอจึงเปลี่ยนสิ่งที่ถูกเบลอ"
        } else {
            "\nLayers: Bring to front / forward one layer / backward one layer / send to back for the selected object.\nThe source image stays underneath; Undo/Redo restores layer order.\nBlur affects only objects below it; moving a blur layer changes what is blurred."
        };
        let ocr = if thai {
            "\nอ่านข้อความ (OCR): อ่านภาษาไทย+อังกฤษหรืออังกฤษจากภาพรวมที่กำลังแก้ไข ผลลัพธ์แก้ไขได้ก่อนคัดลอก ระบบประมวลผลในเครื่องและขออนุญาตก่อนดาวน์โหลดโมเดลภาษา"
        } else {
            "\nRead text (OCR): recognize Thai + English or English from the current composition. Review and edit the result before copying. Processing stays on-device; language models require consent to download."
        };
        let effects = if thai {
            "\nเอฟเฟกต์ (รูปเฟือง): เปิดเงา เลือกสี ความฟุ้ง 0-32 px ระยะ X/Y -32 ถึง 32 px; ขอบอักษรของข้อความและช่องคำพูด เลือกสีและความหนา 1-16 px ทั้งสองเริ่มปิด กดใช้เพื่อยืนยันหนึ่ง Undo กดยกเลิกหรือ Esc เพื่อทิ้งการเปลี่ยนแปลง เงาเบลออยู่ด้านนอกพื้นที่ปิดบัง ใช้เลือกแก้เฉพาะวัตถุ โหมดวาดจะจำค่าสำหรับวัตถุใหม่ด้วย"
        } else {
            ""
        };
        let images = if thai {
            "\nเพิ่มภาพ: เลือกไฟล์ PNG/JPEG/BMP หรือวางภาพจากคลิปบอร์ด (Ctrl+V)\nใช้เลือกเพื่อลากย้ายภาพ ลากมุมเพื่อปรับขนาดโดยรักษาสัดส่วน ต้นฉบับไม่ถูกย่อ\nขยายพื้นที่ทำงาน: กำหนดความกว้าง/สูงพิกเซล เพิ่มพื้นที่สีขาวด้านขวา/ล่าง\nภาพที่เพิ่มจัดชั้น ลบ และย้อนกลับ/ทำซ้ำได้เหมือนวัตถุอื่น"
        } else {
            "\nAdd image: browse PNG/JPEG/BMP or paste an image (Ctrl+V).\nSelect to move; drag corners to resize with a fixed aspect ratio. Original pixels stay unchanged.\nExpand canvas: set pixel width/height; new right/bottom space is white. Use Crop to shrink.\nAdded images share ordering, Delete and Undo/Redo with annotations."
        };
        let projects = if thai {
            "\nเปิดภาพ/งาน (Ctrl+O): เปิดภาพหรือ .snapzy เป็นแท็บใหม่ ค่าเริ่มต้นเลือกหน้าต่างใหม่ได้จากหน้าหลัก\nCtrl+Tab / Ctrl+Shift+Tab สลับแท็บ Ctrl+W ปิดแท็บปัจจุบัน แต่ละงานแยกเครื่องมือและ Undo/Redo\nปุ่ม X ของหน้าต่างเลือกปิดแท็บปัจจุบันหรือทุกแท็บ ยกเลิกแล้วทุกแท็บยังอยู่\nบันทึกงาน (Ctrl+S) / บันทึกเป็น (Ctrl+Shift+S): เก็บภาพและวัตถุเพื่อแก้ไขต่อ เปิดงานแล้ว Undo/Redo เริ่มใหม่\nCopy / PNG ไม่ถือว่าบันทึกงาน ไฟล์งานยังมีข้อมูลต้นฉบับใต้เบลอหรือรูปทรงที่ปิดทับ ควรแชร์ PNG แทน"
        } else {
            "\nOpen image/project (Ctrl+O): add an image or .snapzy as a new tab. Choose New window on the launcher for future opens.\nCtrl+Tab / Ctrl+Shift+Tab cycle tabs; Ctrl+W closes the current tab. Each work retains independent tools and Undo/Redo.\nWindow X offers current tab or all tabs; Cancel keeps the complete set open.\nSave project (Ctrl+S) / Save As (Ctrl+Shift+S): preserve source images and editable objects; reopened work starts fresh Undo/Redo.\nCopy / PNG does not save the project. Projects retain originals beneath blur or covering shapes; share exported PNG instead."
        };
        let combining = if thai {
            "\nรวมภาพ: เลือกภาพที่เปิดอยู่ จัดแนวตั้ง แนวนอน หรืออิสระ พร้อมระยะห่าง ภาพต้นทางไม่เปลี่ยน วัตถุเดิมยังแก้ไขได้\nเลือกภาพเจ้าของ: เลือกวัตถุแล้วกดเพื่อเลือกภาพที่ครอบอยู่ กดซ้ำเลือกชั้นนอก ย้ายและลากมุมภาพเพื่อปรับขนาดพร้อมวัตถุ\nตัดภาพ: เลือกภาพก่อนกด Crop เพื่อตัดเฉพาะภาพ ไม่เลือกภาพจะตัดพื้นที่งาน เลเยอร์วัตถุจัดลำดับภายในภาพเจ้าของ\nไฟล์งานที่บันทึกจาก Alpha 15 ต้องเปิดด้วย Alpha 15 หรือใหม่กว่า อ่านไฟล์ Alpha 14 เดิมได้ ไฟล์งานยังเก็บข้อมูลที่ถูกตัดหรือเบลอ"
        } else {
            "\nCombine images: select open images, order them vertically/horizontally/freely with gaps. Source work is unchanged; imported objects stay editable.\nSelect owning image: select a child, then its image; repeat for outer ancestors. Move or corner-resize the image with its objects.\nCrop: select an image before choosing Crop for image-only clipping; otherwise crop the workspace. Child layer order stays within its owner.\nProjects saved by Alpha 15 require Alpha 15 or newer. Existing Alpha 14 files remain readable. Projects retain cropped and blurred originals."
        };
        notify(
            window,
            &format!(
                "{text}{extra}{callout}{layers}{images}{ocr}{effects}{projects}{combining}{}{}{}",
                if thai {
                    "\nไฮไลต์: ลากเส้นโปร่งใสเพื่อเน้นข้อความ สีเหลือง 24 px ความเข้ม 40% ปรับสี/ขนาด/ความเข้มได้ ใช้เลือกเพื่อย้าย จัดชั้น หรือลบ ค่าแยกจากปากกาปกติ"
                } else {
                    "\nHighlighter: translucent freehand marks; yellow, 24 px and 40% opacity by default. Adjust color/width/opacity; Select to move, reorder or delete. Its defaults are independent of Pen."
                },
                if thai {
                    "\nรูปทรง: คลิกปุ่มรูปทรงโปร่งหรือทึบ แล้วเลือกสี่เหลี่ยมหรือวงรี / วงกลม แต่ละปุ่มจำตัวเลือกล่าสุดแยกกัน ลากจากมุมหนึ่งไปอีกมุม\nรูปแบบเส้น: ทึบ / ประ / จุด ใช้กับปากกา เส้น ลูกศร และกรอบโปร่ง\nดัดโค้ง: หลังวาดเส้นหรือลูกศร ลากจุดกลางได้ทันที หัวลูกศรหันตามโค้ง ลากที่อื่นเพื่อวาดเส้นต่อ ใช้เลือกสำหรับเส้นเก่า\nดับเบิลคลิกจุดกลางเพื่อคืนเส้นตรง ปล่อยเมาส์เพื่อยืนยัน Esc ยกเลิกระหว่างลาก และ Undo/Redo ได้"
                } else {
                    "\nShapes: click Outline shapes or Filled shapes to choose rectangle or ellipse/circle. Each button remembers its last choice independently; drag between corners.\nStroke pattern: solid/dashed/dotted for Pen, Line, Arrow and outline shapes.\nBend: drag the midpoint immediately after drawing a line/arrow; the arrowhead follows the curve. Drag elsewhere to draw another line. Use Select for older objects.\nDouble-click the handle to straighten. Release to commit, Esc to cancel a drag; Undo/Redo restores the curve."
                },
                if thai {
                    "\nปรับรูปทรง: ลากจุดจับ 8 จุดได้ทันทีหลังสร้าง ใช้เลือกเพื่อแก้รูปเดิม มุมปรับสองด้าน ขอบปรับด้านเดียว Esc ยกเลิก Undo/Redo ได้\nความเข้ม %: ปรับทั้งวัตถุ 0–100% รวมภาพ ข้อความ ช่องคำพูด และเบลอ ปกติเริ่ม 100% ไฮไลต์ 40%"
                } else {
                    "\nResize shapes: drag any of 8 handles immediately after drawing; Select for older shapes. Corners change both axes; edges change one. Esc cancels; Undo/Redo supported.\nOpacity %: adjust the complete object 0-100%, including images, text, bubbles and blur. Ordinary defaults 100%, highlighter 40%.\nEffects (gear): object shadow color, softness 0-32 px and offsets -32 to 32 px; text/bubble glyph outline color and width 1-16 px. Both start disabled. Apply commits one Undo step; Cancel or Esc discards changes. Blur shadows remain outside the protected region. Select edits only the selected object; drawing mode also changes defaults for new items."
                }
            ),
            false,
        );
        return;
    }
    let width = if matches!(id, ec::WIDTH | ec::STROKE | ec::OPACITY) {
        let mut text = [0u16; 16];
        let n = unsafe { GetWindowTextW(GetDlgItem(window, id as i32), text.as_mut_ptr(), 16) };
        String::from_utf16_lossy(&text[..n.max(0) as usize])
            .parse::<u32>()
            .ok()
    } else {
        None
    };
    STATE.with(|s| {
        let mut b = s.borrow_mut();
        let Some(p) = b.as_mut().unwrap().previews.get_mut(&(window as usize)) else {
            return;
        };
        p.curve_feedback = None;
        if let Some(tool) = ec::tool(id) {
            p.session.choose(tool);
        } else {
            if id != ec::CONFIRM {
                p.session.cancel();
            }
            if let Some((_, order)) = ec::ORDER_BUTTONS.iter().find(|(button, _)| *button == id) {
                p.session.document.reorder(*order);
            }
            match id {
                ec::UNDO => {
                    p.session.document.undo();
                }
                ec::REDO => {
                    p.session.document.redo();
                }
                ec::DELETE => {
                    p.session.document.delete();
                }
                ec::CONFIRM => {
                    p.session.confirm_crop();
                }
                ec::CANCEL => p.session.cancel(),
                ec::WIDTH => {
                    if let Some(width) = width {
                        if let Err(e) = p.session.set_parameter(width) {
                            p.session.error = Some(e);
                        }
                    }
                }
                ec::OPACITY => {
                    if let Some(value) = width {
                        p.session.set_opacity(value);
                    }
                }
                ec::BACKGROUND => {
                    let mut background = p.session.text_background();
                    background.enabled = unsafe {
                        SendMessageW(GetDlgItem(window, ec::BACKGROUND as i32), BM_GETCHECK, 0, 0)
                    } == BST_CHECKED as isize;
                    p.session.set_background(background);
                }
                ec::STROKE => {
                    if let Some(value) = width.filter(|v| (1..=40).contains(v)) {
                        if p.session.has_independent_callout_border() {
                            if value <= 16 {
                                let mut border = p.session.callout_border();
                                border.width = value;
                                p.session.set_callout_border(border);
                            }
                        } else {
                            p.session.style.width = value;
                            p.session.document.restyle(p.session.style);
                        }
                    }
                }
                _ => {}
            }
        }
    });
    unsafe {
        ReleaseCapture();
        SetFocus(window);
    }
    update_editor(window);
}

fn choose_color(window: HWND, target: usize) {
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut point);
    }
    let colors: [[u8; 3]; 8] = [
        [255, 0, 0],
        [255, 153, 0],
        [255, 224, 0],
        [0, 160, 80],
        [0, 120, 255],
        [170, 60, 220],
        [0, 0, 0],
        [255, 255, 255],
    ];
    let menu = unsafe { CreatePopupMenu() };
    let names = if STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th") {
        [
            "แดง",
            "ส้ม",
            "เหลือง",
            "เขียว",
            "น้ำเงิน",
            "ม่วง",
            "ดำ",
            "ขาว",
            "สีเพิ่มเติม...",
        ]
    } else {
        [
            "Red",
            "Orange",
            "Yellow",
            "Green",
            "Blue",
            "Purple",
            "Black",
            "White",
            "More colors...",
        ]
    };
    let mut menu_icons = native_icons::MenuIcons::default();
    unsafe {
        for (i, name) in names.iter().enumerate() {
            AppendMenuW(menu, MF_STRING, i + 1, wide(name).as_ptr());
            if let Some(&[r, g, b]) = colors.get(i) {
                menu_icons.add_color(
                    menu,
                    i + 1,
                    native_icons::Icon::ShapeFilled,
                    GetDpiForWindow(window),
                    r as u32 | ((g as u32) << 8) | ((b as u32) << 16),
                );
            } else {
                menu_icons.add(
                    menu,
                    i + 1,
                    native_icons::Icon::Palette,
                    GetDpiForWindow(window),
                );
            }
        }
    }
    let id = unsafe {
        TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            point.x,
            point.y,
            0,
            window,
            null(),
        )
    } as usize;
    unsafe {
        DestroyMenu(menu);
    }
    let color = if id == 9 {
        let current = STATE.with(|s| {
            let b = s.borrow();
            let session = &b.as_ref().unwrap().previews[&(window as usize)].session;
            if target == ec::BACK_COLOR {
                session.text_background().rgb
            } else if target == ec::BORDER_COLOR {
                session.callout_border().rgb
            } else {
                session.style.rgb
            }
        });
        let mut custom = [0u32; 16];
        let mut dialog: CHOOSECOLORW = unsafe { zeroed() };
        dialog.lStructSize = size_of::<CHOOSECOLORW>() as u32;
        dialog.hwndOwner = workspace_ui::native_owner(window);
        dialog.lpCustColors = custom.as_mut_ptr();
        dialog.rgbResult =
            current[0] as u32 | ((current[1] as u32) << 8) | ((current[2] as u32) << 16);
        dialog.Flags = CC_FULLOPEN | CC_RGBINIT;
        if unsafe { ChooseColorW(&mut dialog) } != 0 {
            Some([
                dialog.rgbResult as u8,
                (dialog.rgbResult >> 8) as u8,
                (dialog.rgbResult >> 16) as u8,
            ])
        } else {
            None
        }
    } else {
        id.checked_sub(1).and_then(|i| colors.get(i)).copied()
    };
    if let Some(rgb) = color {
        STATE.with(|s| {
            let mut b = s.borrow_mut();
            let p = b
                .as_mut()
                .unwrap()
                .previews
                .get_mut(&(window as usize))
                .unwrap();
            p.session.cancel();
            if target == ec::BACK_COLOR {
                let mut fill = p.session.text_background();
                fill.rgb = rgb;
                p.session.set_background(fill);
            } else if target == ec::BORDER_COLOR {
                let mut border = p.session.callout_border();
                border.rgb = rgb;
                p.session.set_callout_border(border);
            } else {
                p.session.style.rgb = rgb;
                p.session.document.restyle(p.session.style);
            }
        });
        update_editor(window);
    }
}

fn choose_shape(window: HWND, filled: bool) {
    let (current, thai) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        (
            a.previews[&(window as usize)].session.shape_choices()[filled as usize],
            a.settings.language == "th",
        )
    });
    let choices = if filled {
        [Tool::FilledRectangle, Tool::FilledEllipse]
    } else {
        [Tool::Rectangle, Tool::Ellipse]
    };
    let id = if filled {
        ec::FILLED_RECTANGLE
    } else {
        ec::RECTANGLE
    };
    let mut rect: RECT = unsafe { zeroed() };
    let menu = unsafe { CreatePopupMenu() };
    let mut menu_icons = native_icons::MenuIcons::default();
    if menu.is_null() {
        return;
    }
    unsafe {
        GetWindowRect(GetDlgItem(window, id as i32), &mut rect);
        for (i, tool) in choices.iter().enumerate() {
            AppendMenuW(
                menu,
                MF_STRING | if *tool == current { MF_CHECKED } else { 0 },
                i + 1,
                wide(ec::shape_name(*tool, thai)).as_ptr(),
            );
            let icon = match tool {
                Tool::FilledRectangle => native_icons::Icon::ShapeFilled,
                Tool::Ellipse => native_icons::Icon::Ellipse,
                Tool::FilledEllipse => native_icons::Icon::EllipseFilled,
                _ => native_icons::Icon::Square,
            };
            menu_icons.add(menu, i + 1, icon, GetDpiForWindow(window));
        }
    }
    let selected = unsafe {
        TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            rect.left,
            rect.bottom,
            0,
            window,
            null(),
        )
    } as usize;
    unsafe {
        DestroyMenu(menu);
    }
    // Dismissing the menu leaves tool, selection, history and preferences intact.
    if let Some(tool) = selected
        .checked_sub(1)
        .and_then(|i| choices.get(i))
        .copied()
    {
        STATE.with(|s| {
            if let Some(p) = s
                .borrow_mut()
                .as_mut()
                .unwrap()
                .previews
                .get_mut(&(window as usize))
            {
                p.curve_feedback = None;
                p.session.choose(tool);
            }
        });
        unsafe {
            ReleaseCapture();
            SetFocus(window);
        }
        update_editor(window);
    }
}

fn choose_pattern(window: HWND) {
    use crate::editor::Pattern;
    let (enabled, current, thai) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        let p = &a.previews[&(window as usize)];
        (
            p.session.effective_tool().patterned(),
            p.session.pattern(),
            a.settings.language == "th",
        )
    });
    if !enabled {
        return;
    }
    cancel_editor(window);
    let mut rect: RECT = unsafe { zeroed() };
    unsafe {
        GetWindowRect(GetDlgItem(window, ec::PATTERN as i32), &mut rect);
    }
    let menu = unsafe { CreatePopupMenu() };
    let choices = [Pattern::Solid, Pattern::Dashed, Pattern::Dotted];
    let labels = if thai {
        ["────  เส้นทึบ", "— —  เส้นประ", "· · ·  เส้นจุด"]
    } else {
        ["────  Solid", "— —  Dashed", "· · ·  Dotted"]
    };
    unsafe {
        for (i, name) in labels.iter().enumerate() {
            AppendMenuW(
                menu,
                MF_STRING | if choices[i] == current { MF_CHECKED } else { 0 },
                i + 1,
                wide(name).as_ptr(),
            );
        }
    }
    let id = unsafe {
        TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            rect.left,
            rect.bottom,
            0,
            window,
            null(),
        )
    } as usize;
    unsafe {
        DestroyMenu(menu);
    }
    if let Some(pattern) = id.checked_sub(1).and_then(|i| choices.get(i)).copied() {
        STATE.with(|s| {
            if let Some(p) = s
                .borrow_mut()
                .as_mut()
                .unwrap()
                .previews
                .get_mut(&(window as usize))
            {
                p.session.set_pattern(pattern);
            }
        });
        update_editor(window);
    }
}

fn editor_mouse(window: HWND, message: u32, x: i32, y: i32) {
    if STATE.with(|s| {
        s.borrow()
            .as_ref()
            .is_some_and(|a| a.project_exit.is_some() || a.project_modal)
    }) {
        return;
    }
    if message == WM_LBUTTONDOWN {
        unsafe {
            SetFocus(window);
        }
    }
    let crop = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .previews
            .get(&(window as usize))
            .map(|p| p.session.document.crop)
    });
    let Some(crop) = crop else {
        return;
    };
    let view = editor_view(window, crop);
    let radius = 5. * unsafe { GetDpiForWindow(window) } as f64 / 96. / view.scale;
    let raw = Point::new(
        (x as f64 - view.x) / view.scale + crop.x as f64,
        (y as f64 - view.y) / view.scale + crop.y as f64,
    );
    let resizing = STATE.with(|s| {
        let b = s.borrow();
        let p = &b.as_ref().unwrap().previews[&(window as usize)];
        p.session.resize.is_some()
            || p.session.shape_resize.is_some()
            || p.session.shape_handle(raw, radius).is_some()
            || p.session.resize_handle(raw, radius).is_some()
            || p.session.tail_drag.is_some()
            || p.session.tail_handle(raw, radius)
            || p.session.curve_drag.is_some()
            || p.session.curve_handle(raw, radius)
    });
    if message == WM_LBUTTONDBLCLK {
        let reset = STATE.with(|s| {
            let mut b = s.borrow_mut();
            let p = b
                .as_mut()
                .unwrap()
                .previews
                .get_mut(&(window as usize))
                .unwrap();
            if p.session.curve_handle(raw, radius) {
                p.session.cancel();
                let before = p.session.document.render_snapshot();
                if p.session.document.reset_curve() {
                    let replacement = p
                        .session
                        .document
                        .objects
                        .iter()
                        .find(|o| Some(o.id) == p.session.document.selected)
                        .unwrap()
                        .clone();
                    p.curve_feedback = Some((before, replacement));
                }
                true
            } else {
                false
            }
        });
        if reset {
            unsafe {
                ReleaseCapture();
            }
            update_editor(window);
            return;
        }
    }
    let Some(point) = (if resizing {
        Some(raw)
    } else {
        view.point(
            x as f64,
            y as f64,
            !matches!(message, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK),
        )
    }) else {
        return;
    };
    let text_action =
        STATE.with(|s| {
            let b = s.borrow();
            let p = &b.as_ref().unwrap().previews[&(window as usize)];
            if (message == WM_LBUTTONDBLCLK && !resizing)
                || (message == WM_LBUTTONDOWN
                    && matches!(p.session.tool, Tool::Text | Tool::Callout)
                    && !resizing)
            {
                let existing =
                    p.session
                        .document
                        .hit(point, 0.)
                        .and_then(|id| {
                            p.session.document.objects.iter().find(|o| {
                                o.id == id && matches!(o.tool, Tool::Text | Tool::Callout)
                            })
                        })
                        .map(|o| Some(o.id));
                if message == WM_LBUTTONDOWN {
                    Some(existing.flatten())
                } else {
                    existing
                }
            } else {
                None
            }
        });
    if let Some(object) = text_action {
        open_text_entry(window, point, object);
        return;
    }
    let changed = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let p = b
            .as_mut()
            .unwrap()
            .previews
            .get_mut(&(window as usize))
            .unwrap();
        match message {
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => {
                p.curve_feedback = None;
                p.session.press(point, radius);
                true
            }
            WM_MOUSEMOVE => p.session.update(point),
            WM_LBUTTONUP => {
                p.session.update(point);
                if let Some(replacement) =
                    p.session.curve_draft().or_else(|| p.session.shape_draft())
                {
                    p.curve_feedback = Some((p.session.document.render_snapshot(), replacement));
                }
                p.session.release(point);
                true
            }
            _ => false,
        }
    });
    if !changed {
        return;
    }
    unsafe {
        if matches!(message, WM_LBUTTONDOWN | WM_LBUTTONDBLCLK) {
            SetFocus(window);
            SetCapture(window);
        }
        if message == WM_LBUTTONUP {
            ReleaseCapture();
        }
        InvalidateRect(window, null(), 0);
    }
    if message != WM_MOUSEMOVE {
        update_editor(window);
        let error = STATE.with(|s| {
            s.borrow_mut()
                .as_mut()
                .unwrap()
                .previews
                .get_mut(&(window as usize))
                .and_then(|p| p.session.error.take())
        });
        if let Some(error) = error {
            notify_text_error(window, &error);
        }
    }
}

fn open_text_entry(owner: HWND, point: Point, object: Option<u64>) {
    let data = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        if a.text_entry.is_some() {
            return None;
        }
        let p = a.previews.get_mut(&(owner as usize))?;
        p.session.cancel();
        let text = object
            .and_then(|id| p.session.document.objects.iter().find(|o| o.id == id))
            .and_then(|o| {
                if let Content::Text { text, .. } = &o.content {
                    Some(text.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        Some((text, p.guard.token.identity, a.settings.language == "th"))
    });
    let Some((text, identity, thai)) = data else {
        return;
    };
    let window = create_window(
        TEXT_ENTRY,
        &format!(
            "{TITLE} - {}",
            if thai {
                "ข้อความ"
            } else {
                "Text"
            }
        ),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
        WS_EX_CONTROLPARENT,
        workspace_ui::native_owner(owner),
        560,
        300,
    );
    if window.is_null() {
        notify(owner, "Cannot open text entry.", true);
        return;
    }
    STATE.with(|s| {
        s.borrow_mut().as_mut().unwrap().text_entry = Some(TextEntry {
            canvas: false,
            window: window as usize,
            owner: owner as usize,
            point,
            object,
            identity,
        })
    });
    let edit = child(
        window,
        "EDIT",
        &text,
        701,
        WS_TABSTOP
            | WS_BORDER
            | WS_VSCROLL
            | WS_HSCROLL
            | ES_MULTILINE as u32
            | ES_AUTOVSCROLL as u32
            | ES_AUTOHSCROLL as u32
            | ES_WANTRETURN as u32,
    );
    unsafe {
        SendMessageW(edit, EM_SETLIMITTEXT, 8192, 0);
    }
    child(
        window,
        "BUTTON",
        if thai { "ยืนยัน" } else { "Apply" },
        1,
        WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
    );
    child(
        window,
        "BUTTON",
        if thai { "ยกเลิก" } else { "Cancel" },
        2,
        WS_TABSTOP,
    );
    layout(window);
    unsafe {
        EnableWindow(workspace_ui::native_owner(owner), 0);
        ShowWindow(window, SW_SHOW);
        SetForegroundWindow(window);
        SetFocus(edit);
        SendMessageW(edit, EM_SETSEL, 0, -1);
    }
}

fn finish_text_entry(window: HWND, apply: bool) {
    let mut value = vec![0u16; 8193];
    let count = unsafe {
        GetWindowTextW(
            GetDlgItem(window, 701),
            value.as_mut_ptr(),
            value.len() as i32,
        )
    };
    let text = String::from_utf16_lossy(&value[..count.max(0) as usize]).replace("\r\n", "\n");
    let outcome = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        let e = a.text_entry.as_ref()?;
        if e.window != window as usize {
            return None;
        }
        if apply {
            if let Some(p) = a
                .previews
                .get_mut(&e.owner)
                .filter(|p| p.guard.token.identity == e.identity)
            {
                if e.canvas {
                    let mut height = [0u16; 16];
                    let n =
                        unsafe { GetWindowTextW(GetDlgItem(window, 702), height.as_mut_ptr(), 16) };
                    let height =
                        String::from_utf16_lossy(&height[..n.max(0) as usize]).parse::<u32>();
                    let result = match (text.parse::<u32>(), height) {
                        (Ok(w), Ok(h)) => p.session.document.expand_canvas(w, h),
                        _ => Err("Enter a valid width and height in pixels.".into()),
                    };
                    if let Err(error) = result {
                        return Some(Err(error));
                    }
                    return Some(Ok(a.text_entry.take().unwrap().owner));
                }
                if text.trim().is_empty() || text.chars().count() > 4096 {
                    return Some(Err("Enter 1-4096 nonblank characters.".to_string()));
                }
                let opacity = p.session.opacity();
                let result = if let Some(id) = e.object {
                    p.session.document.selected = Some(id);
                    let size = p
                        .session
                        .document
                        .objects
                        .iter()
                        .find(|o| o.id == id)
                        .map(|o| o.parameter())
                        .unwrap_or(p.session.text_size);
                    p.session.document.edit_text(&text, size)
                } else {
                    if p.session.tool == Tool::Callout {
                        p.session.document.add_callout(
                            e.point,
                            &text,
                            p.session.text_size,
                            p.session.style,
                            p.session.text_background(),
                        )
                    } else if p.session.text_background() == Default::default() {
                        p.session.document.add_text(
                            e.point,
                            &text,
                            p.session.text_size,
                            p.session.style,
                        )
                    } else {
                        p.session.document.add_label(
                            e.point,
                            &text,
                            p.session.text_size,
                            p.session.style,
                            p.session.text_background(),
                            false,
                        )
                    }
                };
                if e.object.is_none() {
                    p.session.apply_creation_opacity(
                        result.as_ref().is_ok_and(|changed| *changed),
                        opacity,
                    );
                }
                if let Err(error) = result {
                    return Some(Err(error));
                }
            }
        }
        Some(Ok(a.text_entry.take().unwrap().owner))
    });
    match outcome {
        Some(Err(error)) => notify_text_error(window, &error),
        Some(Ok(owner)) => {
            unsafe {
                DestroyWindow(window);
                EnableWindow(workspace_ui::native_owner(owner as HWND), 1);
                SetForegroundWindow(workspace_ui::native_owner(owner as HWND));
                SetFocus(owner as HWND);
            }
            update_editor(owner as HWND);
        }
        None => {}
    }
}

fn cancel_editor(window: HWND) {
    STATE.with(|s| {
        if let Some(p) = s
            .borrow_mut()
            .as_mut()
            .unwrap()
            .previews
            .get_mut(&(window as usize))
        {
            p.session.cancel();
            p.curve_feedback = None;
        }
    });
    unsafe {
        if GetCapture() == window {
            ReleaseCapture();
        }
    }
    update_editor(window);
}

fn draw_editor_overlay(window: HWND, dc: HDC, view: View, buffer: Option<&mut BackBuffer>) {
    let (draft, selected, crop, resize_bounds, handles, replacement, object_transform) = STATE
        .with(|s| {
            let b = s.borrow();
            let p = &b.as_ref().unwrap().previews[&(window as usize)];
            let selected = p
                .session
                .document
                .objects
                .iter()
                .find(|o| Some(o.id) == p.session.document.selected)
                .cloned();
            let draft = p
                .session
                .gesture
                .as_ref()
                .filter(|g| !matches!(g.tool, Tool::Crop | Tool::Step | Tool::Text | Tool::Callout))
                .and_then(|g| {
                    if g.tool == Tool::Select {
                        selected.clone().map(|mut o| {
                            o.points = Arc::new(
                                o.points
                                    .iter()
                                    .map(|point| {
                                        let scale = p.session.selected_transform().scale;
                                        Point::new(
                                            point.x + (g.end.x - g.start.x) / scale,
                                            point.y + (g.end.y - g.start.y) / scale,
                                        )
                                    })
                                    .collect(),
                            );
                            o
                        })
                    } else {
                        Some(Annotation {
                            id: 0,
                            parent: None,
                            tool: g.tool,
                            points: Arc::new(if matches!(g.tool, Tool::Pen | Tool::Highlighter) {
                                g.points.clone()
                            } else {
                                vec![g.start, g.end]
                            }),
                            style: p.session.style,
                            pattern: p.session.pattern(),
                            rendering: Default::default(),
                            opacity: p.session.opacity(),
                            effects: p.session.creation_effects(),
                            content: if g.tool == Tool::Highlighter {
                                Content::Highlight
                            } else if g.tool == Tool::Blur {
                                Content::Blur {
                                    strength: p.session.blur_strength,
                                }
                            } else {
                                Content::Shape
                            },
                        })
                    }
                });
            let curve = p.session.curve_draft().or_else(|| p.session.shape_draft());
            let mut replacement = curve
                .clone()
                .map(|o| (p.frame.clone(), p.session.document.render_snapshot(), o))
                .or_else(|| {
                    p.curve_feedback
                        .as_ref()
                        .filter(|_| {
                            p.cached
                                .as_ref()
                                .is_none_or(|(r, _)| *r != p.session.document.revision)
                        })
                        .map(|(d, o)| (p.frame.clone(), d.render_snapshot(), o.clone()))
                });
            let draft = if curve.is_some() {
                curve
            } else if let Some((_, _, o)) = &replacement {
                Some(o.clone())
            } else if let Some(d) = &p.session.tail_drag {
                selected.clone().map(|mut o| {
                    if let Content::Text {
                        tail: Some(tip), ..
                    } = &mut o.content
                    {
                        tip.x += d.end.x - d.start.x;
                        tip.y += d.end.y - d.start.y;
                    }
                    o
                })
            } else {
                draft
            };
            if replacement.is_none() {
                if let Some(o) = draft.as_ref().filter(|o| {
                    o.parent.is_none()
                        && o.tool != Tool::Image
                        && o.effects.active(matches!(o.content, Content::Text { .. }))
                }) {
                    replacement = Some((
                        p.frame.clone(),
                        p.session.document.render_snapshot(),
                        o.clone(),
                    ));
                }
            }
            let resize_bounds = p.session.resize.as_ref().map(|r| r.bounds()).or_else(|| {
                p.session
                    .shape_resize
                    .as_ref()
                    .map(|r| r.draft().as_ref().unwrap_or(&r.original).bounds())
            });
            let handles = p.session.tool == Tool::Select
                && selected
                    .as_ref()
                    .is_some_and(|o| matches!(o.tool, Tool::Text | Tool::Callout | Tool::Image));
            let object_transform = if p.session.crop_draft().is_none()
                && (resize_bounds.is_some()
                    || draft
                        .as_ref()
                        .or(selected.as_ref())
                        .is_some_and(|o| o.parent.is_some()))
            {
                p.session.selected_transform()
            } else {
                Default::default()
            };
            (
                draft,
                selected,
                p.session.crop_draft(),
                resize_bounds,
                handles,
                replacement,
                object_transform,
            )
        });
    let map = |p: Point| {
        (
            (view.x + (p.x - view.crop.x as f64) * view.scale).round() as i32,
            (view.y + (p.y - view.crop.y as f64) * view.scale).round() as i32,
        )
    };
    let map_object = |p: Point| map(object_transform.point(p));
    unsafe {
        let saved = SaveDC(dc);
        IntersectClipRect(
            dc,
            view.x.round() as i32,
            view.y.round() as i32,
            (view.x + view.crop.width as f64 * view.scale).round() as i32,
            (view.y + view.crop.height as f64 * view.scale).round() as i32,
        );
        if let (Some(o), Some(buffer)) = (draft.as_ref().filter(|o| o.parent.is_none()), buffer) {
            let pixels = if let Some((source, document, replacement)) = &replacement {
                render::replacement_draft(source, document, replacement, view)
                    .or_else(|| render::draft(o, view))
            } else {
                render::draft(o, view)
            };
            if let Some(pixels) = pixels {
                buffer.blend(&pixels);
            }
        }
        if crop.is_none() {
            STATE.with(|s| {
                let state = s.borrow();
                let document = &state.as_ref().unwrap().previews[&(window as usize)]
                    .session
                    .document;
                let mut parent = draft.as_ref().or(selected.as_ref()).and_then(|o| o.parent);
                for _ in 0..8 {
                    let Some(owner) =
                        parent.and_then(|id| document.objects.iter().find(|o| o.id == id))
                    else {
                        break;
                    };
                    let transform = document.parent_transform(owner.id);
                    let (a, b) = owner.bounds();
                    let a = map(transform.point(a));
                    let b = map(transform.point(b));
                    IntersectClipRect(dc, a.0, a.1, b.0, b.1);
                    parent = owner.parent;
                }
            });
        }
        let bounds = if let Some(bounds) = resize_bounds {
            Some(bounds)
        } else if let Some(crop) = crop {
            Some((
                Point::new(crop.x as f64, crop.y as f64),
                Point::new(crop.right() as f64, crop.bottom() as f64),
            ))
        } else {
            draft.as_ref().or(selected.as_ref()).map(|o| {
                let (a, b) = o.bounds();
                let (x, y, r, b) = (a.x, a.y, b.x, b.y);
                let pad = if matches!(o.tool, Tool::Text | Tool::Callout | Tool::Image)
                    || crate::shape_frame::supported(o.tool)
                {
                    0.
                } else {
                    o.style.width as f64 / 2. + 3. / view.scale
                };
                (Point::new(x - pad, y - pad), Point::new(r + pad, b + pad))
            })
        };
        if let Some((a, b)) = bounds {
            let source_bounds = (a, b);
            let a = map_object(a);
            let b = map_object(b);
            let pen = CreatePen(PS_DOT, 1, 0x00da6716);
            let old = SelectObject(dc, pen);
            let brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
            Rectangle(dc, a.0, a.1, b.0, b.1);
            SelectObject(dc, brush);
            SelectObject(dc, old);
            DeleteObject(pen);
            if handles {
                let half = (4. * GetDpiForWindow(window) as f64 / 96.).round().max(3.) as i32;
                let pen = CreatePen(PS_SOLID, 1, 0x00da6716);
                let old_pen = SelectObject(dc, pen);
                let old_brush = SelectObject(dc, GetStockObject(WHITE_BRUSH));
                for (_, p) in crate::text_frame::positions(source_bounds) {
                    if selected.as_ref().is_some_and(|o| o.tool == Tool::Image)
                        && (p.y - (source_bounds.0.y + source_bounds.1.y) / 2.).abs() < 0.001
                    {
                        continue;
                    }
                    let (x, y) = map_object(p);
                    Rectangle(dc, x - half, y - half, x + half + 1, y + half + 1);
                }
                if let Some(tip) = draft
                    .as_ref()
                    .or(selected.as_ref())
                    .and_then(|o| o.tail_tip())
                {
                    let (x, y) = map_object(tip);
                    Ellipse(dc, x - half, y - half, x + half + 1, y + half + 1);
                }
                SelectObject(dc, old_brush);
                SelectObject(dc, old_pen);
                DeleteObject(pen);
            }
            if let Some(midpoint) = STATE.with(|s| {
                s.borrow().as_ref().unwrap().previews[&(window as usize)]
                    .session
                    .curve_handle_position()
            }) {
                let (x, y) = map(midpoint);
                let half = (4. * GetDpiForWindow(window) as f64 / 96.).round().max(3.) as i32;
                let pen = CreatePen(PS_SOLID, 1, 0x00da6716);
                let old_pen = SelectObject(dc, pen);
                let old_brush = SelectObject(dc, GetStockObject(WHITE_BRUSH));
                Ellipse(dc, x - half, y - half, x + half + 1, y + half + 1);
                SelectObject(dc, old_brush);
                SelectObject(dc, old_pen);
                DeleteObject(pen);
            }
            let shape_handles = STATE.with(|s| {
                s.borrow().as_ref().unwrap().previews[&(window as usize)]
                    .session
                    .shape_handles()
            });
            if !shape_handles.is_empty() {
                let half = (4. * GetDpiForWindow(window) as f64 / 96.).round().max(3.) as i32;
                let pen = CreatePen(PS_SOLID, 1, 0x00da6716);
                let old_pen = SelectObject(dc, pen);
                let old_brush = SelectObject(dc, GetStockObject(WHITE_BRUSH));
                for (_, p) in shape_handles {
                    let (x, y) = map(p);
                    Rectangle(dc, x - half, y - half, x + half + 1, y + half + 1);
                }
                SelectObject(dc, old_brush);
                SelectObject(dc, old_pen);
                DeleteObject(pen);
            }
        }
        RestoreDC(dc, saved);
    }
}

fn complete_work() {
    let detections: Vec<_> = STATE.with(|state| {
        state
            .borrow()
            .as_ref()
            .unwrap()
            .detection_rx
            .try_iter()
            .collect()
    });
    for result in detections {
        complete_detection(result);
    }
    let work: Vec<_> = STATE.with(|s| s.borrow().as_ref().unwrap().rx.try_iter().collect());
    for item in work {
        match item {
            Work::VideoSaved {
                owner,
                identity,
                operation,
                result,
            } => {
                video_ui::saved(owner, identity, operation, result);
            }
            Work::Combined {
                route,
                owner,
                identity,
                started,
                result,
            } => combine_ui::finished(route, owner, identity, started, result),
            Work::ProjectOpened {
                origin,
                path,
                started,
                result,
            } => project_ui::opened(origin, path, started, result),
            Work::ProjectSaved {
                owner,
                identity,
                operation,
                revision,
                path,
                started,
                result,
            } => project_ui::saved(owner, identity, operation, revision, path, started, result),
            Work::Imported {
                owner,
                identity,
                result,
            } => {
                let error = STATE.with(|s| {
                    let mut b=s.borrow_mut();
                    let a=b.as_mut().unwrap();
                    a.importing=false;
                    if !a.previews.get(&owner).is_some_and(|p| p.guard.token.identity==identity) { return None; }
                    a.previews.get_mut(&owner).unwrap().importing=false;
                    let available = crate::image_io::SOURCE_LIMIT.saturating_sub(source_bytes(a));
                    let result = result.and_then(|f| {
                        if f.bgra.len()>available { return Err("Image memory limit reached. Close an older editor or use smaller images.".into()); }
                        let p=a.previews.get_mut(&owner).unwrap();
                        p.session.cancel();
                        p.session.tool=Tool::Select;
                        let changed = p.session.document.add_image(Arc::new(f))?;
                        p.session.apply_creation_opacity(changed, 100);
                        Ok(changed)
                    });
                    result.err()
                });
                if unsafe { IsWindow(owner as HWND) } != 0 {
                    update_editor(owner as HWND);
                    if let Some(error) = error {
                        status(owner as HWND, &error);
                    }
                }
            }
            Work::Composed {
                owner,
                token,
                started,
                result,
            } => {
                let mut error = None;
                let accepted = STATE.with(|s| {
                    let mut b = s.borrow_mut();
                    let a = b.as_mut().unwrap();
                    a.composing = false;
                    if let Some(p) = a.previews.get_mut(&owner) {
                        if p.guard.accepts(token) {
                            match result {
                                Ok(frame) => {
                                    p.cached = Some((token.revision, frame));
                                    p.curve_feedback = None;
                                }
                                Err(e) => {
                                    p.guard.failed = true;
                                    error = Some(e);
                                }
                            }
                            return true;
                        }
                    }
                    false
                });
                trace::record("editor.compose", started);
                if accepted {
                    update_editor(owner as HWND);
                }
                if let Some(error) = error {
                    status(owner as HWND, &error);
                }
                schedule_composition();
            }
            Work::Snapshot {
                mode,
                started,
                result,
            } => match result {
                Ok((frame, desktop, windows)) => {
                    if matches!(mode, Mode::Desktop) {
                        STATE.with(|s| s.borrow_mut().as_mut().unwrap().capture_busy = false);
                        restore_windows();
                        present(frame, started);
                    } else {
                        show_selection(mode, started, frame, desktop, windows);
                    }
                }
                Err(error) => {
                    STATE.with(|s| s.borrow_mut().as_mut().unwrap().capture_busy = false);
                    restore_windows();
                    notify(null_mut(), &error, true);
                }
            },
            Work::Saved {
                owner,
                identity,
                revision,
                started,
                result,
            } => {
                let accepted = STATE.with(|s| {
                    let mut b = s.borrow_mut();
                    let a = b.as_mut().unwrap();
                    a.saving = a.saving.saturating_sub(1);
                    if let Some(p) = a
                        .previews
                        .get_mut(&owner)
                        .filter(|p| p.guard.token.identity == identity)
                    {
                        p.saving = false;
                        true
                    } else {
                        false
                    }
                });
                if !accepted {
                    continue;
                }
                unsafe {
                    EnableWindow(GetDlgItem(owner as HWND, SAVE as i32), 1);
                }
                match result {
                    Ok(()) => {
                        trace::record("export.png", started);
                        STATE.with(|s| {
                            if let Some(p) =
                                s.borrow_mut().as_mut().unwrap().previews.get_mut(&owner)
                            {
                                p.session.document.mark_kept(revision);
                            }
                        });
                        let current = STATE.with(|s| {
                            s.borrow()
                                .as_ref()
                                .unwrap()
                                .previews
                                .get(&owner)
                                .is_some_and(|p| p.session.document.is_kept())
                        });
                        if current {
                            status(owner as HWND, &tr("saved"));
                        } else {
                            update_editor(owner as HWND);
                        }
                    }
                    Err(error) => status(owner as HWND, &error),
                }
            }
            Work::OcrFinished {
                owner,
                identity,
                request,
                revision,
                result,
                started,
            } => {
                let target = STATE.with(|s| {
                    let mut borrow = s.borrow_mut();
                    let app = borrow.as_mut().unwrap();
                    app.ocr_busy = false;
                    app.ocr_windows
                        .iter()
                        .find(|(_, entry)| {
                            entry.owner == owner
                                && entry.identity == identity
                                && entry.request == request
                                && entry.source_revision == revision
                        })
                        .map(|(window, _)| *window)
                });
                trace::record("ocr.worker-complete", started);
                if let Some(target) = target {
                    let window = target as HWND;
                    unsafe {
                        EnableWindow(GetDlgItem(window, OCR_READ as i32), 1);
                    }
                    match result {
                        Ok(text) if text.is_empty() => set_ocr_status(
                            window,
                            if STATE
                                .with(|s| s.borrow().as_ref().unwrap().settings.language == "th")
                            {
                                "ไม่พบข้อความในภาพ"
                            } else {
                                "No text found."
                            },
                        ),
                        Ok(text) => {
                            unsafe {
                                SetWindowTextW(
                                    GetDlgItem(window, OCR_RESULT),
                                    wide(&text).as_ptr(),
                                );
                            }
                            let current_revision = STATE.with(|s| {
                                s.borrow()
                                    .as_ref()
                                    .unwrap()
                                    .previews
                                    .get(&owner)
                                    .map(|preview| preview.session.document.revision)
                            });
                            let thai = STATE
                                .with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
                            let status = if current_revision
                                .is_some_and(|current| current != revision)
                            {
                                if thai {
                                    "ผลลัพธ์มาจากภาพก่อนหน้า ตรวจทานก่อนคัดลอก"
                                } else {
                                    "Image changed during OCR. Review the result before copying."
                                }
                            } else if thai {
                                "อ่านข้อความแล้ว ตรวจทานและแก้ไขก่อนคัดลอก"
                            } else {
                                "Completed. Review and edit before copying."
                            };
                            set_ocr_status(window, status);
                        }
                        Err(error) => set_ocr_status(window, &error),
                    }
                }
            }
        }
    }
}

fn copy_image(window: HWND) {
    match rendered_frame(window) {
        Ok((frame, revision)) => {
            let started = Instant::now();
            match clipboard::copy(window, &frame) {
                Ok(()) => {
                    STATE.with(|s| {
                        if let Some(p) = s
                            .borrow_mut()
                            .as_mut()
                            .unwrap()
                            .previews
                            .get_mut(&(window as usize))
                        {
                            p.session.document.mark_kept(revision);
                        }
                    });
                    status(window, &tr("copied"));
                    trace::record("clipboard.copy", started);
                }
                Err(error) => notify(window, &error, true),
            }
        }
        Err(error) => notify(window, &error, true),
    }
}

fn save_image(window: HWND) {
    if STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.project_modal || a.project_exit.is_some()
    }) {
        return;
    }
    if STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .previews
            .get(&(window as usize))
            .is_none_or(|p| p.saving)
    }) {
        return;
    }
    let (frame, revision) = match rendered_frame(window) {
        Ok(frame) => frame,
        Err(error) => {
            notify(window, &error, true);
            return;
        }
    };
    let mut filename = [0u16; 32768];
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let name = wide(&format!("SnapZy-Rust-{stamp}.png"));
    filename[..name.len()].copy_from_slice(&name);
    let filter: Vec<u16> = "PNG image (*.png)\0*.png\0\0".encode_utf16().collect();
    let extension = wide("png");
    let title = wide(&tr("save"));
    let mut dialog: OPENFILENAMEW = unsafe { zeroed() };
    dialog.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    dialog.hwndOwner = workspace_ui::native_owner(window);
    dialog.lpstrFile = filename.as_mut_ptr();
    dialog.nMaxFile = filename.len() as u32;
    dialog.lpstrFilter = filter.as_ptr();
    dialog.nFilterIndex = 1;
    dialog.lpstrDefExt = extension.as_ptr();
    dialog.lpstrTitle = title.as_ptr();
    dialog.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
    unsafe {
        if GetSaveFileNameW(&mut dialog) == 0 {
            let error = CommDlgExtendedError();
            if error != 0 {
                notify(window, &format!("Save dialog failed ({error})."), true);
            }
            return;
        }
    }
    use std::os::windows::ffi::OsStringExt;
    let end = filename
        .iter()
        .position(|c| *c == 0)
        .unwrap_or(filename.len());
    let path = PathBuf::from(std::ffi::OsString::from_wide(&filename[..end]));
    let started = Instant::now();
    let (sender, main, identity) = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        a.saving += 1;
        if let Some(p) = a.previews.get_mut(&(window as usize)) {
            p.saving = true;
        }
        (
            a.tx.clone(),
            a.main,
            a.previews[&(window as usize)].guard.token.identity,
        )
    });
    status(window, &tr("saving"));
    unsafe {
        EnableWindow(GetDlgItem(window, SAVE as i32), 0);
    }
    let owner = window as usize;
    std::thread::spawn(move || {
        let result = write_atomic_png(&frame, &path);
        let _ = sender.send(Work::Saved {
            owner,
            identity,
            revision,
            started,
            result,
        });
        unsafe {
            PostMessageW(main as HWND, COMPLETE, 0, 0);
        }
    });
}

pub fn write_atomic_png(frame: &Frame, path: &std::path::Path) -> Result<(), String> {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let filename = path
        .file_name()
        .ok_or("Choose a PNG filename.")?
        .to_string_lossy();
    let temp = path.with_file_name(format!(".{filename}.{}-{suffix}.tmp", std::process::id()));
    let result = (|| {
        let mut output = BufWriter::new(
            fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp)
                .map_err(|e| e.to_string())?,
        );
        frame.write_png(&mut output)?;
        output.flush().map_err(|e| e.to_string())?;
        output.get_ref().sync_all().map_err(|e| e.to_string())?;
        drop(output);
        fs::rename(&temp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn show_hotkeys() {
    let (existing, main, settings) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        (a.settings_window, a.main, a.settings.clone())
    });
    if existing != 0 {
        unsafe {
            SetForegroundWindow(existing as HWND);
        }
        return;
    }
    let window = create_window(
        HOTKEYS,
        &format!("{} - {}", TITLE, tr("keys")),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
        0,
        main as HWND,
        560,
        350,
    );
    if window.is_null() {
        return;
    }
    STATE.with(|s| s.borrow_mut().as_mut().unwrap().settings_window = window as usize);
    child(window, "STATIC", &tr("hotkey_hint"), HOTKEY_HINT, 0);
    for (action, enable_id, modifier_id, key_id) in HOTKEY_ROWS {
        let binding = settings.binding(action);
        let modifier = child(
            window,
            "COMBOBOX",
            "",
            modifier_id,
            WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let key = child(
            window,
            "COMBOBOX",
            "",
            key_id,
            WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        child(
            window,
            "BUTTON",
            &tr(action.label_key()),
            enable_id,
            WS_TABSTOP | BS_AUTOCHECKBOX as u32,
        );
        for value in [
            "Ctrl + Alt",
            "Ctrl + Shift",
            "Alt + Shift",
            "Ctrl + Alt + Shift",
        ] {
            unsafe {
                SendMessageW(modifier, CB_ADDSTRING, 0, wide(value).as_ptr() as LPARAM);
            }
        }
        for value in ["F8", "F9", "F10", "F11", "Print Screen"] {
            unsafe {
                SendMessageW(key, CB_ADDSTRING, 0, wide(value).as_ptr() as LPARAM);
            }
        }
        unsafe {
            SendMessageW(
                GetDlgItem(window, enable_id),
                BM_SETCHECK,
                if binding.enabled {
                    BST_CHECKED as usize
                } else {
                    BST_UNCHECKED as usize
                },
                0,
            );
            SendMessageW(
                modifier,
                CB_SETCURSEL,
                settings::HOTKEY_MODIFIERS
                    .iter()
                    .position(|value| *value == binding.modifiers)
                    .unwrap_or(0),
                0,
            );
            SendMessageW(
                key,
                CB_SETCURSEL,
                settings::HOTKEY_KEYS
                    .iter()
                    .position(|value| *value == binding.key)
                    .unwrap_or(0),
                0,
            );
        }
    }
    child(
        window,
        "BUTTON",
        &tr("apply"),
        APPLY as i32,
        WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
    );
    layout(window);
    unsafe {
        EnableWindow(main as HWND, 0);
        ShowWindow(window, SW_SHOW);
        SetForegroundWindow(window);
    }
}

fn hotkey_form_binding(window: HWND, row: (HotkeyAction, i32, i32, i32)) -> Option<HotkeyBinding> {
    let (_, enable_id, modifier_id, key_id) = row;
    let (enabled, modifier, key) = unsafe {
        (
            SendMessageW(GetDlgItem(window, enable_id), BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
            SendMessageW(GetDlgItem(window, modifier_id), CB_GETCURSEL, 0, 0),
            SendMessageW(GetDlgItem(window, key_id), CB_GETCURSEL, 0, 0),
        )
    };
    if !(0..settings::HOTKEY_MODIFIERS.len() as isize).contains(&modifier)
        || !(0..settings::HOTKEY_KEYS.len() as isize).contains(&key)
    {
        return None;
    }
    Some(HotkeyBinding::new(
        enabled,
        settings::HOTKEY_MODIFIERS[modifier as usize],
        settings::HOTKEY_KEYS[key as usize],
    ))
}

fn unregister_hotkeys(main: HWND, active: &[(HotkeyAction, i32)]) {
    for (_, id) in active {
        unsafe {
            UnregisterHotKey(main, *id);
        }
    }
}

fn register_hotkeys(main: HWND, settings: &Settings) -> Result<Vec<(HotkeyAction, i32)>, String> {
    let mut registered = Vec::new();
    let mut combinations = Vec::new();
    for (index, (action, _, _, _)) in HOTKEY_ROWS.iter().enumerate() {
        let binding = settings.binding(*action);
        if !binding.enabled {
            continue;
        }
        if !binding.is_valid() {
            unregister_hotkeys(main, &registered);
            return Err(tr("conflict"));
        }
        if combinations.contains(&(binding.modifiers, binding.key)) {
            unregister_hotkeys(main, &registered);
            return Err(tr("duplicate"));
        }
        combinations.push((binding.modifiers, binding.key));
        let id = HOTKEY_ID_BASE + index as i32;
        let ok =
            unsafe { RegisterHotKey(main, id, binding.modifiers | MOD_NOREPEAT, binding.key) != 0 };
        if !ok {
            unregister_hotkeys(main, &registered);
            if crate::settings::is_test_run() {
                trace::test_note(&format!(
                    "hotkey-register action={action:?} id={id} error={}",
                    unsafe { GetLastError() }
                ));
            }
            return Err(tr("conflict"));
        }
        registered.push((*action, id));
    }
    Ok(registered)
}

fn apply_hotkeys(window: HWND) {
    let (main, old_settings, old_active) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        (a.main, a.settings.clone(), a.active_hotkeys.clone())
    });
    let mut settings = old_settings.clone();
    for row in HOTKEY_ROWS {
        let Some(binding) = hotkey_form_binding(window, row) else {
            notify(window, &tr("invalid_hotkey"), true);
            return;
        };
        settings.set_binding(row.0, binding);
    }
    let area = settings.binding(HotkeyAction::Area);
    settings.hotkey_enabled = area.enabled;
    settings.hotkey_modifiers = area.modifiers;
    settings.hotkey_key = area.key;
    if crate::settings::is_test_run() {
        trace::test_note("hotkey-form read six command rows");
    }
    if let Err(error) = settings.validate() {
        notify(window, &error, true);
        return;
    }
    unregister_hotkeys(main as HWND, &old_active);
    let active = match register_hotkeys(main as HWND, &settings) {
        Ok(active) => active,
        Err(error) => {
            let restored = register_hotkeys(main as HWND, &old_settings).unwrap_or_default();
            STATE.with(|s| s.borrow_mut().as_mut().unwrap().active_hotkeys = restored);
            notify(window, &error, true);
            return;
        }
    };
    if let Err(error) = settings.save() {
        unregister_hotkeys(main as HWND, &active);
        let restored = register_hotkeys(main as HWND, &old_settings).unwrap_or_default();
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().active_hotkeys = restored);
        notify(window, &error, true);
        return;
    }
    STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        a.settings = settings;
        a.active_hotkeys = active;
    });
    notify(window, &tr("success"), false);
}

fn close_window(window: HWND) {
    let class = class_name(window);
    if class == workspace_ui::HOST {
        project_ui::close_host(window);
        return;
    }
    if class == OCR_WINDOW {
        STATE.with(|s| {
            s.borrow_mut()
                .as_mut()
                .unwrap()
                .ocr_windows
                .remove(&(window as usize));
        });
        unsafe {
            DestroyWindow(window);
        }
        return;
    }
    if class == SCROLL_PANEL {
        cancel_scroll();
        return;
    }
    if class == VIDEO_PANEL {
        stop_video();
        return;
    }
    if class == TEXT_ENTRY {
        finish_text_entry(window, false);
        return;
    }
    if class == MAIN {
        let available = STATE.with(|s| s.borrow().as_ref().unwrap().tray_added);
        if available {
            unsafe {
                ShowWindow(window, SW_HIDE);
            }
        } else {
            exit_app();
        }
        return;
    }
    if class == OVERLAY {
        finish_selection(true);
        return;
    }
    if class == HOTKEYS {
        let main = STATE.with(|s| {
            let mut b = s.borrow_mut();
            let a = b.as_mut().unwrap();
            a.settings_window = 0;
            a.main
        });
        unsafe {
            DestroyWindow(window);
            EnableWindow(main as HWND, 1);
            SetForegroundWindow(main as HWND);
        }
        return;
    }
    if !workspace_ui::can_switch(workspace_ui::native_owner(window)) {
        return;
    }
    workspace_ui::commit_numeric();
    if STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .previews
            .get(&(window as usize))
            .is_some_and(|p| p.saving)
    }) {
        notify(window, &tr("wait"), false);
        return;
    }
    if !project_ui::allow_close(window) {
        return;
    }
    remove_preview(window);
}
fn remove_preview(window: HWND) {
    let (owned, host, remaining, preview, tips) = STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        let app = borrow.as_mut().unwrap();
        let owned = app
            .ocr_windows
            .iter()
            .filter(|(_, entry)| entry.owner == window as usize)
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        for key in &owned {
            app.ocr_windows.remove(key);
        }
        let key = workspace_ui::doc_key(app, window as usize);
        let host = key.and_then(|key| app.workspace.host_for(key));
        if let Some(key) = key {
            app.workspace.remove_doc(key);
        }
        let remaining = host.is_some_and(|h| app.workspace.host(h).is_some());
        let preview = app.previews.remove(&(window as usize));
        let tips = host
            .filter(|_| !remaining)
            .and_then(|h| app.tab_tips.remove(&h.window));
        (owned, host, remaining, preview, tips)
    });
    drop(preview);
    drop(tips);
    unsafe {
        for key in owned {
            DestroyWindow(key as HWND);
        }
        DestroyWindow(window);
    }
    if let Some(host) = host {
        if remaining {
            workspace_ui::refresh(host.window as HWND);
            unsafe {
                if GetForegroundWindow() == host.window as HWND {
                    if let Some(active) = workspace_ui::active(host.window as HWND) {
                        SetFocus(active);
                    }
                }
            }
        } else {
            unsafe {
                DestroyWindow(host.window as HWND);
            }
        }
    }
}

fn exit_app() {
    if video_ui::saving() {
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().exit_after_video_exports = true);
        let main = STATE.with(|s| s.borrow().as_ref().unwrap().main) as HWND;
        status(main, &tr("video_exporting"));
        return;
    }
    let recording = STATE.with(|s| s.borrow().as_ref().unwrap().recording.is_some());
    if recording {
        let answer = unsafe {
            MessageBoxW(
                STATE.with(|s| s.borrow().as_ref().unwrap().main) as HWND,
                wide(&tr("video_exit")).as_ptr(),
                wide(DIALOG_TITLE).as_ptr(),
                MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
            )
        };
        if answer == IDYES {
            STATE.with(|s| s.borrow_mut().as_mut().unwrap().exit_after_video = true);
            stop_video();
        }
        return;
    }
    video_ui::start_exit();
}
fn finish_exit() {
    video_ui::shutdown_all();
    let (main, windows) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        let mut windows: Vec<_> = a.previews.keys().copied().collect();
        windows.extend(a.ocr_windows.keys().copied());
        windows.extend(a.video_previews.keys().copied());
        windows.extend(a.workspace.hosts.iter().map(|h| h.key.window));
        if a.settings_window != 0 {
            windows.push(a.settings_window);
        }
        if let Some(selection) = &a.selection {
            windows.push(selection.window);
        }
        if let Some(recording) = &a.recording {
            windows.push(recording.panel);
        }
        (a.main, windows)
    });
    cancel_scroll();
    tray(false);
    for window in windows {
        unsafe {
            DestroyWindow(window as HWND);
        }
    }
    unsafe {
        DestroyWindow(main as HWND);
        PostQuitMessage(0);
    }
}

fn handle_command(window: HWND, id: usize) {
    if STATE.with(|s| s.borrow().as_ref().is_some_and(|a| a.recording.is_some()))
        && !matches!(id, VIDEO_STOP | EXIT)
    {
        return;
    }
    if let Some((_, key, mode)) = video_audio_choices()
        .into_iter()
        .find(|(command, _, _)| *command == id)
    {
        let mut settings = STATE.with(|s| s.borrow().as_ref().unwrap().settings.clone());
        settings.video_audio = mode;
        match settings.save() {
            Ok(()) => {
                STATE.with(|s| s.borrow_mut().as_mut().unwrap().settings = settings);
                let main = STATE.with(|s| s.borrow().as_ref().unwrap().main) as HWND;
                status(
                    main,
                    &tr("video_audio_selected").replace("{mode}", &tr(key)),
                );
            }
            Err(error) => notify(window, &error, true),
        }
        return;
    }
    if let Some((_, seconds, _)) = delay_choices()
        .into_iter()
        .find(|(command, _, _)| *command == id)
    {
        set_capture_delay(window, seconds);
        return;
    }
    if id == ec::PROJECT_OPEN && class_name(window) == MAIN {
        project_ui::open(window);
        return;
    }
    if id == SCROLL_DETECT_START {
        confirm_scroll_selection(None);
        return;
    }
    if id == SCROLL_DETECT_CANCEL {
        finish_selection(true);
        return;
    }
    if matches!(id, ec::MORE | ec::LAYERS) {
        show_editor_menu(window, id);
        return;
    }
    if ec::is_command(id) {
        editor_command(window, id);
        return;
    }
    match id {
        WINDOW_MENU | SCROLL_OPTIONS | OPEN_OPTIONS | MORE => show_launcher_menu(window, id),
        DELAY_OPTIONS => show_delay_menu(window),
        OPEN_TAB | OPEN_WINDOW => {
            if STATE.with(|s| {
                s.borrow()
                    .as_ref()
                    .is_some_and(|a| a.project_modal || a.project_exit.is_some())
            }) {
                return;
            }
            let mut settings = STATE.with(|s| s.borrow().as_ref().unwrap().settings.clone());
            settings.open_mode = if id == OPEN_TAB {
                settings::OpenMode::Tab
            } else {
                settings::OpenMode::Window
            };
            match settings.save() {
                Ok(()) => {
                    STATE.with(|s| s.borrow_mut().as_mut().unwrap().settings = settings);
                    refresh_open_mode();
                }
                Err(error) => notify(window, &error, true),
            }
        }
        AREA => start_capture(Mode::Area),
        WINDOW => start_capture(Mode::Window),
        DESKTOP => start_capture(Mode::Desktop),
        SCROLL => start_capture(Mode::Scroll),
        VIDEO => show_video_menu(window),
        VIDEO_MONITOR => start_capture(Mode::VideoMonitor),
        VIDEO_WINDOW => start_capture(Mode::VideoWindow),
        VIDEO_STOP => stop_video(),
        SCROLL_AUTO | SCROLL_MANUAL => {
            let mut settings = STATE.with(|s| s.borrow().as_ref().unwrap().settings.clone());
            settings.scroll_automatic = id == SCROLL_AUTO;
            match settings.save() {
                Ok(()) => {
                    STATE.with(|s| s.borrow_mut().as_mut().unwrap().settings = settings);
                    refresh_scroll_mode();
                }
                Err(error) => notify(window, &error, true),
            }
        }
        SCROLL_PAUSE => {
            STATE.with(|s| {
                if let Some(job) = &s.borrow().as_ref().unwrap().scrolling {
                    job.toggle_pause();
                }
            });
            refresh_scroll_pause();
        }
        SCROLL_STOP => stop_scroll(),
        SCROLL_CANCEL => cancel_scroll(),
        COPY => copy_image(window),
        SAVE => save_image(window),
        OCR_READ => start_ocr(window),
        OCR_COPY => ocr_copy(window),
        OCR_CLOSE => close_window(window),
        KEYS => show_hotkeys(),
        APPLY => apply_hotkeys(window),
        LANGUAGE | ec::LANG => {
            let mut settings = STATE.with(|s| s.borrow().as_ref().unwrap().settings.clone());
            settings.language = if settings.language == "en" {
                "th"
            } else {
                "en"
            }
            .into();
            match settings.save() {
                Ok(()) => {
                    STATE.with(|s| s.borrow_mut().as_mut().unwrap().settings = settings);
                    refresh_language();
                }
                Err(error) => notify(window, &error, true),
            }
        }
        HIDE => unsafe {
            ShowWindow(window, SW_HIDE);
        },
        EXIT => exit_app(),
        ABOUT => notify(
            window,
            &format!(
                "{TITLE}\n\nNative Windows capture\nMIT License\nCopyright (c) 2026 jairlinethai\n\ngithub.com/jairlinethai8989/snapzy-rust"
            ),
            false,
        ),
        _ => {}
    }
}

fn fill(dc: HDC, rect: RECT, color: u32) {
    unsafe {
        let brush = CreateSolidBrush(color);
        FillRect(dc, &rect, brush);
        DeleteObject(brush);
    }
}

fn launcher_icon_command(id: usize) -> Option<IconCommand> {
    Some(match id {
        AREA => IconCommand::AreaCapture,
        WINDOW => IconCommand::WindowCapture,
        WINDOW_MENU => IconCommand::DesktopCapture,
        SCROLL => IconCommand::ScrollingCapture,
        VIDEO => IconCommand::VideoCapture,
        ec::PROJECT_OPEN => IconCommand::OpenImage,
        MORE => IconCommand::More,
        _ => return None,
    })
}

fn launcher_option_icon_command(id: usize) -> Option<IconCommand> {
    Some(match id {
        DELAY_OPTIONS => IconCommand::Delay,
        SCROLL_OPTIONS => IconCommand::ScrollingMode,
        OPEN_OPTIONS => IconCommand::OpenDestination,
        _ => return None,
    })
}

fn delay_button_label(seconds: u32, thai: bool) -> String {
    match (seconds, thai) {
        (0, false) => "Delay: None".into(),
        (0, true) => "หน่วง: ไม่หน่วง".into(),
        (value, false) => format!("Delay: {value}s"),
        (value, true) => format!("หน่วง: {value} วิ"),
    }
}

fn scroll_mode_button_label(automatic: bool, thai: bool) -> &'static str {
    match (automatic, thai) {
        (true, false) => "Scroll: Auto",
        (false, false) => "Scroll: Manual",
        (true, true) => "เลื่อน: อัตโนมัติ",
        (false, true) => "เลื่อน: เลื่อนเอง",
    }
}

fn open_mode_button_label(tab: bool, thai: bool) -> &'static str {
    match (tab, thai) {
        (true, false) => "Open: Tab",
        (false, false) => "Open: Window",
        (true, true) => "เปิด: แท็บ",
        (false, true) => "เปิด: หน้าต่าง",
    }
}

fn draw_launcher_button(item: &DRAWITEMSTRUCT) {
    let id = item.CtlID as usize;
    let Some(command) = launcher_icon_command(id) else {
        return;
    };
    let disabled = item.itemState & ODS_DISABLED != 0;
    let pressed = item.itemState & ODS_SELECTED != 0;
    let hovered = item.itemState & ODS_HOTLIGHT != 0;
    let focused = item.itemState & ODS_FOCUS != 0;
    let dpi = unsafe { GetDpiForWindow(item.hwndItem) }.max(96);
    let visual_state = if disabled {
        VisualState::Disabled
    } else if pressed {
        VisualState::Pressed
    } else if hovered {
        VisualState::Hover
    } else if focused {
        VisualState::Focused
    } else {
        match native_icons::role(command) {
            native_icons::Role::Destructive => VisualState::Destructive,
            native_icons::Role::Primary => VisualState::Primary,
            native_icons::Role::Normal => VisualState::Normal,
        }
    };
    let contrast = native_icons::active_high_contrast_palette(visual_state);
    let background = contrast.map(|colors| colors.background).unwrap_or_else(|| {
        if matches!(visual_state, VisualState::Primary) {
            native_icons::UI_ACCENT
        } else if pressed {
            native_icons::UI_ACCENT_DARK
        } else if hovered || focused {
            native_icons::UI_SURFACE_HOVER
        } else {
            native_icons::UI_SURFACE
        }
    });
    let foreground = contrast.map(|colors| colors.foreground).unwrap_or_else(|| {
        if matches!(visual_state, VisualState::Primary | VisualState::Pressed) {
            0x00ffffff
        } else if disabled {
            0x00aaa29a
        } else {
            native_icons::UI_TEXT
        }
    });
    let border_color = contrast.map(|colors| colors.border).unwrap_or(if focused {
        native_icons::UI_ACCENT
    } else {
        native_icons::UI_BORDER
    });
    fill(item.hDC, item.rcItem, background);
    unsafe {
        let border = CreateSolidBrush(border_color);
        FrameRect(item.hDC, &item.rcItem, border);
        DeleteObject(border);
    }
    if id == WINDOW_MENU {
        unsafe {
            let pen = CreatePen(PS_SOLID, (dpi as i32 / 96).max(1), foreground);
            let previous = SelectObject(item.hDC, pen);
            let center_x = (item.rcItem.left + item.rcItem.right) / 2;
            let center_y = (item.rcItem.top + item.rcItem.bottom) / 2;
            MoveToEx(
                item.hDC,
                center_x - scale(item.hwndItem, 3),
                center_y - 1,
                null_mut(),
            );
            LineTo(item.hDC, center_x, center_y + scale(item.hwndItem, 3));
            LineTo(item.hDC, center_x + scale(item.hwndItem, 3), center_y - 1);
            SelectObject(item.hDC, previous);
            DeleteObject(pen);
        }
    } else {
        let primary = matches!(id, AREA | WINDOW | SCROLL | VIDEO);
        let mut icon_bounds = item.rcItem;
        if primary {
            icon_bounds.bottom = icon_bounds.top + scale(item.hwndItem, 29);
        }
        if contrast.is_some() || matches!(visual_state, VisualState::Primary | VisualState::Pressed)
        {
            native_icons::draw_gdi_color(
                item.hDC,
                icon_bounds,
                native_icons::icon_for(command),
                foreground,
                dpi,
                background,
            );
        } else {
            native_icons::draw_gdi(
                item.hDC,
                icon_bounds,
                native_icons::icon_for(command),
                visual_state,
                dpi,
                background,
            );
        }
        if primary {
            centered_text(
                item.hDC,
                item.hwndItem,
                &tr(match id {
                    AREA => "area",
                    WINDOW => "window",
                    SCROLL => "scroll",
                    VIDEO => "video",
                    _ => "",
                }),
                RECT {
                    left: item.rcItem.left + scale(item.hwndItem, 2),
                    top: item.rcItem.top + scale(item.hwndItem, 26),
                    right: item.rcItem.right - scale(item.hwndItem, 2),
                    bottom: item.rcItem.bottom - scale(item.hwndItem, 1),
                },
                12,
                primary_label_font_weight(),
                foreground,
            );
        }
    }
    if focused {
        unsafe {
            let mut rect = item.rcItem;
            rect.left += scale(item.hwndItem, 2);
            rect.top += scale(item.hwndItem, 2);
            rect.right -= scale(item.hwndItem, 2);
            rect.bottom -= scale(item.hwndItem, 2);
            DrawFocusRect(item.hDC, &rect);
        }
    }
}

fn draw_launcher_option_button(item: &DRAWITEMSTRUCT) {
    let id = item.CtlID as usize;
    let Some(command) = launcher_option_icon_command(id) else {
        return;
    };
    let disabled = item.itemState & ODS_DISABLED != 0;
    let pressed = item.itemState & ODS_SELECTED != 0;
    let hovered = item.itemState & ODS_HOTLIGHT != 0;
    let focused = item.itemState & ODS_FOCUS != 0;
    let state = if disabled {
        VisualState::Disabled
    } else if pressed {
        VisualState::Pressed
    } else if hovered {
        VisualState::Hover
    } else if focused {
        VisualState::Focused
    } else {
        VisualState::Normal
    };
    let contrast = native_icons::active_high_contrast_palette(state);
    let background = contrast.map(|colors| colors.background).unwrap_or_else(|| {
        if pressed || hovered || focused {
            native_icons::UI_SURFACE_HOVER
        } else {
            native_icons::UI_SURFACE
        }
    });
    let foreground = contrast
        .map(|colors| colors.foreground)
        .unwrap_or(if disabled {
            0x00aaa29a
        } else {
            native_icons::UI_TEXT
        });
    fill(item.hDC, item.rcItem, background);
    unsafe {
        let border = CreateSolidBrush(contrast.map(|colors| colors.border).unwrap_or(if focused {
            native_icons::UI_ACCENT
        } else {
            native_icons::UI_BORDER
        }));
        FrameRect(item.hDC, &item.rcItem, border);
        DeleteObject(border);
    }
    let dpi = unsafe { GetDpiForWindow(item.hwndItem) }.max(96);
    let mut icon_bounds = item.rcItem;
    icon_bounds.left += scale(item.hwndItem, 5);
    icon_bounds.right = icon_bounds.left + scale(item.hwndItem, 20);
    if contrast.is_some() {
        native_icons::draw_gdi_color(
            item.hDC,
            icon_bounds,
            native_icons::icon_for(command),
            foreground,
            dpi,
            background,
        );
    } else {
        native_icons::draw_gdi(
            item.hDC,
            icon_bounds,
            native_icons::icon_for(command),
            state,
            dpi,
            background,
        );
    }
    // Keep the full control name for accessibility; display the concise value beside its icon.
    let label = STATE.with(|state| {
        let state = state.borrow();
        let settings = &state.as_ref().unwrap().settings;
        let thai = settings.language == "th";
        match id {
            DELAY_OPTIONS => match (settings.capture_delay_seconds, thai) {
                (0, false) => "No delay".into(),
                (0, true) => "ไม่หน่วง".into(),
                (seconds, false) => format!("{seconds} sec"),
                (seconds, true) => format!("{seconds} วินาที"),
            },
            SCROLL_OPTIONS => {
                let automatic = settings.scroll_automatic;
                match (automatic, thai) {
                    (true, false) => "Auto",
                    (false, false) => "Manual",
                    (true, true) => "อัตโนมัติ",
                    (false, true) => "เลื่อนเอง",
                }
                .into()
            }
            _ => match (settings.open_mode == settings::OpenMode::Tab, thai) {
                (true, false) => "New tab",
                (false, false) => "New window",
                (true, true) => "แท็บใหม่",
                (false, true) => "หน้าต่างใหม่",
            }
            .into(),
        }
    });
    text(
        item.hDC,
        item.hwndItem,
        &label,
        RECT {
            left: item.rcItem.left + scale(item.hwndItem, 29),
            top: item.rcItem.top,
            right: item.rcItem.right - scale(item.hwndItem, 15),
            bottom: item.rcItem.bottom,
        },
        13,
        FW_NORMAL as i32,
        foreground,
    );
    unsafe {
        let pen = CreatePen(PS_SOLID, scale(item.hwndItem, 1), foreground);
        let previous = SelectObject(item.hDC, pen);
        let x = item.rcItem.right - scale(item.hwndItem, 9);
        let y = (item.rcItem.top + item.rcItem.bottom) / 2;
        MoveToEx(
            item.hDC,
            x - scale(item.hwndItem, 3),
            y - scale(item.hwndItem, 2),
            null_mut(),
        );
        LineTo(item.hDC, x, y + scale(item.hwndItem, 2));
        LineTo(
            item.hDC,
            x + scale(item.hwndItem, 3),
            y - scale(item.hwndItem, 2),
        );
        SelectObject(item.hDC, previous);
        DeleteObject(pen);
    }
    if focused {
        unsafe {
            let mut rect = item.rcItem;
            rect.left += scale(item.hwndItem, 2);
            rect.top += scale(item.hwndItem, 2);
            rect.right -= scale(item.hwndItem, 2);
            rect.bottom -= scale(item.hwndItem, 2);
            DrawFocusRect(item.hDC, &rect);
        }
    }
}

fn draw_editor_work_button(item: &DRAWITEMSTRUCT) {
    let command = if item.CtlID as usize == COPY {
        IconCommand::Copy
    } else {
        IconCommand::SavePng
    };
    let disabled = item.itemState & ODS_DISABLED != 0;
    let pressed = item.itemState & ODS_SELECTED != 0;
    let hovered = item.itemState & ODS_HOTLIGHT != 0;
    let focused = item.itemState & ODS_FOCUS != 0;
    let primary = command == IconCommand::SavePng;
    let state = if disabled {
        VisualState::Disabled
    } else if pressed {
        VisualState::Pressed
    } else if hovered {
        VisualState::Hover
    } else if focused {
        VisualState::Focused
    } else if primary {
        VisualState::Primary
    } else {
        VisualState::Normal
    };
    let contrast = native_icons::active_high_contrast_palette(state);
    let background = if disabled {
        native_icons::UI_SURFACE_ALT
    } else if pressed {
        if primary {
            native_icons::UI_ACCENT_DARK
        } else {
            native_icons::UI_SURFACE_HOVER
        }
    } else if primary {
        native_icons::UI_ACCENT
    } else if hovered || focused {
        native_icons::UI_SURFACE_HOVER
    } else {
        native_icons::UI_SURFACE
    };
    let background = contrast
        .map(|colors| colors.background)
        .unwrap_or(background);
    let foreground = contrast.map(|colors| colors.foreground);
    fill(item.hDC, item.rcItem, background);
    unsafe {
        let border = CreateSolidBrush(contrast.map(|colors| colors.border).unwrap_or(if primary {
            native_icons::UI_ACCENT
        } else {
            native_icons::UI_BORDER
        }));
        FrameRect(item.hDC, &item.rcItem, border);
        DeleteObject(border);
    }
    let dpi = unsafe { GetDpiForWindow(item.hwndItem) }.max(96);
    let mut icon_bounds = item.rcItem;
    if primary {
        icon_bounds.right = icon_bounds.left + scale(item.hwndItem, 34);
    }
    if let Some(foreground) = foreground.or(primary.then_some(0x00ffffff)) {
        native_icons::draw_gdi_color(
            item.hDC,
            icon_bounds,
            native_icons::icon_for(command),
            foreground,
            dpi,
            background,
        );
    } else {
        native_icons::draw_gdi(
            item.hDC,
            icon_bounds,
            native_icons::icon_for(command),
            state,
            dpi,
            background,
        );
    }
    if primary {
        text(
            item.hDC,
            item.hwndItem,
            if STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th") {
                "PNG"
            } else {
                "Save PNG"
            },
            RECT {
                left: item.rcItem.left + scale(item.hwndItem, 34),
                top: item.rcItem.top,
                right: item.rcItem.right - scale(item.hwndItem, 4),
                bottom: item.rcItem.bottom,
            },
            12,
            FW_SEMIBOLD as i32,
            foreground.unwrap_or(if disabled { 0x00aaa29a } else { 0x00ffffff }),
        );
    }
    if focused {
        unsafe {
            let mut rect = item.rcItem;
            rect.left += scale(item.hwndItem, 2);
            rect.top += scale(item.hwndItem, 2);
            rect.right -= scale(item.hwndItem, 2);
            rect.bottom -= scale(item.hwndItem, 2);
            DrawFocusRect(item.hDC, &rect);
        }
    }
}

fn text(dc: HDC, window: HWND, value: &str, rect: RECT, size: i32, weight: i32, color: u32) {
    draw_text(
        dc,
        window,
        value,
        rect,
        size,
        weight,
        color,
        DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
    );
}

fn centered_text(
    dc: HDC,
    window: HWND,
    value: &str,
    rect: RECT,
    size: i32,
    weight: i32,
    color: u32,
) {
    draw_text(
        dc,
        window,
        value,
        rect,
        size,
        weight,
        color,
        primary_label_text_flags(),
    );
}

fn draw_text(
    dc: HDC,
    window: HWND,
    value: &str,
    rect: RECT,
    size: i32,
    weight: i32,
    color: u32,
    flags: u32,
) {
    unsafe {
        let old = SelectObject(dc, font(window, size, weight));
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, color);
        let mut rect = rect;
        let value = wide(value);
        DrawTextW(
            dc,
            value.as_ptr(),
            (value.len() - 1) as i32,
            &mut rect,
            flags,
        );
        SelectObject(dc, old);
    }
}

fn draw_frame(dc: HDC, frame: &Frame, rect: RECT) {
    if rect.right <= rect.left || rect.bottom <= rect.top {
        return;
    }
    let mut info: BITMAPINFO = unsafe { zeroed() };
    info.bmiHeader = capture::header(frame.width, frame.height);
    unsafe {
        SetStretchBltMode(
            dc,
            if rect.right - rect.left == frame.width as i32
                && rect.bottom - rect.top == frame.height as i32
            {
                COLORONCOLOR
            } else {
                HALFTONE
            },
        );
        SetBrushOrgEx(dc, 0, 0, null_mut());
        StretchDIBits(
            dc,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            0,
            0,
            frame.width as i32,
            frame.height as i32,
            frame.bgra.as_ptr() as *const c_void,
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
    }
}

fn paint(window: HWND) {
    let mut paint: PAINTSTRUCT = unsafe { zeroed() };
    let target = unsafe { BeginPaint(window, &mut paint) };
    if target.is_null() {
        return;
    }
    let bounds = client(window);
    let class = class_name(window);
    let mut buffer = if class == PREVIEW && bounds.right > 0 && bounds.bottom > 0 {
        // Native paints are serialized; share one display surface, not one per captured image.
        let existing = STATE.with(|s| s.borrow_mut().as_mut().unwrap().back_buffer.take());
        existing
            .filter(|b| b.width == bounds.right as u32 && b.height == bounds.bottom as u32)
            .or_else(|| BackBuffer::new(target, bounds.right as u32, bounds.bottom as u32).ok())
    } else {
        None
    };
    let dc = buffer.as_ref().map_or(target, |b| b.dc);
    let started = if class == MAIN {
        let palette = native_icons::active_high_contrast_palette(VisualState::Normal);
        fill(
            dc,
            bounds,
            palette.map_or(native_icons::UI_SURFACE, |p| p.background),
        );
        let icon = STATE.with(|s| s.borrow().as_ref().unwrap().icon);
        unsafe {
            DrawIconEx(
                dc,
                scale(window, 8),
                scale(window, 5),
                icon as HICON,
                scale(window, 24),
                scale(window, 24),
                0,
                null_mut(),
                DI_NORMAL,
            );
        }
        text(
            dc,
            window,
            "SnapZy",
            RECT {
                left: scale(window, 40),
                top: scale(window, 1),
                right: scale(window, 138),
                bottom: scale(window, 20),
            },
            15,
            FW_BOLD as i32,
            palette.map_or(native_icons::UI_TEXT, |p| p.foreground),
        );
        text(
            dc,
            window,
            SUBTITLE,
            RECT {
                left: scale(window, 40),
                top: scale(window, 18),
                right: scale(window, 138),
                bottom: scale(window, 32),
            },
            11,
            FW_NORMAL as i32,
            palette.map_or(native_icons::UI_MUTED, |p| p.foreground),
        );
        STATE
            .with(|s| s.borrow_mut().as_mut().unwrap().first_paint.take())
            .map(|clock| ("launcher.first-paint", clock))
    } else if class == PREVIEW {
        let palette = native_icons::active_high_contrast_palette(VisualState::Normal);
        fill(
            dc,
            bounds,
            palette.map_or(native_icons::UI_SURFACE_ALT, |p| p.background),
        );
        fill(
            dc,
            RECT {
                left: 0,
                top: 0,
                right: bounds.right,
                bottom: scale(window, 104),
            },
            palette.map_or(native_icons::UI_SURFACE, |p| p.background),
        );
        let displayed = STATE.with(|s| {
            s.borrow()
                .as_ref()
                .unwrap()
                .previews
                .get(&(window as usize))
                .map(|p| {
                    p.cached
                        .as_ref()
                        .map(|(_, f)| f.clone())
                        .unwrap_or_else(|| p.frame.clone())
                })
        });
        if let Some(frame) = displayed {
            let crop = STATE.with(|s| {
                s.borrow().as_ref().unwrap().previews[&(window as usize)]
                    .session
                    .document
                    .crop
            });
            let view = editor_view(window, crop);
            draw_frame(
                dc,
                &frame,
                RECT {
                    left: view.x.round() as i32,
                    top: view.y.round() as i32,
                    right: (view.x + crop.width as f64 * view.scale).round() as i32,
                    bottom: (view.y + crop.height as f64 * view.scale).round() as i32,
                },
            );
            draw_editor_overlay(window, dc, view, buffer.as_mut());
        } else {
            status(window, "Cannot render this image.");
        }
        STATE
            .with(|s| {
                s.borrow_mut()
                    .as_mut()
                    .unwrap()
                    .previews
                    .get_mut(&(window as usize))
                    .and_then(|p| p.first_paint.take())
            })
            .map(|clock| ("capture.preview-first-paint", clock))
    } else if class == OVERLAY {
        let snapshot = STATE.with(|s| {
            let mut b = s.borrow_mut();
            let a = b.as_mut().unwrap();
            a.selection.as_mut().map(|p| {
                (
                    p.frame.clone(),
                    p.desktop,
                    selection_rect(p),
                    p.first_paint.take(),
                    p.detected_pane
                        .as_ref()
                        .is_some_and(|d| d.source == scroll_detect::Source::Window),
                )
            })
        });
        if let Some((frame, desktop, selection, clock, window_fallback)) = snapshot {
            draw_frame(dc, &frame, bounds);
            if let Some(rect) = selection {
                unsafe {
                    let pen = CreatePen(
                        PS_SOLID,
                        3,
                        if window_fallback {
                            0x0030b5ed
                        } else {
                            0x00f36517
                        },
                    );
                    let old_pen = SelectObject(dc, pen);
                    let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
                    Rectangle(
                        dc,
                        rect.x - desktop.x,
                        rect.y - desktop.y,
                        rect.right() - desktop.x,
                        rect.bottom() - desktop.y,
                    );
                    SelectObject(dc, old_brush);
                    SelectObject(dc, old_pen);
                    DeleteObject(pen);
                }
            }
            clock.map(|clock| ("capture.selection-first-paint", clock))
        } else {
            None
        }
    } else {
        fill(dc, bounds, 0x00ffffff);
        None
    };
    if let Some(buffer) = buffer {
        if !buffer.present(target) {
            status(window, "Cannot display the editor image.");
        }
        STATE.with(|s| {
            s.borrow_mut().as_mut().unwrap().back_buffer = Some(buffer);
        });
    }
    unsafe {
        EndPaint(window, &paint);
    }
    if let Some((stage, clock)) = started {
        trace::record(stage, clock);
    }
}

unsafe extern "system" fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        if !STATE.with(|s| s.borrow().is_some()) {
            return DefWindowProcW(window, message, wparam, lparam);
        }
        if class_name(window) == video_ui::CLASS {
            return video_ui::procedure(window, message, wparam, lparam);
        }
        match message {
            WM_ACTIVATE if class_name(window) == workspace_ui::HOST => {
                if wparam & 0xffff != WA_INACTIVE as usize {
                    workspace_ui::activate(window);
                }
                DefWindowProcW(window, message, wparam, lparam)
            }
            WM_SETFOCUS if class_name(window) == workspace_ui::HOST => {
                if let Some(doc) = workspace_ui::active(window) {
                    SetFocus(doc);
                }
                0
            }
            WM_NOTIFY if class_name(window) == workspace_ui::HOST => {
                workspace_ui::notification(lparam)
            }
            WM_DRAWITEM if class_name(window) == workspace_ui::HOST => {
                workspace_ui::draw(&*(lparam as *const DRAWITEMSTRUCT));
                1
            }
            WM_PAINT if class_name(window) == workspace_ui::HOST => {
                let mut ps: PAINTSTRUCT = zeroed();
                let dc = BeginPaint(window, &mut ps);
                FillRect(dc, &ps.rcPaint, GetSysColorBrush(COLOR_WINDOW));
                EndPaint(window, &ps);
                0
            }
            WM_DRAWITEM if [PREVIEW, MAIN].contains(&class_name(window).as_str()) => {
                let item = &*(lparam as *const DRAWITEMSTRUCT);
                if class_name(window) == MAIN
                    && launcher_icon_command(item.CtlID as usize).is_some()
                {
                    draw_launcher_button(item);
                    return 1;
                }
                if class_name(window) == MAIN
                    && launcher_option_icon_command(item.CtlID as usize).is_some()
                {
                    draw_launcher_option_button(item);
                    return 1;
                }
                if class_name(window) == PREVIEW && matches!(item.CtlID as usize, COPY | SAVE) {
                    draw_editor_work_button(item);
                    return 1;
                }
                let state = STATE.with(|s| {
                    s.borrow()
                        .as_ref()
                        .unwrap()
                        .previews
                        .get(&(window as usize))
                        .map(|p| {
                            let mut style = p.session.style;
                            if item.CtlID as usize == ec::BACK_COLOR {
                                style.rgb = p.session.text_background().rgb;
                            } else if item.CtlID as usize == ec::BORDER_COLOR {
                                style = p.session.callout_border();
                            }
                            (p.session.tool, style, p.session.shape_choices())
                        })
                });
                let state = state.or_else(|| {
                    (class_name(window) == MAIN && item.CtlID as usize == ec::PROJECT_OPEN)
                        .then_some((
                            Tool::Select,
                            crate::editor::Style::default(),
                            [Tool::Rectangle, Tool::FilledRectangle],
                        ))
                });
                if let Some((tool, style, shapes)) = state {
                    ec::draw(item, tool, style, shapes);
                }
                1
            }
            WM_PAINT => {
                paint(window);
                0
            }
            WM_ERASEBKGND => 1,
            WM_COMMAND => {
                if class_name(window) == TEXT_ENTRY {
                    if (wparam >> 16) == BN_CLICKED as usize {
                        finish_text_entry(window, (wparam & 0xffff) == 1);
                    }
                    return 0;
                }
                if class_name(window) == OCR_WINDOW {
                    if (wparam >> 16) == BN_CLICKED as usize {
                        PostMessageW(window, COMMAND, wparam & 0xffff, 0);
                    }
                    return 0;
                }
                if matches!(wparam & 0xffff, ec::WIDTH | ec::STROKE | ec::OPACITY)
                    && (wparam >> 16) == EN_KILLFOCUS as usize
                    && class_name(window) == PREVIEW
                {
                    editor_command(window, wparam & 0xffff);
                    return 0;
                }
                if (wparam >> 16) == BN_CLICKED as usize {
                    if class_name(window) == PREVIEW {
                        handle_command(window, wparam & 0xffff);
                    } else {
                        PostMessageW(window, COMMAND, wparam & 0xffff, 0);
                    }
                }
                0
            }
            COMMAND => {
                handle_command(window, wparam);
                0
            }
            COMPLETE => {
                complete_work();
                0
            }
            WM_TIMER if wparam == CAPTURE_TIMER => {
                perform_capture();
                0
            }
            WM_TIMER if wparam == VIDEO_TIMER => {
                poll_video();
                0
            }
            WM_MOUSEACTIVATE if class_name(window) == SCROLL_PANEL => MA_NOACTIVATE as isize,
            WM_MOUSEACTIVATE
                if class_name(window) == OVERLAY
                    && STATE.with(|s| {
                        s.borrow()
                            .as_ref()
                            .and_then(|a| a.selection.as_ref())
                            .is_some_and(|p| matches!(p.mode, Mode::Scroll))
                    }) =>
            {
                MA_NOACTIVATE as isize
            }
            WM_TIMER if wparam == SCROLL_HOVER_TIMER => {
                poll_scroll_hover();
                0
            }
            WM_TIMER if wparam == SCROLL_TIMER => {
                poll_scroll();
                0
            }
            WM_CLOSE => {
                PostMessageW(window, CLOSE, 0, 0);
                0
            }
            CLOSE => {
                if wparam != 0
                    && STATE.with(|s| {
                        s.borrow()
                            .as_ref()
                            .and_then(|a| workspace_ui::doc_key(a, window as usize))
                            .is_none_or(|key| key.identity != wparam as u64)
                    })
                {
                    return 0;
                }
                close_window(window);
                0
            }
            WM_SIZE => {
                if STATE.with(|s| s.borrow().as_ref().is_some_and(|a| a.main != 0)) {
                    layout(window);
                }
                0
            }
            WM_DPICHANGED => {
                let rect = *(lparam as *const RECT);
                SetWindowPos(
                    window,
                    null_mut(),
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                layout(window);
                0
            }
            WM_GETMINMAXINFO => {
                let limits = &mut *(lparam as *mut MINMAXINFO);
                let editor = [PREVIEW, workspace_ui::HOST].contains(&class_name(window).as_str());
                let ocr = class_name(window) == OCR_WINDOW;
                let panel = [SCROLL_PANEL, VIDEO_PANEL].contains(&class_name(window).as_str());
                let launcher = class_name(window) == MAIN;
                limits.ptMinTrackSize.x = scale(
                    window,
                    if editor {
                        780
                    } else if ocr {
                        460
                    } else if panel {
                        360
                    } else if launcher {
                        360
                    } else if class_name(window) == HOTKEYS {
                        560
                    } else {
                        400
                    },
                );
                limits.ptMinTrackSize.y = scale(
                    window,
                    if editor {
                        if class_name(window) == workspace_ui::HOST {
                            456
                        } else {
                            420
                        }
                    } else if ocr {
                        340
                    } else if panel {
                        140
                    } else if launcher {
                        140
                    } else if class_name(window) == HOTKEYS {
                        350
                    } else {
                        240
                    },
                );
                0
            }
            WM_HOTKEY => {
                if wparam == SCROLL_ESCAPE_HOTKEY as usize {
                    match class_name(window).as_str() {
                        OVERLAY => finish_selection(true),
                        SCROLL_PANEL => stop_scroll(),
                        _ => {}
                    }
                    return 0;
                }
                if wparam == VIDEO_ESCAPE_HOTKEY as usize {
                    stop_video();
                    return 0;
                }
                if class_name(window) == MAIN {
                    let action = STATE.with(|s| {
                        s.borrow().as_ref().and_then(|app| {
                            app.active_hotkeys
                                .iter()
                                .find(|(_, id)| *id as usize == wparam)
                                .map(|(action, _)| *action)
                        })
                    });
                    match action {
                        Some(HotkeyAction::Area) => start_capture(Mode::Area),
                        Some(HotkeyAction::Window) => start_capture(Mode::Window),
                        Some(HotkeyAction::Desktop) => start_capture(Mode::Desktop),
                        Some(HotkeyAction::Scrolling) => start_capture(Mode::Scroll),
                        Some(HotkeyAction::Video) => show_video_menu(window),
                        Some(HotkeyAction::Delay) => show_delay_menu(window),
                        None => {}
                    }
                }
                0
            }
            TRAY => {
                if lparam as u32 == WM_LBUTTONUP {
                    show_launcher();
                } else if lparam as u32 == WM_RBUTTONUP {
                    let menu = CreatePopupMenu();
                    AppendMenuW(menu, MF_STRING, AREA, wide(&tr("area")).as_ptr());
                    AppendMenuW(menu, MF_STRING, WINDOW, wide(&tr("window")).as_ptr());
                    AppendMenuW(menu, MF_STRING, DESKTOP, wide(&tr("desktop")).as_ptr());
                    AppendMenuW(menu, MF_STRING, SCROLL, wide(&tr("scroll")).as_ptr());
                    if STATE.with(|s| s.borrow().as_ref().unwrap().recording.is_some()) {
                        AppendMenuW(
                            menu,
                            MF_STRING,
                            VIDEO_STOP,
                            wide(&tr("video_stop")).as_ptr(),
                        );
                    } else {
                        AppendMenuW(menu, MF_STRING, VIDEO, wide(&tr("video")).as_ptr());
                    }
                    AppendMenuW(menu, MF_SEPARATOR, 0, null());
                    AppendMenuW(menu, MF_STRING, EXIT, wide(&tr("exit")).as_ptr());
                    let mut cursor = zeroed();
                    GetCursorPos(&mut cursor);
                    SetForegroundWindow(window);
                    let command = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_NONOTIFY,
                        cursor.x,
                        cursor.y,
                        0,
                        window,
                        null(),
                    );
                    DestroyMenu(menu);
                    PostMessageW(window, WM_NULL, 0, 0);
                    if command != 0 {
                        PostMessageW(window, COMMAND, command as usize, 0);
                    }
                }
                0
            }
            WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP if class_name(window) == OVERLAY => {
                let x = (lparam as u32 & 0xffff) as u16 as i16 as i32;
                let y = ((lparam as u32 >> 16) & 0xffff) as u16 as i16 as i32;
                let scrolling = STATE.with(|state| {
                    state
                        .borrow()
                        .as_ref()
                        .and_then(|app| app.selection.as_ref())
                        .is_some_and(|selection| matches!(selection.mode, Mode::Scroll))
                });
                if scrolling {
                    if message == WM_LBUTTONDOWN {
                        let accepted = STATE
                            .with(|state| {
                                let mut state = state.borrow_mut();
                                let selection = state.as_mut()?.selection.as_mut()?;
                                selection.start = Some((x, y));
                                let point = (selection.desktop.x + x, selection.desktop.y + y);
                                selection.pressed_pane = selection
                                    .detected_pane
                                    .as_ref()
                                    .filter(|p| p.pane.bounds.contains(point.0, point.1))
                                    .map(|p| p.pane.clone());
                                Some(())
                            })
                            .is_some();
                        if accepted {
                            SetCapture(window);
                        }
                    } else if message == WM_LBUTTONUP {
                        let click = STATE.with(|state| {
                            let mut state = state.borrow_mut();
                            let selection = state.as_mut()?.selection.as_mut()?;
                            selection.start.take()?;
                            selection.pressed_pane.take().map(|pane| {
                                ((selection.desktop.x + x, selection.desktop.y + y), pane)
                            })
                        });
                        ReleaseCapture();
                        if let Some(click) = click {
                            trace::record("scroll.detect.click-confirmed", Instant::now());
                            confirm_scroll_selection(Some(click));
                        }
                    } else {
                        STATE.with(|state| {
                            if let Some(selection) = state
                                .borrow_mut()
                                .as_mut()
                                .and_then(|a| a.selection.as_mut())
                            {
                                selection.end = (x, y);
                            }
                        });
                        let origin = STATE.with(|state| {
                            state
                                .borrow()
                                .as_ref()
                                .and_then(|a| a.selection.as_ref())
                                .map(|s| (s.desktop.x, s.desktop.y))
                        });
                        if let Some((dx, dy)) = origin {
                            update_scroll_hover((dx + x, dy + y));
                        }
                    }
                    return 0;
                }
                STATE.with(|s| {
                    let mut b = s.borrow_mut();
                    if let Some(p) = &mut b.as_mut().unwrap().selection {
                        p.end = (
                            x.clamp(0, p.desktop.width as i32),
                            y.clamp(0, p.desktop.height as i32),
                        );
                        if message == WM_LBUTTONDOWN {
                            p.start = Some(p.end);
                        }
                    }
                });
                if message == WM_LBUTTONDOWN {
                    SetCapture(window);
                }
                if message == WM_LBUTTONUP {
                    finish_selection(false);
                } else {
                    InvalidateRect(window, null(), 0);
                }
                0
            }
            WM_CAPTURECHANGED | WM_CANCELMODE if class_name(window) == OVERLAY => {
                STATE.with(|state| {
                    if let Some(selection) = state
                        .borrow_mut()
                        .as_mut()
                        .and_then(|a| a.selection.as_mut())
                    {
                        selection.start = None;
                        selection.pressed_pane = None;
                    }
                });
                if message == WM_CANCELMODE {
                    ReleaseCapture();
                }
                0
            }
            WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_LBUTTONDBLCLK
                if class_name(window) == PREVIEW =>
            {
                let x = (lparam as u32 & 0xffff) as u16 as i16 as i32;
                let y = ((lparam as u32 >> 16) & 0xffff) as u16 as i16 as i32;
                editor_mouse(window, message, x, y);
                0
            }
            WM_CAPTURECHANGED | WM_CANCELMODE if class_name(window) == PREVIEW => {
                let unfinished = STATE.with(|s| {
                    s.borrow()
                        .as_ref()
                        .unwrap()
                        .previews
                        .get(&(window as usize))
                        .is_some_and(|p| {
                            p.session.resize.is_some()
                                || p.session.shape_resize.is_some()
                                || p.session.curve_drag.is_some()
                                || p.session.tail_drag.is_some()
                                || p.session.gesture.as_ref().is_some_and(|g| !g.ready)
                        })
                });
                if unfinished {
                    cancel_editor(window);
                }
                0
            }
            WM_SETCURSOR
                if class_name(window) == PREVIEW && (lparam as u32 & 0xffff) == HTCLIENT =>
            {
                let cursor = STATE.with(|s| {
                    let b = s.borrow();
                    let p = b.as_ref().unwrap().previews.get(&(window as usize))?;
                    let mut point: POINT = zeroed();
                    GetCursorPos(&mut point);
                    ScreenToClient(window, &mut point);
                    let view = editor_view(window, p.session.document.crop);
                    let raw = Point::new(
                        (point.x as f64 - view.x) / view.scale + view.crop.x as f64,
                        (point.y as f64 - view.y) / view.scale + view.crop.y as f64,
                    );
                    let radius = 5. * GetDpiForWindow(window) as f64 / 96. / view.scale;
                    let text_cursor = p
                        .session
                        .resize
                        .as_ref()
                        .map(|r| r.handle)
                        .or_else(|| p.session.resize_handle(raw, radius));
                    text_cursor
                        .map(|handle| match handle {
                            crate::text_frame::Handle::Left | crate::text_frame::Handle::Right => {
                                IDC_SIZEWE
                            }
                            crate::text_frame::Handle::TopLeft
                            | crate::text_frame::Handle::BottomRight => IDC_SIZENWSE,
                            _ => IDC_SIZENESW,
                        })
                        .or_else(|| {
                            p.session
                                .shape_resize
                                .as_ref()
                                .map(|r| r.handle)
                                .or_else(|| p.session.shape_handle(raw, radius))
                                .map(|handle| match handle {
                                    crate::shape_frame::Handle::Left
                                    | crate::shape_frame::Handle::Right => IDC_SIZEWE,
                                    crate::shape_frame::Handle::Top
                                    | crate::shape_frame::Handle::Bottom => IDC_SIZENS,
                                    crate::shape_frame::Handle::TopLeft
                                    | crate::shape_frame::Handle::BottomRight => IDC_SIZENWSE,
                                    _ => IDC_SIZENESW,
                                })
                        })
                });
                let name = cursor.unwrap_or(IDC_ARROW);
                SetCursor(LoadCursorW(null_mut(), name));
                1
            }
            WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
                if class_name(window) == OVERLAY {
                    finish_selection(true);
                } else if class_name(window) == HOTKEYS {
                    close_window(window);
                } else if class_name(window) == SCROLL_PANEL {
                    stop_scroll();
                } else if class_name(window) == VIDEO_PANEL {
                    stop_video();
                } else if class_name(window) == PREVIEW {
                    cancel_editor(window);
                }
                0
            }
            WM_KEYDOWN if class_name(window) == PREVIEW && wparam == VK_RETURN as usize => {
                editor_command(window, ec::CONFIRM);
                0
            }
            WM_KEYDOWN if class_name(window) == PREVIEW && wparam == VK_DELETE as usize => {
                editor_command(window, ec::DELETE);
                0
            }
            WM_CTLCOLORSTATIC => {
                SetBkMode(wparam as HDC, TRANSPARENT as i32);
                if let Some(palette) =
                    native_icons::active_high_contrast_palette(VisualState::Normal)
                {
                    SetTextColor(wparam as HDC, palette.foreground);
                    SetDCBrushColor(wparam as HDC, palette.background);
                } else {
                    SetTextColor(wparam as HDC, native_icons::UI_TEXT);
                    SetDCBrushColor(
                        wparam as HDC,
                        if [MAIN, PREVIEW].contains(&class_name(window).as_str()) {
                            native_icons::UI_SURFACE
                        } else {
                            0x00ffffff
                        },
                    );
                }
                GetStockObject(DC_BRUSH) as isize
            }
            _ => {
                let taskbar = STATE.with(|s| s.borrow().as_ref().unwrap().taskbar_message);
                if message == taskbar {
                    let added = tray(true);
                    STATE.with(|s| s.borrow_mut().as_mut().unwrap().tray_added = added);
                    if !added {
                        show_launcher();
                    }
                    return 0;
                }
                DefWindowProcW(window, message, wparam, lparam)
            }
        }
    }
}

pub fn run(start_in_tray: bool) -> Result<(), String> {
    let started = Instant::now();
    unsafe {
        let mutex = CreateMutexW(
            null(),
            0,
            wide(&settings::instance_mutex_name(
                settings::is_test_run(),
                std::process::id(),
            ))
            .as_ptr(),
        );
        if mutex.is_null() {
            return Err("Cannot create the application instance guard.".into());
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let existing = FindWindowW(wide(MAIN).as_ptr(), null());
            if !existing.is_null() {
                ShowWindow(existing, SW_RESTORE);
                SetForegroundWindow(existing);
            }
            CloseHandle(mutex);
            return Ok(());
        }
        let module = GetModuleHandleW(null());
        let icon = LoadImageW(module, 1usize as *const u16, IMAGE_ICON, 64, 64, LR_SHARED) as usize;
        let (tx, rx) = mpsc::channel();
        let (detector, detection_rx) = DetectionService::new();
        STATE.with(|s| {
            *s.borrow_mut() = Some(App {
                main: 0,
                settings: Settings::load(),
                settings_window: 0,
                previews: HashMap::new(),
                workspace: crate::workspace::Workspace::default(),
                tab_tips: HashMap::new(),
                capture_route: None,
                selection: None,
                scrolling: None,
                recording: None,
                video_previews: HashMap::new(),
                video_exit: None,
                exit_after_video_exports: false,
                pending: None,
                capture_busy: false,
                restore: Vec::new(),
                saving: 0,
                composing: false,
                importing: false,
                next_preview: 0,
                project_opening: false,
                project_reserved: 0,
                combine_assets: vec![],
                project_modal: false,
                project_exit: None,
                next_detection_identity: 0,
                next_video_session: 0,
                exit_after_video: false,
                text_entry: None,
                effects_open: false,
                ocr_windows: HashMap::new(),
                ocr_busy: false,
                next_ocr_identity: 0,
                tx,
                rx,
                detector,
                detection_rx,
                font_cache: HashMap::new(),
                back_buffer: None,
                icon,
                tray_added: false,
                active_hotkeys: Vec::new(),
                taskbar_message: RegisterWindowMessageW(wide(TASKBAR_CREATED).as_ptr()),
                first_paint: Some(started),
                launcher_tips: None,
            })
        });
        for name in [
            MAIN,
            workspace_ui::HOST,
            PREVIEW,
            OVERLAY,
            HOTKEYS,
            TEXT_ENTRY,
            SCROLL_PANEL,
            VIDEO_PANEL,
            video_ui::CLASS,
            OCR_WINDOW,
        ] {
            let class_name = wide(name);
            let class = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW | if name == PREVIEW { CS_DBLCLKS } else { 0 },
                lpfnWndProc: Some(procedure),
                hInstance: module,
                hIcon: icon as HICON,
                hIconSm: icon as HICON,
                hCursor: LoadCursorW(
                    null_mut(),
                    if name == OVERLAY {
                        IDC_CROSS
                    } else {
                        IDC_ARROW
                    },
                ),
                lpszClassName: class_name.as_ptr(),
                ..zeroed()
            };
            if RegisterClassExW(&class) == 0 {
                CloseHandle(mutex);
                return Err(format!("Cannot register native window class ({name})."));
            }
        }
        let main = create_window(
            MAIN,
            TITLE,
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            0,
            null_mut(),
            360,
            140,
        );
        if main.is_null() {
            CloseHandle(mutex);
            return Err("Cannot create the launcher.".into());
        }
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().main = main as usize);
        for (id, key) in [
            (AREA, "area"),
            (WINDOW, "window"),
            (SCROLL, "scroll"),
            (VIDEO, "video"),
        ] {
            child(
                main,
                "BUTTON",
                &tr(key),
                id as i32,
                WS_TABSTOP | BS_OWNERDRAW as u32,
            );
        }
        child(
            main,
            "BUTTON",
            "",
            WINDOW_MENU as i32,
            WS_TABSTOP | BS_OWNERDRAW as u32,
        );
        child(main, "BUTTON", "EN", LANGUAGE as i32, WS_TABSTOP);
        child(
            main,
            "BUTTON",
            "",
            MORE as i32,
            WS_TABSTOP | BS_OWNERDRAW as u32,
        );
        child(
            main,
            "BUTTON",
            ec::label(ec::PROJECT_OPEN, false),
            ec::PROJECT_OPEN as i32,
            WS_TABSTOP | BS_OWNERDRAW as u32,
        );
        child(main, "STATIC", &tr("ready"), STATUS, 0);
        child(
            main,
            "BUTTON",
            "Scrolling: Automatic",
            SCROLL_OPTIONS as i32,
            WS_TABSTOP | BS_OWNERDRAW as u32,
        );
        child(
            main,
            "BUTTON",
            "Delay: None",
            DELAY_OPTIONS as i32,
            WS_TABSTOP | BS_OWNERDRAW as u32,
        );
        child(
            main,
            "BUTTON",
            "Open: New tab",
            OPEN_OPTIONS as i32,
            WS_TABSTOP | BS_OWNERDRAW as u32,
        );
        let launcher_tips = native_tooltips::Tips::new(
            main,
            &[
                TooltipEntry::new(AREA, IconCommand::AreaCapture),
                TooltipEntry::new(WINDOW, IconCommand::WindowCapture),
                TooltipEntry::new(WINDOW_MENU, IconCommand::DesktopCapture),
                TooltipEntry::new(SCROLL, IconCommand::ScrollingCapture),
                TooltipEntry::new(VIDEO, IconCommand::VideoCapture),
                TooltipEntry::new(ec::PROJECT_OPEN, IconCommand::OpenImage),
                TooltipEntry::new(LANGUAGE, IconCommand::Language),
                TooltipEntry::new(MORE, IconCommand::More),
                TooltipEntry::new(DELAY_OPTIONS, IconCommand::Delay),
                TooltipEntry::new(SCROLL_OPTIONS, IconCommand::ScrollingMode),
                TooltipEntry::new(OPEN_OPTIONS, IconCommand::OpenDestination),
            ],
            STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th"),
        );
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().launcher_tips = launcher_tips);
        layout(main);
        refresh_language();
        let added = tray(true);
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().tray_added = added);
        if !added {
            EnableWindow(GetDlgItem(main, HIDE as i32), 0);
        }
        let settings = STATE.with(|s| s.borrow().as_ref().unwrap().settings.clone());
        match register_hotkeys(main, &settings) {
            Ok(active) => {
                STATE.with(|s| s.borrow_mut().as_mut().unwrap().active_hotkeys = active);
            }
            Err(error) => notify(main, &error, true),
        }
        if !start_in_tray || !added {
            ShowWindow(main, SW_SHOW);
            UpdateWindow(main);
            trace::record("launcher.ready", started);
        } else {
            STATE.with(|s| s.borrow_mut().as_mut().unwrap().first_paint = None);
            trace::record("tray.ready", started);
        }
        let mut message: MSG = zeroed();
        loop {
            let result = GetMessageW(&mut message, null_mut(), 0, 0);
            if result <= 0 {
                break;
            }
            let top = GetAncestor(message.hwnd, GA_ROOT);
            if class_name(top) == workspace_ui::HOST
                && message.message == WM_KEYDOWN
                && GetKeyState(VK_CONTROL as i32) < 0
            {
                if message.wParam == VK_TAB as usize {
                    workspace_ui::cycle(top, GetKeyState(VK_SHIFT as i32) < 0);
                    continue;
                }
                if message.wParam == 0x57 {
                    if workspace_ui::can_switch(top) {
                        if let Some(doc) = workspace_ui::active(top) {
                            close_window(doc);
                        }
                    }
                    continue;
                }
            }
            let root = if class_name(top) == workspace_ui::HOST {
                workspace_ui::active(top).unwrap_or(top)
            } else {
                top
            };
            let root_class = class_name(root);
            if root_class == video_ui::CLASS && message.message == WM_KEYDOWN {
                if message.wParam == VK_ESCAPE as usize {
                    video_ui::close(root);
                    continue;
                }
            }
            if root_class == TEXT_ENTRY {
                if message.message == WM_KEYDOWN && message.wParam == VK_ESCAPE as usize {
                    finish_text_entry(root, false);
                    continue;
                }
                if IsDialogMessageW(root, &message) != 0 {
                    continue;
                }
            }
            if message.message == WM_KEYDOWN && message.wParam == VK_ESCAPE as usize {
                if root_class == OVERLAY {
                    finish_selection(true);
                    continue;
                }
                if root_class == HOTKEYS {
                    close_window(root);
                    continue;
                }
                if root_class == OCR_WINDOW {
                    close_window(root);
                    continue;
                }
                if root_class == SCROLL_PANEL {
                    stop_scroll();
                    continue;
                }
                if root_class == VIDEO_PANEL {
                    stop_video();
                    continue;
                }
                if root_class == PREVIEW {
                    cancel_editor(root);
                    continue;
                }
            }
            if message.message == WM_KEYDOWN
                && root_class == PREVIEW
                && class_name(message.hwnd) != "Edit"
            {
                if message.wParam == VK_RETURN as usize {
                    editor_command(root, ec::CONFIRM);
                    continue;
                }
                if message.wParam == VK_DELETE as usize {
                    editor_command(root, ec::DELETE);
                    continue;
                }
            }
            if message.message == WM_KEYDOWN
                && root_class == PREVIEW
                && class_name(message.hwnd) == "Edit"
                && message.wParam == VK_RETURN as usize
            {
                let id = GetDlgCtrlID(message.hwnd) as usize;
                editor_command(root, id);
                continue;
            }
            if message.message == WM_KEYDOWN
                && root_class == PREVIEW
                && class_name(message.hwnd) != "Edit"
                && GetKeyState(VK_CONTROL as i32) < 0
            {
                let owner = root;
                match message.wParam as u16 {
                    0x5a => {
                        editor_command(owner, ec::UNDO);
                        continue;
                    }
                    0x59 => {
                        editor_command(owner, ec::REDO);
                        continue;
                    }
                    VK_C => {
                        copy_image(owner);
                        continue;
                    }
                    VK_S => {
                        project_ui::save(owner, GetKeyState(VK_SHIFT as i32) < 0, false);
                        continue;
                    }
                    VK_O => {
                        project_ui::open(owner);
                        continue;
                    }
                    VK_V => {
                        import_image(owner, true);
                        continue;
                    }
                    _ => {}
                }
            }
            if ![MAIN, PREVIEW, HOTKEYS, OCR_WINDOW, video_ui::CLASS].contains(&root_class.as_str())
                || IsDialogMessageW(root, &message) == 0
            {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        if let Some(app) = STATE.with(|s| s.borrow_mut().take()) {
            unregister_hotkeys(main, &app.active_hotkeys);
            for handle in app.font_cache.values() {
                DeleteObject(*handle as HGDIOBJ);
            }
        }
        CloseHandle(mutex);
    }
    Ok(())
}

#[cfg(test)]
mod text_error_tests {
    #[test]
    fn source_preview_cache_never_reveals_clipped_original_pixels() {
        let frame = crate::frame::Frame::new(20, 20, vec![255; 1600]).unwrap();
        let mut d = crate::editor::Document::new(20, 20);
        assert!(super::source_is_unmodified(&frame, &d));
        d.source_clip = Some(crate::frame::Rect::new(0, 0, 10, 10).unwrap());
        assert!(!super::source_is_unmodified(&frame, &d));
        d.source_clip = None;
        assert!(!super::source_is_unmodified(&frame, &d));
    }
    #[test]
    fn launcher_version_labels_match_package_metadata() {
        assert!(super::TITLE.ends_with(env!("CARGO_PKG_VERSION")));
        assert_eq!(super::SUBTITLE, "Rust Preview");
    }
    #[test]
    fn compact_launcher_menus_preserve_existing_command_routes() {
        assert_eq!(
            super::launcher_menu_commands(super::WINDOW_MENU),
            &[super::WINDOW, super::DESKTOP]
        );
        assert_eq!(
            super::launcher_menu_commands(super::SCROLL_OPTIONS),
            &[super::SCROLL_AUTO, super::SCROLL_MANUAL]
        );
        assert_eq!(
            super::launcher_menu_commands(super::OPEN_OPTIONS),
            &[super::OPEN_TAB, super::OPEN_WINDOW]
        );
        assert_eq!(
            super::launcher_menu_commands(super::MORE),
            &[super::KEYS, super::ABOUT, super::HIDE, super::EXIT]
        );
    }
    #[test]
    fn hotkey_rows_cover_every_capture_action_and_use_unique_controls() {
        let actions = super::HOTKEY_ROWS
            .iter()
            .map(|(action, _, _, _)| *action)
            .collect::<Vec<_>>();
        assert_eq!(actions, crate::settings::HotkeyAction::ALL);
        let mut ids = std::collections::HashSet::new();
        for (_, enable, modifier, key) in super::HOTKEY_ROWS {
            assert!(ids.insert(enable));
            assert!(ids.insert(modifier));
            assert!(ids.insert(key));
        }
    }
    #[test]
    fn delay_choices_are_visible_and_keep_the_supported_order() {
        assert_eq!(
            super::delay_choices()
                .into_iter()
                .map(|(_, seconds, _)| seconds)
                .collect::<Vec<_>>(),
            vec![0, 3, 5, 10]
        );
    }
    #[test]
    fn every_owner_draw_launcher_button_has_a_native_icon() {
        for id in [
            super::AREA,
            super::WINDOW,
            super::WINDOW_MENU,
            super::SCROLL,
            super::VIDEO,
            super::ec::PROJECT_OPEN,
            super::MORE,
        ] {
            assert!(super::launcher_icon_command(id).is_some(), "missing {id}");
        }
    }
    #[test]
    fn compact_editor_menus_keep_all_secondary_and_layer_commands() {
        assert_eq!(
            super::editor_menu_commands(super::ec::LAYERS),
            &[
                super::ec::FRONT,
                super::ec::FORWARD,
                super::ec::BACKWARD,
                super::ec::BACK,
            ]
        );
        assert_eq!(
            super::editor_menu_commands(super::ec::MORE),
            &[
                super::ec::ADD_IMAGE,
                super::ec::PASTE_IMAGE,
                super::ec::CANVAS,
                super::ec::EFFECTS,
                super::ec::COMBINE,
                super::ec::PROJECT_OPEN,
                super::ec::PROJECT_SAVE,
            ]
        );
    }
    #[test]
    fn text_layout_errors_follow_language_and_unknown_details_are_retained() {
        for message in [
            "Invalid text frame.",
            "Text frame is outside the supported coordinates.",
            "Text memory limit reached. Use shorter text or fewer edits.",
            "Text is too large. Use shorter lines or a smaller size.",
        ] {
            assert_eq!(super::text_error(message, false), message);
            assert!(
                super::text_error(message, true)
                    .chars()
                    .any(|c| ('\u{0e00}'..='\u{0e7f}').contains(&c))
            );
        }
        assert_eq!(
            super::text_error("Unexpected native failure", true),
            "Unexpected native failure"
        );
    }

    #[test]
    fn launcher_option_labels_are_compact_but_keep_the_meaning() {
        assert_eq!(super::delay_button_label(0, false), "Delay: None");
        assert_eq!(super::delay_button_label(10, false), "Delay: 10s");
        assert_eq!(super::delay_button_label(5, true), "หน่วง: 5 วิ");
        assert_eq!(super::scroll_mode_button_label(true, false), "Scroll: Auto");
        assert_eq!(
            super::scroll_mode_button_label(false, true),
            "เลื่อน: เลื่อนเอง"
        );
        assert_eq!(super::open_mode_button_label(true, false), "Open: Tab");
        assert_eq!(super::open_mode_button_label(false, true), "เปิด: หน้าต่าง");
        for label in [
            super::delay_button_label(10, false),
            super::delay_button_label(10, true),
            super::scroll_mode_button_label(true, false).to_string(),
            super::scroll_mode_button_label(false, true).to_string(),
            super::open_mode_button_label(true, false).to_string(),
            super::open_mode_button_label(false, true).to_string(),
        ] {
            assert!(label.chars().count() <= 18, "label is too wide: {label}");
        }
    }

    #[test]
    fn launcher_option_buttons_have_semantic_icons() {
        use crate::native_icons::Command as IconCommand;
        assert_eq!(
            super::launcher_option_icon_command(super::DELAY_OPTIONS),
            Some(IconCommand::Delay)
        );
        assert_eq!(
            super::launcher_option_icon_command(super::SCROLL_OPTIONS),
            Some(IconCommand::ScrollingMode)
        );
        assert_eq!(
            super::launcher_option_icon_command(super::OPEN_OPTIONS),
            Some(IconCommand::OpenDestination)
        );
    }

    #[test]
    fn launcher_primary_labels_are_centered_and_use_the_ui_typeface() {
        assert_ne!(
            super::primary_label_text_flags() & windows_sys::Win32::Graphics::Gdi::DT_CENTER,
            0
        );
        assert_eq!(
            super::primary_label_font_weight(),
            windows_sys::Win32::Graphics::Gdi::FW_SEMIBOLD as i32
        );
        assert_eq!(super::UI_FONT_FACE, "Segoe UI Variable Text");
    }
}

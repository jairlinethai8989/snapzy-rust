use crate::{
    capture::{self, Candidate},
    clipboard,
    frame::{Frame, Rect},
    settings::Settings,
    trace,
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
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
    UI::{
        Controls::{BST_CHECKED, BST_UNCHECKED, Dialogs::*},
        HiDpi::*,
        Input::KeyboardAndMouse::*,
        Shell::*,
        WindowsAndMessaging::*,
    },
};

const MAIN: &str = "SnapZyRustPreview.Launcher";
const PREVIEW: &str = "SnapZyRustPreview.Capture";
const OVERLAY: &str = "SnapZyRustPreview.Selection";
const HOTKEYS: &str = "SnapZyRustPreview.Hotkeys";
const TITLE: &str = "SnapZy Rust Preview 0.1.0-alpha.1";
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
const COPY: usize = 201;
const SAVE: usize = 202;
const APPLY: usize = 301;
const ENABLE: i32 = 302;
const MODIFIER: i32 = 303;
const KEY: i32 = 304;
const STATUS: i32 = 401;
const CAPTURE_TIMER: usize = 501;

thread_local! { static STATE: RefCell<Option<App>> = const { RefCell::new(None) }; }

#[derive(Clone, Copy, Debug)]
enum Mode {
    Area,
    Window,
    Desktop,
}

struct Preview {
    frame: Arc<Frame>,
    first_paint: Option<Instant>,
    kept: bool,
}

struct Selection {
    window: usize,
    frame: Arc<Frame>,
    desktop: Rect,
    mode: Mode,
    windows: Vec<Candidate>,
    start: Option<(i32, i32)>,
    end: (i32, i32),
    first_paint: Option<Instant>,
}

enum Work {
    Snapshot {
        mode: Mode,
        started: Instant,
        result: Result<(Frame, Rect, Vec<Candidate>), String>,
    },
    Saved {
        owner: usize,
        started: Instant,
        result: Result<(), String>,
    },
}

struct App {
    main: usize,
    settings: Settings,
    settings_window: usize,
    previews: HashMap<usize, Preview>,
    selection: Option<Selection>,
    pending: Option<(Mode, Instant)>,
    capture_busy: bool,
    restore: Vec<usize>,
    saving: usize,
    tx: mpsc::Sender<Work>,
    rx: mpsc::Receiver<Work>,
    font_cache: HashMap<(u32, i32, i32), usize>,
    icon: usize,
    tray_added: bool,
    active_hotkey: i32,
    taskbar_message: u32,
    first_paint: Option<Instant>,
}

pub fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
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
        ("saving", true) => "กำลังบันทึก...",
        ("saved", false) => "Saved",
        ("saved", true) => "บันทึกแล้ว",
        ("enable", false) => "Enable area capture hotkey",
        ("enable", true) => "เปิดคีย์ลัดจับภาพพื้นที่",
        ("apply", false) => "Apply",
        ("apply", true) => "นำไปใช้",
        ("success", false) => "Hotkey settings saved.",
        ("success", true) => "บันทึกการตั้งค่าคีย์ลัดแล้ว",
        ("conflict", false) => {
            "This hotkey is used by another application. Choose a different combination."
        }
        ("conflict", true) => "คีย์ลัดนี้ถูกใช้งานโดยโปรแกรมอื่น กรุณาเลือกชุดปุ่มใหม่",
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
    if crate::settings::is_test_run() {
        trace::test_note(&format!("notification error={error} text={text}"));
    }
    unsafe {
        MessageBoxW(
            owner,
            wide(text).as_ptr(),
            wide(TITLE).as_ptr(),
            MB_OK
                | if error {
                    MB_ICONWARNING
                } else {
                    MB_ICONINFORMATION
                },
        );
    }
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
                    wide("Segoe UI").as_ptr(),
                ) as usize
            }) as HFONT
    })
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
    let rect = client(window);
    let logical_width = rect.right * 96 / unsafe { GetDpiForWindow(window) }.max(96) as i32;
    if class == MAIN {
        let gap = 10;
        let width = (logical_width - 32 - 2 * gap) / 3;
        for (i, id) in [AREA, WINDOW, DESKTOP].iter().enumerate() {
            position(
                window,
                *id as i32,
                16 + i as i32 * (width + gap),
                90,
                width,
                56,
            );
        }
        position(window, LANGUAGE as i32, logical_width - 100, 20, 40, 30);
        position(window, ABOUT as i32, logical_width - 52, 20, 36, 30);
        position(window, KEYS as i32, 16, 163, 105, 34);
        position(window, HIDE as i32, logical_width - 168, 163, 152, 34);
        position(window, STATUS, 132, 167, (logical_width - 308).max(20), 28);
    } else if class == PREVIEW {
        position(window, COPY as i32, logical_width - 228, 12, 94, 34);
        position(window, SAVE as i32, logical_width - 126, 12, 110, 34);
        position(window, STATUS, 16, 16, (logical_width - 260).max(10), 28);
    } else if class == HOTKEYS {
        position(window, ENABLE, 16, 18, logical_width - 32, 28);
        position(window, MODIFIER, 16, 62, logical_width / 2 - 24, 220);
        position(
            window,
            KEY,
            logical_width / 2,
            62,
            logical_width / 2 - 16,
            220,
        );
        position(window, APPLY as i32, logical_width - 126, 112, 110, 34);
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
    let (main, previews, thai) = STATE.with(|s| {
        let borrow = s.borrow();
        let a = borrow.as_ref().unwrap();
        (
            a.main,
            a.previews.keys().copied().collect::<Vec<_>>(),
            a.settings.language == "th",
        )
    });
    for (id, key) in [
        (AREA, "area"),
        (WINDOW, "window"),
        (DESKTOP, "desktop"),
        (KEYS, "keys"),
        (HIDE, "hide"),
    ] {
        unsafe {
            SetWindowTextW(GetDlgItem(main as HWND, id as i32), wide(&tr(key)).as_ptr());
        }
    }
    unsafe {
        SetWindowTextW(
            GetDlgItem(main as HWND, LANGUAGE as i32),
            wide(if thai { "TH" } else { "EN" }).as_ptr(),
        );
    }
    status(main as HWND, &tr("ready"));
    for window in previews {
        unsafe {
            SetWindowTextW(
                GetDlgItem(window as HWND, COPY as i32),
                wide(&tr("copy")).as_ptr(),
            );
            SetWindowTextW(
                GetDlgItem(window as HWND, SAVE as i32),
                wide(&tr("save")).as_ptr(),
            );
        }
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
            CW_USEDEFAULT,
            CW_USEDEFAULT,
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
    let started = Instant::now();
    let windows = STATE.with(|s| {
        let mut borrow = s.borrow_mut();
        let app = borrow.as_mut().unwrap();
        if app.capture_busy || app.settings_window != 0 {
            return None;
        }
        app.capture_busy = true;
        app.pending = Some((mode, started));
        let mut windows: Vec<_> = app.previews.keys().copied().collect();
        windows.push(app.main);
        Some((app.main, windows))
    });
    if let Some((main, windows)) = windows {
        let visible: Vec<_> = windows
            .into_iter()
            .filter(|window| unsafe { IsWindowVisible(*window as HWND) } != 0)
            .collect();
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().restore = visible.clone());
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
        (a.main, a.pending.take(), a.tx.clone())
    });
    unsafe {
        KillTimer(main as HWND, CAPTURE_TIMER);
    }
    if let Some((mode, started)) = task {
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
    for window in windows {
        unsafe {
            if IsWindow(window as HWND) != 0 {
                ShowWindow(window as HWND, SW_SHOWNA);
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
    let window = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            wide(OVERLAY).as_ptr(),
            wide(&tr(match mode {
                Mode::Window => "window",
                _ => "area",
            }))
            .as_ptr(),
            WS_POPUP,
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
        s.borrow_mut().as_mut().unwrap().selection = Some(Selection {
            window: window as usize,
            frame: Arc::new(frame),
            desktop,
            mode,
            windows,
            start: None,
            end: (cursor.x - desktop.x, cursor.y - desktop.y),
            first_paint: Some(started),
        })
    });
    unsafe {
        ShowWindow(window, SW_SHOW);
        SetForegroundWindow(window);
        SetFocus(window);
        UpdateWindow(window);
    }
}

fn selection_rect(selection: &Selection) -> Option<Rect> {
    if matches!(selection.mode, Mode::Window) {
        let (x, y) = (
            selection.desktop.x + selection.end.0,
            selection.desktop.y + selection.end.1,
        );
        capture::window_at(&selection.windows, x, y)
            .map(|(_, window)| window.rect)
            .or_else(|| capture::monitor_at(x, y).ok())?
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
    if let Some(selection) = selection {
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

fn present(frame: Frame, started: Instant) {
    let full = STATE.with(|s| {
        let borrow = s.borrow();
        let app = borrow.as_ref().unwrap();
        app.previews.len() >= 16
            || app
                .previews
                .values()
                .map(|p| p.frame.bgra.len())
                .sum::<usize>()
                + frame.bgra.len()
                > 256 * 1024 * 1024
    });
    if full {
        notify(
            null_mut(),
            "Close an older capture before opening another preview.",
            true,
        );
        return;
    }
    let title = format!("{} - {}", TITLE, tr("preview"));
    let window = create_window(
        PREVIEW,
        &title,
        WS_OVERLAPPEDWINDOW,
        0,
        null_mut(),
        800,
        580,
    );
    if window.is_null() {
        notify(null_mut(), "Cannot open the image preview.", true);
        return;
    }
    let dimensions = format!("{} x {} px", frame.width, frame.height);
    STATE.with(|s| {
        s.borrow_mut().as_mut().unwrap().previews.insert(
            window as usize,
            Preview {
                frame: Arc::new(frame),
                first_paint: Some(started),
                kept: false,
            },
        )
    });
    child(window, "BUTTON", &tr("copy"), COPY as i32, WS_TABSTOP);
    child(
        window,
        "BUTTON",
        &tr("save"),
        SAVE as i32,
        WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
    );
    child(window, "STATIC", &dimensions, STATUS, 0);
    layout(window);
    unsafe {
        ShowWindow(window, SW_SHOW);
        SetForegroundWindow(window);
        UpdateWindow(window);
    }
}

fn complete_work() {
    let work: Vec<_> = STATE.with(|s| s.borrow().as_ref().unwrap().rx.try_iter().collect());
    for item in work {
        match item {
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
                started,
                result,
            } => {
                STATE.with(|s| {
                    let mut b = s.borrow_mut();
                    let a = b.as_mut().unwrap();
                    a.saving = a.saving.saturating_sub(1);
                });
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
                                p.kept = true;
                            }
                        });
                        status(owner as HWND, &tr("saved"));
                    }
                    Err(error) => notify(null_mut(), &error, true),
                }
            }
        }
    }
}

fn copy_image(window: HWND) {
    let frame = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .previews
            .get(&(window as usize))
            .map(|p| p.frame.clone())
    });
    if let Some(frame) = frame {
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
                        p.kept = true;
                    }
                });
                status(window, &tr("copied"));
                trace::record("clipboard.copy", started);
            }
            Err(error) => notify(window, &error, true),
        }
    }
}

fn save_image(window: HWND) {
    let frame = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .previews
            .get(&(window as usize))
            .map(|p| p.frame.clone())
    });
    let Some(frame) = frame else {
        return;
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
    dialog.hwndOwner = window;
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
    let (sender, main) = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        a.saving += 1;
        (a.tx.clone(), a.main)
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
        370,
        164,
    );
    if window.is_null() {
        return;
    }
    STATE.with(|s| s.borrow_mut().as_mut().unwrap().settings_window = window as usize);
    child(
        window,
        "BUTTON",
        &tr("enable"),
        ENABLE,
        WS_TABSTOP | BS_AUTOCHECKBOX as u32,
    );
    let modifier = child(
        window,
        "COMBOBOX",
        "",
        MODIFIER,
        WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
    );
    let key = child(
        window,
        "COMBOBOX",
        "",
        KEY,
        WS_TABSTOP | CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
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
            GetDlgItem(window, ENABLE),
            BM_SETCHECK,
            if settings.hotkey_enabled {
                BST_CHECKED as usize
            } else {
                BST_UNCHECKED as usize
            },
            0,
        );
        SendMessageW(
            modifier,
            CB_SETCURSEL,
            [3, 6, 5, 7]
                .iter()
                .position(|m| *m == settings.hotkey_modifiers)
                .unwrap_or(0),
            0,
        );
        SendMessageW(
            key,
            CB_SETCURSEL,
            [119, 120, 121, 122, 44]
                .iter()
                .position(|k| *k == settings.hotkey_key)
                .unwrap_or(0),
            0,
        );
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

fn apply_hotkeys(window: HWND) {
    let (main, old_id, mut settings) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        (a.main, a.active_hotkey, a.settings.clone())
    });
    let (enabled, modifier, key) = unsafe {
        (
            SendMessageW(GetDlgItem(window, ENABLE), BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
            SendMessageW(GetDlgItem(window, MODIFIER), CB_GETCURSEL, 0, 0),
            SendMessageW(GetDlgItem(window, KEY), CB_GETCURSEL, 0, 0),
        )
    };
    if crate::settings::is_test_run() {
        trace::test_note(&format!(
            "hotkey-form enabled={enabled} modifier-index={modifier} key-index={key}"
        ));
    }
    if !(0..4).contains(&modifier) || !(0..5).contains(&key) {
        return;
    }
    let new_id = if old_id == 1 { 2 } else { 1 };
    settings.hotkey_enabled = enabled;
    settings.hotkey_modifiers = [3, 6, 5, 7][modifier as usize];
    settings.hotkey_key = [119, 120, 121, 122, 44][key as usize];
    let same = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        old_id != 0
            && enabled
            && a.settings.hotkey_modifiers == settings.hotkey_modifiers
            && a.settings.hotkey_key == settings.hotkey_key
    });
    let registered = enabled
        && !same
        && unsafe {
            RegisterHotKey(
                main as HWND,
                new_id,
                settings.hotkey_modifiers | MOD_NOREPEAT,
                settings.hotkey_key,
            )
        } != 0;
    if crate::settings::is_test_run() {
        trace::test_note(&format!(
            "hotkey-register enabled={enabled} same={same} registered={registered} error={}",
            unsafe { GetLastError() }
        ));
    }
    if enabled && !same && !registered {
        notify(window, &tr("conflict"), true);
        return;
    }
    if let Err(error) = settings.save() {
        if enabled && !same {
            unsafe {
                UnregisterHotKey(main as HWND, new_id);
            }
        }
        notify(window, &error, true);
        return;
    }
    if old_id != 0 && !same {
        unsafe {
            UnregisterHotKey(main as HWND, old_id);
        }
    }
    STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        a.settings = settings;
        a.active_hotkey = if enabled {
            if same { old_id } else { new_id }
        } else {
            0
        };
    });
    notify(window, &tr("success"), false);
}

fn close_window(window: HWND) {
    let class = class_name(window);
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
    let kept = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .previews
            .get(&(window as usize))
            .map(|p| p.kept)
    });
    if kept == Some(false)
        && unsafe {
            MessageBoxW(
                window,
                wide(&tr("discard")).as_ptr(),
                wide(TITLE).as_ptr(),
                MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
            )
        } != IDYES
    {
        return;
    }
    STATE.with(|s| {
        s.borrow_mut()
            .as_mut()
            .unwrap()
            .previews
            .remove(&(window as usize))
    });
    unsafe {
        DestroyWindow(window);
    }
}

fn exit_app() {
    let (main, saving, unsaved, windows) = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        let mut windows: Vec<_> = a.previews.keys().copied().collect();
        if a.settings_window != 0 {
            windows.push(a.settings_window);
        }
        if let Some(selection) = &a.selection {
            windows.push(selection.window);
        }
        (
            a.main,
            a.saving,
            a.previews.values().any(|p| !p.kept),
            windows,
        )
    });
    if saving > 0 {
        notify(main as HWND, &tr("wait"), false);
        return;
    }
    if unsaved
        && unsafe {
            MessageBoxW(
                main as HWND,
                wide(&tr("exit_unsaved")).as_ptr(),
                wide(TITLE).as_ptr(),
                MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
            )
        } != IDYES
    {
        return;
    }
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
    match id {
        AREA => start_capture(Mode::Area),
        WINDOW => start_capture(Mode::Window),
        DESKTOP => start_capture(Mode::Desktop),
        COPY => copy_image(window),
        SAVE => save_image(window),
        KEYS => show_hotkeys(),
        APPLY => apply_hotkeys(window),
        LANGUAGE => {
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

fn text(dc: HDC, window: HWND, value: &str, rect: RECT, size: i32, weight: i32, color: u32) {
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
            DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
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
    let dc = unsafe { BeginPaint(window, &mut paint) };
    if dc.is_null() {
        return;
    }
    let bounds = client(window);
    let class = class_name(window);
    let started = if class == MAIN {
        fill(dc, bounds, 0x00ffffff);
        let icon = STATE.with(|s| s.borrow().as_ref().unwrap().icon);
        unsafe {
            DrawIconEx(
                dc,
                scale(window, 16),
                scale(window, 20),
                icon as HICON,
                scale(window, 42),
                scale(window, 42),
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
                left: scale(window, 70),
                top: scale(window, 18),
                right: bounds.right - scale(window, 115),
                bottom: scale(window, 45),
            },
            20,
            FW_BOLD as i32,
            0x003e2717,
        );
        text(
            dc,
            window,
            "Rust Preview 0.1.0-alpha.1",
            RECT {
                left: scale(window, 70),
                top: scale(window, 45),
                right: bounds.right - scale(window, 110),
                bottom: scale(window, 70),
            },
            12,
            FW_NORMAL as i32,
            0x00786659,
        );
        STATE
            .with(|s| s.borrow_mut().as_mut().unwrap().first_paint.take())
            .map(|clock| ("launcher.first-paint", clock))
    } else if class == PREVIEW {
        fill(dc, bounds, 0x00f2f0eb);
        fill(
            dc,
            RECT {
                left: 0,
                top: 0,
                right: bounds.right,
                bottom: scale(window, 58),
            },
            0x00ffffff,
        );
        let frame = STATE.with(|s| {
            s.borrow()
                .as_ref()
                .unwrap()
                .previews
                .get(&(window as usize))
                .map(|p| p.frame.clone())
        });
        if let Some(frame) = frame {
            let width = (bounds.right - scale(window, 32)).max(1);
            let height = (bounds.bottom - scale(window, 90)).max(1);
            let ratio = (width as f64 / frame.width as f64)
                .min(height as f64 / frame.height as f64)
                .min(1.0);
            let w = (frame.width as f64 * ratio).round() as i32;
            let h = (frame.height as f64 * ratio).round() as i32;
            let x = (bounds.right - w) / 2;
            let y = scale(window, 66) + (height - h) / 2;
            draw_frame(
                dc,
                &frame,
                RECT {
                    left: x,
                    top: y,
                    right: x + w,
                    bottom: y + h,
                },
            );
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
                )
            })
        });
        if let Some((frame, desktop, selection, clock)) = snapshot {
            draw_frame(dc, &frame, bounds);
            if let Some(rect) = selection {
                unsafe {
                    let pen = CreatePen(PS_SOLID, 3, 0x00f36517);
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
        match message {
            WM_PAINT => {
                paint(window);
                0
            }
            WM_ERASEBKGND => 1,
            WM_COMMAND => {
                if (wparam >> 16) == BN_CLICKED as usize {
                    PostMessageW(window, COMMAND, wparam & 0xffff, 0);
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
            WM_CLOSE => {
                PostMessageW(window, CLOSE, 0, 0);
                0
            }
            CLOSE => {
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
                limits.ptMinTrackSize.x = scale(window, 400);
                limits.ptMinTrackSize.y = scale(window, 240);
                0
            }
            WM_HOTKEY => {
                start_capture(Mode::Area);
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
            WM_KEYDOWN if wparam == VK_ESCAPE as usize => {
                if class_name(window) == OVERLAY {
                    finish_selection(true);
                } else if class_name(window) == HOTKEYS {
                    close_window(window);
                }
                0
            }
            WM_CTLCOLORSTATIC => {
                SetBkMode(wparam as HDC, TRANSPARENT as i32);
                SetTextColor(wparam as HDC, 0x00665649);
                GetStockObject(WHITE_BRUSH) as isize
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
            wide("Local\\SnapZyRustPreview.Instance").as_ptr(),
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
        STATE.with(|s| {
            *s.borrow_mut() = Some(App {
                main: 0,
                settings: Settings::load(),
                settings_window: 0,
                previews: HashMap::new(),
                selection: None,
                pending: None,
                capture_busy: false,
                restore: Vec::new(),
                saving: 0,
                tx,
                rx,
                font_cache: HashMap::new(),
                icon,
                tray_added: false,
                active_hotkey: 0,
                taskbar_message: RegisterWindowMessageW(wide(TASKBAR_CREATED).as_ptr()),
                first_paint: Some(started),
            })
        });
        for name in [MAIN, PREVIEW, OVERLAY, HOTKEYS] {
            let class_name = wide(name);
            let class = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
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
            480,
            216,
        );
        if main.is_null() {
            CloseHandle(mutex);
            return Err("Cannot create the launcher.".into());
        }
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().main = main as usize);
        for (id, key) in [
            (AREA, "area"),
            (WINDOW, "window"),
            (DESKTOP, "desktop"),
            (KEYS, "keys"),
            (HIDE, "hide"),
        ] {
            child(main, "BUTTON", &tr(key), id as i32, WS_TABSTOP);
        }
        child(main, "BUTTON", "EN", LANGUAGE as i32, WS_TABSTOP);
        child(main, "BUTTON", "?", ABOUT as i32, WS_TABSTOP);
        child(main, "STATIC", &tr("ready"), STATUS, 0);
        layout(main);
        refresh_language();
        let added = tray(true);
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().tray_added = added);
        if !added {
            EnableWindow(GetDlgItem(main, HIDE as i32), 0);
        }
        let settings = STATE.with(|s| s.borrow().as_ref().unwrap().settings.clone());
        if settings.hotkey_enabled {
            if RegisterHotKey(
                main,
                1,
                settings.hotkey_modifiers | MOD_NOREPEAT,
                settings.hotkey_key,
            ) != 0
            {
                STATE.with(|s| s.borrow_mut().as_mut().unwrap().active_hotkey = 1);
            } else {
                notify(main, &tr("conflict"), true);
            }
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
            let root = GetAncestor(message.hwnd, GA_ROOT);
            let root_class = class_name(root);
            if message.message == WM_KEYDOWN && message.wParam == VK_ESCAPE as usize {
                if root_class == OVERLAY {
                    finish_selection(true);
                    continue;
                }
                if root_class == HOTKEYS {
                    close_window(root);
                    continue;
                }
            }
            if message.message == WM_KEYDOWN
                && root_class == PREVIEW
                && GetKeyState(VK_CONTROL as i32) < 0
            {
                let owner = GetAncestor(message.hwnd, GA_ROOT);
                match message.wParam as u16 {
                    VK_C => {
                        copy_image(owner);
                        continue;
                    }
                    VK_S => {
                        save_image(owner);
                        continue;
                    }
                    _ => {}
                }
            }
            if ![MAIN, PREVIEW, HOTKEYS].contains(&root_class.as_str())
                || IsDialogMessageW(root, &message) == 0
            {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        UnregisterHotKey(main, 1);
        UnregisterHotKey(main, 2);
        if let Some(app) = STATE.with(|s| s.borrow_mut().take()) {
            for handle in app.font_cache.values() {
                DeleteObject(*handle as HGDIOBJ);
            }
        }
        CloseHandle(mutex);
    }
    Ok(())
}

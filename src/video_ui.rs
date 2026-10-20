use super::*;
use crate::{
    video_clip::{Clip, time_label},
    video_playback::Player,
};
use windows_sys::Win32::System::SystemServices::{SS_BLACKRECT, SS_PATHELLIPSIS, SS_RIGHT};
use windows_sys::Win32::UI::Controls::*;
const TBM_GETPOS: u32 = WM_USER;

pub(super) const CLASS: &str = "SnapZyRustPreview.VideoPreview";
const SURFACE: i32 = 1001;
const PLAY: i32 = 1002;
const SEEK: i32 = 1003;
const TIME: i32 = 1004;
const MUTE: i32 = 1005;
const VOLUME: i32 = 1006;
const CLIP_COPY: i32 = 1007;
const CLIP_SAVE: i32 = 1008;
const FOLDER: i32 = 1009;
const HEADING: i32 = 1010;
const TICK: usize = 1;

#[derive(Clone, Copy)]
enum After {
    Stay,
    Close,
    Exit,
}
pub(super) struct Preview {
    pub clip: Clip,
    player: Option<Player>,
    loaded: Instant,
    ready: bool,
    dragging: bool,
    after: After,
    icon_font: usize,
    tips: Option<Tips>,
}
pub(super) struct ExitPlan {
    windows: Vec<(usize, u64)>,
    index: usize,
}
struct Tips {
    window: usize,
    _labels: Vec<Vec<u16>>,
}
impl Drop for Tips {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.window as HWND);
        }
    }
}

fn text(en: &str, th: &str) -> String {
    STATE
        .with(|s| {
            if s.borrow().as_ref().unwrap().settings.language == "th" {
                th
            } else {
                en
            }
        })
        .into()
}
fn set_text(window: HWND, id: i32, value: &str) {
    unsafe {
        let control = GetDlgItem(window, id);
        let mut current = vec![0u16; GetWindowTextLengthW(control).max(0) as usize + 1];
        let len =
            GetWindowTextW(control, current.as_mut_ptr(), current.len() as i32).max(0) as usize;
        if String::from_utf16_lossy(&current[..len]) != value {
            SetWindowTextW(control, wide(value).as_ptr());
        }
    }
}
fn with_player<T>(window: HWND, f: impl FnOnce(&Player) -> T) -> Option<T> {
    let (identity, player) = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let p = b.as_mut()?.video_previews.get_mut(&(window as usize))?;
        Some((p.clip.identity, p.player.take()?))
    })?;
    let result = f(&player);
    let mut player = Some(player);
    STATE.with(|s| {
        if let Some(p) = s
            .borrow_mut()
            .as_mut()
            .and_then(|a| a.video_previews.get_mut(&(window as usize)))
            && p.clip.identity == identity
            && p.player.is_none()
        {
            p.player = player.take();
        }
    });
    drop(player);
    Some(result)
}
pub(super) fn pause_all() {
    let windows = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .video_previews
            .keys()
            .copied()
            .collect::<Vec<_>>()
    });
    for window in windows {
        with_player(window as HWND, |p| p.pause());
    }
}
pub(super) fn shutdown_all() {
    let players = STATE.with(|s| {
        s.borrow_mut()
            .as_mut()
            .unwrap()
            .video_previews
            .values_mut()
            .filter_map(|p| p.player.take())
            .collect::<Vec<_>>()
    });
    drop(players);
}
pub(super) fn saving() -> bool {
    STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .video_previews
            .values()
            .any(|p| p.clip.saving)
    })
}

pub(super) fn present(path: PathBuf, reason: String, identity: u64) {
    let main = STATE.with(|s| s.borrow().as_ref().unwrap().main) as HWND;
    if STATE.with(|s| s.borrow().as_ref().unwrap().video_previews.len() >= 8) {
        notify(
            main,
            &format!(
                "{}\n{}",
                text(
                    "Close an older video preview. Clip retained at:",
                    "กรุณาปิดพรีวิววิดีโอเก่า คลิปเก็บไว้ที่:"
                ),
                path.display()
            ),
            false,
        );
        return;
    }
    let window = create_window(
        CLASS,
        &format!("{TITLE} | {}", text("Video preview", "พรีวิววิดีโอ")),
        WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
        WS_EX_APPWINDOW,
        null_mut(),
        760,
        540,
    );
    if window.is_null() {
        notify(
            main,
            &format!(
                "{}\n{}",
                text(
                    "Cannot open preview. Clip retained at:",
                    "เปิดพรีวิวไม่สำเร็จ คลิปเก็บไว้ที่:"
                ),
                path.display()
            ),
            true,
        );
        return;
    }
    unsafe {
        let controls = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_BAR_CLASSES,
        };
        InitCommonControlsEx(&controls);
    }
    STATE.with(|s| {
        s.borrow_mut().as_mut().unwrap().video_previews.insert(
            window as usize,
            Preview {
                clip: Clip::new(identity, path.clone(), reason),
                player: None,
                loaded: Instant::now(),
                ready: false,
                dragging: false,
                after: After::Stay,
                icon_font: 0,
                tips: None,
            },
        );
    });
    child(
        window,
        "STATIC",
        &path.file_name().unwrap_or_default().to_string_lossy(),
        HEADING,
        SS_PATHELLIPSIS,
    );
    child(window, "STATIC", "", SURFACE, SS_BLACKRECT);
    for id in [PLAY, MUTE, CLIP_COPY, CLIP_SAVE, FOLDER] {
        child(window, "BUTTON", "", id, WS_TABSTOP | BS_PUSHBUTTON as u32);
    }
    child(
        window,
        "msctls_trackbar32",
        "",
        SEEK,
        WS_TABSTOP | TBS_NOTICKS,
    );
    child(window, "STATIC", "00:00 / 00:00", TIME, SS_RIGHT);
    child(
        window,
        "msctls_trackbar32",
        "",
        VOLUME,
        WS_TABSTOP | TBS_NOTICKS,
    );
    child(
        window,
        "STATIC",
        &text("Loading preview...", "กำลังเปิดพรีวิว..."),
        STATUS,
        0,
    );
    unsafe {
        SendMessageW(GetDlgItem(window, SEEK), TBM_SETRANGEMAX, 0, 10000);
        SendMessageW(GetDlgItem(window, VOLUME), TBM_SETRANGEMAX, 0, 100);
        SendMessageW(GetDlgItem(window, VOLUME), TBM_SETPOS, 1, 100);
    }
    refresh(window);
    layout(window);
    unsafe {
        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);
    }
    let surface = unsafe { GetDlgItem(window, SURFACE) };
    match Player::open(surface as usize, &path) {
        Ok(player) => {
            let mut player = Some(player);
            STATE.with(|s| {
                if let Some(p) = s
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .video_previews
                    .get_mut(&(window as usize))
                    && p.clip.identity == identity
                {
                    p.player = player.take();
                }
            });
            drop(player);
            unsafe {
                SetTimer(window, TICK, 100, None);
            }
            layout(window);
        }
        Err(e) => playback_error(window, &e),
    }
}

pub(super) fn refresh(window: HWND) {
    unsafe {
        SetWindowTextW(
            window,
            wide(&format!("{TITLE} | {}", text("Video preview", "พรีวิววิดีโอ"))).as_ptr(),
        );
    }
    let old = STATE.with(|s| {
        s.borrow_mut()
            .as_mut()
            .unwrap()
            .video_previews
            .get_mut(&(window as usize))
            .and_then(|p| p.tips.take())
    });
    drop(old);
    unsafe {
        let font = CreateFontW(
            -scale(window, 18),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            CLEARTYPE_QUALITY as u32,
            0,
            wide("Segoe MDL2 Assets").as_ptr(),
        );
        if !font.is_null() {
            let old = STATE.with(|s| {
                let mut b = s.borrow_mut();
                let p = b
                    .as_mut()
                    .unwrap()
                    .video_previews
                    .get_mut(&(window as usize))?;
                Some(std::mem::replace(&mut p.icon_font, font as usize))
            });
            for id in [PLAY, MUTE, CLIP_COPY, CLIP_SAVE, FOLDER] {
                SendMessageW(GetDlgItem(window, id), WM_SETFONT, font as usize, 1);
            }
            if let Some(old) = old {
                if old != 0 {
                    DeleteObject(old as HGDIOBJ);
                }
            }
        }
    }
    for (id, symbol) in [
        (PLAY, "\u{e768}"),
        (MUTE, "\u{e767}"),
        (CLIP_COPY, "\u{e8c8}"),
        (CLIP_SAVE, "\u{e74e}"),
        (FOLDER, "\u{ed25}"),
    ] {
        set_text(window, id, symbol);
    }
    let items = [
        (PLAY, text("Play / Pause", "เล่น / พัก")),
        (SEEK, text("Position", "ตำแหน่งเวลา")),
        (MUTE, text("Mute / Unmute", "ปิด / เปิดเสียง")),
        (VOLUME, text("Volume", "ระดับเสียง")),
        (CLIP_COPY, text("Copy clip", "คัดลอกคลิป")),
        (CLIP_SAVE, text("Save MP4", "บันทึก MP4")),
        (FOLDER, text("Show in folder", "แสดงในโฟลเดอร์")),
    ];
    let tips = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST,
            wide("tooltips_class32").as_ptr(),
            null(),
            WS_POPUP | TTS_ALWAYSTIP,
            0,
            0,
            0,
            0,
            window,
            null_mut(),
            GetModuleHandleW(null()),
            null(),
        )
    };
    let labels = items
        .iter()
        .map(|(_, label)| wide(label))
        .collect::<Vec<_>>();
    if !tips.is_null() {
        unsafe {
            SendMessageW(tips, TTM_SETMAXTIPWIDTH, 0, scale(window, 340) as isize);
            for ((id, _), label) in items.iter().zip(&labels) {
                let info = TTTOOLINFOW {
                    cbSize: size_of::<TTTOOLINFOW>() as u32,
                    uFlags: TTF_IDISHWND | TTF_SUBCLASS,
                    hwnd: window,
                    uId: GetDlgItem(window, *id) as usize,
                    lpszText: label.as_ptr() as *mut u16,
                    ..zeroed()
                };
                SendMessageW(tips, TTM_ADDTOOLW, 0, &info as *const _ as isize);
            }
        }
    }
    let mut owned = Some(Tips {
        window: tips as usize,
        _labels: labels,
    });
    STATE.with(|s| {
        if let Some(p) = s
            .borrow_mut()
            .as_mut()
            .unwrap()
            .video_previews
            .get_mut(&(window as usize))
        {
            p.tips = owned.take();
        }
    });
    drop(owned);
    tick(window);
}

fn layout(window: HWND) {
    let rect = client(window);
    let dpi = unsafe { GetDpiForWindow(window) }.max(96) as i32;
    let (w, h) = (rect.right * 96 / dpi, rect.bottom * 96 / dpi);
    for (id, x, y, width, height) in [
        (HEADING, 16, 14, w - 32, 24),
        (SURFACE, 16, 48, w - 32, h - 212),
        (PLAY, 16, h - 148, 40, 34),
        (SEEK, 64, h - 144, w - 270, 30),
        (TIME, w - 196, h - 140, 180, 24),
        (MUTE, 16, h - 100, 40, 34),
        (VOLUME, 64, h - 96, 136, 30),
        (CLIP_COPY, w - 156, h - 100, 40, 34),
        (CLIP_SAVE, w - 108, h - 100, 40, 34),
        (FOLDER, w - 60, h - 100, 44, 34),
        (STATUS, 16, h - 54, w - 32, 44),
    ] {
        position(window, id, x, y, width.max(1), height);
    }
    let icon_font = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .and_then(|a| a.video_previews.get(&(window as usize)))
            .map(|p| p.icon_font)
    });
    if let Some(font) = icon_font.filter(|f| *f != 0) {
        for id in [PLAY, MUTE, CLIP_COPY, CLIP_SAVE, FOLDER] {
            unsafe {
                SendMessageW(GetDlgItem(window, id), WM_SETFONT, font, 1);
            }
        }
    }
    let size = client(unsafe { GetDlgItem(window, SURFACE) });
    with_player(window, |p| p.resize(size.right, size.bottom));
}
fn playback_error(window: HWND, error: &str) {
    unsafe {
        KillTimer(window, TICK);
    }
    let player = STATE.with(|s| {
        s.borrow_mut()
            .as_mut()
            .and_then(|a| a.video_previews.get_mut(&(window as usize)))
            .and_then(|p| p.player.take())
    });
    drop(player);
    for id in [PLAY, SEEK, MUTE, VOLUME] {
        unsafe {
            EnableWindow(GetDlgItem(window, id), 0);
        }
    }
    status(
        window,
        &format!(
            "{} {error}",
            text(
                "Preview unavailable; clip retained. Copy, save or show its folder.",
                "เปิดพรีวิวไม่ได้ คลิปยังอยู่ สามารถคัดลอก บันทึก หรือเปิดโฟลเดอร์ได้"
            )
        ),
    );
}
fn tick(window: HWND) {
    let Some(state) = with_player(window, |p| p.snapshot()) else {
        for id in [PLAY, SEEK, MUTE, VOLUME] {
            unsafe {
                EnableWindow(GetDlgItem(window, id), 0);
            }
        }
        return;
    };
    let state = match state {
        Ok(s) => s,
        Err(e) => {
            playback_error(window, &e);
            return;
        }
    };
    let info = STATE.with(|s| {
        let b = s.borrow();
        let p = b.as_ref()?.video_previews.get(&(window as usize))?;
        Some((
            p.dragging,
            p.loaded.elapsed(),
            p.clip.reason.clone(),
            p.ready,
        ))
    });
    let Some((dragging, elapsed, reason, was_ready)) = info else {
        return;
    };
    if !state.ready && elapsed.as_secs() >= 30 {
        playback_error(window, &text("Loading timed out.", "หมดเวลารอเปิดคลิป"));
        return;
    }
    for (id, enabled) in [
        (PLAY, state.ready),
        (SEEK, state.ready),
        (MUTE, state.ready && state.audio),
        (VOLUME, state.ready && state.audio),
    ] {
        unsafe {
            EnableWindow(GetDlgItem(window, id), i32::from(enabled));
        }
    }
    set_text(
        window,
        PLAY,
        if state.paused || state.ended {
            "\u{e768}"
        } else {
            "\u{e769}"
        },
    );
    set_text(
        window,
        MUTE,
        if state.muted { "\u{e74f}" } else { "\u{e767}" },
    );
    set_text(
        window,
        TIME,
        &format!(
            "{} / {}",
            time_label(state.current),
            time_label(state.duration)
        ),
    );
    if !dragging && state.duration.is_finite() && state.duration > 0. {
        unsafe {
            SendMessageW(
                GetDlgItem(window, SEEK),
                TBM_SETPOS,
                1,
                (state.current / state.duration * 10000.).clamp(0., 10000.) as isize,
            );
        }
    }
    if state.volume.is_finite() {
        unsafe {
            SendMessageW(
                GetDlgItem(window, VOLUME),
                TBM_SETPOS,
                1,
                (state.volume * 100.).clamp(0., 100.) as isize,
            );
        }
    }
    if state.ready && !was_ready {
        STATE.with(|s| {
            if let Some(p) = s
                .borrow_mut()
                .as_mut()
                .unwrap()
                .video_previews
                .get_mut(&(window as usize))
            {
                p.ready = true;
            }
        });
        status(
            window,
            &if reason == "Stopped" {
                text("Ready", "พร้อม")
            } else {
                reason
            },
        );
        let r = client(unsafe { GetDlgItem(window, SURFACE) });
        with_player(window, |p| p.resize(r.right, r.bottom));
    }
}

fn allowed() -> bool {
    STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.recording.is_none()
            && a.video_exit.is_none()
            && a.project_exit.is_none()
            && !a.project_modal
    })
}
fn command(window: HWND, id: i32) {
    if !allowed() {
        return;
    }
    let result = match id {
        PLAY => with_player(window, |p| p.toggle()),
        MUTE => with_player(window, |p| p.snapshot().and_then(|s| p.mute(!s.muted))),
        CLIP_COPY => {
            let source = STATE.with(|s| {
                s.borrow()
                    .as_ref()
                    .unwrap()
                    .video_previews
                    .get(&(window as usize))
                    .map(|p| p.clip.source.clone())
            });
            let Some(source) = source else { return };
            let result = clipboard::copy_file(window, &source);
            if result.is_ok() {
                STATE.with(|s| {
                    if let Some(p) = s
                        .borrow_mut()
                        .as_mut()
                        .unwrap()
                        .video_previews
                        .get_mut(&(window as usize))
                    {
                        p.clip.copied = true;
                    }
                });
                status(window, &text("Clip copied", "คัดลอกคลิปแล้ว"));
            }
            Some(result)
        }
        CLIP_SAVE => {
            save(window, After::Stay);
            None
        }
        FOLDER => {
            let source = STATE.with(|s| {
                s.borrow()
                    .as_ref()
                    .unwrap()
                    .video_previews
                    .get(&(window as usize))
                    .map(|p| p.clip.source.clone())
            });
            if let Some(source) = source {
                let parameters = wide(&format!("/select,\"{}\"", source.display()));
                let result = unsafe {
                    ShellExecuteW(
                        window,
                        wide("open").as_ptr(),
                        wide("explorer.exe").as_ptr(),
                        parameters.as_ptr(),
                        null(),
                        SW_SHOWNORMAL,
                    )
                };
                if result as usize <= 32 {
                    status(
                        window,
                        &text("Cannot open the recording folder.", "เปิดโฟลเดอร์คลิปไม่สำเร็จ"),
                    );
                }
            }
            None
        }
        _ => None,
    };
    if let Some(Err(e)) = result {
        status(window, &e);
    }
    tick(window);
}

fn save(window: HWND, after: After) -> bool {
    let ready = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .video_previews
            .get(&(window as usize))
            .is_some_and(|p| !p.clip.saving)
    });
    if !ready {
        return false;
    }
    let Some(modal) = project_ui::Modal::enter() else {
        return false;
    };
    let destination = video_destination(window);
    drop(modal);
    let destination = match destination {
        Ok(Some(p)) => p,
        Ok(None) => return false,
        Err(e) => {
            status(window, &e);
            return false;
        }
    };
    let job = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut()?;
        let p = a.video_previews.get_mut(&(window as usize))?;
        let op = p.clip.begin_save()?;
        p.after = after;
        Some((
            p.clip.identity,
            op,
            p.clip.source.clone(),
            a.tx.clone(),
            a.main,
        ))
    });
    let Some((identity, operation, source, tx, main)) = job else {
        return false;
    };
    unsafe {
        EnableWindow(GetDlgItem(window, CLIP_SAVE), 0);
    }
    status(window, &tr("video_exporting"));
    let owner = window as usize;
    if let Err(e) = std::thread::Builder::new()
        .name("video-export".into())
        .spawn(move || {
            let result = crate::video_file::export_completed(&source, &destination, operation);
            let _ = tx.send(Work::VideoSaved {
                owner,
                identity,
                operation,
                result,
            });
            unsafe {
                PostMessageW(main as HWND, COMPLETE, 0, 0);
            }
        })
    {
        saved(owner, identity, operation, Err(e.to_string()));
        return false;
    }
    true
}

pub(super) fn saved(owner: usize, identity: u64, operation: u64, result: Result<PathBuf, String>) {
    let after = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut()?;
        let p = a.video_previews.get_mut(&owner)?;
        if !p.clip.finish_save(identity, operation, result.is_ok()) {
            return None;
        }
        Some(std::mem::replace(&mut p.after, After::Stay))
    });
    let Some(after) = after else { return };
    let window = owner as HWND;
    unsafe {
        EnableWindow(GetDlgItem(window, CLIP_SAVE), 1);
    }
    match result {
        Ok(_) => {
            status(window, &text("MP4 saved", "บันทึก MP4 แล้ว"));
            match after {
                After::Stay => (),
                After::Close => destroy(window),
                After::Exit => {
                    advance_exit();
                    return;
                }
            }
        }
        Err(e) => {
            STATE.with(|s| {
                let mut b = s.borrow_mut();
                let a = b.as_mut().unwrap();
                a.video_exit = None;
                a.exit_after_video_exports = false;
            });
            status(
                window,
                &format!(
                    "{} {e}",
                    text("Save failed; original retained.", "บันทึกไม่สำเร็จ ต้นฉบับยังอยู่")
                ),
            );
        }
    }
    let exiting = STATE.with(|s| s.borrow().as_ref().unwrap().exit_after_video_exports);
    if exiting && !saving() {
        STATE.with(|s| s.borrow_mut().as_mut().unwrap().exit_after_video_exports = false);
        super::exit_app();
    }
}

fn decision(window: HWND) -> i32 {
    let Some(_modal) = project_ui::Modal::enter() else {
        return IDCANCEL;
    };
    unsafe {
        ShowWindow(window, SW_RESTORE);
        SetForegroundWindow(window);
    }
    let title = wide(DIALOG_TITLE);
    let heading = wide(&text("Keep this recording?", "เก็บคลิปนี้หรือไม่?"));
    let content = wide(&text(
        "This clip has not been copied or saved. Keep Local retains it in the recording folder.",
        "คลิปนี้ยังไม่ได้คัดลอกหรือบันทึก เลือกเก็บไว้ในเครื่องเพื่อคงคลิปไว้ในโฟลเดอร์บันทึก",
    ));
    let labels = [
        wide(&text("Save MP4", "บันทึก MP4")),
        wide(&text("Keep Local", "เก็บไว้ในเครื่อง")),
        wide(&tr("cancel")),
    ];
    let buttons = [IDYES, IDNO, IDCANCEL]
        .into_iter()
        .zip(&labels)
        .map(|(id, label)| TASKDIALOG_BUTTON {
            nButtonID: id,
            pszButtonText: label.as_ptr(),
        })
        .collect::<Vec<_>>();
    let config = TASKDIALOGCONFIG {
        cbSize: size_of::<TASKDIALOGCONFIG>() as u32,
        hwndParent: window,
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
        pszWindowTitle: title.as_ptr(),
        pszMainInstruction: heading.as_ptr(),
        pszContent: content.as_ptr(),
        cButtons: 3,
        pButtons: buttons.as_ptr(),
        nDefaultButton: IDCANCEL,
        cxWidth: TASK_DIALOG_WIDTH,
        ..Default::default()
    };
    let mut chosen = IDCANCEL;
    if unsafe { TaskDialogIndirect(&config, &mut chosen, null_mut(), null_mut()) } < 0 {
        let fallback = text(
            "Yes: Save MP4\nNo: Keep Local\nCancel: Back to preview",
            "Yes: บันทึก MP4\nNo: เก็บไว้ในเครื่อง\nCancel: กลับพรีวิว",
        );
        chosen = unsafe {
            MessageBoxW(
                window,
                wide(&fallback).as_ptr(),
                title.as_ptr(),
                MB_YESNOCANCEL | MB_ICONQUESTION | MB_DEFBUTTON3,
            )
        };
    }
    chosen
}
pub(super) fn close(window: HWND) {
    if !allowed() {
        return;
    }
    let state = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .video_previews
            .get(&(window as usize))
            .map(|p| (p.clip.saving, p.clip.needs_review()))
    });
    match state {
        Some((true, _)) => (),
        Some((false, false)) => destroy(window),
        Some((false, true)) => match decision(window) {
            IDYES => {
                save(window, After::Close);
            }
            IDNO => destroy(window),
            _ => (),
        },
        None => (),
    }
}
fn destroy(window: HWND) {
    let player = STATE.with(|s| {
        s.borrow_mut()
            .as_mut()
            .and_then(|a| a.video_previews.get_mut(&(window as usize)))
            .and_then(|p| p.player.take())
    });
    drop(player);
    unsafe {
        DestroyWindow(window);
    }
}

pub(super) fn start_exit() {
    if STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.video_exit.is_some() || a.project_exit.is_some() || a.project_modal
    }) {
        return;
    }
    STATE.with(|s| s.borrow_mut().as_mut().unwrap().exit_after_video_exports = false);
    let windows = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.video_previews
            .iter()
            .map(|(w, p)| (*w, p.clip.identity))
            .collect::<Vec<_>>()
    });
    STATE.with(|s| {
        s.borrow_mut().as_mut().unwrap().video_exit = Some(ExitPlan { windows, index: 0 })
    });
    pause_all();
    advance_exit();
}
fn advance_exit() {
    loop {
        let next = STATE.with(|s| {
            let b = s.borrow();
            let plan = b.as_ref()?.video_exit.as_ref()?;
            Some(plan.windows.get(plan.index).copied())
        });
        let Some(next) = next else { return };
        let Some((owner, identity)) = next else {
            STATE.with(|s| s.borrow_mut().as_mut().unwrap().video_exit = None);
            project_ui::start_exit();
            return;
        };
        let state = STATE.with(|s| {
            s.borrow()
                .as_ref()
                .unwrap()
                .video_previews
                .get(&owner)
                .filter(|p| p.clip.identity == identity)
                .map(|p| p.clip.needs_review())
        });
        let choice = match state {
            Some(true) => decision(owner as HWND),
            Some(false) => IDNO,
            None => IDCANCEL,
        };
        if choice == IDCANCEL {
            STATE.with(|s| s.borrow_mut().as_mut().unwrap().video_exit = None);
            return;
        }
        STATE.with(|s| {
            if let Some(plan) = s.borrow_mut().as_mut().unwrap().video_exit.as_mut() {
                plan.index += 1;
            }
        });
        if choice == IDYES {
            if !save(owner as HWND, After::Exit) {
                STATE.with(|s| s.borrow_mut().as_mut().unwrap().video_exit = None);
            }
            return;
        }
    }
}

pub(super) unsafe fn procedure(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            WM_COMMAND => {
                if wparam >> 16 == BN_CLICKED as usize {
                    command(window, (wparam & 0xffff) as i32);
                }
                0
            }
            WM_HSCROLL => {
                if !allowed() {
                    return 0;
                }
                let id = GetDlgCtrlID(lparam as HWND);
                if id == SEEK {
                    let code = (wparam & 0xffff) as u32;
                    STATE.with(|s| {
                        if let Some(p) = s
                            .borrow_mut()
                            .as_mut()
                            .unwrap()
                            .video_previews
                            .get_mut(&(window as usize))
                        {
                            p.dragging = code == TB_THUMBTRACK;
                        }
                    });
                    if code != TB_THUMBTRACK {
                        let pos = SendMessageW(lparam as HWND, TBM_GETPOS, 0, 0) as f64 / 10000.;
                        if let Some(Err(e)) = with_player(window, |p| {
                            p.snapshot().and_then(|s| p.seek(s.duration * pos))
                        }) {
                            status(window, &e);
                        }
                    }
                } else if id == VOLUME {
                    let value = SendMessageW(lparam as HWND, TBM_GETPOS, 0, 0) as f64 / 100.;
                    with_player(window, |p| p.volume(value));
                }
                0
            }
            WM_TIMER if wparam == TICK => {
                tick(window);
                0
            }
            WM_SIZE => {
                layout(window);
                0
            }
            WM_DPICHANGED => {
                let r = &*(lparam as *const RECT);
                SetWindowPos(
                    window,
                    null_mut(),
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                refresh(window);
                layout(window);
                0
            }
            WM_GETMINMAXINFO => {
                let p = &mut *(lparam as *mut MINMAXINFO);
                p.ptMinTrackSize = POINT {
                    x: scale(window, 640),
                    y: scale(window, 460),
                };
                0
            }
            WM_CLOSE => {
                close(window);
                0
            }
            WM_PAINT => {
                let mut ps = zeroed();
                let dc = BeginPaint(window, &mut ps);
                FillRect(dc, &ps.rcPaint, GetSysColorBrush(COLOR_WINDOW));
                EndPaint(window, &ps);
                0
            }
            WM_CTLCOLORSTATIC => {
                SetBkMode(wparam as HDC, TRANSPARENT as i32);
                SetTextColor(wparam as HDC, 0x00665649);
                GetStockObject(WHITE_BRUSH) as isize
            }
            WM_DESTROY => {
                KillTimer(window, TICK);
                let preview = STATE.with(|s| {
                    s.borrow_mut()
                        .as_mut()
                        .and_then(|a| a.video_previews.remove(&(window as usize)))
                });
                if let Some(mut p) = preview {
                    drop(p.player.take());
                    drop(p.tips.take());
                    if p.icon_font != 0 {
                        DeleteObject(p.icon_font as HGDIOBJ);
                    }
                }
                0
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}

use super::*;
use crate::combine::{self, Layout, Request, Source};

const CLASS: &str = "SnapZyRustPreview.Combine";
const MODE: i32 = 1200;
const GAP: i32 = 1201;
const SIZE: i32 = 1202;
const ROW: i32 = 1300;
const UP: i32 = 1400;
const DOWN: i32 = 1500;

struct Panel {
    sources: Vec<Source>,
    thai: bool,
    done: bool,
    request: Option<Request>,
}

fn text(thai: bool, en: &str, th: &str) -> String {
    if thai { th } else { en }.into()
}

fn busy(a: &App) -> bool {
    a.capture_busy
        || a.selection.is_some()
        || a.scrolling.is_some()
        || a.importing
        || a.project_opening
        || a.project_modal
        || a.project_exit.is_some()
        || a.text_entry.is_some()
        || a.effects_open
}

fn refresh_editors() {
    let windows: Vec<_> = STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .previews
            .keys()
            .copied()
            .collect()
    });
    for window in windows {
        update_editor(window as HWND);
    }
}

pub(super) fn begin(owner: HWND) {
    workspace_ui::commit_numeric();
    let input = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        let p = a.previews.get(&(owner as usize))?;
        if busy(a) || a.previews.len() >= 16 {
            return None;
        }
        let mut sources: Vec<_> = a
            .previews
            .iter()
            .map(|(&window, p)| Source {
                owner: window,
                identity: p.guard.token.identity,
                title: p
                    .project_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| {
                        format!(
                            "{} {}",
                            if a.settings.language == "th" {
                                "ภาพ"
                            } else {
                                "Image"
                            },
                            p.guard.token.identity
                        )
                    }),
                width: p.session.document.crop.width,
                height: p.session.document.crop.height,
                checked: window == owner as usize,
            })
            .collect();
        sources.sort_by_key(|s| s.identity);
        Some((sources, p.guard.token.identity, a.settings.language == "th"))
    });
    let Some((sources, identity, thai)) = input else {
        return;
    };
    if sources.len() < 2 {
        notify(
            owner,
            &text(
                thai,
                "Open at least two images before combining.",
                "เปิดภาพอย่างน้อยสองภาพก่อนรวมภาพ",
            ),
            false,
        );
        return;
    }
    STATE.with(|s| s.borrow_mut().as_mut().unwrap().project_modal = true);
    let route = workspace_ui::route(owner);
    let choice = show(workspace_ui::native_owner(owner), sources, thai);
    STATE.with(|s| s.borrow_mut().as_mut().unwrap().project_modal = false);
    let choice = match choice {
        Ok(Some(r)) => r,
        Ok(None) => return,
        Err(e) => {
            notify(owner, &e, true);
            return;
        }
    };
    let work = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        if busy(a)
            || a.previews.len() >= 16
            || source_bytes(a) + 4 > crate::image_io::SOURCE_LIMIT
            || !a
                .previews
                .get(&(owner as usize))
                .is_some_and(|p| p.guard.token.identity == identity)
        {
            return None;
        }
        let mut snapshots = vec![];
        for &(window, expected) in &choice.sources {
            let p = a.previews.get(&window)?;
            let session = &p.session;
            if p.guard.token.identity != expected
                || p.saving
                || p.importing
                || session.gesture.is_some()
                || session.resize.is_some()
                || session.shape_resize.is_some()
                || session.tail_drag.is_some()
                || session.curve_drag.is_some()
            {
                return None;
            }
            snapshots.push((p.frame.clone(), session.document.render_snapshot()));
        }
        a.combine_assets = snapshots
            .iter()
            .flat_map(|(f, d)| std::iter::once(f.clone()).chain(d.image_sources()))
            .collect();
        a.project_opening = true;
        a.project_reserved = 4;
        Some((snapshots, a.tx.clone(), a.main))
    });
    let Some((snapshots, tx, main)) = work else {
        notify(
            owner,
            &text(
                thai,
                "A source closed, is busy, has an unfinished edit, or the image limit was reached. Finish the edit and try again.",
                "ภาพต้นทางถูกปิด กำลังทำงาน มีการแก้ไขที่ยังไม่เสร็จ หรือเกินขีดจำกัด กรุณาแก้ไขให้เสร็จแล้วลองใหม่",
            ),
            true,
        );
        return;
    };
    let started = Instant::now();
    refresh_editors();
    let owner = owner as usize;
    std::thread::spawn(move || {
        let result = combine::build(&snapshots, choice.layout, choice.gap).and_then(|project| {
            let preview = render::flatten(&project.frame, &project.document)?;
            Ok((project, preview))
        });
        let _ = tx.send(Work::Combined {
            route,
            owner,
            identity,
            started,
            result,
        });
        unsafe {
            PostMessageW(main as HWND, COMPLETE, 0, 0);
        }
    });
}

pub(super) fn finished(
    route: crate::workspace::Route,
    owner: usize,
    identity: u64,
    started: Instant,
    result: Result<(crate::project::Project, Arc<Frame>), String>,
) {
    let live = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        a.project_opening = false;
        a.project_reserved = 0;
        a.combine_assets.clear();
        a.previews
            .get(&owner)
            .is_some_and(|p| p.guard.token.identity == identity)
    });
    refresh_editors();
    if !live {
        return;
    }
    if STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .workspace
            .resolve(route)
            .is_err()
    }) {
        return;
    }
    match result {
        Ok((project, preview)) => {
            let revision = project.document.revision;
            present_project_cached(
                project.frame,
                Some(project.document),
                None,
                started,
                Some((revision, preview)),
                route,
                None,
            );
        }
        Err(error) => notify(owner as HWND, &error, true),
    }
}

unsafe fn refresh(window: HWND, state: &Panel) {
    unsafe {
        for (i, source) in state.sources.iter().enumerate() {
            let title: String = source.title.chars().take(46).collect();
            let label = format!(
                "{}. {}   {} x {} px",
                i + 1,
                title,
                source.width,
                source.height
            );
            SetWindowTextW(GetDlgItem(window, ROW + i as i32), wide(&label).as_ptr());
            SendMessageW(
                GetDlgItem(window, ROW + i as i32),
                BM_SETCHECK,
                if source.checked {
                    BST_CHECKED
                } else {
                    BST_UNCHECKED
                } as usize,
                0,
            );
            EnableWindow(GetDlgItem(window, UP + i as i32), (i > 0) as i32);
            EnableWindow(
                GetDlgItem(window, DOWN + i as i32),
                (i + 1 < state.sources.len()) as i32,
            );
        }
        update_size(window, state);
    }
}

unsafe fn current(window: HWND, state: &Panel) -> Result<Request, String> {
    unsafe {
        let mode = match SendMessageW(GetDlgItem(window, MODE), CB_GETCURSEL, 0, 0) {
            1 => Layout::Horizontal,
            2 => Layout::Free,
            _ => Layout::Vertical,
        };
        let mut buffer = [0u16; 32];
        let n = GetWindowTextW(GetDlgItem(window, GAP), buffer.as_mut_ptr(), 32);
        combine::request(
            &state.sources,
            mode,
            &String::from_utf16_lossy(&buffer[..n.max(0) as usize]),
        )
    }
}

unsafe fn update_size(window: HWND, state: &Panel) {
    unsafe {
        let free = SendMessageW(GetDlgItem(window, MODE), CB_GETCURSEL, 0, 0) == 2;
        EnableWindow(GetDlgItem(window, GAP), (!free) as i32);
        let request = current(window, state);
        let label = request
            .as_ref()
            .map(|r| format!("{} x {} px", r.canvas.width, r.canvas.height))
            .unwrap_or_else(|_| {
                text(
                    state.thai,
                    "Select 2+ images; gap 0-500 px",
                    "เลือกอย่างน้อย 2 ภาพ ระยะห่าง 0–500 px",
                )
            });
        SetWindowTextW(GetDlgItem(window, SIZE), wide(&label).as_ptr());
        EnableWindow(GetDlgItem(window, 1), request.is_ok() as i32);
    }
}

unsafe extern "system" fn procedure(window: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(l as *const CREATESTRUCTW);
            SetWindowLongPtrW(window, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let ptr = GetWindowLongPtrW(window, GWLP_USERDATA) as *mut Panel;
        if ptr.is_null() {
            return DefWindowProcW(window, message, w, l);
        }
        let state = &mut *ptr;
        match message {
            WM_CLOSE | WM_DESTROY => {
                state.done = true;
                0
            }
            WM_COMMAND => {
                let id = (w & 0xffff) as i32;
                if id == 2 {
                    state.done = true
                } else if id == 1 {
                    if let Ok(r) = current(window, state) {
                        state.request = Some(r);
                        state.done = true
                    }
                } else if id >= ROW && id < ROW + state.sources.len() as i32 {
                    state.sources[(id - ROW) as usize].checked =
                        SendMessageW(GetDlgItem(window, id), BM_GETCHECK, 0, 0)
                            == BST_CHECKED as isize;
                    update_size(window, state);
                } else if id >= UP && id < UP + state.sources.len() as i32 {
                    let i = (id - UP) as usize;
                    if i > 0 {
                        state.sources.swap(i, i - 1);
                        refresh(window, state)
                    }
                } else if id >= DOWN && id < DOWN + state.sources.len() as i32 {
                    let i = (id - DOWN) as usize;
                    if i + 1 < state.sources.len() {
                        state.sources.swap(i, i + 1);
                        refresh(window, state)
                    }
                } else if id == MODE || id == GAP {
                    update_size(window, state)
                }
                0
            }
            _ => DefWindowProcW(window, message, w, l),
        }
    }
}

fn show(owner: HWND, sources: Vec<Source>, thai: bool) -> Result<Option<Request>, String> {
    unsafe {
        let instance = GetModuleHandleW(null());
        let name = wide(CLASS);
        let mut class: WNDCLASSW = zeroed();
        class.lpfnWndProc = Some(procedure);
        class.hInstance = instance;
        class.lpszClassName = name.as_ptr();
        class.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        class.hbrBackground = (COLOR_WINDOW + 1) as HBRUSH;
        RegisterClassW(&class);
        let mut state = Box::new(Panel {
            sources,
            thai,
            done: false,
            request: None,
        });
        let dpi = GetDpiForWindow(owner).max(96) as i32;
        let scale = |n: i32| n * dpi / 96;
        let bottom = 20 + state.sources.len() as i32 * 30;
        let mut bounds = RECT {
            left: 0,
            top: 0,
            right: scale(590),
            bottom: scale(bottom + 140),
        };
        AdjustWindowRectEx(
            &mut bounds,
            WS_CAPTION | WS_SYSMENU,
            0,
            WS_EX_DLGMODALFRAME | WS_EX_CONTROLPARENT,
        );
        let window = CreateWindowExW(
            WS_EX_DLGMODALFRAME | WS_EX_CONTROLPARENT,
            name.as_ptr(),
            wide(&text(thai, "Combine images", "รวมภาพ")).as_ptr(),
            WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            owner,
            null_mut(),
            instance,
            (&mut *state as *mut Panel).cast(),
        );
        if window.is_null() {
            return Err("Cannot open image combination.".into());
        }
        let child = |kind: &str,
                     label: &str,
                     id: i32,
                     x: i32,
                     y: i32,
                     width: i32,
                     height: i32,
                     flags: u32| {
            let c = CreateWindowExW(
                0,
                wide(kind).as_ptr(),
                wide(label).as_ptr(),
                WS_CHILD | WS_VISIBLE | flags,
                scale(x),
                scale(y),
                scale(width),
                scale(height),
                window,
                id as HMENU,
                instance,
                null(),
            );
            SendMessageW(c, WM_SETFONT, GetStockObject(DEFAULT_GUI_FONT) as usize, 1);
            c
        };
        for i in 0..state.sources.len() {
            let y = 16 + i as i32 * 30;
            child(
                "BUTTON",
                "",
                ROW + i as i32,
                16,
                y,
                470,
                26,
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
            );
            child("BUTTON", "↑", UP + i as i32, 494, y, 34, 26, WS_TABSTOP);
            child("BUTTON", "↓", DOWN + i as i32, 534, y, 34, 26, WS_TABSTOP);
        }
        child(
            "STATIC",
            &text(thai, "Layout", "การจัดวาง"),
            1210,
            16,
            bottom + 4,
            90,
            24,
            0,
        );
        let modes = child(
            "COMBOBOX",
            "",
            MODE,
            112,
            bottom,
            200,
            140,
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        );
        for label in [
            text(thai, "Vertical", "แนวตั้ง"),
            text(thai, "Horizontal", "แนวนอน"),
            text(thai, "Free", "อิสระ"),
        ] {
            SendMessageW(modes, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        }
        SendMessageW(modes, CB_SETCURSEL, 0, 0);
        child(
            "STATIC",
            &text(thai, "Gap (px)", "ระยะห่าง (px)"),
            1211,
            328,
            bottom + 4,
            136,
            24,
            0,
        );
        let gap = child(
            "EDIT",
            "16",
            GAP,
            480,
            bottom,
            88,
            28,
            WS_TABSTOP | WS_BORDER | ES_NUMBER as u32,
        );
        SendMessageW(gap, EM_SETLIMITTEXT, 3, 0);
        child("STATIC", "", SIZE, 16, bottom + 44, 552, 24, 0);
        child(
            "BUTTON",
            &text(thai, "Combine", "รวมภาพ"),
            1,
            340,
            bottom + 88,
            110,
            32,
            WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
        );
        child(
            "BUTTON",
            &text(thai, "Cancel", "ยกเลิก"),
            2,
            458,
            bottom + 88,
            110,
            32,
            WS_TABSTOP,
        );
        refresh(window, &state);
        EnableWindow(owner, 0);
        ShowWindow(window, SW_SHOW);
        SetForegroundWindow(window);
        SetFocus(GetDlgItem(window, ROW));
        let mut message: MSG = zeroed();
        while !state.done {
            let status = GetMessageW(&mut message, null_mut(), 0, 0);
            if status <= 0 {
                if status == 0 {
                    PostQuitMessage(message.wParam as i32)
                }
                break;
            }
            if IsDialogMessageW(window, &message) == 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        DestroyWindow(window);
        if IsWindow(owner) != 0 {
            EnableWindow(owner, 1);
            SetForegroundWindow(owner);
        }
        Ok(state.request.take())
    }
}

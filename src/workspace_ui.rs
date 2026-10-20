use super::*;
use crate::workspace::{Key, Route};
use windows_sys::Win32::UI::Controls::*;

pub(super) const HOST: &str = "SnapZyRustPreview.Workspace";
const TABS: i32 = 1600;

pub(super) struct Tips {
    window: HWND,
    texts: Vec<Vec<u16>>,
    geometry: Vec<(i32, i32, i32, i32)>,
    labels: Vec<String>,
    thai: bool,
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
fn refresh_tips(host: HWND, labels: &[String]) {
    let (old, thai) = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        (
            a.tab_tips.remove(&(host as usize)),
            a.settings.language == "th",
        )
    });
    unsafe {
        let tabs = GetDlgItem(host, TABS);
        let mut tips = old.unwrap_or_else(|| Tips {
            window: CreateWindowExW(
                WS_EX_TOPMOST,
                TOOLTIPS_CLASSW,
                null(),
                WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
                0,
                0,
                0,
                0,
                host,
                null_mut(),
                GetModuleHandleW(null()),
                null(),
            ),
            texts: vec![],
            geometry: vec![],
            labels: vec![],
            thai,
        });
        let geometry = (0..labels.len())
            .flat_map(|i| {
                let mut r: RECT = zeroed();
                SendMessageW(tabs, TCM_GETITEMRECT, i, &mut r as *mut _ as isize);
                [
                    (r.left, r.top, r.right - 24, r.bottom),
                    (r.right - 24, r.top, r.right, r.bottom),
                ]
            })
            .collect::<Vec<_>>();
        if tips.geometry != geometry || tips.labels != labels || tips.thai != thai {
            for i in 0..tips.texts.len() {
                let info = TTTOOLINFOW {
                    cbSize: size_of::<TTTOOLINFOW>() as u32,
                    hwnd: tabs,
                    uId: i + 1,
                    ..zeroed()
                };
                SendMessageW(tips.window, TTM_DELTOOLW, 0, &info as *const _ as isize);
            }
            tips.texts = labels
                .iter()
                .flat_map(|label| {
                    [
                        wide(label),
                        wide(&format!(
                            "{}\n{label}",
                            if thai {
                                "ปิดแท็บ"
                            } else {
                                "Close tab"
                            }
                        )),
                    ]
                })
                .collect();
            for (i, &(left, top, right, bottom)) in geometry.iter().enumerate() {
                let info = TTTOOLINFOW {
                    cbSize: size_of::<TTTOOLINFOW>() as u32,
                    hwnd: tabs,
                    uId: i + 1,
                    uFlags: TTF_SUBCLASS,
                    rect: RECT {
                        left,
                        top,
                        right,
                        bottom,
                    },
                    lpszText: tips.texts[i].as_mut_ptr(),
                    ..zeroed()
                };
                SendMessageW(tips.window, TTM_ADDTOOLW, 0, &info as *const _ as isize);
            }
            SendMessageW(
                tips.window,
                TTM_SETMAXTIPWIDTH,
                0,
                scale(host, 600) as isize,
            );
            tips.geometry = geometry;
            tips.labels = labels.to_vec();
            tips.thai = thai;
        }
        STATE.with(|s| {
            s.borrow_mut()
                .as_mut()
                .unwrap()
                .tab_tips
                .insert(host as usize, tips);
        });
    }
}

pub(super) fn doc_key(a: &App, window: usize) -> Option<Key> {
    a.previews.get(&window).map(|p| Key {
        window,
        identity: p.guard.token.identity,
    })
}
pub(super) fn route(owner: HWND) -> Route {
    STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.workspace
            .route(a.settings.open_mode, doc_key(a, owner as usize))
    })
}
pub(super) fn native_owner(window: HWND) -> HWND {
    unsafe { GetAncestor(window, GA_ROOT) }
}
pub(super) fn commit_numeric() {
    unsafe {
        let focus = GetFocus();
        if [ec::WIDTH, ec::STROKE, ec::OPACITY].contains(&(GetDlgCtrlID(focus) as usize))
            && class_name(GetParent(focus)) == PREVIEW
        {
            SetFocus(GetParent(focus));
        }
    }
}
pub(super) fn active(window: HWND) -> Option<HWND> {
    STATE.with(|s| {
        s.borrow()
            .as_ref()?
            .workspace
            .hosts
            .iter()
            .find(|h| h.key.window == window as usize)?
            .active
            .map(|d| d.window as HWND)
    })
}
pub(super) fn selection(owner: HWND) -> Option<Key> {
    let host = native_owner(owner);
    STATE.with(|s| {
        s.borrow()
            .as_ref()?
            .workspace
            .hosts
            .iter()
            .find(|h| h.key.window == host as usize)?
            .active
    })
}
pub(super) fn restore_selection(doc: Option<Key>) {
    let Some(doc) = doc else {
        return;
    };
    let host = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut()?;
        let host = a.workspace.host_for(doc)?;
        a.workspace.select(host, doc);
        Some(host.window as HWND)
    });
    if let Some(host) = host {
        refresh(host);
        unsafe {
            SetFocus(doc.window as HWND);
        }
    }
}
pub(super) fn can_switch(window: HWND) -> bool {
    STATE.with(|s| {
        let b = s.borrow();
        let Some(a) = b.as_ref() else {
            return false;
        };
        if a.project_modal || a.project_exit.is_some() || a.text_entry.is_some() || a.effects_open {
            return false;
        }
        let Some(d) = a
            .workspace
            .hosts
            .iter()
            .find(|h| h.key.window == window as usize)
            .and_then(|h| h.active)
        else {
            return true;
        };
        let Some(p) = a.previews.get(&d.window) else {
            return false;
        };
        !unfinished(&p.session)
    })
}
pub(super) fn unfinished(s: &Session) -> bool {
    s.gesture.is_some()
        || s.resize.is_some()
        || s.shape_resize.is_some()
        || s.tail_drag.is_some()
        || s.curve_drag.is_some()
}
pub(super) fn destination(route: Route) -> Option<HWND> {
    let target = STATE
        .with(|s| s.borrow().as_ref().unwrap().workspace.resolve(route))
        .ok()?;
    if let Some(h) = target {
        return Some(h.window as HWND);
    }
    let host = create_window(
        HOST,
        TITLE,
        WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
        WS_EX_CONTROLPARENT,
        null_mut(),
        800,
        616,
    );
    if host.is_null() {
        return None;
    }
    STATE.with(|s| {
        s.borrow_mut()
            .as_mut()
            .unwrap()
            .workspace
            .add_host(host as usize);
    });
    unsafe {
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_TAB_CLASSES,
        });
        let tabs = child(
            host,
            "SysTabControl32",
            "",
            TABS,
            WS_TABSTOP | TCS_FIXEDWIDTH | TCS_OWNERDRAWFIXED | TCS_FOCUSNEVER,
        );
        SendMessageW(
            tabs,
            TCM_SETITEMSIZE,
            0,
            ((scale(host, 30) << 16) | scale(host, 152)) as isize,
        );
        SetWindowSubclass(tabs, Some(tab_proc), 1, 0);
    }
    Some(host)
}
pub(super) fn attach(host: HWND, doc: HWND) -> bool {
    let accepted = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        let Some(key) = doc_key(a, doc as usize) else {
            return false;
        };
        let Some(h) = a
            .workspace
            .hosts
            .iter()
            .find(|h| h.key.window == host as usize)
            .map(|h| h.key)
        else {
            return false;
        };
        a.workspace.add_doc(h, key)
    });
    if accepted {
        refresh(host);
    }
    accepted
}
pub(super) fn refresh_doc(doc: HWND) {
    let host = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref()?;
        a.workspace
            .host_for(doc_key(a, doc as usize)?)
            .map(|h| h.window as HWND)
    });
    if let Some(host) = host {
        refresh(host);
    }
}
pub(super) fn refresh(host: HWND) {
    let data = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref()?;
        let h = a
            .workspace
            .hosts
            .iter()
            .find(|h| h.key.window == host as usize)?;
        let labels = h
            .tabs
            .iter()
            .map(|d| {
                let p = &a.previews[&d.window];
                let name = p
                    .project_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .or_else(|| p.source_label.clone())
                    .unwrap_or_else(|| {
                        format!(
                            "{} {}",
                            if a.settings.language == "th" {
                                "ภาพ"
                            } else {
                                "Image"
                            },
                            d.identity
                        )
                    });
                format!(
                    "{}{}",
                    if p.session.document.is_project_saved() {
                        ""
                    } else {
                        "* "
                    },
                    name
                )
            })
            .collect::<Vec<_>>();
        Some((h.tabs.clone(), h.active, labels))
    });
    let Some((docs, selected, labels)) = data else {
        return;
    };
    unsafe {
        let tabs = GetDlgItem(host, TABS);
        let count = SendMessageW(tabs, TCM_GETITEMCOUNT, 0, 0) as usize;
        for i in (docs.len()..count).rev() {
            SendMessageW(tabs, TCM_DELETEITEM, i, 0);
        }
        for (i, label) in labels.iter().enumerate() {
            let mut name = wide(label);
            let item = TCITEMW {
                mask: TCIF_TEXT,
                pszText: name.as_mut_ptr(),
                ..zeroed()
            };
            SendMessageW(
                tabs,
                if i < count {
                    TCM_SETITEMW
                } else {
                    TCM_INSERTITEMW
                },
                i,
                &item as *const _ as isize,
            );
        }
        if let Some(index) = docs.iter().position(|d| Some(*d) == selected) {
            SendMessageW(tabs, TCM_SETCURSEL, index, 0);
            SetWindowTextW(host, wide(&format!("{TITLE} - {}", labels[index])).as_ptr());
        }
        for d in docs {
            ShowWindow(
                d.window as HWND,
                if Some(d) == selected {
                    SW_SHOWNA
                } else {
                    SW_HIDE
                },
            );
        }
        layout_host(host);
        refresh_tips(host, &labels);
        InvalidateRect(tabs, null(), 0);
    }
}
pub(super) fn layout_host(host: HWND) {
    let docs = STATE
        .with(|s| {
            s.borrow().as_ref().and_then(|a| {
                a.workspace
                    .hosts
                    .iter()
                    .find(|h| h.key.window == host as usize)
                    .map(|h| h.tabs.clone())
            })
        })
        .unwrap_or_default();
    let r = client(host);
    let height = scale(host, 36);
    unsafe {
        SendMessageW(
            GetDlgItem(host, TABS),
            WM_SETFONT,
            font(host, 14, FW_NORMAL as i32) as usize,
            0,
        );
        SendMessageW(
            GetDlgItem(host, TABS),
            TCM_SETITEMSIZE,
            0,
            ((scale(host, 30) << 16) | scale(host, 152)) as isize,
        );
        MoveWindow(GetDlgItem(host, TABS), 0, 0, r.right, height, 1);
        for doc in docs {
            let window = doc.window as HWND;
            let current = client(window);
            if current.right != r.right || current.bottom != (r.bottom - height).max(1) {
                MoveWindow(window, 0, height, r.right, (r.bottom - height).max(1), 1);
            } else {
                SetWindowPos(
                    window,
                    null_mut(),
                    0,
                    height,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
}
pub(super) fn select(host: HWND, index: usize) {
    if !can_switch(host) {
        return;
    }
    unsafe {
        if let Some(doc) = active(host) {
            SetFocus(doc);
        }
    }
    let doc = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut()?;
        let h = a
            .workspace
            .hosts
            .iter()
            .find(|h| h.key.window == host as usize)?;
        let (key, doc) = (h.key, *h.tabs.get(index)?);
        a.workspace.select(key, doc);
        Some(doc.window as HWND)
    });
    refresh(host);
    if let Some(doc) = doc {
        unsafe {
            SetFocus(doc);
        }
    }
}
pub(super) fn cycle(host: HWND, back: bool) {
    if !can_switch(host) {
        return;
    }
    unsafe {
        if let Some(doc) = active(host) {
            SetFocus(doc);
        }
    }
    let doc = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut()?;
        let key = a
            .workspace
            .hosts
            .iter()
            .find(|h| h.key.window == host as usize)?
            .key;
        a.workspace.cycle(key, back)
    });
    refresh(host);
    if let Some(doc) = doc {
        unsafe {
            SetFocus(doc.window as HWND);
        }
    }
}
pub(super) fn remove_empty(host: HWND) {
    let removed = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        let key = a
            .workspace
            .hosts
            .iter()
            .find(|h| h.key.window == host as usize)
            .map(|h| h.key);
        key.is_some_and(|key| a.workspace.remove_empty_host(key))
    });
    if removed {
        let tips = STATE.with(|s| {
            s.borrow_mut()
                .as_mut()
                .unwrap()
                .tab_tips
                .remove(&(host as usize))
        });
        drop(tips);
        unsafe {
            DestroyWindow(host);
        }
    }
}
pub(super) fn reveal(doc: HWND) {
    let host = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut()?;
        let key = doc_key(a, doc as usize)?;
        let h = a.workspace.host_for(key)?;
        a.workspace.select(h, key);
        Some(h.window as HWND)
    });
    if let Some(host) = host {
        refresh(host);
        unsafe {
            ShowWindow(host, SW_RESTORE);
            SetForegroundWindow(host);
            SetFocus(doc);
        }
    }
}
pub(super) fn activate(host: HWND) {
    STATE.with(|s| {
        let mut b = s.borrow_mut();
        let Some(a) = b.as_mut() else {
            return;
        };
        if let Some(key) = a
            .workspace
            .hosts
            .iter()
            .find(|h| h.key.window == host as usize)
            .map(|h| h.key)
        {
            a.workspace.activate(key);
        }
    });
}
pub(super) unsafe fn notification(l: LPARAM) -> LRESULT {
    unsafe {
        let n = &*(l as *const NMHDR);
        let host = GetParent(n.hwndFrom);
        match n.code {
            TCN_SELCHANGING => (!can_switch(host)) as isize,
            TCN_SELCHANGE => {
                select(
                    host,
                    SendMessageW(n.hwndFrom, TCM_GETCURSEL, 0, 0).max(0) as usize,
                );
                0
            }
            _ => 0,
        }
    }
}
pub(super) unsafe fn draw(item: &DRAWITEMSTRUCT) {
    unsafe {
        let mut buffer = vec![0u16; 32768];
        let mut data = TCITEMW {
            mask: TCIF_TEXT,
            pszText: buffer.as_mut_ptr(),
            cchTextMax: buffer.len() as i32,
            ..zeroed()
        };
        SendMessageW(
            item.hwndItem,
            TCM_GETITEMW,
            item.itemID as usize,
            &mut data as *mut _ as isize,
        );
        let len = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        let full = String::from_utf16_lossy(&buffer[..len]);
        let dirty = full.starts_with("* ");
        let path = full.strip_prefix("* ").unwrap_or(&full);
        let filename = std::path::Path::new(path)
            .file_name()
            .map(|p| p.to_string_lossy())
            .unwrap_or_else(|| path.into());
        let label = format!("{}{}", if dirty { "* " } else { "" }, filename);
        let mut r = item.rcItem;
        FillRect(
            item.hDC,
            &r,
            GetSysColorBrush(if item.itemState & ODS_SELECTED != 0 {
                COLOR_WINDOW
            } else {
                COLOR_BTNFACE
            }),
        );
        SetBkMode(item.hDC, TRANSPARENT as i32);
        SetTextColor(item.hDC, GetSysColor(COLOR_WINDOWTEXT));
        let old = SelectObject(
            item.hDC,
            font(GetParent(item.hwndItem), 13, FW_NORMAL as i32),
        );
        r.left += 6;
        r.right -= 24;
        DrawTextW(
            item.hDC,
            wide(&label).as_ptr(),
            -1,
            &mut r,
            DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        r.left = item.rcItem.right - 22;
        r.right = item.rcItem.right - 2;
        DrawTextW(
            item.hDC,
            wide("x").as_ptr(),
            -1,
            &mut r,
            DT_SINGLELINE | DT_VCENTER | DT_CENTER,
        );
        SelectObject(item.hDC, old);
    }
}
unsafe extern "system" fn tab_proc(
    window: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    unsafe {
        if msg == WM_MOUSEMOVE {
            let host = GetParent(window);
            let labels = STATE.with(|s| {
                s.borrow()
                    .as_ref()
                    .and_then(|a| a.tab_tips.get(&(host as usize)))
                    .map(|t| t.labels.clone())
            });
            if let Some(labels) = labels {
                refresh_tips(host, &labels);
            }
        }
        if msg == WM_LBUTTONDOWN {
            let x = (l as u32 & 0xffff) as u16 as i16 as i32;
            let y = ((l as u32 >> 16) & 0xffff) as u16 as i16 as i32;
            for i in 0..SendMessageW(window, TCM_GETITEMCOUNT, 0, 0) {
                let mut r: RECT = zeroed();
                SendMessageW(
                    window,
                    TCM_GETITEMRECT,
                    i as usize,
                    &mut r as *mut _ as isize,
                );
                if x >= r.right - 24 && x < r.right && y >= r.top && y < r.bottom {
                    let host = GetParent(window);
                    if can_switch(host) {
                        let doc = STATE.with(|s| {
                            s.borrow()
                                .as_ref()
                                .and_then(|a| {
                                    a.workspace
                                        .hosts
                                        .iter()
                                        .find(|h| h.key.window == host as usize)
                                })
                                .and_then(|h| h.tabs.get(i as usize).copied())
                        });
                        if let Some(doc) = doc {
                            PostMessageW(doc.window as HWND, CLOSE, doc.identity as usize, 0);
                        }
                    }
                    return 0;
                }
            }
        }
        if msg == WM_NCDESTROY {
            RemoveWindowSubclass(window, Some(tab_proc), 1);
        }
        DefSubclassProc(window, msg, w, l)
    }
}

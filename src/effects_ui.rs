use crate::{effects::Effects, ui::wide};
use std::{
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Controls::{Dialogs::*, *},
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::{EnableWindow, SetFocus},
        WindowsAndMessaging::*,
    },
};

const CLASS: &str = "SnapZyRustPreview.Effects";
const SHADOW: i32 = 1100;
const SHADOW_COLOR: i32 = 1101;
const SOFT: i32 = 1102;
const X: i32 = 1103;
const Y: i32 = 1104;
const OUTLINE: i32 = 1105;
const OUTLINE_COLOR: i32 = 1106;
const WIDTH: i32 = 1107;
struct Panel {
    fx: Effects,
    text: bool,
    thai: bool,
    done: bool,
    accepted: bool,
}
fn label(thai: bool, en: &str, th: &str) -> Vec<u16> {
    wide(if thai { th } else { en })
}

fn numbers(fx: Effects, text: bool, values: [&str; 4]) -> Result<Effects, String> {
    let invalid = || "Invalid numeric effect settings.".to_string();
    let mut next = fx;
    next.shadow.blur = values[0].trim().parse().map_err(|_| invalid())?;
    next.shadow.x = values[1].trim().parse().map_err(|_| invalid())?;
    next.shadow.y = values[2].trim().parse().map_err(|_| invalid())?;
    if text {
        next.outline.width = values[3].trim().parse().map_err(|_| invalid())?;
    }
    next.validate()
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
        match message {
            WM_CLOSE => {
                (*ptr).done = true;
                0
            }
            WM_DESTROY => {
                (*ptr).done = true;
                0
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
                SetBkMode(w as HDC, TRANSPARENT as i32);
                SetTextColor(w as HDC, 0);
                GetStockObject(WHITE_BRUSH) as isize
            }
            WM_COMMAND => {
                let id = (w & 0xffff) as i32;
                if w >> 16 != BN_CLICKED as usize {
                    return 0;
                }
                if matches!(id, SHADOW_COLOR | OUTLINE_COLOR) {
                    let rgb = if id == SHADOW_COLOR {
                        (*ptr).fx.shadow.rgb
                    } else {
                        (*ptr).fx.outline.rgb
                    };
                    let mut custom = [0u32; 16];
                    let mut dialog: CHOOSECOLORW = zeroed();
                    dialog.lStructSize = size_of::<CHOOSECOLORW>() as u32;
                    dialog.hwndOwner = window;
                    dialog.rgbResult =
                        rgb[0] as u32 | ((rgb[1] as u32) << 8) | ((rgb[2] as u32) << 16);
                    dialog.lpCustColors = custom.as_mut_ptr();
                    dialog.Flags = CC_RGBINIT | CC_FULLOPEN;
                    if ChooseColorW(&mut dialog) != 0 {
                        let c = dialog.rgbResult;
                        let rgb = [c as u8, (c >> 8) as u8, (c >> 16) as u8];
                        if id == SHADOW_COLOR {
                            (*ptr).fx.shadow.rgb = rgb;
                        } else {
                            (*ptr).fx.outline.rgb = rgb;
                        }
                        InvalidateRect(GetDlgItem(window, id), null(), 1);
                    }
                } else if id == 2 {
                    (*ptr).done = true;
                } else if id == 1 {
                    let texts = [SOFT, X, Y, WIDTH].map(|id| {
                        let mut buffer = [0u16; 32];
                        let n = GetWindowTextW(GetDlgItem(window, id), buffer.as_mut_ptr(), 32);
                        String::from_utf16_lossy(&buffer[..n.max(0) as usize])
                    });
                    let mut fx = (*ptr).fx;
                    fx.shadow.enabled = SendMessageW(GetDlgItem(window, SHADOW), BM_GETCHECK, 0, 0)
                        == BST_CHECKED as isize;
                    if (*ptr).text {
                        fx.outline.enabled =
                            SendMessageW(GetDlgItem(window, OUTLINE), BM_GETCHECK, 0, 0)
                                == BST_CHECKED as isize;
                    }
                    match numbers(
                        fx,
                        (*ptr).text,
                        [&texts[0], &texts[1], &texts[2], &texts[3]],
                    ) {
                        Ok(fx) => {
                            (*ptr).fx = fx;
                            (*ptr).accepted = true;
                            (*ptr).done = true;
                        }
                        Err(_) => {
                            MessageBoxW(
                                window,
                                label(
                                    (*ptr).thai,
                                    "Softness: 0-32 px; offsets: -32 to 32 px; outline: 1-16 px.",
                                    "ความฟุ้ง 0–32 px ระยะเงา -32 ถึง 32 px ขอบอักษร 1–16 px",
                                )
                                .as_ptr(),
                                label((*ptr).thai, "Check effect settings", "ตรวจสอบค่าเอฟเฟกต์")
                                    .as_ptr(),
                                MB_OK | MB_ICONWARNING,
                            );
                        }
                    }
                }
                0
            }
            WM_DRAWITEM => {
                let item = &*(l as *const DRAWITEMSTRUCT);
                let rgb = if item.CtlID as i32 == SHADOW_COLOR {
                    (*ptr).fx.shadow.rgb
                } else {
                    (*ptr).fx.outline.rgb
                };
                let bg = CreateSolidBrush(if item.itemState & ODS_DISABLED != 0 {
                    0xeeeeee
                } else {
                    rgb[0] as u32 | ((rgb[1] as u32) << 8) | ((rgb[2] as u32) << 16)
                });
                FillRect(item.hDC, &item.rcItem, bg);
                DeleteObject(bg);
                FrameRect(item.hDC, &item.rcItem, GetStockObject(GRAY_BRUSH) as HBRUSH);
                if item.itemState & ODS_FOCUS != 0 {
                    let mut r = item.rcItem;
                    InflateRect(&mut r, -3, -3);
                    DrawFocusRect(item.hDC, &r);
                }
                1
            }
            _ => DefWindowProcW(window, message, w, l),
        }
    }
}

pub fn show(owner: HWND, fx: Effects, text: bool, thai: bool) -> Result<Option<Effects>, String> {
    unsafe {
        let mut common: INITCOMMONCONTROLSEX = zeroed();
        common.dwSize = size_of::<INITCOMMONCONTROLSEX>() as u32;
        common.dwICC = ICC_UPDOWN_CLASS;
        InitCommonControlsEx(&common);
        let instance = GetModuleHandleW(null());
        let mut class: WNDCLASSW = zeroed();
        class.lpfnWndProc = Some(procedure);
        class.hInstance = instance;
        // Keep the class name alive while registering.
        let name = wide(CLASS);
        class.lpszClassName = name.as_ptr();
        class.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
        class.hbrBackground = (COLOR_WINDOW + 1) as HBRUSH;
        RegisterClassW(&class);
        let mut state = Box::new(Panel {
            fx,
            text,
            thai,
            done: false,
            accepted: false,
        });
        let dpi = GetDpiForWindow(owner).max(96) as i32;
        let scale = |v: i32| v * dpi / 96;
        let style = WS_CAPTION | WS_SYSMENU;
        let ex = WS_EX_DLGMODALFRAME | WS_EX_CONTROLPARENT;
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: scale(390),
            bottom: scale(356),
        };
        AdjustWindowRectEx(&mut rect, style, 0, ex);
        let mut parent: RECT = zeroed();
        GetWindowRect(owner, &mut parent);
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let window = CreateWindowExW(
            ex,
            name.as_ptr(),
            label(thai, "Effects", "เอฟเฟกต์").as_ptr(),
            style,
            parent.left + (parent.right - parent.left - width) / 2,
            parent.top + (parent.bottom - parent.top - height) / 2,
            width,
            height,
            owner,
            null_mut(),
            instance,
            (&mut *state as *mut Panel).cast(),
        );
        if window.is_null() {
            return Err("Cannot open Effects settings.".into());
        }
        let font = CreateFontW(
            -scale(14),
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
            wide(crate::ui::UI_FONT_FACE).as_ptr(),
        );
        let used_font = if font.is_null() {
            GetStockObject(DEFAULT_GUI_FONT)
        } else {
            font
        };
        let create =
            |kind: &str, title: &str, id: i32, x: i32, y: i32, w: i32, h: i32, flags: u32| {
                let child = CreateWindowExW(
                    0,
                    wide(kind).as_ptr(),
                    wide(title).as_ptr(),
                    WS_CHILD | WS_VISIBLE | flags,
                    scale(x),
                    scale(y),
                    scale(w),
                    scale(h),
                    window,
                    id as HMENU,
                    instance,
                    null(),
                );
                SendMessageW(child, WM_SETFONT, used_font as usize, 1);
                child
            };
        create(
            "BUTTON",
            if thai {
                "เงาวัตถุ"
            } else {
                "Object shadow"
            },
            SHADOW,
            20,
            16,
            250,
            26,
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        create(
            "STATIC",
            if thai {
                "สีเงา"
            } else {
                "Shadow color"
            },
            1120,
            20,
            56,
            160,
            24,
            0,
        );
        create(
            "BUTTON",
            "",
            SHADOW_COLOR,
            210,
            52,
            140,
            28,
            BS_OWNERDRAW as u32 | WS_TABSTOP,
        );
        let fields = [
            (
                SOFT,
                if thai {
                    "ความฟุ้ง (px)"
                } else {
                    "Softness (px)"
                },
                fx.shadow.blur as i32,
                0,
                32,
                92,
            ),
            (
                X,
                if thai {
                    "แนวนอน (px)"
                } else {
                    "Horizontal (px)"
                },
                fx.shadow.x,
                -32,
                32,
                130,
            ),
            (
                Y,
                if thai {
                    "แนวตั้ง (px)"
                } else {
                    "Vertical (px)"
                },
                fx.shadow.y,
                -32,
                32,
                168,
            ),
            (
                WIDTH,
                if thai {
                    "ความหนาขอบ (px)"
                } else {
                    "Outline width (px)"
                },
                fx.outline.width as i32,
                1,
                16,
                278,
            ),
        ];
        for (id, title, value, min, max, y) in fields {
            create("STATIC", title, id + 30, 20, y + 4, 180, 24, 0);
            let edit = create(
                "EDIT",
                &value.to_string(),
                id,
                210,
                y,
                140,
                28,
                WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            );
            SendMessageW(edit, EM_SETLIMITTEXT, 4, 0);
            let spin = create(
                "msctls_updown32",
                "",
                id + 100,
                330,
                y,
                20,
                28,
                UDS_ALIGNRIGHT | UDS_SETBUDDYINT | UDS_ARROWKEYS,
            );
            SendMessageW(spin, UDM_SETBUDDY, edit as usize, 0);
            SendMessageW(spin, UDM_SETRANGE32, min as isize as usize, max as isize);
            SendMessageW(spin, UDM_SETPOS32, 0, value as isize);
        }
        create(
            "BUTTON",
            if thai {
                "ขอบตัวอักษร"
            } else {
                "Text outline"
            },
            OUTLINE,
            20,
            206,
            270,
            26,
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        create(
            "STATIC",
            if thai {
                "สีขอบอักษร"
            } else {
                "Outline color"
            },
            1121,
            20,
            246,
            180,
            24,
            0,
        );
        create(
            "BUTTON",
            "",
            OUTLINE_COLOR,
            210,
            242,
            140,
            28,
            BS_OWNERDRAW as u32 | WS_TABSTOP,
        );
        create(
            "BUTTON",
            if thai { "ใช้" } else { "Apply" },
            1,
            180,
            320,
            80,
            28,
            BS_DEFPUSHBUTTON as u32 | WS_TABSTOP,
        );
        create(
            "BUTTON",
            if thai { "ยกเลิก" } else { "Cancel" },
            2,
            270,
            320,
            80,
            28,
            WS_TABSTOP,
        );
        SendMessageW(
            GetDlgItem(window, SHADOW),
            BM_SETCHECK,
            if fx.shadow.enabled {
                BST_CHECKED
            } else {
                BST_UNCHECKED
            } as usize,
            0,
        );
        SendMessageW(
            GetDlgItem(window, OUTLINE),
            BM_SETCHECK,
            if fx.outline.enabled {
                BST_CHECKED
            } else {
                BST_UNCHECKED
            } as usize,
            0,
        );
        for id in [OUTLINE, OUTLINE_COLOR, WIDTH, WIDTH + 100, WIDTH + 30, 1121] {
            EnableWindow(GetDlgItem(window, id), text as i32);
        }
        EnableWindow(owner, 0);
        ShowWindow(window, SW_SHOW);
        SetForegroundWindow(window);
        SetFocus(GetDlgItem(window, SHADOW));
        let mut message: MSG = zeroed();
        while !state.done {
            let status = GetMessageW(&mut message, null_mut(), 0, 0);
            if status <= 0 {
                if status == 0 {
                    PostQuitMessage(message.wParam as i32);
                }
                break;
            }
            if IsDialogMessageW(window, &message) == 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        DestroyWindow(window);
        if !font.is_null() {
            DeleteObject(font);
        }
        if IsWindow(owner) != 0 {
            EnableWindow(owner, 1);
            SetForegroundWindow(owner);
            SetFocus(owner);
        }
        Ok(state.accepted.then_some(state.fx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numeric_fields_reject_bad_values_without_partial_changes() {
        let fx = Effects::default();
        for values in [
            ["6", "33", "3", "2"],
            ["6", "-33", "3", "2"],
            ["NaN", "3", "3", "2"],
            ["6", "3", "3", "0"],
        ] {
            assert!(numbers(fx, true, values).is_err());
        }
        let next = numbers(fx, true, ["0", "-32", "32", "16"]).unwrap();
        assert_eq!(next.shadow.x, -32);
        assert_eq!(next.outline.width, 16);
        assert_eq!(numbers(fx, false, ["6", "3", "3", "ignored"]).unwrap(), fx);
    }
}

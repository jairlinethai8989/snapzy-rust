use crate::frame::{Frame, Rect};
use std::{
    ffi::c_void,
    mem::{size_of, zeroed},
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, RECT},
    Graphics::{
        Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmFlush, DwmGetWindowAttribute},
        Gdi::*,
    },
    System::Threading::GetCurrentProcessId,
    UI::WindowsAndMessaging::*,
};

#[derive(Clone, Debug)]
pub struct Candidate {
    pub window: usize,
    pub rect: Rect,
}

pub fn desktop_rect() -> Result<Rect, String> {
    unsafe {
        Rect::new(
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN) as u32,
            GetSystemMetrics(SM_CYVIRTUALSCREEN) as u32,
        )
    }
}

pub fn snapshot(rect: Rect) -> Result<Frame, String> {
    unsafe {
        let screen = GetDC(null_mut());
        if screen.is_null() {
            return Err("Cannot access the desktop display.".into());
        }
        let result = capture_from_dc(screen, rect);
        ReleaseDC(null_mut(), screen);
        result
    }
}

unsafe fn capture_from_dc(screen: HDC, rect: Rect) -> Result<Frame, String> {
    unsafe {
        let bytes = Frame::byte_len(rect.width, rect.height)?;
        let mut info: BITMAPINFO = zeroed();
        info.bmiHeader = header(rect.width, rect.height);
        let mut pixels: *mut c_void = null_mut();
        let bitmap = CreateDIBSection(screen, &info, DIB_RGB_COLORS, &mut pixels, null_mut(), 0);
        if bitmap.is_null() || pixels.is_null() {
            if !bitmap.is_null() {
                DeleteObject(bitmap);
            }
            return Err("Cannot allocate the capture bitmap.".into());
        }
        let dc = CreateCompatibleDC(screen);
        if dc.is_null() {
            DeleteObject(bitmap);
            return Err("Cannot allocate a capture display context.".into());
        }
        let previous = SelectObject(dc, bitmap);
        let selected = !previous.is_null() && previous as isize != -1;
        let _ = DwmFlush();
        let ok = selected
            && BitBlt(
                dc,
                0,
                0,
                rect.width as i32,
                rect.height as i32,
                screen,
                rect.x,
                rect.y,
                SRCCOPY | CAPTUREBLT,
            ) != 0
            && GdiFlush() != 0;
        let result = if ok {
            let mut data = std::slice::from_raw_parts(pixels as *const u8, bytes).to_vec();
            for pixel in data.chunks_exact_mut(4) {
                pixel[3] = 255;
            }
            Frame::new(rect.width, rect.height, data)
        } else {
            Err("Windows could not capture this desktop region.".into())
        };
        if selected {
            SelectObject(dc, previous);
        }
        DeleteDC(dc);
        DeleteObject(bitmap);
        result
    }
}

pub fn header(width: u32, height: u32) -> BITMAPINFOHEADER {
    BITMAPINFOHEADER {
        biSize: size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32),
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..unsafe { zeroed() }
    }
}

pub fn window_candidates() -> Vec<Candidate> {
    let mut windows = Vec::new();
    unsafe {
        EnumWindows(
            Some(collect_window),
            &mut windows as *mut Vec<Candidate> as LPARAM,
        );
    }
    windows
}

pub fn current_window_rect(window: usize) -> Option<Rect> {
    let window = window as HWND;
    unsafe {
        if IsWindow(window) == 0 || IsWindowVisible(window) == 0 || IsIconic(window) != 0 {
            return None;
        }
        let mut bounds: RECT = zeroed();
        if DwmGetWindowAttribute(
            window,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            &mut bounds as *mut _ as _,
            size_of::<RECT>() as u32,
        ) < 0
            && GetWindowRect(window, &mut bounds) == 0
        {
            return None;
        }
        Rect::from_edges(bounds.left, bounds.top, bounds.right, bounds.bottom).ok()
    }
}

unsafe extern "system" fn collect_window(window: HWND, parameter: LPARAM) -> i32 {
    unsafe {
        if IsWindowVisible(window) == 0
            || IsIconic(window) != 0
            || GetWindowTextLengthW(window) == 0
        {
            return 1;
        }
        let mut process = 0;
        GetWindowThreadProcessId(window, &mut process);
        if process == GetCurrentProcessId() {
            return 1;
        }
        let mut cloaked: u32 = 0;
        if DwmGetWindowAttribute(window, DWMWA_CLOAKED as u32, &mut cloaked as *mut _ as _, 4) >= 0
            && cloaked != 0
        {
            return 1;
        }
        let mut class = [0u16; 128];
        let count = GetClassNameW(window, class.as_mut_ptr(), class.len() as i32);
        let name = String::from_utf16_lossy(&class[..count.max(0) as usize]);
        if [
            "Progman",
            "WorkerW",
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
        ]
        .contains(&name.as_str())
        {
            return 1;
        }
        if let Some(rect) = current_window_rect(window as usize) {
            (&mut *(parameter as *mut Vec<Candidate>)).push(Candidate {
                window: window as usize,
                rect,
            });
        }
        1
    }
}

pub fn window_at(windows: &[Candidate], x: i32, y: i32) -> Option<(usize, Candidate)> {
    windows
        .iter()
        .enumerate()
        .find(|(_, item)| item.rect.contains(x, y))
        .map(|(i, item)| (i, item.clone()))
}

pub fn validate_window(windows: &[Candidate], index: usize, desktop: Rect) -> Result<Rect, String> {
    let chosen = windows
        .get(index)
        .ok_or("The selected window is no longer available.")?;
    unsafe {
        if IsWindow(chosen.window as HWND) == 0 || IsIconic(chosen.window as HWND) != 0 {
            return Err("The selected window was closed or minimized.".into());
        }
    }
    let visible = chosen
        .rect
        .intersection(desktop)
        .ok_or("The selected window is outside the display.")?;
    if windows[..index]
        .iter()
        .any(|other| other.rect.intersection(visible).is_some())
    {
        return Err(
            "Part of this window is covered. Bring it to the front and capture again.".into(),
        );
    }
    Ok(visible)
}

pub fn monitor_at(x: i32, y: i32) -> Result<Rect, String> {
    unsafe {
        let monitor = MonitorFromPoint(
            windows_sys::Win32::Foundation::POINT { x, y },
            MONITOR_DEFAULTTONEAREST,
        );
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return Err("Cannot determine the selected display.".into());
        }
        Rect::from_edges(
            info.rcMonitor.left,
            info.rcMonitor.top,
            info.rcMonitor.right,
            info.rcMonitor.bottom,
        )
    }
}

pub fn monitor_handle_at(x: i32, y: i32) -> Result<usize, String> {
    let monitor = unsafe {
        MonitorFromPoint(
            windows_sys::Win32::Foundation::POINT { x, y },
            MONITOR_DEFAULTTONEAREST,
        )
    };
    if monitor.is_null() {
        Err("Cannot determine the selected display.".into())
    } else {
        Ok(monitor as usize)
    }
}

pub fn window_pid(window: usize) -> Result<u32, String> {
    let mut pid = 0;
    let thread = unsafe { GetWindowThreadProcessId(window as HWND, &mut pid) };
    if thread == 0 || pid == 0 {
        Err("The selected window is no longer available.".into())
    } else {
        Ok(pid)
    }
}

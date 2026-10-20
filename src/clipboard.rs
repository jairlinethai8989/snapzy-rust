use crate::frame::Frame;
use std::{ptr, thread, time::Duration};
use windows_sys::Win32::{
    Foundation::{GlobalFree, HWND},
    System::{DataExchange::*, Memory::*},
};

#[cfg(test)]
#[path = "../tests/support/native_clipboard.rs"]
mod native_clipboard;

pub fn file_payload(path: &std::path::Path) -> Result<Vec<u8>, String> {
    use std::os::windows::ffi::OsStrExt;
    let name: Vec<u16> = path.as_os_str().encode_wide().collect();
    if !path.is_absolute()
        || name.contains(&0)
        || name.len() > 32760
        || !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("mp4"))
    {
        return Err("Invalid clip path.".into());
    }
    let mut bytes = Vec::with_capacity(20 + (name.len() + 2) * 2);
    bytes.extend_from_slice(&20u32.to_le_bytes());
    bytes.extend_from_slice(&[0; 12]);
    bytes.extend_from_slice(&1u32.to_le_bytes());
    for ch in name.into_iter().chain([0, 0]) {
        bytes.extend_from_slice(&ch.to_le_bytes());
    }
    Ok(bytes)
}

pub fn copy_file(owner: HWND, path: &std::path::Path) -> Result<(), String> {
    if !path.metadata().is_ok_and(|m| m.is_file() && m.len() > 0) {
        return Err("The recorded clip is missing or empty.".into());
    }
    let bytes = file_payload(path)?;
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
        if memory.is_null() {
            return Err("Cannot allocate clipboard file data.".into());
        }
        let target = GlobalLock(memory);
        if target.is_null() {
            GlobalFree(memory);
            return Err("Cannot access clipboard file data.".into());
        }
        ptr::copy_nonoverlapping(bytes.as_ptr(), target as *mut u8, bytes.len());
        GlobalUnlock(memory);
        let mut opened = false;
        for _ in 0..5 {
            if OpenClipboard(owner) != 0 {
                opened = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if !opened {
            GlobalFree(memory);
            return Err("The clipboard is busy. Try Copy again.".into());
        }
        let transferred = EmptyClipboard() != 0 && !SetClipboardData(15, memory).is_null();
        if transferred {
            let effect = RegisterClipboardFormatW(crate::ui::wide("Preferred DropEffect").as_ptr());
            let data = GlobalAlloc(GMEM_MOVEABLE, 4);
            if effect != 0 && !data.is_null() {
                let p = GlobalLock(data);
                if !p.is_null() {
                    ptr::write_unaligned(p as *mut u32, 1);
                    GlobalUnlock(data);
                    if SetClipboardData(effect, data).is_null() {
                        GlobalFree(data);
                    }
                } else {
                    GlobalFree(data);
                }
            } else if !data.is_null() {
                GlobalFree(data);
            }
        }
        CloseClipboard();
        if !transferred {
            GlobalFree(memory);
            return Err("Windows could not copy the clip.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn clip_file_payload_is_wide_dropfiles_with_two_nulls_not_video_bytes() {
    let bytes = file_payload(std::path::Path::new("C:\\clips\\ไทย.mp4")).unwrap();
    assert!(bytes.len() >= 24);
    assert_eq!(&bytes[..4], &20u32.to_le_bytes());
    assert_eq!(&bytes[4..16], &[0; 12]);
    assert_eq!(&bytes[16..20], &1u32.to_le_bytes());
    let text: Vec<u16> = bytes[20..]
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    assert_eq!(
        String::from_utf16(&text[..text.len() - 2]).unwrap(),
        "C:\\clips\\ไทย.mp4"
    );
    assert_eq!(&text[text.len() - 2..], &[0, 0]);
    for path in ["relative.mp4", "C:\\clips\\bad.txt", "C:\\clips\\bad\0.mp4"] {
        assert!(file_payload(std::path::Path::new(path)).is_err());
    }
}

#[cfg(test)]
#[test]
fn text_clipboard_payload_is_null_terminated_utf16() {
    assert_eq!(
        text_payload("ไทย OCR"),
        [0x0e44, 0x0e17, 0x0e22, 0x0020, 0x004f, 0x0043, 0x0052, 0]
    );
}

pub fn read(owner: HWND) -> Result<(Vec<u8>, bool), String> {
    unsafe {
        if OpenClipboard(owner) == 0 {
            return Err("The clipboard is busy. Try Paste again.".into());
        }
        let result = (|| {
            let png = RegisterClipboardFormatW(crate::ui::wide("PNG").as_ptr());
            let (format, is_png) = if png != 0 && IsClipboardFormatAvailable(png) != 0 {
                (png, true)
            } else if IsClipboardFormatAvailable(17) != 0 {
                (17, false)
            } else {
                (8, false)
            };
            let handle = GetClipboardData(format);
            if handle.is_null() {
                return Err("The clipboard does not contain a supported image.".into());
            }
            let len = GlobalSize(handle);
            if len == 0 || len > crate::image_limits::INPUT_FILE_LIMIT + 124 {
                return Err("Clipboard image exceeds the 256 MiB input limit.".into());
            }
            let source = GlobalLock(handle);
            if source.is_null() {
                return Err("Cannot read clipboard pixels.".into());
            }
            let bytes = std::slice::from_raw_parts(source as *const u8, len).to_vec();
            GlobalUnlock(handle);
            Ok((bytes, is_png))
        })();
        CloseClipboard();
        result
    }
}

pub fn copy(owner: HWND, frame: &Frame) -> Result<(), String> {
    let len = 40usize
        .checked_add(frame.bgra.len())
        .ok_or("Clipboard image size overflow.")?;
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, len);
        if memory.is_null() {
            return Err("Cannot allocate clipboard data.".into());
        }
        let target = GlobalLock(memory);
        if target.is_null() {
            GlobalFree(memory);
            return Err("Cannot access clipboard data.".into());
        }
        let output = std::slice::from_raw_parts_mut(target as *mut u8, len);
        if let Err(error) = frame.write_dib(output) {
            GlobalUnlock(memory);
            GlobalFree(memory);
            return Err(error);
        }
        GlobalUnlock(memory);
        let mut opened = false;
        for _ in 0..5 {
            if OpenClipboard(owner) != 0 {
                opened = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if !opened {
            GlobalFree(memory);
            return Err("The clipboard is busy. Try Copy again.".into());
        }
        let transferred = EmptyClipboard() != 0 && !SetClipboardData(8, memory).is_null();
        CloseClipboard();
        if !transferred {
            GlobalFree(memory);
            return Err("Windows could not copy this image.".into());
        }
    }
    Ok(())
}

fn text_payload(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

pub fn copy_text(owner: HWND, text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Err("There is no recognized text to copy.".into());
    }
    let payload = text_payload(text);
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, payload.len() * size_of::<u16>());
        if memory.is_null() {
            return Err("Cannot allocate clipboard text.".into());
        }
        let target = GlobalLock(memory);
        if target.is_null() {
            GlobalFree(memory);
            return Err("Cannot access clipboard text.".into());
        }
        ptr::copy_nonoverlapping(payload.as_ptr(), target as *mut u16, payload.len());
        GlobalUnlock(memory);
        let mut opened = false;
        for _ in 0..5 {
            if OpenClipboard(owner) != 0 {
                opened = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if !opened {
            GlobalFree(memory);
            return Err("The clipboard is busy. Try Copy again.".into());
        }
        let transferred = EmptyClipboard() != 0 && !SetClipboardData(13, memory).is_null();
        CloseClipboard();
        if !transferred {
            GlobalFree(memory);
            return Err("Windows could not copy the recognized text.".into());
        }
    }
    Ok(())
}

#[cfg(windows)]
use std::mem::size_of;

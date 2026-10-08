use crate::frame::Frame;
use std::{ptr, thread, time::Duration};
use windows_sys::Win32::{
    Foundation::{GlobalFree, HWND},
    System::{DataExchange::*, Memory::*},
};

pub fn copy(owner: HWND, frame: &Frame) -> Result<(), String> {
    let bytes = frame.dib();
    unsafe {
        let memory = GlobalAlloc(GMEM_MOVEABLE, bytes.len());
        if memory.is_null() {
            return Err("Cannot allocate clipboard data.".into());
        }
        let target = GlobalLock(memory);
        if target.is_null() {
            GlobalFree(memory);
            return Err("Cannot access clipboard data.".into());
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
        let transferred = EmptyClipboard() != 0 && !SetClipboardData(8, memory).is_null();
        CloseClipboard();
        if !transferred {
            GlobalFree(memory);
            return Err("Windows could not copy this image.".into());
        }
    }
    Ok(())
}

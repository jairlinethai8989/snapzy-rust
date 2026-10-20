#![cfg_attr(not(test), allow(dead_code))]

use crate::native_icons::{self, Command};
use std::{collections::HashSet, mem::zeroed, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::HWND,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Controls::*, WindowsAndMessaging::*},
};

const INITIAL_DELAY_MS: u32 = 400;
const MAX_WIDTH: i32 = 340;

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub control_id: usize,
    pub command: Command,
}

impl Entry {
    pub const fn new(control_id: usize, command: Command) -> Self {
        Self {
            control_id,
            command,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelError {
    DuplicateControl(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    entries: Vec<Entry>,
}

impl Model {
    pub fn try_new(entries: &[Entry]) -> Result<Self, ModelError> {
        let mut ids = HashSet::new();
        for entry in entries {
            if !ids.insert(entry.control_id) {
                return Err(ModelError::DuplicateControl(entry.control_id));
            }
        }
        Ok(Self {
            entries: entries.to_vec(),
        })
    }

    pub fn new(entries: &[Entry]) -> Self {
        Self::try_new(entries).expect("tooltip control ids must be unique")
    }

    pub const fn initial_delay_ms(&self) -> u32 {
        INITIAL_DELAY_MS
    }

    pub const fn max_width(&self) -> i32 {
        MAX_WIDTH
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn text(&self, control_id: usize, thai: bool) -> &'static str {
        self.entries
            .iter()
            .find(|entry| entry.control_id == control_id)
            .map_or("", |entry| {
                native_icons::accessible_name(entry.command, thai)
            })
    }
}

pub struct Tips {
    window: HWND,
    owner: HWND,
    model: Model,
    texts: Vec<Vec<u16>>,
}

impl Tips {
    pub fn new(owner: HWND, entries: &[Entry], thai: bool) -> Option<Self> {
        let model = Model::new(entries);
        unsafe {
            let window = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                TOOLTIPS_CLASSW,
                null_mut(),
                WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
                0,
                0,
                0,
                0,
                owner,
                null_mut(),
                GetModuleHandleW(null_mut()),
                null_mut(),
            );
            if window.is_null() {
                return None;
            }
            SendMessageW(
                window,
                TTM_SETDELAYTIME,
                TTDT_INITIAL as usize,
                INITIAL_DELAY_MS as isize,
            );
            SendMessageW(window, TTM_SETMAXTIPWIDTH, 0, MAX_WIDTH as isize);
            let mut result = Self {
                window,
                owner,
                model,
                texts: Vec::with_capacity(entries.len()),
            };
            for entry in entries {
                result
                    .texts
                    .push(wide(native_icons::accessible_name(entry.command, thai)));
                let child = GetDlgItem(owner, entry.control_id as i32);
                if child.is_null() {
                    continue;
                }
                let mut info: TTTOOLINFOW = zeroed();
                info.cbSize = std::mem::size_of::<TTTOOLINFOW>() as u32;
                info.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
                info.hwnd = owner;
                info.uId = child as usize;
                info.lpszText = result.texts.last_mut().unwrap().as_mut_ptr();
                SendMessageW(window, TTM_ADDTOOLW, 0, &info as *const _ as isize);
            }
            Some(result)
        }
    }

    pub fn refresh(&mut self, thai: bool) {
        for (index, entry) in self.model.entries.iter().enumerate() {
            self.texts[index] = wide(native_icons::accessible_name(entry.command, thai));
            unsafe {
                let child = GetDlgItem(self.owner, entry.control_id as i32);
                if child.is_null() {
                    continue;
                }
                let mut info: TTTOOLINFOW = zeroed();
                info.cbSize = std::mem::size_of::<TTTOOLINFOW>() as u32;
                info.hwnd = self.owner;
                info.uId = child as usize;
                info.lpszText = self.texts[index].as_mut_ptr();
                SendMessageW(
                    self.window,
                    TTM_UPDATETIPTEXTW,
                    0,
                    &info as *const _ as isize,
                );
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_icons::Command;

    #[test]
    fn tooltip_model_uses_compact_delay_width_and_bilingual_text() {
        let model = Model::new(&[
            Entry::new(101, Command::AreaCapture),
            Entry::new(102, Command::WindowCapture),
            Entry::new(126, Command::More),
        ]);
        assert_eq!(model.initial_delay_ms(), 400);
        assert_eq!(model.max_width(), 340);
        assert_eq!(model.len(), 3);
        assert_ne!(model.text(101, false), model.text(101, true));
        assert!(!model.text(126, false).is_empty());
    }

    #[test]
    fn tooltip_model_rejects_duplicate_control_ids() {
        let result = Model::try_new(&[
            Entry::new(101, Command::AreaCapture),
            Entry::new(101, Command::WindowCapture),
        ]);
        assert_eq!(result, Err(ModelError::DuplicateControl(101)));
    }
}

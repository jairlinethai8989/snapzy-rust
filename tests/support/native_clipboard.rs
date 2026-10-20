use super::copy_file;
use std::{
    ptr,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    System::{DataExchange::*, Memory::*, StationsAndDesktops::*, Threading::GetCurrentThreadId},
    UI::{Shell::DragQueryFileW, WindowsAndMessaging::*},
};

#[test]
#[ignore = "Requires a Windows host that permits private window stations; never falls back to the user's clipboard"]
fn file_clipboard_survives_export_and_owner_close_in_an_isolated_window_station() {
    const CHILD: &str = "SNAPZY_ISOLATED_CLIPBOARD_TEST";
    if std::env::var_os(CHILD).as_deref() != Some(std::ffi::OsStr::new("1")) {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "clipboard::native_clipboard::file_clipboard_survives_export_and_owner_close_in_an_isolated_window_station", "--nocapture", "--ignored"])
            .env(CHILD, "1").spawn().unwrap();
        let start = Instant::now();
        loop {
            if let Some(result) = child.try_wait().unwrap() {
                assert!(result.success(), "isolated clipboard test failed: {result}");
                break;
            }
            if start.elapsed() > Duration::from_secs(20) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("isolated clipboard test timed out");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        return;
    }
    // Only this single-test child changes its own station. Never switch the
    // interactive desktop or open the user's clipboard, including on failure.
    unsafe {
        let old_station = GetProcessWindowStation();
        let old_desktop = GetThreadDesktop(GetCurrentThreadId());
        let station = CreateWindowStationW(
            crate::ui::wide(&format!("SnapZyClipboardTest-{}", std::process::id())).as_ptr(),
            0,
            0x37f,
            ptr::null(),
        );
        assert!(
            !station.is_null(),
            "cannot create isolated clipboard station: {}",
            std::io::Error::last_os_error()
        );
        if SetProcessWindowStation(station) == 0 {
            CloseWindowStation(station);
            panic!("cannot enter isolated station");
        }
        let mut desktop = ptr::null_mut();
        let mut window = ptr::null_mut();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_eq!(GetProcessWindowStation(), station);
            assert_ne!(station, old_station);
            desktop = CreateDesktopW(
                crate::ui::wide("ClipboardTest").as_ptr(),
                ptr::null(),
                ptr::null(),
                0,
                0x1ff,
                ptr::null(),
            );
            assert!(!desktop.is_null(), "cannot create isolated desktop");
            assert_ne!(
                SetThreadDesktop(desktop),
                0,
                "cannot enter isolated desktop"
            );
            window = CreateWindowExW(
                0,
                crate::ui::wide("STATIC").as_ptr(),
                ptr::null(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            );
            assert!(!window.is_null(), "cannot create isolated clipboard owner");
            let directory = std::env::temp_dir()
                .join(format!("snapzy-isolated-clipboard-{}", std::process::id()));
            std::fs::create_dir_all(&directory).unwrap();
            let source = directory.join("คลิปต้นฉบับ.mp4");
            let saved = directory.join("exported.mp4");
            std::fs::write(&source, b"clipboard-file-fixture").unwrap();
            copy_file(window, &source).unwrap();
            crate::video_file::export_completed(&source, &saved, 1).unwrap();
            DestroyWindow(window);
            window = ptr::null_mut();
            assert_ne!(OpenClipboard(ptr::null_mut()), 0);
            let handle = GetClipboardData(15);
            assert!(!handle.is_null());
            assert_eq!(DragQueryFileW(handle, u32::MAX, ptr::null_mut(), 0), 1);
            let mut name = vec![0; 32768];
            let len = DragQueryFileW(handle, 0, name.as_mut_ptr(), name.len() as u32);
            assert_eq!(
                String::from_utf16(&name[..len as usize]).unwrap(),
                source.to_string_lossy()
            );
            let effect = RegisterClipboardFormatW(crate::ui::wide("Preferred DropEffect").as_ptr());
            let effect = GetClipboardData(effect);
            assert!(!effect.is_null());
            assert!(GlobalSize(effect) >= 4);
            let effect_data = GlobalLock(effect);
            assert!(!effect_data.is_null());
            assert_eq!(ptr::read_unaligned(effect_data as *const u32), 1);
            GlobalUnlock(effect);
            CloseClipboard();
            assert_eq!(std::fs::read(&source).unwrap(), b"clipboard-file-fixture");
            assert_eq!(std::fs::read(&saved).unwrap(), b"clipboard-file-fixture");
            std::fs::remove_dir_all(directory).unwrap();
        }));
        CloseClipboard();
        if !window.is_null() {
            DestroyWindow(window);
        }
        assert_ne!(SetProcessWindowStation(old_station), 0);
        assert_ne!(SetThreadDesktop(old_desktop), 0);
        if !desktop.is_null() {
            CloseDesktop(desktop);
        }
        CloseWindowStation(station);
        if let Err(e) = result {
            std::panic::resume_unwind(e);
        }
    }
}

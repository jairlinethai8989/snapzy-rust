#![cfg_attr(not(test), windows_subsystem = "windows")]

mod audio;
mod audio_capture;
mod callout;
mod capture;
mod clipboard;
mod combine;
mod compact_ui;
mod composition;
mod editor;
mod editor_controls;
mod effects;
mod effects_ui;
mod frame;
mod geometry;
mod glyphs;
mod image_io;
mod image_limits;
mod interaction;
mod memory;
mod native_icons;
mod native_tooltips;
mod neosnap;
mod ocr;
mod paint_buffer;
mod project;
mod render;
mod scroll;
mod scroll_auto;
mod scroll_detect;
mod scroll_job;
mod scroll_probe;
mod settings;
mod shape_frame;
mod sheet_viewport;
mod text_frame;
mod trace;
mod ui;
mod video;
mod video_capture;
mod video_clip;
mod video_file;
mod video_media;
mod video_playback;
mod video_service;
mod video_worker;
mod workspace;

fn main() {
    let arguments: Vec<_> = std::env::args_os().collect();
    if let Some(index) = arguments.iter().position(|arg| arg == "--test-data-dir") {
        let Some(path) = arguments.get(index + 1) else {
            std::process::exit(2);
        };
        if settings::use_test_directory(std::path::PathBuf::from(path)).is_err() {
            std::process::exit(2);
        }
    }
    if let Some(index) = arguments
        .iter()
        .position(|arg| arg == "--scroll-detect-worker")
    {
        let success = arguments
            .get(index + 1)
            .is_some_and(|path| scroll_probe::worker_main(std::path::Path::new(path)).is_ok());
        std::process::exit(if success { 0 } else { 2 });
    }
    if arguments.iter().any(|arg| arg == "--video-worker") {
        std::process::exit(if video_worker::worker_main().is_ok() {
            0
        } else {
            2
        });
    }
    if let Err(error) = ui::run(std::env::args().any(|arg| arg == "--tray")) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                std::ptr::null_mut(),
                ui::wide(&error).as_ptr(),
                ui::wide("SnapZy Rust Preview").as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                    | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::frame::{Frame, Rect};

    #[test]
    fn crop_preserves_pixels_with_negative_desktop_origin() {
        let desktop = Rect::new(-1920, -307, 4, 3).unwrap();
        let mut bytes = Vec::new();
        for n in 0..12u8 {
            bytes.extend_from_slice(&[n, n + 1, n + 2, 255]);
        }
        let image = Frame::new(4, 3, bytes).unwrap();
        let crop = image
            .crop(desktop, Rect::new(-1919, -306, 2, 2).unwrap())
            .unwrap();
        assert_eq!((crop.width, crop.height), (2, 2));
        assert_eq!(
            crop.bgra,
            [5, 6, 7, 255, 6, 7, 8, 255, 9, 10, 11, 255, 10, 11, 12, 255]
        );
    }

    #[test]
    fn crop_rejects_outside_or_empty_regions() {
        let desktop = Rect::new(-2, -1, 2, 2).unwrap();
        let image = Frame::new(2, 2, vec![255; 16]).unwrap();
        assert!(
            image
                .crop(desktop, Rect::new(-3, -1, 2, 2).unwrap())
                .is_err()
        );
        assert!(Rect::new(0, 0, 0, 20).is_err());
        assert!(Rect::new(i32::MAX, 0, 2, 2).is_err());
        assert!(Frame::new(100_000, 100_000, Vec::new()).is_err());
    }

    #[test]
    fn png_round_trip_retains_native_resolution_and_channels() {
        let image = Frame::new(2, 1, vec![1, 2, 3, 255, 10, 20, 30, 255]).unwrap();
        let mut bytes = Vec::new();
        image.write_png(&mut bytes).unwrap();
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; decoder.output_buffer_size().unwrap()];
        let info = decoder.next_frame(&mut pixels).unwrap();
        assert_eq!((info.width, info.height), (2, 1));
        assert_eq!(
            &pixels[..info.buffer_size()],
            &[3, 2, 1, 255, 30, 20, 10, 255]
        );
    }
}

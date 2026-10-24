#![cfg_attr(not(test), windows_subsystem = "windows")]

mod audio;
mod audio_capture;
mod callout;
mod capture;
mod capture_result;
mod clipboard;
mod combine;
mod compact_ui;
mod composition;
mod desktop_bridge;
mod editor;
mod editor_controls;
mod editor_service;
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
    if arguments.iter().any(|arg| arg == "--editor-service") {
        if let Some(directory) = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("fonts")))
        {
            glyphs::register_bundled_fonts(&directory);
        }
        std::process::exit(if editor_service::run_stdio().is_ok() {
            0
        } else {
            2
        });
    }
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
    let desktop_engine = arguments.iter().any(|arg| arg == "--tauri-engine");
    if desktop_engine {
        let Some(index) = arguments.iter().position(|arg| arg == "--tauri-result-dir") else {
            std::process::exit(2);
        };
        let Some(path) = arguments.get(index + 1) else {
            std::process::exit(2);
        };
        if capture_result::activate(std::path::PathBuf::from(path)).is_err() {
            std::process::exit(2);
        }
        desktop_bridge::activate();
    }
    if let Err(error) = ui::run(desktop_engine || std::env::args().any(|arg| arg == "--tray")) {
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
    use super::editor::{Point, Tool};
    use super::editor_service::{EditorCommand, EditorService};
    use super::frame::{Frame, Rect};

    #[test]
    fn editor_service_opens_draws_renders_and_undoes_with_rust_state() {
        let mut png = Vec::new();
        Frame::new(24, 16, vec![255; 24 * 16 * 4])
            .unwrap()
            .write_png(&mut png)
            .unwrap();

        let mut service = EditorService::open_bytes(&png).unwrap();
        let opened = service.view();
        assert_eq!((opened.width, opened.height), (24, 16));
        assert_eq!(opened.revision, 0);
        assert!(!opened.can_undo);

        service
            .apply(EditorCommand::Choose { tool: Tool::Line })
            .unwrap();
        service
            .apply(EditorCommand::Press {
                point: Point::new(2., 3.),
                tolerance: 3.,
            })
            .unwrap();
        service
            .apply(EditorCommand::Release {
                point: Point::new(20., 12.),
            })
            .unwrap();
        let edited = service.view();
        assert_eq!(edited.revision, 1);
        assert!(edited.can_undo);
        assert_ne!(service.preview_png().unwrap(), png);

        service.apply(EditorCommand::Undo).unwrap();
        assert_eq!(service.view().revision, opened.revision);
        assert!(!service.view().can_undo);
        assert!(service.view().can_redo);
    }

    #[test]
    fn editor_service_adds_text_and_callout_as_editable_rust_objects() {
        let mut png = Vec::new();
        Frame::new(320, 180, vec![255; 320 * 180 * 4])
            .unwrap()
            .write_png(&mut png)
            .unwrap();

        let mut plain = EditorService::open_bytes(&png).unwrap();
        let text_view = plain
            .apply(EditorCommand::AddText {
                point: Point::new(32., 40.),
                text: "SnapZy text".into(),
                callout: false,
            })
            .unwrap();
        assert_eq!(text_view.revision, 1);
        assert!(text_view.selected);
        let text_png = plain.preview_png().unwrap();
        assert_ne!(text_png, png);

        let mut bubble = EditorService::open_bytes(&png).unwrap();
        let callout_view = bubble
            .apply(EditorCommand::AddText {
                point: Point::new(32., 40.),
                text: "SnapZy text".into(),
                callout: true,
            })
            .unwrap();
        assert_eq!(callout_view.revision, 1);
        assert!(callout_view.selected);
        assert_ne!(bubble.preview_png().unwrap(), text_png);

        bubble.apply(EditorCommand::Undo).unwrap();
        assert_eq!(bubble.view().revision, 0);
        assert!(bubble.view().can_redo);
    }

    #[test]
    fn editor_service_duplicates_rendered_area_as_selected_image_layer_and_undoes_once() {
        let mut pixels = vec![255; 64 * 48 * 4];
        for y in 8..16 {
            for x in 6..14 {
                let index = (y * 64 + x) * 4;
                pixels[index..index + 4].copy_from_slice(&[21, 82, 173, 255]);
            }
        }
        let original = Frame::new(64, 48, pixels).unwrap();
        let mut png = Vec::new();
        original.write_png(&mut png).unwrap();
        let mut service = EditorService::open_bytes(&png).unwrap();

        let duplicated = service
            .apply(EditorCommand::DuplicateArea {
                start: Point::new(14., 16.),
                end: Point::new(6., 8.),
            })
            .unwrap();

        assert_eq!(duplicated.revision, 1);
        assert!(duplicated.selected);
        assert!(duplicated.can_undo);
        let rendered = super::image_io::decode(
            &service.preview_png().unwrap(),
            super::image_limits::IMAGE_LIMIT,
        )
        .unwrap();
        for y in 24..32 {
            for x in 22..30 {
                let index = (y * 64 + x) * 4;
                assert_eq!(&rendered.bgra[index..index + 4], &[21, 82, 173, 255]);
            }
        }

        service.apply(EditorCommand::Undo).unwrap();
        assert_eq!(service.view().revision, 0);
        assert_eq!(service.preview_png().unwrap(), png);
        service.apply(EditorCommand::Redo).unwrap();
        assert!(service.view().selected);
    }

    #[test]
    fn editor_service_duplicate_selection_uses_the_editable_object_path() {
        let mut png = Vec::new();
        Frame::new(80, 60, vec![255; 80 * 60 * 4])
            .unwrap()
            .write_png(&mut png)
            .unwrap();
        let mut service = EditorService::open_bytes(&png).unwrap();
        service
            .apply(EditorCommand::Choose { tool: Tool::Line })
            .unwrap();
        service
            .apply(EditorCommand::Press {
                point: Point::new(10., 12.),
                tolerance: 4.,
            })
            .unwrap();
        service
            .apply(EditorCommand::Release {
                point: Point::new(40., 30.),
            })
            .unwrap();

        let duplicated = service.apply(EditorCommand::DuplicateSelection).unwrap();

        assert_eq!(duplicated.revision, 2);
        assert_eq!(duplicated.tool, Tool::Select);
        assert!(duplicated.selected);
        service.apply(EditorCommand::Undo).unwrap();
        assert_eq!(service.view().revision, 1);
        assert!(service.view().selected);
    }

    #[test]
    fn editor_service_rejects_duplication_during_an_unfinished_gesture() {
        let mut png = Vec::new();
        Frame::new(80, 60, vec![255; 80 * 60 * 4])
            .unwrap()
            .write_png(&mut png)
            .unwrap();
        let mut service = EditorService::open_bytes(&png).unwrap();
        service
            .apply(EditorCommand::Choose { tool: Tool::Line })
            .unwrap();
        service
            .apply(EditorCommand::Press {
                point: Point::new(10., 12.),
                tolerance: 4.,
            })
            .unwrap();

        let error = service
            .apply(EditorCommand::DuplicateArea {
                start: Point::new(2., 2.),
                end: Point::new(20., 20.),
            })
            .unwrap_err();

        assert!(error.contains("Finish"));
        assert_eq!(service.view().revision, 0);
        service
            .apply(EditorCommand::Release {
                point: Point::new(40., 30.),
            })
            .unwrap();
        assert_eq!(service.view().revision, 1);
    }

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

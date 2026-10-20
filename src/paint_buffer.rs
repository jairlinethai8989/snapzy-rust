use crate::{capture, frame::Frame, render::Draft};
use std::{mem::zeroed, ptr::null_mut};
use windows_sys::Win32::Graphics::Gdi::*;

pub struct BackBuffer {
    pub dc: HDC,
    pub width: u32,
    pub height: u32,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    bits: *mut u8,
    length: usize,
}
impl BackBuffer {
    pub fn new(target: HDC, width: u32, height: u32) -> Result<Self, String> {
        let length = Frame::byte_len(width, height)?;
        if length > 64 * 1024 * 1024 {
            return Err("Editor display buffer exceeds the memory limit.".into());
        }
        unsafe {
            let dc = CreateCompatibleDC(target);
            if dc.is_null() {
                return Err("Cannot create editor drawing context.".into());
            }
            let mut info: BITMAPINFO = zeroed();
            info.bmiHeader = capture::header(width, height);
            let mut bits = null_mut();
            let bitmap = CreateDIBSection(target, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if bitmap.is_null() || bits.is_null() {
                if !bitmap.is_null() {
                    DeleteObject(bitmap);
                }
                DeleteDC(dc);
                return Err("Cannot create editor display buffer.".into());
            }
            let previous = SelectObject(dc, bitmap);
            if previous.is_null() || previous as isize == -1 {
                DeleteObject(bitmap);
                DeleteDC(dc);
                return Err("Cannot select editor display buffer.".into());
            }
            Ok(Self {
                dc,
                width,
                height,
                bitmap,
                previous,
                bits: bits.cast(),
                length,
            })
        }
    }
    pub fn blend(&mut self, draft: &Draft) {
        // Finish queued GDI drawing before mixing CPU-rendered premultiplied pixels.
        unsafe {
            GdiFlush();
        }
        let pixels = unsafe { std::slice::from_raw_parts_mut(self.bits, self.length) };
        for y in 0..draft.bounds.height as usize {
            let dy = draft.bounds.y as i64 + y as i64;
            if dy < 0 || dy >= self.height as i64 {
                continue;
            }
            for x in 0..draft.bounds.width as usize {
                let dx = draft.bounds.x as i64 + x as i64;
                if dx < 0 || dx >= self.width as i64 {
                    continue;
                }
                let src = &draft.rgba[(y * draft.bounds.width as usize + x) * 4..][..4];
                if src[3] == 0 {
                    continue;
                }
                let dst = &mut pixels[(dy as usize * self.width as usize + dx as usize) * 4..][..4];
                dst[3] = 255;
                crate::render::blend_pixel(dst, src, draft.multiply);
            }
        }
    }
    pub fn present(&self, target: HDC) -> bool {
        unsafe {
            BitBlt(
                target,
                0,
                0,
                self.width as i32,
                self.height as i32,
                self.dc,
                0,
                0,
                SRCCOPY,
            ) != 0
        }
    }
}
impl Drop for BackBuffer {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.bitmap);
            DeleteDC(self.dc);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gdi_highlight_draft_matches_flattened_image() {
        use crate::editor::{Document, Point, Style, View};
        let mut buffer = BackBuffer::new(std::ptr::null_mut(), 100, 80).unwrap();
        unsafe {
            PatBlt(buffer.dc, 0, 0, 100, 80, WHITENESS);
        }
        let source = std::sync::Arc::new(Frame::new(100, 80, vec![255; 32000]).unwrap());
        let mut d = Document::new(100, 80);
        d.add_highlight(
            vec![Point::new(10., 20.), Point::new(80., 50.)],
            Style {
                rgb: [255, 242, 88],
                width: 24,
            },
            40,
        );
        let draft = crate::render::draft(
            &d.objects[0],
            View {
                crop: d.crop,
                x: 0.,
                y: 0.,
                scale: 1.,
            },
        )
        .unwrap();
        buffer.blend(&draft);
        let final_image = crate::render::flatten(&source, &d).unwrap();
        for y in 0..80 {
            for x in 0..100 {
                let c = unsafe { GetPixel(buffer.dc, x, y) };
                let p = &final_image.bgra[((y * 100 + x) * 4) as usize..][..4];
                assert_eq!(
                    c,
                    p[2] as u32 | ((p[1] as u32) << 8) | ((p[0] as u32) << 16)
                );
            }
        }
    }
    #[test]
    fn editor_paint_buffer_can_render_offscreen_without_touching_the_window() {
        let dc = unsafe { GetDC(std::ptr::null_mut()) };
        let result = BackBuffer::new(dc, 80, 60);
        unsafe {
            ReleaseDC(std::ptr::null_mut(), dc);
        }
        assert!(
            result.is_ok(),
            "Editor needs an offscreen surface before drawing its background."
        );
    }
    #[test]
    fn invalid_display_size_is_rejected() {
        assert!(BackBuffer::new(std::ptr::null_mut(), 0, 60).is_err());
        assert!(BackBuffer::new(std::ptr::null_mut(), 32767, 32767).is_err());
    }
}

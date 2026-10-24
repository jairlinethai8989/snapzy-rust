#[derive(Clone, Debug, PartialEq)]
pub struct Mask {
    pub width: u32,
    pub height: u32,
    pub alpha: Vec<u8>,
}
pub const BUNDLED_FONT_FILES: [&str; 4] = [
    "Kanit-Regular.ttf",
    "Prompt-Regular.ttf",
    "NotoSansThai-Regular.ttf",
    "Sarabun-Regular.ttf",
];

pub fn register_bundled_fonts(directory: &std::path::Path) -> usize {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Graphics::Gdi::AddFontResourceExW;

    BUNDLED_FONT_FILES
        .iter()
        .filter(|file| {
            let path = directory.join(file);
            if !path.is_file() {
                return false;
            }
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe { AddFontResourceExW(wide.as_ptr(), 0x10, std::ptr::null_mut()) > 0 }
        })
        .count()
}
pub fn step(number: u32, diameter: u32) -> Result<Mask, String> {
    let mask = raster(&number.to_string(), diameter / 2)?;
    let side = diameter - 4;
    if mask.width <= side && mask.height <= side {
        return Ok(mask);
    }
    let ratio = (side as f64 / mask.width as f64).min(side as f64 / mask.height as f64);
    let width = (mask.width as f64 * ratio).floor().max(1.) as u32;
    let height = (mask.height as f64 * ratio).floor().max(1.) as u32;
    let source = image::GrayImage::from_raw(mask.width, mask.height, mask.alpha)
        .ok_or("Invalid number mask.")?;
    let resized = image::imageops::resize(
        &source,
        width,
        height,
        image::imageops::FilterType::Triangle,
    );
    Ok(Mask {
        width,
        height,
        alpha: resized.into_raw(),
    })
}
pub fn raster(text: &str, size: u32) -> Result<Mask, String> {
    raster_font(text, size, crate::editor::FontFamily::default())
}
pub fn raster_font(text: &str, size: u32, font: crate::editor::FontFamily) -> Result<Mask, String> {
    wrapped_font(text, size, None, font)
}
pub fn wrapped(text: &str, size: u32, width: Option<u32>) -> Result<Mask, String> {
    wrapped_font(text, size, width, crate::editor::FontFamily::default())
}
pub fn wrapped_font(
    text: &str,
    size: u32,
    width: Option<u32>,
    font: crate::editor::FontFamily,
) -> Result<Mask, String> {
    wrapped_weight(text, size, width, 400, 128, 24, font)
}
pub fn dotnet_text(text: &str, size: u32, width: Option<u32>) -> Result<Mask, String> {
    dotnet_text_font(text, size, width, crate::editor::FontFamily::default())
}
pub fn dotnet_text_font(
    text: &str,
    size: u32,
    width: Option<u32>,
    font: crate::editor::FontFamily,
) -> Result<Mask, String> {
    wrapped_weight(text, size, width, 600, 256, 1, font)
}
pub fn dotnet_step(number: u32, source_size: u32) -> Result<Mask, String> {
    wrapped_weight(
        &number.to_string(),
        dotnet_step_font_size(number, source_size),
        None,
        700,
        512,
        24,
        crate::editor::FontFamily::default(),
    )
}
pub fn dotnet_step_font_size(number: u32, source_size: u32) -> u32 {
    let radius = crate::editor::dotnet_step_radius(number, source_size);
    let digits = number.to_string().len() as f64;
    (radius * 1.2)
        .min(radius * 2.8 / digits)
        .round()
        .clamp(8., 512.) as u32
}
fn wrapped_weight(
    text: &str,
    size: u32,
    width: Option<u32>,
    weight: i32,
    maximum_size: u32,
    minimum_width: u32,
    font_family: crate::editor::FontFamily,
) -> Result<Mask, String> {
    use std::{
        mem::{size_of, zeroed},
        ptr::null_mut,
    };
    use windows_sys::Win32::{Foundation::RECT, Graphics::Gdi::*};
    if !(8..=maximum_size).contains(&size)
        || text.chars().count() > 4096
        || text.trim().is_empty()
        || width.is_some_and(|w| !(minimum_width..=32767).contains(&w))
    {
        return Err("Invalid text or font size.".into());
    }
    unsafe {
        let dc = CreateCompatibleDC(null_mut());
        if dc.is_null() {
            return Err("Cannot create text canvas.".into());
        }
        let font = CreateFontW(
            -(size as i32),
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            0,
            0,
            ANTIALIASED_QUALITY as u32,
            0,
            crate::ui::wide(font_family.face_name()).as_ptr(),
        );
        if font.is_null() {
            DeleteDC(dc);
            return Err("Cannot create text font.".into());
        }
        let old_font = SelectObject(dc, font);
        let value = crate::ui::wide(&text.replace('\r', ""));
        let mut rect: RECT = zeroed();
        rect.right = width.map(|w| w as i32 - 8).unwrap_or(0);
        let flags = DT_NOPREFIX | DT_EXPANDTABS | if width.is_some() { DT_WORDBREAK } else { 0 };
        DrawTextW(
            dc,
            value.as_ptr(),
            (value.len() - 1) as i32,
            &mut rect,
            flags | DT_CALCRECT,
        );
        let w = ((rect.right + 8).max(1) as u32).max(width.unwrap_or(0));
        if width.is_some() {
            rect = RECT {
                left: 0,
                top: 0,
                right: w as i32 - 8,
                bottom: 0,
            };
            DrawTextW(
                dc,
                value.as_ptr(),
                (value.len() - 1) as i32,
                &mut rect,
                flags | DT_CALCRECT,
            );
        }
        let h = (rect.bottom + 8).max(1) as u32;
        let result = (|| {
            let length = crate::frame::Frame::byte_len(w, h)?;
            if length > 4 * 1024 * 1024 {
                return Err("Text is too large. Use shorter lines or a smaller size.".into());
            }
            let mut info: BITMAPINFO = zeroed();
            info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = w as i32;
            info.bmiHeader.biHeight = -(h as i32);
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            let mut bits = null_mut();
            let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if bitmap.is_null() || bits.is_null() {
                return Err("Cannot allocate text pixels.".into());
            }
            let old_bitmap = SelectObject(dc, bitmap);
            std::ptr::write_bytes(bits, 0, length);
            SetBkMode(dc, TRANSPARENT as i32);
            SetTextColor(dc, 0xffffff);
            let mut area = RECT {
                left: 4,
                top: 4,
                right: w as i32 - 4,
                bottom: h as i32 - 4,
            };
            DrawTextW(
                dc,
                value.as_ptr(),
                (value.len() - 1) as i32,
                &mut area,
                flags,
            );
            GdiFlush();
            // Font quality is grayscale, and only coverage is retained, never RGB subpixels.
            let alpha = std::slice::from_raw_parts(bits as *const u8, length)
                .chunks_exact(4)
                .map(|p| ((p[0] as u16 + p[1] as u16 + p[2] as u16) / 3) as u8)
                .collect();
            SelectObject(dc, old_bitmap);
            DeleteObject(bitmap);
            Ok(Mask {
                width: w,
                height: h,
                alpha,
            })
        })();
        SelectObject(dc, old_font);
        DeleteObject(font);
        DeleteDC(dc);
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_font_files_cover_every_private_thai_family() {
        assert_eq!(
            BUNDLED_FONT_FILES,
            [
                "Kanit-Regular.ttf",
                "Prompt-Regular.ttf",
                "NotoSansThai-Regular.ttf",
                "Sarabun-Regular.ttf",
            ]
        );
    }
    #[test]
    fn wrapping_reflows_without_reducing_font_and_keeps_requested_empty_space() {
        let text = "English words in a longer line";
        let original = raster(text, 24).unwrap();
        let narrow = wrapped(text, 24, Some(140)).unwrap();
        assert_eq!(narrow.width, 140);
        assert!(narrow.height > original.height);
        let wide = wrapped("A", 24, Some(300)).unwrap();
        assert_eq!(wide.width, 300);
        assert_eq!(wide.height, raster("A", 24).unwrap().height);
    }
    #[test]
    fn wrapped_long_word_and_thai_keep_all_glyphs_and_line_breaks() {
        let word = "Supercalifragilisticexpialidocious";
        assert_eq!(
            wrapped(word, 24, Some(24)).unwrap(),
            raster(word, 24).unwrap()
        );
        let thai = wrapped("ภาษาไทย\nEnglish words in a longer line", 24, Some(160)).unwrap();
        assert!(thai.height > raster("ภาษาไทย\nEnglish", 24).unwrap().height);
        assert!(thai.alpha.iter().any(|a| *a > 0 && *a < 255));
        assert!(wrapped("A", 24, Some(23)).is_err());
        assert!(wrapped("A", 24, Some(32768)).is_err());
    }
    #[test]
    fn native_multiline_thai_english_masks_have_antialiased_coverage() {
        let single = raster("ภาษาไทย English", 24).unwrap();
        let multi = raster("ภาษาไทย\nEnglish", 24).unwrap();
        assert!(single.width > 50 && single.height >= 24);
        assert!(multi.height > single.height);
        assert!(multi.alpha.iter().any(|a| *a > 0 && *a < 255));
        assert_eq!(multi.alpha.len(), (multi.width * multi.height) as usize);
    }
}

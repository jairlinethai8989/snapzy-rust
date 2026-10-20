use crate::frame::Frame;

pub const SOURCE_LIMIT: usize = crate::image_limits::SOURCE_LIMIT;
pub const INPUT_FILE_LIMIT: usize = crate::image_limits::INPUT_FILE_LIMIT;

pub fn decode(bytes: &[u8], budget: usize) -> Result<Frame, String> {
    use image::{ImageDecoder, ImageReader};
    if bytes.len() > INPUT_FILE_LIMIT {
        return Err("Image file exceeds the 256 MiB input limit.".into());
    }
    let mut reader = ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::Bmp)
    ) {
        return Err("Choose a PNG, JPEG or BMP image.".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(32767);
    limits.max_image_height = Some(32767);
    limits.max_alloc = Some((crate::image_limits::IMAGE_LIMIT * 4) as u64);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    if Frame::byte_len(w, h)? > budget.min(crate::image_limits::IMAGE_LIMIT) {
        return Err(
            "Image memory limit reached. Close an older editor or use smaller images.".into(),
        );
    }
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    image.apply_orientation(orientation);
    let rgba = image.into_rgba8();
    let (w, h) = rgba.dimensions();
    let mut bgra = rgba.into_raw();
    for p in bgra.chunks_exact_mut(4) {
        p.swap(0, 2);
    }
    Frame::new(w, h, bgra)
}
pub fn dib(bytes: &[u8], budget: usize) -> Result<Frame, String> {
    let bad = || "Unsupported or incomplete clipboard bitmap.".to_string();
    if bytes.len() < 40 {
        return Err(bad());
    }
    let u32at = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
    let header = u32at(0) as usize;
    if !matches!(header, 40 | 108 | 124) || header > bytes.len() {
        return Err(bad());
    }
    let w = u32at(4) as i32;
    let signed_h = u32at(8) as i32;
    let h = signed_h.checked_abs().ok_or_else(bad)? as u32;
    let bits = u16::from_le_bytes(bytes[14..16].try_into().unwrap());
    let compression = u32at(16);
    let bitfields = compression == 3
        && header >= 108
        && bits == 32
        && u32at(40) == 0x00ff0000
        && u32at(44) == 0x0000ff00
        && u32at(48) == 0x000000ff
        && matches!(u32at(52), 0 | 0xff000000);
    if w <= 0
        || bytes[12..14] != [1, 0]
        || !matches!(bits, 24 | 32)
        || (compression != 0 && !bitfields)
        || u32at(32) != 0
    {
        return Err(bad());
    }
    let len = Frame::byte_len(w as u32, h)?;
    if len > budget.min(crate::image_limits::IMAGE_LIMIT) {
        return Err("Clipboard image exceeds the available image memory.".into());
    }
    let stride = (w as usize * bits as usize).div_ceil(32) * 4;
    let needed = header
        .checked_add(stride.checked_mul(h as usize).ok_or_else(bad)?)
        .ok_or_else(bad)?;
    if bytes.len() < needed {
        return Err(bad());
    }
    let alpha = bitfields && u32at(52) == 0xff000000;
    let mut pixels = vec![0; len];
    for y in 0..h as usize {
        let row = if signed_h < 0 { y } else { h as usize - 1 - y };
        for x in 0..w as usize {
            let from = header + row * stride + x * (bits as usize / 8);
            let to = (y * w as usize + x) * 4;
            pixels[to..to + 3].copy_from_slice(&bytes[from..from + 3]);
            pixels[to + 3] = if alpha { bytes[from + 3] } else { 255 };
        }
    }
    Frame::new(w as u32, h, pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dib_24_bit_padding_and_v5_alpha_masks_preserve_rows_and_channels() {
        let mut bytes = vec![0; 48];
        bytes[..4].copy_from_slice(&40u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&1i32.to_le_bytes());
        bytes[8..12].copy_from_slice(&2i32.to_le_bytes());
        bytes[12] = 1;
        bytes[14] = 24;
        bytes[40..48].copy_from_slice(&[7, 8, 9, 99, 1, 2, 3, 99]);
        assert_eq!(
            dib(&bytes, 8).unwrap().bgra,
            vec![1, 2, 3, 255, 7, 8, 9, 255]
        );
        let mut v5 = vec![0; 132];
        v5[..4].copy_from_slice(&124u32.to_le_bytes());
        v5[4..8].copy_from_slice(&1i32.to_le_bytes());
        v5[8..12].copy_from_slice(&(-2i32).to_le_bytes());
        v5[12] = 1;
        v5[14] = 32;
        v5[16] = 3;
        for (offset, mask) in [
            (40, 0x00ff0000u32),
            (44, 0x0000ff00),
            (48, 0x000000ff),
            (52, 0xff000000),
        ] {
            v5[offset..offset + 4].copy_from_slice(&mask.to_le_bytes());
        }
        v5[124..].copy_from_slice(&[1, 2, 3, 128, 4, 5, 6, 0]);
        assert_eq!(dib(&v5, 8).unwrap().bgra, vec![1, 2, 3, 128, 4, 5, 6, 0]);
        v5[40] = 1;
        assert!(dib(&v5, 8).is_err());
    }
    #[test]
    fn png_preserves_alpha_and_rejects_budget_before_pixel_decode() {
        let frame = Frame::new(2, 1, vec![10, 20, 30, 0, 90, 80, 70, 128]).unwrap();
        let mut bytes = vec![];
        frame.write_png(&mut bytes).unwrap();
        assert_eq!(decode(&bytes, 8).unwrap(), frame);
        assert!(decode(&bytes, 7).is_err());
        assert!(decode(&bytes[..bytes.len() / 2], 8).is_err());
        assert!(decode(b"not an image", 8).is_err());
    }
    #[test]
    fn png_roundtrip_accepts_an_image_larger_than_the_previous_limit() {
        let width = 3000;
        let height = 3000;
        let frame = Frame::new(
            width,
            height,
            [17, 83, 191, 255].repeat(width as usize * height as usize),
        )
        .unwrap();
        assert!(frame.bgra.len() > 32 * 1024 * 1024);
        let mut bytes = vec![];
        frame.write_png(&mut bytes).unwrap();
        assert_eq!(decode(&bytes, frame.bgra.len()).unwrap(), frame);
    }
    #[test]
    fn clipboard_dib_accepts_bottom_up_and_top_down_and_rejects_malformed_headers() {
        let frame = Frame::new(
            2,
            2,
            vec![1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255],
        )
        .unwrap();
        let bytes = frame.dib();
        assert_eq!(dib(&bytes, 16).unwrap(), frame);
        let mut top = bytes.clone();
        top[8..12].copy_from_slice(&(-2i32).to_le_bytes());
        top[40..48].copy_from_slice(&bytes[48..56]);
        top[48..56].copy_from_slice(&bytes[40..48]);
        assert_eq!(dib(&top, 16).unwrap(), frame);
        for n in [0, 12, 39, 55] {
            assert!(dib(&bytes[..n], 16).is_err());
        }
        assert!(dib(&bytes, 15).is_err());
        let mut bad = bytes;
        bad[8..12].copy_from_slice(&i32::MIN.to_le_bytes());
        assert!(dib(&bad, SOURCE_LIMIT).is_err());
    }
}

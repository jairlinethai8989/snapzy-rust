use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Result<Self, String> {
        if width == 0
            || height == 0
            || width > 32767
            || height > 32767
            || x.checked_add(width as i32).is_none()
            || y.checked_add(height as i32).is_none()
        {
            return Err("Invalid or oversized capture region.".into());
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub fn from_edges(left: i32, top: i32, right: i32, bottom: i32) -> Result<Self, String> {
        let width = right.checked_sub(left).ok_or("Invalid capture width.")?;
        let height = bottom.checked_sub(top).ok_or("Invalid capture height.")?;
        if width <= 0 || height <= 0 {
            return Err("Select a non-empty image region.".into());
        }
        Self::new(left, top, width as u32, height as u32)
    }

    pub fn right(self) -> i32 {
        self.x + self.width as i32
    }
    pub fn bottom(self) -> i32 {
        self.y + self.height as i32
    }
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.right() && y < self.bottom()
    }
    pub fn intersection(self, other: Self) -> Option<Self> {
        Self::from_edges(
            self.x.max(other.x),
            self.y.max(other.y),
            self.right().min(other.right()),
            self.bottom().min(other.bottom()),
        )
        .ok()
    }
}

#[derive(Debug, PartialEq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

impl Frame {
    pub fn byte_len(width: u32, height: u32) -> Result<usize, String> {
        Rect::new(0, 0, width, height)?;
        let count = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .filter(|n| *n <= crate::image_limits::IMAGE_LIMIT)
            .ok_or("Capture exceeds the 128 MiB image limit.")?;
        Ok(count)
    }

    pub fn new(width: u32, height: u32, bgra: Vec<u8>) -> Result<Self, String> {
        if Self::byte_len(width, height)? != bgra.len() {
            return Err("Capture pixel data is incomplete.".into());
        }
        Ok(Self {
            width,
            height,
            bgra,
        })
    }

    pub fn crop(&self, desktop: Rect, region: Rect) -> Result<Self, String> {
        if (self.width, self.height) != (desktop.width, desktop.height)
            || desktop.intersection(region) != Some(region)
        {
            return Err("The capture region is outside the desktop snapshot.".into());
        }
        let x = (region.x - desktop.x) as usize;
        let y = (region.y - desktop.y) as usize;
        let stride = self.width as usize * 4;
        let row = region.width as usize * 4;
        let mut bytes = Vec::with_capacity(Self::byte_len(region.width, region.height)?);
        for offset in 0..region.height as usize {
            let begin = (y + offset) * stride + x * 4;
            bytes.extend_from_slice(&self.bgra[begin..begin + row]);
        }
        Self::new(region.width, region.height, bytes)
    }

    pub fn write_png(&self, output: impl Write) -> Result<(), String> {
        let mut encoder = png::Encoder::new(output, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        let mut stream = writer.stream_writer().map_err(|e| e.to_string())?;
        let mut row = Vec::new();
        row.try_reserve_exact(self.width as usize * 4)
            .map_err(|_| "Not enough memory to export this image.")?;
        row.resize(self.width as usize * 4, 0);
        for source in self.bgra.chunks_exact(self.width as usize * 4) {
            for (from, to) in source.chunks_exact(4).zip(row.chunks_exact_mut(4)) {
                to.copy_from_slice(&[from[2], from[1], from[0], from[3]]);
            }
            stream.write_all(&row).map_err(|e| e.to_string())?;
        }
        stream.finish().map_err(|e| e.to_string())
    }

    pub fn dib(&self) -> Vec<u8> {
        let mut bytes = vec![0; 40 + self.bgra.len()];
        self.write_dib(&mut bytes)
            .expect("allocated exact DIB size");
        bytes
    }

    pub fn write_dib(&self, bytes: &mut [u8]) -> Result<(), String> {
        if bytes.len() != 40 + self.bgra.len() {
            return Err("Clipboard bitmap buffer has an invalid size.".into());
        }
        bytes[0..4].copy_from_slice(&40u32.to_le_bytes());
        bytes[4..8].copy_from_slice(&(self.width as i32).to_le_bytes());
        bytes[8..12].copy_from_slice(&(self.height as i32).to_le_bytes());
        bytes[12..14].copy_from_slice(&1u16.to_le_bytes());
        bytes[14..16].copy_from_slice(&32u16.to_le_bytes());
        bytes[16..20].copy_from_slice(&0u32.to_le_bytes());
        bytes[20..24].copy_from_slice(&(self.bgra.len() as u32).to_le_bytes());
        bytes[24..40].fill(0);
        // Windows clipboard bitmap synthesis needs bottom-up rows on target PCs.
        let stride = self.width as usize * 4;
        for (target, source) in bytes[40..]
            .chunks_exact_mut(stride)
            .zip(self.bgra.chunks_exact(stride).rev())
        {
            target.copy_from_slice(source);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipboard_is_bottom_up_bgra_dib_at_source_size_for_windows_paste() {
        let frame = Frame::new(1, 2, vec![0, 0, 255, 255, 255, 0, 0, 255]).unwrap();
        let dib = frame.dib();
        assert_eq!(dib.len(), 48);
        assert_eq!(i32::from_le_bytes(dib[8..12].try_into().unwrap()), 2);
        assert_eq!(u16::from_le_bytes(dib[14..16].try_into().unwrap()), 32);
        assert_eq!(&dib[40..], &[255, 0, 0, 255, 0, 0, 255, 255]);
    }
    #[test]
    fn image_bytes_obey_the_new_per_image_limit() {
        assert!(Frame::byte_len(9000, 4000).is_err());
        assert!(Frame::byte_len(8000, 4000).is_ok());
    }
    #[test]
    fn touching_windows_do_not_count_as_occlusion() {
        let a = Rect::new(-100, 0, 100, 100).unwrap();
        let b = Rect::new(0, 0, 100, 100).unwrap();
        assert_eq!(a.intersection(b), None);
        assert_eq!(
            a.intersection(Rect::new(-1, 10, 10, 10).unwrap()),
            Some(Rect::new(-1, 10, 1, 10).unwrap())
        );
    }
}

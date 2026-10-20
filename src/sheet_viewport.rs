use crate::frame::{Frame, Rect};

// Port of Neo Snap 0.1.20: require persistent borders AND a broad colored header.
pub fn find(input: &Frame) -> Option<Rect> {
    let (width, height) = (input.width as usize, input.height as usize);
    if width < 300 || height < 250 {
        return None;
    }
    let pixel = |x: usize, y: usize| {
        let at = (y * width + x) * 4;
        &input.bgra[at..at + 3]
    };
    let sum = |p: &[u8]| p.iter().map(|v| *v as u32).sum::<u32>();
    let column_at = |x: usize, y: usize, neutral: bool| {
        let p = pixel(x, y);
        let low = *p.iter().min().unwrap();
        let high = *p.iter().max().unwrap();
        high <= 240
            && (!neutral || high - low <= 24)
            && (sum(pixel(x - 3, y)) > sum(p) + 30 || sum(pixel(x + 3, y)) > sum(p) + 30)
    };
    let mut columns = Vec::new();
    let (y_start, y_end) = (height * 2 / 5, height * 17 / 20);
    let samples = (y_end - y_start).div_ceil(3);
    for x in 4..width - 4 {
        let votes = (y_start..y_end)
            .step_by(3)
            .filter(|&y| column_at(x, y, true))
            .count();
        if votes * 100 >= samples * 60 && columns.last().is_none_or(|last| x - last > 5) {
            columns.push(x);
        }
    }
    if columns.len() < 4 || (columns.last()? - columns[0]) * 2 < width {
        return None;
    }
    let mut top = (0..height * 7 / 10 - 18).find(|&y| {
        columns
            .iter()
            .filter(|&&x| {
                (1..=16)
                    .step_by(3)
                    .filter(|&offset| column_at(x, y + offset, false))
                    .count()
                    >= 4
            })
            .count()
            >= 4.max((columns.len() * 60).div_ceil(100))
    })?;
    let mut run = 0;
    let mut header_top = None;
    for y in top.saturating_sub(250)..height.min(top + 250) {
        let (mut colored, mut count) = (0, 0);
        for x in (4..width - 4).step_by(7) {
            let p = pixel(x, y);
            let low = *p.iter().min().unwrap();
            let high = *p.iter().max().unwrap();
            colored += usize::from(high - low > 45 && high > 60 && high < 245);
            count += 1;
        }
        run = if colored * 100 > count * 45 {
            run + 1
        } else {
            0
        };
        if run >= 12 {
            header_top = Some(y + 1 - run);
            break;
        }
    }
    while top < height - 20
        && columns
            .iter()
            .filter(|&&x| column_at(x, top, false))
            .count()
            < 3.max(columns.len() / 2)
    {
        top += 1;
    }
    top = top.min(header_top?);
    if top < 1 || top >= height * 7 / 10 {
        return None;
    }
    Rect::new(0, top as i32, input.width, (height - top) as u32).ok()
}

pub fn find_in_desktop(input: &Frame, desktop: Rect, client: Rect) -> Option<Rect> {
    let cropped = input.crop(desktop, client).ok()?;
    let found = find(&cropped)?;
    Rect::new(
        client.x + found.x,
        client.y + found.y,
        found.width,
        found.height,
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use crate::frame::{Frame, Rect};

    fn sheet(header: bool, borders: bool) -> Frame {
        let (width, height) = (600usize, 500usize);
        let mut pixels = vec![255; width * height * 4];
        for y in 0..height {
            for x in 0..width {
                let color = if header && (80..140).contains(&y) {
                    [75, 140, 10, 255]
                } else if borders && y >= 140 && [40, 160, 280, 400, 520].contains(&x) {
                    [30, 30, 30, 255]
                } else {
                    [255, 255, 255, 255]
                };
                pixels[(y * width + x) * 4..(y * width + x + 1) * 4].copy_from_slice(&color);
            }
        }
        Frame::new(width as u32, height as u32, pixels).unwrap()
    }

    #[test]
    fn matches_dotnet_grid_header_and_keeps_frozen_rows_without_mutating_pixels() {
        let frame = sheet(true, true);
        let original = frame.bgra.clone();
        assert_eq!(
            super::find(&frame),
            Some(Rect::new(0, 80, 600, 420).unwrap())
        );
        assert_eq!(frame.bgra, original);
    }

    #[test]
    fn rejects_a_header_without_grid_and_a_grid_without_header() {
        assert_eq!(super::find(&sheet(true, false)), None);
        assert_eq!(super::find(&sheet(false, true)), None);
        let frame = Frame::new(200, 200, vec![255; 200 * 200 * 4]).unwrap();
        assert_eq!(super::find(&frame), None);
    }

    #[test]
    fn screen_coordinates_preserve_negative_monitor_origins_and_reject_outside_crops() {
        let frame = sheet(true, true);
        let desktop = Rect::new(-600, -500, 600, 500).unwrap();
        assert_eq!(
            super::find_in_desktop(&frame, desktop, desktop),
            Some(Rect::new(-600, -420, 600, 420).unwrap())
        );
        assert!(
            super::find_in_desktop(&frame, desktop, Rect::new(-601, -500, 600, 500).unwrap())
                .is_none()
        );
    }

    #[test]
    fn recognizes_an_explicit_local_lark_screenshot() {
        let Ok(path) = std::env::var("SNAPZY_TEST_SHEET_IMAGE") else {
            return;
        };
        let rgba = image::open(path).unwrap().into_rgba8();
        let (width, height) = rgba.dimensions();
        let mut bytes = rgba.into_raw();
        for pixel in bytes.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        let frame = Frame::new(width, height, bytes).unwrap();
        let found = super::find(&frame).expect("Lark table was not detected");
        assert!(found.y > 0 && found.y < 200, "Unexpected header: {found:?}");
        assert_eq!(found.width, width);
        eprintln!("Local Lark screenshot viewport: {found:?}");
    }
}

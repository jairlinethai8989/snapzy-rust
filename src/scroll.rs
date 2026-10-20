use crate::frame::Frame;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    Added,
    Unchanged,
    Ambiguous,
    Limit,
}
pub struct Stitcher {
    first: Arc<Frame>,
    previous: Arc<Frame>,
    strips: Vec<(Vec<u8>, usize)>,
    footer: usize,
    pub height: u32,
    retained: usize,
    budget: usize,
    memory_budget: usize,
}
impl Stitcher {
    pub fn new(first: Frame, budget: usize) -> Result<Self, String> {
        Self::new_with_memory(first, budget, crate::image_limits::SCROLL_WORK_LIMIT)
    }
    pub fn new_with_memory(
        first: Frame,
        budget: usize,
        memory_budget: usize,
    ) -> Result<Self, String> {
        if first.width < 24 || first.height < 80 || first.bgra.len() > budget {
            return Err("Scrolling region is too small or exceeds available image memory.".into());
        }
        if first.bgra.len() > memory_budget {
            return Err("Not enough free memory to finish this scrolling image.".into());
        }
        let height = first.height;
        let retained = first.bgra.len();
        let first = Arc::new(first);
        Ok(Self {
            previous: first.clone(),
            first,
            strips: vec![],
            footer: 0,
            height,
            retained,
            budget,
            memory_budget,
        })
    }
    pub fn add(&mut self, frame: Frame) -> Result<Outcome, String> {
        self.add_with_memory(frame, self.memory_budget)
    }
    pub fn add_with_memory(
        &mut self,
        frame: Frame,
        memory_budget: usize,
    ) -> Result<Outcome, String> {
        self.memory_budget = memory_budget;
        if frame.width != self.first.width || frame.height != self.first.height {
            return Err("Scrolling region changed size. Select it again.".into());
        }
        let w = frame.width as usize;
        let h = frame.height as usize;
        let Some(shift) = find_shift(&self.previous.bgra, &frame.bgra, w, h) else {
            return Ok(Outcome::Ambiguous);
        };
        if shift == 0 {
            return Ok(Outcome::Unchanged);
        }
        let footer = self
            .footer
            .max(detect_footer(&self.previous.bgra, &frame.bgra, w, h, shift));
        if shift >= h - footer || self.strips.iter().any(|(_, s)| *s >= h - footer) {
            return Ok(Outcome::Ambiguous);
        }
        let strip_height = h.min(shift + h.min(480) / 3);
        let bytes = strip_height * w * 4;
        let output_bytes = (self.height as usize + shift)
            .checked_mul(w * 4)
            .ok_or("Scrolling image size overflow.")?;
        let previous_bytes = if Arc::ptr_eq(&self.previous, &self.first) {
            0
        } else {
            self.previous.bgra.len()
        };
        let retained = self
            .retained
            .saturating_sub(previous_bytes)
            .saturating_add(frame.bgra.len())
            .saturating_add(bytes);
        if self.height as usize + shift > 32767
            || output_bytes > self.budget
            || output_bytes.saturating_add(bytes) > memory_budget
            || retained > crate::image_limits::SCROLL_WORK_LIMIT
        {
            return Ok(Outcome::Limit);
        }
        let strip = frame.bgra[(h - strip_height) * w * 4..].to_vec();
        self.strips.push((strip, shift));
        self.previous = Arc::new(frame);
        self.footer = footer;
        self.height += shift as u32;
        self.retained = retained;
        Ok(Outcome::Added)
    }
    pub fn finish(self) -> Result<Frame, String> {
        let memory_budget = self.memory_budget;
        self.finish_with_memory(memory_budget)
    }
    pub fn finish_with_memory(self, memory_budget: usize) -> Result<Frame, String> {
        let stride = self.first.width as usize * 4;
        let output_len = Frame::byte_len(self.first.width, self.height)?;
        if output_len > memory_budget {
            return Err("Not enough free memory to finish this scrolling image.".into());
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(output_len)
            .map_err(|_| "Not enough free memory to finish this scrolling image.")?;
        pixels.extend_from_slice(
            &self.first.bgra[..(self.first.height as usize - self.footer) * stride],
        );
        for (strip, shift) in self.strips {
            let end = strip.len() - self.footer * stride;
            pixels.extend_from_slice(&strip[end - shift * stride..end]);
        }
        pixels.extend_from_slice(
            &self.previous.bgra[self.previous.bgra.len() - self.footer * stride..],
        );
        Frame::new(self.first.width, self.height, pixels)
    }
}

// Adapted from public SnapZy ScrollStitcher.cs, MIT, (c) 2026 jairlinethai.
// Keep stationary chrome out of motion votes and retain footer lookbehind.
fn delta(a: &[u8], at: usize, b: &[u8], bt: usize) -> i32 {
    (0..3)
        .map(|i| (a[at + i] as i32 - b[bt + i] as i32).abs())
        .sum()
}
fn find_shift(old: &[u8], new: &[u8], w: usize, h: usize) -> Option<usize> {
    let moving: Vec<_> = (0..h)
        .map(|y| {
            (2..w - 2)
                .step_by((w / 160).max(2))
                .filter(|x| delta(old, (y * w + x) * 4, new, (y * w + x) * 4) > 24)
                .take(2)
                .count()
                >= 2
        })
        .collect();
    if moving.iter().filter(|v| **v).count() < 3 {
        return Some(0);
    }
    // A footer may animate before it becomes detectable. Motion evidence must
    // come from above the reserved footer band, not its unaligned pixels.
    let end = (moving.iter().rposition(|v| *v).unwrap() + 1).min(h - 160.min(h / 3));
    let score = |shift: usize| {
        let overlap = h - shift;
        let mut error = 0.;
        let mut total = 0.;
        let mut rows = 0;
        for y in (2..overlap.saturating_sub(4)).step_by((overlap / 140).max(2)) {
            if !moving[y] || y + shift >= end {
                continue;
            }
            rows += 1;
            for x in (2..w - 2).step_by((w / 160).max(2)) {
                let a = ((y + shift) * w + x) * 4;
                let b = (y * w + x) * 4;
                let weight = if old[a..a + 3].iter().chain(&new[b..b + 3]).any(|v| *v < 235) {
                    4.
                } else {
                    1.
                };
                error += weight * delta(old, a, new, b) as f64 / 3.;
                total += weight;
            }
        }
        if rows < 6 || total < 100. {
            0.
        } else {
            1. - error / (255. * total)
        }
    };
    if score(0) > 0.985 {
        return Some(0);
    }
    let maximum = (h * 86 / 100).min(h - (h / 5).max(40));
    let scores: Vec<_> = (1..=maximum).map(|s| (s, score(s))).collect();
    let &(shift, best) = scores.iter().max_by(|a, b| a.1.total_cmp(&b.1))?;
    let competing = scores
        .iter()
        .filter(|(s, _)| s.abs_diff(shift) > (h / 50).max(5))
        .map(|(_, s)| *s)
        .fold(0., f64::max);
    (best >= 0.96 && best - competing >= 0.004).then_some(shift)
}
fn detect_footer(old: &[u8], new: &[u8], w: usize, h: usize, shift: usize) -> usize {
    let step = (w / 160).max(2);
    let samples = (w - 5) / step + 1;
    let mut first = h;
    let mut stable = 0;
    let mut separator = 0;
    for y in (h - 160.min(h / 3)..h).rev() {
        let mut changed = 0;
        let mut gray = 0;
        for x in (2..w - 2).step_by(step) {
            let a = (y * w + x) * 4;
            if delta(old, a, new, a) > 24 {
                changed += 1;
            }
            let low = *new[a..a + 3].iter().min().unwrap();
            let high = *new[a..a + 3].iter().max().unwrap();
            if low >= 160 && high <= 240 && high - low <= 12 {
                gray += 1;
            }
        }
        if changed < 2 {
            stable += 1;
            separator = 0;
        } else {
            if (separator == 0 && stable < 3)
                || separator >= 2
                || changed * 100 > samples * 15
                || gray * 100 < samples * 85
            {
                break;
            }
            separator += 1;
            stable = 0;
        }
        first = y;
    }
    if h - first < 5 {
        return 0;
    }
    for y in first.max(1)..h - 4 {
        let mut edges = 0;
        let mut stationary = 0;
        for x in (2..w - 2).step_by(step) {
            let a = (y * w + x) * 4;
            if delta(new, a, new, a - w * 4) > 36 {
                edges += 1;
            }
            if y >= shift && delta(old, a, new, a - shift * w * 4) > 24 {
                stationary += 1;
            }
        }
        if edges * 100 <= samples * 35 || stationary * 100 <= samples * 20 {
            continue;
        }
        let mut boundary = y;
        if edges * 100 <= samples * 75 {
            boundary -= 1;
            while boundary > first {
                let differences = (0..w)
                    .filter(|x| {
                        let a = (boundary * w + x) * 4;
                        delta(new, a, new, a - w * 4) > 36
                    })
                    .take(2)
                    .count();
                if differences >= 2 {
                    break;
                }
                boundary -= 1;
            }
        }
        return h - boundary;
    }
    0
}

#[derive(Default)]
pub struct Settler {
    pending: Option<(Frame, std::time::Duration)>,
    unsettled_since: Option<std::time::Duration>,
}
impl Settler {
    pub fn stalled(&self, now: std::time::Duration) -> bool {
        self.unsettled_since
            .is_some_and(|since| now.saturating_sub(since) >= std::time::Duration::from_secs(3))
    }
    pub fn offer(&mut self, frame: Frame, now: std::time::Duration) -> Option<Frame> {
        self.unsettled_since.get_or_insert(now);
        if let Some((pending, since)) = &self.pending {
            if pending.width == frame.width
                && pending.height == frame.height
                && pending.bgra == frame.bgra
            {
                if now.saturating_sub(*since) >= std::time::Duration::from_millis(200) {
                    self.unsettled_since = None;
                    return Some(frame);
                }
                return None;
            }
        }
        self.pending = Some((frame, now));
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const W: usize = 240;
    const H: usize = 400;
    fn pixel(row: usize, x: usize) -> [u8; 4] {
        // Independent synthetic document: unequal table rows with distinct ink.
        let cell = row / 37 + row / 83;
        if row.is_multiple_of(37) || x.is_multiple_of(61) {
            [30, 30, 30, 255]
        } else if (x * 19 + cell * 53) % 97 < 16 && row % 37 > 8 && row % 37 < 27 {
            [(row % 127) as u8, (cell % 137) as u8, 50, 255]
        } else {
            [250, 250, 250, 255]
        }
    }
    fn viewport(offset: usize, header: usize, footer: usize, changing: bool) -> Frame {
        let mut pixels = Vec::with_capacity(W * H * 4);
        for y in 0..H {
            for x in 0..W {
                let p = if y < header {
                    [15, 130, 45, 255]
                } else if y >= H - footer {
                    if changing {
                        [(offset % 200) as u8, 177, 190, 255]
                    } else if y == H - footer || (y >= H - footer + 5 && x < W / 2) {
                        [190, 190, 190, 255]
                    } else {
                        [245, 245, 245, 255]
                    }
                } else {
                    pixel(y - header + offset, x)
                };
                pixels.extend_from_slice(&p);
            }
        }
        Frame::new(W as u32, H as u32, pixels).unwrap()
    }
    fn golden(offset: usize, header: usize, footer: usize) -> Vec<u8> {
        let first = viewport(0, header, footer, false);
        let last = viewport(offset, header, footer, false);
        let mut expected = first.bgra[..header * W * 4].to_vec();
        for y in 0..H - header - footer + offset {
            for x in 0..W {
                expected.extend_from_slice(&pixel(y, x));
            }
        }
        expected.extend_from_slice(&last.bgra[(H - footer) * W * 4..]);
        expected
    }
    #[test]
    fn body_pixels_join_without_missing_or_duplicate_rows() {
        let mut s = Stitcher::new(viewport(0, 0, 0, false), 4_000_000).unwrap();
        for offset in [137, 246, 311] {
            assert_eq!(
                s.add(viewport(offset, 0, 0, false)).unwrap(),
                Outcome::Added
            );
        }
        let image = s.finish().unwrap();
        assert_eq!(image.height, 711);
        assert_eq!(image.bgra, golden(311, 0, 0));
    }
    #[test]
    fn frozen_header_and_light_footer_appear_only_once() {
        for footer in [12, 42, 64] {
            let mut s = Stitcher::new(viewport(0, 70, footer, false), 4_000_000).unwrap();
            for offset in [102, 193, 263] {
                assert_eq!(
                    s.add(viewport(offset, 70, footer, false)).unwrap(),
                    Outcome::Added
                );
            }
            assert_eq!(s.finish().unwrap().bgra, golden(263, 70, footer));
        }
    }
    #[test]
    fn late_footer_repairs_earlier_pending_strips_losslessly() {
        let mut s = Stitcher::new(viewport(0, 55, 42, true), 4_000_000).unwrap();
        for (offset, changing) in [(94, true), (181, true), (253, false), (321, false)] {
            assert_eq!(
                s.add(viewport(offset, 55, 42, changing)).unwrap(),
                Outcome::Added,
                "offset={offset}"
            );
        }
        assert_eq!(s.finish().unwrap().bgra, golden(321, 55, 42));
    }
    #[test]
    fn unchanged_frames_do_not_duplicate_output() {
        let mut s = Stitcher::new(viewport(0, 45, 12, false), 4_000_000).unwrap();
        assert_eq!(
            s.add(viewport(0, 45, 12, false)).unwrap(),
            Outcome::Unchanged
        );
        assert_eq!(s.finish().unwrap().bgra, golden(0, 45, 12));
    }
    #[test]
    fn reversed_or_ambiguous_frames_preserve_last_valid_image() {
        let mut s = Stitcher::new(viewport(200, 45, 12, false), 4_000_000).unwrap();
        let old = s.first.bgra.clone();
        assert_eq!(
            s.add(viewport(50, 45, 12, false)).unwrap(),
            Outcome::Ambiguous
        );
        assert_eq!(s.finish().unwrap().bgra, old);
    }
    #[test]
    fn limits_and_dimension_changes_do_not_corrupt_partial_result() {
        assert!(Stitcher::new(viewport(0, 0, 0, false), 10).is_err());
        let mut s = Stitcher::new(viewport(0, 0, 0, false), W * H * 4 + 10).unwrap();
        assert_eq!(s.add(viewport(137, 0, 0, false)).unwrap(), Outcome::Limit);
        assert!(s.add(Frame::new(1, 1, vec![0; 4]).unwrap()).is_err());
        assert_eq!(s.finish().unwrap().bgra, golden(0, 0, 0));
    }
    #[test]
    fn low_available_memory_stops_before_committing_an_unfinishable_step() {
        let first = viewport(0, 0, 0, false);
        let mut s = Stitcher::new_with_memory(first, 4_000_000, 600_000).unwrap();
        assert_eq!(s.add(viewport(137, 0, 0, false)).unwrap(), Outcome::Limit);
        assert_eq!(s.height, H as u32);
        assert_eq!(s.finish_with_memory(600_000).unwrap().height, H as u32);
    }
    #[test]
    fn settling_requires_stable_body_not_just_frozen_header() {
        let mut gate = Settler::default();
        let ms = std::time::Duration::from_millis;
        assert!(gate.offer(viewport(0, 55, 42, false), ms(0)).is_none());
        assert!(gate.offer(viewport(0, 55, 42, false), ms(100)).is_none());
        assert!(gate.offer(viewport(30, 55, 42, false), ms(250)).is_none());
        assert!(gate.offer(viewport(60, 55, 42, false), ms(500)).is_none());
        let frame = gate.offer(viewport(60, 55, 42, false), ms(750)).unwrap();
        assert_eq!(frame.bgra, viewport(60, 55, 42, false).bgra);
        assert!(
            gate.offer(viewport(60, 55, 42, false), ms(800)).is_some(),
            "A still-stable frame must not restart the settling delay"
        );
    }
    #[test]
    fn persistent_animation_is_reported_until_a_stable_frame_recovers() {
        let mut gate = Settler::default();
        let ms = std::time::Duration::from_millis;
        gate.offer(viewport(0, 55, 42, false), ms(0));
        gate.offer(viewport(30, 55, 42, false), ms(1500));
        gate.offer(viewport(60, 55, 42, false), ms(3300));
        assert!(gate.stalled(ms(3300)));
        assert!(gate.offer(viewport(60, 55, 42, false), ms(3600)).is_some());
        assert!(!gate.stalled(ms(3600)));
    }
    #[test]
    fn repeating_grid_does_not_guess_one_of_multiple_scroll_offsets() {
        let periodic = |offset| {
            let mut f = viewport(0, 55, 42, false);
            for y in 55..358 {
                for x in 0..W {
                    let p = pixel((y - 55 + offset) % 37, x);
                    f.bgra[(y * W + x) * 4..(y * W + x + 1) * 4].copy_from_slice(&p);
                }
            }
            f
        };
        let mut s = Stitcher::new(periodic(0), 4_000_000).unwrap();
        let before = s.first.bgra.clone();
        assert_eq!(s.add(periodic(7)).unwrap(), Outcome::Ambiguous);
        assert_eq!(s.finish().unwrap().bgra, before);
    }
    #[test]
    fn horizontal_movement_is_not_appended_as_vertical_motion() {
        let mut changed = viewport(94, 55, 42, false);
        for row in changed.bgra.chunks_exact_mut(W * 4).skip(55).take(303) {
            row.rotate_left(17 * 4);
        }
        let mut s = Stitcher::new(viewport(0, 55, 42, false), 4_000_000).unwrap();
        assert_eq!(s.add(changed).unwrap(), Outcome::Ambiguous);
        assert_eq!(s.finish().unwrap().bgra, golden(0, 55, 42));
    }
    #[test]
    fn changed_scale_is_not_mistaken_for_downward_scroll() {
        let mut changed = viewport(94, 55, 42, false);
        for y in 55..358 {
            for x in 0..W {
                let row = (y - 55) * 101 / 100 + 94;
                let p = pixel(row, x * 101 / 100);
                changed.bgra[(y * W + x) * 4..(y * W + x + 1) * 4].copy_from_slice(&p);
            }
        }
        let mut s = Stitcher::new(viewport(0, 55, 42, false), 4_000_000).unwrap();
        assert_eq!(s.add(changed).unwrap(), Outcome::Ambiguous);
        assert_eq!(s.finish().unwrap().bgra, golden(0, 55, 42));
    }
    #[test]
    fn blended_scrollbar_separators_do_not_repeat_at_joins() {
        for separator in [1, 2] {
            let blended = |offset| {
                let mut f = viewport(offset, 55, 42, false);
                for y in 358..358 + separator {
                    for x in 0..W {
                        let p = if x < 4 {
                            pixel(y - 55 + offset, x)
                        } else {
                            [190, 190, 190, 255]
                        };
                        f.bgra[(y * W + x) * 4..(y * W + x + 1) * 4].copy_from_slice(&p);
                    }
                }
                f
            };
            let mut s = Stitcher::new(blended(0), 4_000_000).unwrap();
            for offset in [94, 181, 253] {
                assert_eq!(s.add(blended(offset)).unwrap(), Outcome::Added);
            }
            let last = blended(253);
            let mut expected = golden(253, 55, 42);
            let footer_start = expected.len() - 42 * W * 4;
            expected[footer_start..].copy_from_slice(&last.bgra[358 * W * 4..]);
            assert_eq!(s.finish().unwrap().bgra, expected);
        }
    }
}

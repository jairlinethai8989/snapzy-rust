#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shadow {
    pub enabled: bool,
    pub rgb: [u8; 3],
    pub blur: u32,
    pub x: i32,
    pub y: i32,
}
impl Default for Shadow {
    fn default() -> Self {
        Self {
            enabled: false,
            rgb: [0; 3],
            blur: 6,
            x: 3,
            y: 3,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outline {
    pub enabled: bool,
    pub rgb: [u8; 3],
    pub width: u32,
}
impl Default for Outline {
    fn default() -> Self {
        Self {
            enabled: false,
            rgb: [0; 3],
            width: 2,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effects {
    pub shadow: Shadow,
    pub outline: Outline,
}
impl Effects {
    pub fn validate(self) -> Result<Self, String> {
        if self.shadow.blur > 32
            || !(-32..=32).contains(&self.shadow.x)
            || !(-32..=32).contains(&self.shadow.y)
            || !(1..=16).contains(&self.outline.width)
        {
            return Err(
                "Invalid effect settings: softness 0-32, offsets -32 to 32, outline 1-16 px."
                    .into(),
            );
        }
        Ok(self)
    }
    pub fn active(self, text: bool) -> bool {
        self.shadow.enabled || (text && self.outline.enabled)
    }
    pub fn reach(self, text: bool) -> i32 {
        let outline = if text && self.outline.enabled {
            self.outline.width as i32
        } else {
            0
        };
        outline
            + if self.shadow.enabled {
                3 * self.shadow.blur as i32 + self.shadow.x.abs().max(self.shadow.y.abs())
            } else {
                0
            }
    }
}

pub fn outlined(mask: &crate::glyphs::Mask, radius: u32) -> Result<crate::glyphs::Mask, String> {
    use std::collections::VecDeque;
    if !(1..=16).contains(&radius) {
        return Err("Invalid outline width.".into());
    }
    let width = mask.width + 2 * radius;
    let height = mask.height + 2 * radius;
    let mut alpha = vec![0; crate::frame::Frame::byte_len(width, height)? / 4];
    let r = radius as i32;
    // Circular dilation by scanlines, using a sliding maximum rather than per-pixel disks.
    for dy in -r..=r {
        let dx = ((r * r - dy * dy) as f64).sqrt().floor() as i32;
        for y in 0..mask.height as i32 {
            let mut queue = VecDeque::<(i32, u8)>::new();
            let mut next = 0;
            for x in -r..mask.width as i32 + r {
                let right = (x + dx).min(mask.width as i32 - 1);
                while next <= right {
                    let v = mask.alpha[(y as u32 * mask.width + next as u32) as usize];
                    while queue.back().is_some_and(|(_, a)| *a <= v) {
                        queue.pop_back();
                    }
                    queue.push_back((next, v));
                    next += 1;
                }
                while queue.front().is_some_and(|(i, _)| *i < x - dx) {
                    queue.pop_front();
                }
                let i = ((y + r + dy) as u32 * width + (x + r) as u32) as usize;
                alpha[i] = alpha[i].max(queue.front().map(|(_, a)| *a).unwrap_or(0));
            }
        }
    }
    Ok(crate::glyphs::Mask {
        width,
        height,
        alpha,
    })
}

pub fn soft_mask(
    alpha: Vec<u8>,
    width: u32,
    height: u32,
    softness: u32,
) -> Result<Vec<u8>, String> {
    let image = image::GrayImage::from_raw(width, height, alpha).ok_or("Invalid shadow mask.")?;
    if softness == 0 {
        return Ok(image.into_raw());
    }
    if softness > 32 {
        return Err("Invalid shadow softness.".into());
    }
    Ok(image::imageops::fast_blur(&image, softness as f32).into_raw())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shadow_softness_uses_existing_fast_gaussian_approximation() {
        let mut alpha = vec![0; 64 * 64];
        for y in 20..44 {
            for x in 20..44 {
                alpha[y * 64 + x] = 255;
            }
        }
        let source = image::GrayImage::from_raw(64, 64, alpha.clone()).unwrap();
        let expected = image::imageops::fast_blur(&source, 6.).into_raw();
        assert_eq!(soft_mask(alpha, 64, 64, 6).unwrap(), expected);
    }
}

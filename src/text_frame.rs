use crate::editor::{Annotation, Content, Point};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Left,
    Right,
}
pub const HANDLES: [Handle; 6] = [
    Handle::TopLeft,
    Handle::TopRight,
    Handle::BottomRight,
    Handle::BottomLeft,
    Handle::Left,
    Handle::Right,
];
pub fn positions((a, b): (Point, Point)) -> [(Handle, Point); 6] {
    let p = [
        a,
        Point::new(b.x, a.y),
        b,
        Point::new(a.x, b.y),
        Point::new(a.x, (a.y + b.y) / 2.),
        Point::new(b.x, (a.y + b.y) / 2.),
    ];
    std::array::from_fn(|i| (HANDLES[i], p[i]))
}
pub fn hit(bounds: (Point, Point), point: Point, radius: f64) -> Option<Handle> {
    if !radius.is_finite() || radius < 0. {
        return None;
    }
    positions(bounds)
        .into_iter()
        .find(|(_, p)| (p.x - point.x).abs() <= radius && (p.y - point.y).abs() <= radius)
        .map(|(h, _)| h)
}
#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub size: u32,
    pub width: Option<u32>,
    pub anchor: Point,
    pub right: bool,
    pub bottom: bool,
}
pub struct Resize {
    pub original: Annotation,
    pub handle: Handle,
    pub start: Point,
    pub end: Point,
}
impl Resize {
    pub fn proposal(&self) -> Option<Proposal> {
        let Content::Text {
            size,
            wrap_width,
            layout,
            ..
        } = self.original.content
        else {
            return None;
        };
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        if !dx.is_finite() || !dy.is_finite() || (dx.abs() < 0.001 && dy.abs() < 0.001) {
            return None;
        }
        let (a, b) = self.original.bounds();
        if matches!(self.handle, Handle::Left | Handle::Right) {
            let right = self.handle == Handle::Left;
            let width = ((b.x - a.x) + if right { -dx } else { dx })
                .round()
                .clamp(24., 32767.) as u32;
            if width == (b.x - a.x).round() as u32 {
                return None;
            }
            return Some(Proposal {
                size,
                width: Some(width),
                anchor: Point::new(if right { b.x } else { a.x }, a.y),
                right,
                bottom: false,
            });
        }
        let right = matches!(self.handle, Handle::TopLeft | Handle::BottomLeft);
        let bottom = matches!(self.handle, Handle::TopLeft | Handle::TopRight);
        let anchor = Point::new(
            if right { b.x } else { a.x },
            if bottom { b.y } else { a.y },
        );
        let vx = if right { a.x - b.x } else { b.x - a.x };
        let vy = if bottom { a.y - b.y } else { b.y - a.y };
        let ratio = ((vx + dx) * vx + (vy + dy) * vy) / (vx * vx + vy * vy);
        let new_size = (size as f64 * ratio).round().clamp(8., 256.) as u32;
        if new_size == size {
            return None;
        }
        let width = match layout {
            crate::editor::TextLayout::DotNet {
                bubble: Some(frame),
                ..
            } => Some(
                (frame.width * new_size as f64 / size as f64)
                    .round()
                    .clamp(24., 32767.) as u32,
            ),
            _ => wrap_width.map(|w| {
                (w as f64 * new_size as f64 / size as f64)
                    .round()
                    .clamp(24., 32767.) as u32
            }),
        };
        Some(Proposal {
            size: new_size,
            width,
            anchor,
            right,
            bottom,
        })
    }
    pub fn bounds(&self) -> (Point, Point) {
        if self.original.tool == crate::editor::Tool::Image {
            let (a, b) = self.original.bounds();
            let right = matches!(self.handle, Handle::TopLeft | Handle::BottomLeft);
            let bottom = matches!(self.handle, Handle::TopLeft | Handle::TopRight);
            let anchor = Point::new(
                if right { b.x } else { a.x },
                if bottom { b.y } else { a.y },
            );
            let vx = if right { a.x - b.x } else { b.x - a.x };
            let vy = if bottom { a.y - b.y } else { b.y - a.y };
            let dx = self.end.x - self.start.x;
            let dy = self.end.y - self.start.y;
            if !dx.is_finite() || !dy.is_finite() {
                return (a, b);
            }
            let ratio = (((vx + dx) * vx + (vy + dy) * vy) / (vx * vx + vy * vy))
                .clamp(1. / vx.abs().min(vy.abs()), 32767. / vx.abs().max(vy.abs()));
            let width = (b.x - a.x) * ratio;
            let height = (b.y - a.y) * ratio;
            let origin = Point::new(
                anchor.x - if right { width } else { 0. },
                anchor.y - if bottom { height } else { 0. },
            );
            return (origin, Point::new(origin.x + width, origin.y + height));
        }
        let Some(p) = self.proposal() else {
            return self.original.bounds();
        };
        let (a, b) = self.original.bounds();
        let corner = !matches!(self.handle, Handle::Left | Handle::Right);
        let ratio = p.size as f64 / self.original.parameter() as f64;
        let width = if corner {
            (b.x - a.x) * ratio
        } else {
            p.width.unwrap() as f64
        };
        let height = if corner {
            (b.y - a.y) * ratio
        } else {
            b.y - a.y
        };
        let origin = Point::new(
            p.anchor.x - if p.right { width } else { 0. },
            p.anchor.y - if p.bottom { height } else { 0. },
        );
        (origin, Point::new(origin.x + width, origin.y + height))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn image_corner_drag_keeps_aspect_and_opposite_corner_without_resampling() {
        use crate::{
            editor::{Document, Point},
            frame::Frame,
        };
        use std::sync::Arc;
        let mut d = Document::new(200, 100);
        d.add_image(Arc::new(Frame::new(40, 20, vec![255; 3200]).unwrap()))
            .unwrap();
        let r = super::Resize {
            original: d.objects[0].clone(),
            handle: super::Handle::BottomRight,
            start: Point::new(40., 20.),
            end: Point::new(80., 40.),
        };
        assert_eq!(r.bounds(), (Point::new(0., 0.), Point::new(80., 40.)));
    }
    use super::*;
    use crate::editor::{Content, Style, Tool};
    use std::sync::Arc;
    fn text() -> Annotation {
        Annotation {
            id: 1,
            parent: None,
            tool: Tool::Text,
            points: Arc::new(vec![Point::new(100., 100.)]),
            style: Style::default(),
            pattern: Default::default(),
            rendering: Default::default(),
            opacity: 100,
            effects: Default::default(),
            content: Content::Text {
                text: Arc::from("Text"),
                size: 24,
                wrap_width: Some(120),
                background: Default::default(),
                tail: None,
                layout: crate::editor::TextLayout::Native,
                mask: Arc::new(crate::glyphs::Mask {
                    width: 120,
                    height: 60,
                    alpha: vec![0; 7200],
                }),
            },
        }
    }
    #[test]
    fn six_handles_hit_in_source_coordinates_and_corners_take_priority() {
        let bounds = (Point::new(100., 100.), Point::new(220., 160.));
        let expected = [
            (Handle::TopLeft, Point::new(100., 100.)),
            (Handle::TopRight, Point::new(220., 100.)),
            (Handle::BottomRight, Point::new(220., 160.)),
            (Handle::BottomLeft, Point::new(100., 160.)),
            (Handle::Left, Point::new(100., 130.)),
            (Handle::Right, Point::new(220., 130.)),
        ];
        assert_eq!(positions(bounds), expected);
        for (h, p) in expected {
            assert_eq!(hit(bounds, Point::new(p.x + 2., p.y + 2.), 5.), Some(h));
        }
        assert_eq!(hit(bounds, Point::new(160., 130.), 5.), None);
        assert_eq!(
            hit(bounds, Point::new(100., 130.), 40.),
            Some(Handle::TopLeft)
        );
        assert_eq!(hit(bounds, Point::new(100., 100.), f64::NAN), None);
    }
    #[test]
    fn side_resize_changes_width_only_and_keeps_opposite_edge() {
        let r = Resize {
            original: text(),
            handle: Handle::Left,
            start: Point::new(100., 130.),
            end: Point::new(140., 130.),
        };
        assert_eq!(
            r.proposal(),
            Some(Proposal {
                size: 24,
                width: Some(80),
                anchor: Point::new(220., 100.),
                right: true,
                bottom: false
            })
        );
        assert_eq!(r.bounds(), (Point::new(140., 100.), Point::new(220., 160.)));
        let r = Resize {
            end: Point::new(400., 130.),
            ..r
        };
        assert_eq!(r.proposal().unwrap().width, Some(24));
        let r = Resize { end: r.start, ..r };
        assert_eq!(r.proposal(), None);
    }
    #[test]
    fn corner_scaling_is_uniform_fixed_anchor_and_clamped_without_flipping() {
        for (handle, start, end, anchor, right, bottom) in [
            (
                Handle::TopLeft,
                Point::new(100., 100.),
                Point::new(-20., 40.),
                Point::new(220., 160.),
                true,
                true,
            ),
            (
                Handle::TopRight,
                Point::new(220., 100.),
                Point::new(340., 40.),
                Point::new(100., 160.),
                false,
                true,
            ),
            (
                Handle::BottomRight,
                Point::new(220., 160.),
                Point::new(340., 220.),
                Point::new(100., 100.),
                false,
                false,
            ),
            (
                Handle::BottomLeft,
                Point::new(100., 160.),
                Point::new(-20., 220.),
                Point::new(220., 100.),
                true,
                false,
            ),
        ] {
            let r = Resize {
                original: text(),
                handle,
                start,
                end,
            };
            assert_eq!(
                r.proposal(),
                Some(Proposal {
                    size: 48,
                    width: Some(240),
                    anchor,
                    right,
                    bottom
                })
            );
            let r = Resize { end: anchor, ..r };
            assert_eq!(r.proposal().unwrap().size, 8);
            let r = Resize {
                end: Point::new(
                    start.x + (end.x - start.x) * 1000.,
                    start.y + (end.y - start.y) * 1000.,
                ),
                ..r
            };
            assert_eq!(r.proposal().unwrap().size, 256);
        }
    }
}

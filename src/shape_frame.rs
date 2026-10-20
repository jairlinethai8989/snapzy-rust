use crate::editor::{Annotation, Point, Tool};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Left,
    Right,
    Top,
    Bottom,
}

pub fn supported(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Rectangle | Tool::Ellipse | Tool::FilledRectangle | Tool::FilledEllipse
    )
}

pub fn positions((a, b): (Point, Point)) -> [(Handle, Point); 8] {
    use Handle::*;
    let x = (a.x + b.x) / 2.;
    let y = (a.y + b.y) / 2.;
    [
        (TopLeft, a),
        (TopRight, Point::new(b.x, a.y)),
        (BottomRight, b),
        (BottomLeft, Point::new(a.x, b.y)),
        (Left, Point::new(a.x, y)),
        (Right, Point::new(b.x, y)),
        (Top, Point::new(x, a.y)),
        (Bottom, Point::new(x, b.y)),
    ]
}

pub struct Resize {
    pub original: Annotation,
    pub handle: Handle,
    pub start: Point,
    pub end: Point,
}
impl Resize {
    pub fn draft(&self) -> Option<Annotation> {
        use Handle::*;
        if !supported(self.original.tool)
            || self.start == self.end
            || [self.start.x, self.start.y, self.end.x, self.end.y]
                .iter()
                .any(|v| !v.is_finite())
        {
            return None;
        }
        let (mut a, mut b) = self.original.bounds();
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        if matches!(self.handle, TopLeft | BottomLeft | Left) {
            a.x = (a.x + dx).min(b.x - 1.);
        }
        if matches!(self.handle, TopRight | BottomRight | Right) {
            b.x = (b.x + dx).max(a.x + 1.);
        }
        if matches!(self.handle, TopLeft | TopRight | Top) {
            a.y = (a.y + dy).min(b.y - 1.);
        }
        if matches!(self.handle, BottomLeft | BottomRight | Bottom) {
            b.y = (b.y + dy).max(a.y + 1.);
        }
        if !valid_bounds((a, b)) || (a, b) == self.original.bounds() {
            return None;
        }
        let mut object = self.original.clone();
        object.points = Arc::new(vec![a, b]);
        Some(object)
    }
}

pub fn valid_bounds((a, b): (Point, Point)) -> bool {
    [a.x, a.y, b.x, b.y]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 32767.)
        && (1. ..=32767.).contains(&(b.x - a.x))
        && (1. ..=32767.).contains(&(b.y - a.y))
}

use crate::editor::{CalloutStyle, Point};

#[cfg(test)]
pub fn perimeter((a, b): (Point, Point), tip: Point) -> Option<tiny_skia::Path> {
    perimeter_style((a, b), tip, CalloutStyle::Rounded)
}

pub fn perimeter_style((a, b): (Point, Point), tip: Point, style: CalloutStyle) -> Option<tiny_skia::Path> {
    if [a, b, tip]
        .iter()
        .any(|p| !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 65535. || p.y.abs() > 65535.)
        || b.x <= a.x
        || b.y <= a.y
    {
        return None;
    }
    if style == CalloutStyle::Ellipse {
        let center = Point::new((a.x + b.x) / 2., (a.y + b.y) / 2.);
        let rx = (b.x - a.x) / 2.;
        let ry = (b.y - a.y) / 2.;
        let point = |i: usize| {
            let angle = -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::TAU / 64.;
            Point::new(center.x + rx * angle.cos(), center.y + ry * angle.sin())
        };
        let outside = ((tip.x - center.x) / rx).powi(2) + ((tip.y - center.y) / ry).powi(2) > 1.;
        let nearest = if outside {
            (1..64).min_by(|&i, &j| {
                distance(tip, point(i - 1), point(i + 1))
                    .total_cmp(&distance(tip, point(j - 1), point(j + 1)))
            })
        } else { None };
        let mut builder = tiny_skia::PathBuilder::new();
        let start = point(0);
        builder.move_to(start.x as f32, start.y as f32);
        for i in 1..=64 {
            if Some(i) == nearest {
                builder.line_to(tip.x as f32, tip.y as f32);
            }
            let p = point(i);
            builder.line_to(p.x as f32, p.y as f32);
        }
        builder.close();
        return builder.finish();
    }
    let radius = if style == CalloutStyle::Square { 0. } else { 8f64.min((b.x - a.x) / 4.).min((b.y - a.y) / 4.) };
    let edges = [
        (
            Point::new(a.x + radius, a.y),
            Point::new(b.x - radius, a.y),
            Point::new(b.x, a.y),
        ),
        (
            Point::new(b.x, a.y + radius),
            Point::new(b.x, b.y - radius),
            b,
        ),
        (
            Point::new(b.x - radius, b.y),
            Point::new(a.x + radius, b.y),
            Point::new(a.x, b.y),
        ),
        (
            Point::new(a.x, b.y - radius),
            Point::new(a.x, a.y + radius),
            a,
        ),
    ];
    let outside = tip.x < a.x || tip.x > b.x || tip.y < a.y || tip.y > b.y;
    let edge = (0..4).min_by(|&i, &j| {
        distance(tip, edges[i].0, edges[i].1).total_cmp(&distance(tip, edges[j].0, edges[j].1))
    })?;
    let mut builder = tiny_skia::PathBuilder::new();
    builder.move_to(edges[0].0.x as f32, edges[0].0.y as f32);
    for (index, (start, end, corner)) in edges.iter().enumerate() {
        if outside && index == edge {
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let length = dx.hypot(dy);
            let half = 10f64.min(length / 4.);
            let along = (((tip.x - start.x) * dx + (tip.y - start.y) * dy) / length)
                .clamp(half, length - half);
            for p in [
                Point::new(
                    start.x + dx / length * (along - half),
                    start.y + dy / length * (along - half),
                ),
                tip,
                Point::new(
                    start.x + dx / length * (along + half),
                    start.y + dy / length * (along + half),
                ),
            ] {
                builder.line_to(p.x as f32, p.y as f32);
            }
        }
        builder.line_to(end.x as f32, end.y as f32);
        let next = edges[(index + 1) % 4].0;
        if radius > 0. {
            builder.quad_to(corner.x as f32, corner.y as f32, next.x as f32, next.y as f32);
        }
    }
    builder.close();
    builder.finish()
}
fn distance(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let t = if dx * dx + dy * dy == 0. {
        0.
    } else {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy)
    }
    .clamp(0., 1.);
    (p.x - a.x - t * dx).hypot(p.y - a.y - t * dy)
}
#[cfg(test)]
pub fn hit(body: (Point, Point), tip: Point, point: Point, tolerance: f64, filled: bool) -> bool {
    hit_style(body, tip, point, tolerance, filled, CalloutStyle::Rounded)
}

pub fn hit_style(body: (Point, Point), tip: Point, point: Point, tolerance: f64, filled: bool, style: CalloutStyle) -> bool {
    if !point.x.is_finite() || !point.y.is_finite() || !tolerance.is_finite() || tolerance < 0. {
        return false;
    }
    let Some(path) = perimeter_style(body, tip, style) else {
        return false;
    };
    let mut outline = Vec::new();
    for segment in path.segments() {
        match segment {
            tiny_skia::PathSegment::MoveTo(p) | tiny_skia::PathSegment::LineTo(p) => {
                outline.push(Point::new(p.x as f64, p.y as f64))
            }
            tiny_skia::PathSegment::QuadTo(c, b) => {
                let a = *outline.last().unwrap();
                for i in 1..=12 {
                    let t = i as f64 / 12.;
                    let u = 1. - t;
                    outline.push(Point::new(
                        u * u * a.x + 2. * u * t * c.x as f64 + t * t * b.x as f64,
                        u * u * a.y + 2. * u * t * c.y as f64 + t * t * b.y as f64,
                    ));
                }
            }
            _ => {}
        }
    }
    let mut inside = false;
    for i in 0..outline.len() {
        let a = outline[i];
        let b = outline[(i + 1) % outline.len()];
        if distance(point, a, b) <= tolerance {
            return true;
        }
        if (a.y > point.y) != (b.y > point.y)
            && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
        {
            inside = !inside;
        }
    }
    filled && inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::CalloutStyle;

    #[test]
    fn frame_presets_keep_tail_and_distinct_outlines() {
        let body = (Point::new(20., 20.), Point::new(120., 80.));
        let tip = Point::new(70., 110.);
        let rounded = perimeter_style(body, tip, CalloutStyle::Rounded).unwrap();
        let square = perimeter_style(body, tip, CalloutStyle::Square).unwrap();
        let ellipse = perimeter_style(body, tip, CalloutStyle::Ellipse).unwrap();
        assert_eq!(rounded.bounds().bottom(), 110.);
        assert_eq!(square.bounds().bottom(), 110.);
        assert_eq!(ellipse.bounds().bottom(), 110.);
        assert!(rounded.segments().any(|s| matches!(s, tiny_skia::PathSegment::QuadTo(..))));
        assert!(!square.segments().any(|s| matches!(s, tiny_skia::PathSegment::QuadTo(..))));
        assert!(hit_style(body, tip, tip, 2., false, CalloutStyle::Ellipse));
    }
    #[test]
    fn perimeter_reaches_each_tail_without_internal_base_line() {
        let body = (Point::new(20., 20.), Point::new(120., 80.));
        for (tip, want) in [
            (Point::new(60., 0.), [20., 0., 120., 80.]),
            (Point::new(150., 50.), [20., 20., 150., 80.]),
            (Point::new(60., 110.), [20., 20., 120., 110.]),
            (Point::new(0., 50.), [0., 20., 120., 80.]),
        ] {
            let path = perimeter(body, tip).expect("callout perimeter");
            let r = path.bounds();
            assert_eq!([r.left(), r.top(), r.right(), r.bottom()], want);
            assert_eq!(
                path.segments()
                    .filter(|s| matches!(s, tiny_skia::PathSegment::Close))
                    .count(),
                1
            );
            assert!(hit(body, tip, tip, 2., false));
            assert!(!hit(body, tip, Point::new(70., 50.), 1., false));
            assert!(hit(body, tip, Point::new(70., 50.), 1., true));
        }
    }
    #[test]
    fn inside_tip_has_no_tail_and_invalid_geometry_is_rejected() {
        let body = (Point::new(20., 20.), Point::new(120., 80.));
        let path = perimeter(body, Point::new(70., 50.)).unwrap();
        assert_eq!(path.bounds().left(), 20.);
        assert!(!hit(
            body,
            Point::new(70., 50.),
            Point::new(19., 19.),
            0.,
            true
        ));
        assert!(perimeter(body, Point::new(f64::NAN, 0.)).is_none());
        assert!(perimeter((body.0, body.0), Point::new(0., 0.)).is_none());
    }
}

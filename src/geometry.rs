use crate::editor::{Annotation, PathRendering, Point, Tool};

pub fn bend(o: &Annotation, midpoint: Point, crop: crate::frame::Rect) -> Option<Annotation> {
    o.curve_midpoint()?;
    if !midpoint.x.is_finite() || !midpoint.y.is_finite() {
        return None;
    }
    let m = Point::new(
        midpoint.x.clamp(crop.x as f64, crop.right() as f64),
        midpoint.y.clamp(crop.y as f64, crop.bottom() as f64),
    );
    let a = o.points[0];
    let b = o.points[1];
    let c = Point::new(2. * m.x - (a.x + b.x) / 2., 2. * m.y - (a.y + b.y) / 2.);
    if c.x.abs() > 32767. || c.y.abs() > 32767. {
        return None;
    }
    let mut result = o.clone();
    result.points = std::sync::Arc::new(
        if (m.x - (a.x + b.x) / 2.).abs() < 0.000001 && (m.y - (a.y + b.y) / 2.).abs() < 0.000001 {
            vec![a, b]
        } else {
            vec![a, b, c]
        },
    );
    Some(result)
}

pub fn curve_point(o: &Annotation, t: f64) -> Point {
    let a = o.points[0];
    let b = o.points[1];
    let c = o
        .points
        .get(2)
        .copied()
        .unwrap_or(Point::new((a.x + b.x) / 2., (a.y + b.y) / 2.));
    let u = 1. - t;
    Point::new(
        u * u * a.x + 2. * u * t * c.x + t * t * b.x,
        u * u * a.y + 2. * u * t * c.y + t * t * b.y,
    )
}

pub fn curve_bounds(o: &Annotation) -> (Point, Point) {
    let a = o.points[0];
    let b = o.points[1];
    let c = o.points[2];
    let mut low = Point::new(a.x.min(b.x), a.y.min(b.y));
    let mut high = Point::new(a.x.max(b.x), a.y.max(b.y));
    for (a, b, c) in [(a.x, b.x, c.x), (a.y, b.y, c.y)] {
        let t = (a - c) / (a - 2. * c + b);
        if t > 0. && t < 1. {
            let p = curve_point(o, t);
            low.x = low.x.min(p.x);
            low.y = low.y.min(p.y);
            high.x = high.x.max(p.x);
            high.y = high.y.max(p.y);
        }
    }
    (low, high)
}

pub fn polyline(o: &Annotation) -> Vec<Point> {
    if o.rendering == PathRendering::DotNet && matches!(o.tool, Tool::Pen | Tool::Highlighter) {
        let Some(first) = o.points.first().copied() else {
            return vec![];
        };
        let mut path = vec![first];
        let mut start = first;
        for pair in o.points[1..].windows(2) {
            let control = pair[0];
            let end = Point::new((pair[0].x + pair[1].x) / 2., (pair[0].y + pair[1].y) / 2.);
            let curvature =
                (start.x - 2. * control.x + end.x).hypot(start.y - 2. * control.y + end.y);
            let n = curvature.sqrt().ceil().clamp(1., 4096.) as usize;
            for i in 1..=n {
                let t = i as f64 / n as f64;
                let u = 1. - t;
                path.push(Point::new(
                    u * u * start.x + 2. * u * t * control.x + t * t * end.x,
                    u * u * start.y + 2. * u * t * control.y + t * t * end.y,
                ));
            }
            start = end;
        }
        if let Some(last) = o.points.last().copied()
            && path.last() != Some(&last)
        {
            path.push(last);
        }
        return path;
    }
    if matches!(o.tool, Tool::Line | Tool::Arrow) && o.points.len() == 3 {
        let a = o.points[0];
        let b = o.points[1];
        let c = o.points[2];
        // The quadratic interpolation error is bounded by |a-2c+b| / (4*n*n).
        let curvature = (a.x - 2. * c.x + b.x).hypot(a.y - 2. * c.y + b.y);
        let n = curvature.sqrt().ceil().clamp(1., 4096.) as usize;
        return (0..=n)
            .map(|i| curve_point(o, i as f64 / n as f64))
            .collect();
    }
    if matches!(o.tool, Tool::Ellipse | Tool::FilledEllipse) {
        let (a, b) = o.bounds();
        let rx = (b.x - a.x) / 2.;
        let ry = (b.y - a.y) / 2.;
        let n = (std::f64::consts::PI * rx.max(ry).sqrt() * 2.)
            .ceil()
            .clamp(32., 4096.) as usize;
        return (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64 * std::f64::consts::TAU;
                Point::new(a.x + rx + rx * t.cos(), a.y + ry + ry * t.sin())
            })
            .collect();
    }
    o.points.as_ref().clone()
}

pub fn arrowhead(o: &Annotation) -> [(Point, Point); 2] {
    let a = o.points[0];
    let b = o.points[1];
    let mut tangent = o.points.get(2).copied().unwrap_or(a);
    if (b.x - tangent.x).hypot(b.y - tangent.y) < 0.000001 {
        tangent = a;
    }
    let angle = (b.y - tangent.y).atan2(b.x - tangent.x);
    let (length, spread) = if o.rendering == PathRendering::DotNet {
        (o.style.width as f64 * 3. + 10., std::f64::consts::PI / 6.)
    } else {
        (
            (o.style.width as f64 * 3.)
                .max(12.)
                .min((b.x - a.x).hypot(b.y - a.y) * 0.6),
            0.5,
        )
    };
    let tip = |t: f64| Point::new(b.x - length * t.cos(), b.y - length * t.sin());
    [(b, tip(angle - spread)), (b, tip(angle + spread))]
}

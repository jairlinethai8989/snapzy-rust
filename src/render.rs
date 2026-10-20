use crate::{
    editor::{Annotation, Content, Document, Point, View, segments},
    frame::{Frame, Rect},
};
use std::sync::Arc;

pub struct Draft {
    pub bounds: Rect,
    pub rgba: Vec<u8>,
    pub multiply: bool,
}

#[cfg(test)]
mod effect_tests {
    use super::*;
    use crate::editor::{Point, Style, Tool};
    fn white() -> Arc<Frame> {
        Arc::new(Frame::new(80, 80, vec![255; 80 * 80 * 4]).unwrap())
    }
    fn px(f: &Frame, x: usize, y: usize) -> &[u8] {
        &f.bgra[(y * f.width as usize + x) * 4..][..4]
    }
    fn shadow(d: &mut Document) {
        let mut fx = crate::effects::Effects::default();
        fx.shadow.enabled = true;
        fx.shadow.blur = 0;
        fx.shadow.x = 10;
        fx.shadow.y = 10;
        d.set_effects(fx).unwrap();
    }
    #[test]
    fn hard_shadow_is_behind_object_and_obeys_group_opacity() {
        let source = white();
        let mut d = Document::new(80, 80);
        d.add(
            Tool::FilledRectangle,
            vec![Point::new(10., 10.), Point::new(30., 30.)],
            Style::default(),
        );
        shadow(&mut d);
        let f = flatten(&source, &d).unwrap();
        assert_eq!(px(&f, 35, 35), &[0, 0, 0, 255]);
        assert_eq!(px(&f, 25, 25), &[0, 0, 255, 255]);
        d.set_opacity(50);
        let f = flatten(&source, &d).unwrap();
        assert_eq!(px(&f, 35, 35), &[127, 127, 127, 255]);
        assert_eq!(px(&f, 25, 25), &[127, 127, 255, 255]);
        d.set_opacity(0);
        assert_eq!(flatten(&source, &d).unwrap().bgra, source.bgra);
    }
    #[test]
    fn text_outline_expands_glyph_not_background() {
        let source = white();
        let mut d = Document::new(80, 80);
        d.add_text(Point::new(20., 20.), "A", 24, Style::default())
            .unwrap();
        if let Content::Text { mask, .. } = &mut d.objects[0].content {
            *mask = Arc::new(crate::glyphs::Mask {
                width: 3,
                height: 3,
                alpha: vec![0, 0, 0, 0, 255, 0, 0, 0, 0],
            });
        }
        let mut fx = crate::effects::Effects::default();
        fx.outline.enabled = true;
        fx.outline.width = 2;
        d.set_effects(fx).unwrap();
        let f = flatten(&source, &d).unwrap();
        assert_eq!(px(&f, 19, 21), &[0, 0, 0, 255]);
        assert_eq!(px(&f, 21, 21), &[0, 0, 255, 255]);
        assert_eq!(px(&f, 18, 21), &[255; 4]);
    }
    #[test]
    fn blur_shadow_does_not_change_protected_pixels() {
        let mut pixels = vec![255; 80 * 80 * 4];
        for y in 0..80 {
            for x in 0..80 {
                pixels[(y * 80 + x) * 4] = ((x * 17 + y * 23) % 256) as u8;
            }
        }
        let source = Arc::new(Frame::new(80, 80, pixels).unwrap());
        let mut d = Document::new(80, 80);
        d.add_blur(Point::new(10., 10.), Point::new(30., 30.), 8);
        let original = flatten(&source, &d).unwrap();
        shadow(&mut d);
        let f = flatten(&source, &d).unwrap();
        for y in 10..30 {
            for x in 10..30 {
                assert_eq!(px(&f, x, y), px(&original, x, y));
            }
        }
        assert_eq!(px(&f, 35, 35), &[0, 0, 0, 255]);
    }
    #[test]
    fn highlighter_shadow_blends_normally_but_ink_still_multiplies() {
        let source = Arc::new(Frame::new(80, 80, [100, 150, 200, 255].repeat(80 * 80)).unwrap());
        let mut d = Document::new(80, 80);
        d.add_highlight(
            vec![Point::new(10., 20.), Point::new(50., 20.)],
            Style {
                rgb: [255, 242, 88],
                width: 8,
            },
            40,
        );
        let plain = flatten(&source, &d).unwrap();
        shadow(&mut d);
        let decorated = flatten(&source, &d).unwrap();
        assert_eq!(px(&plain, 25, 20), px(&decorated, 25, 20));
        assert_eq!(px(&decorated, 25, 30), &[60, 90, 120, 255]);
        assert_eq!(source.bgra, [100, 150, 200, 255].repeat(80 * 80));
    }
    #[test]
    fn image_shadow_uses_source_alpha_not_its_rectangle() {
        let source = white();
        let mut d = Document::new(80, 80);
        let image = Arc::new(Frame::new(2, 1, vec![0, 0, 255, 255, 0, 0, 255, 0]).unwrap());
        d.add_image(image.clone()).unwrap();
        shadow(&mut d);
        let output = flatten(&source, &d).unwrap();
        assert_eq!(px(&output, 10, 10), &[0, 0, 0, 255]);
        assert_eq!(px(&output, 11, 10), &[255; 4]);
        assert_eq!(image.bgra, vec![0, 0, 255, 255, 0, 0, 255, 0]);
    }
    #[test]
    fn softened_negative_shadow_matches_clipped_tile() {
        let source = Arc::new(Frame::new(540, 340, vec![255; 540 * 340 * 4]).unwrap());
        let mut d = Document::new(540, 340);
        d.add(
            Tool::FilledEllipse,
            vec![Point::new(190., 80.), Point::new(410., 270.)],
            Style::default(),
        );
        let mut fx = crate::effects::Effects::default();
        fx.shadow.enabled = true;
        fx.shadow.blur = 3;
        fx.shadow.x = -20;
        fx.shadow.y = -12;
        d.set_effects(fx).unwrap();
        d.set_opacity(50);
        let full = flatten(&source, &d).unwrap();
        assert_ne!(px(&full, 185, 175), &[255; 4]);
        d.crop = Rect::new(220, 100, 80, 160).unwrap();
        let cropped = flatten(&source, &d).unwrap();
        for y in 0..160 {
            for x in 0..80 {
                assert_eq!(px(&cropped, x, y), px(&full, x + 220, y + 100));
            }
        }
    }
    #[test]
    fn shadow_replacement_includes_old_and_new_extents() {
        let source = Arc::new(Frame::new(540, 340, vec![255; 540 * 340 * 4]).unwrap());
        let mut d = Document::new(540, 340);
        d.add(
            Tool::FilledRectangle,
            vec![Point::new(200., 80.), Point::new(300., 210.)],
            Style::default(),
        );
        shadow(&mut d);
        d.set_opacity(50);
        let before = flatten(&source, &d).unwrap();
        let mut replacement = d.objects[0].clone();
        replacement.points = Arc::new(vec![Point::new(190., 70.), Point::new(340., 230.)]);
        let view = View {
            crop: d.crop,
            x: 0.,
            y: 0.,
            scale: 1.,
        };
        let draft = replacement_draft(&source, &d, &replacement, view).unwrap();
        d.objects[0] = replacement;
        let committed = flatten(&source, &d).unwrap();
        for y in 0..340 {
            for x in 0..540 {
                let actual = if x as i32 >= draft.bounds.x
                    && (x as i32) < draft.bounds.right()
                    && y as i32 >= draft.bounds.y
                    && (y as i32) < draft.bounds.bottom()
                {
                    let i = (((y as i32 - draft.bounds.y) as u32 * draft.bounds.width
                        + (x as i32 - draft.bounds.x) as u32)
                        * 4) as usize;
                    vec![
                        draft.rgba[i + 2],
                        draft.rgba[i + 1],
                        draft.rgba[i],
                        draft.rgba[i + 3],
                    ]
                } else {
                    px(&before, x, y).to_vec()
                };
                assert_eq!(actual, px(&committed, x, y), "replacement mismatch {x},{y}");
            }
        }
    }
    #[test]
    fn disabled_effects_leave_existing_output_unchanged() {
        let source = white();
        let mut d = Document::new(80, 80);
        d.add_highlight(
            vec![Point::new(5., 15.), Point::new(70., 15.)],
            Style {
                rgb: [255, 242, 88],
                width: 24,
            },
            40,
        );
        d.add_callout(
            Point::new(10., 35.),
            "Thai",
            12,
            Style::default(),
            Default::default(),
        )
        .unwrap();
        let before = flatten(&source, &d).unwrap();
        for o in &mut d.objects {
            o.effects.shadow.rgb = [250, 30, 200];
            o.effects.shadow.blur = 32;
            o.effects.outline.width = 16;
        }
        assert_eq!(before.bgra, flatten(&source, &d).unwrap().bgra);
        assert_eq!(source.bgra, vec![255; 80 * 80 * 4]);
    }
    #[test]
    fn thai_outline_preserves_original_glyph_coverage() {
        let source = Arc::new(Frame::new(360, 100, vec![255; 360 * 100 * 4]).unwrap());
        let mut d = Document::new(360, 100);
        d.add_text(Point::new(20., 20.), "ข้อความ กิ กุ ก้", 24, Style::default())
            .unwrap();
        let plain = flatten(&source, &d).unwrap();
        let mut fx = crate::effects::Effects::default();
        fx.outline.enabled = true;
        fx.outline.rgb = [0, 0, 255];
        d.set_effects(fx).unwrap();
        let outlined = flatten(&source, &d).unwrap();
        assert_ne!(plain.bgra, outlined.bgra);
        for (a, b) in plain
            .bgra
            .chunks_exact(4)
            .zip(outlined.bgra.chunks_exact(4))
        {
            if a == [0, 0, 255, 255] {
                assert_eq!(a, b, "opaque glyph fill changed");
            }
        }
    }
    #[test]
    fn effects_render_timing_fixture() {
        use std::time::Instant;
        let source = Arc::new(Frame::new(600, 6000, vec![255; 600 * 6000 * 4]).unwrap());
        let mut d = Document::new(600, 6000);
        for i in 0..6 {
            let y = 40. + i as f64 * 900.;
            d.add(
                Tool::FilledEllipse,
                vec![Point::new(40., y), Point::new(380., y + 180.)],
                Style::default(),
            );
            d.add_text(
                Point::new(60., y + 200.),
                "ข้อความภาษาไทย / Shadow",
                24,
                Style::default(),
            )
            .unwrap();
        }
        for enabled in [false, true] {
            for o in &mut d.objects {
                o.effects.shadow.enabled = enabled;
                o.effects.outline.enabled = enabled;
            }
            let mut samples = Vec::new();
            for _ in 0..3 {
                let start = Instant::now();
                let result = flatten(&source, &d).unwrap();
                samples.push(start.elapsed().as_secs_f64() * 1000.);
                assert_eq!((result.width, result.height), (600, 6000));
                std::hint::black_box(result);
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "effects_render 600x6000 enabled={enabled} median_ms={:.2} samples={samples:?}",
                samples[1]
            );
        }
    }
}

pub fn replacement_draft(
    source: &Arc<Frame>,
    document: &Document,
    replacement: &Annotation,
    view: View,
) -> Option<Draft> {
    if !view.scale.is_finite() || view.scale <= 0. {
        return None;
    }
    if document
        .objects
        .iter()
        .any(|o| matches!(o.content, Content::Group { .. }))
    {
        // Group rasters are composed on workers; pointer feedback stays geometry-only.
        return None;
    }
    let old = document
        .objects
        .iter()
        .find(|o| o.id == replacement.id)
        .unwrap_or(replacement);
    let mut left = f64::INFINITY;
    let mut top = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for o in [old, replacement] {
        for (a, b) in segments(o) {
            for p in [a, b] {
                left = left.min(p.x);
                top = top.min(p.y);
                right = right.max(p.x);
                bottom = bottom.max(p.y);
            }
        }
    }
    let halo: i32 = document
        .objects
        .iter()
        .filter_map(|o| {
            if let Content::Blur { strength } = o.content {
                Some(blur_halo(strength) as i32)
            } else {
                None
            }
        })
        .sum();
    let effect_reach = [old, replacement]
        .iter()
        .map(|o| o.effects.reach(matches!(o.content, Content::Text { .. })))
        .max()
        .unwrap_or(0);
    let pad = old.style.width.max(replacement.style.width) as f64 / 2.
        + effect_reach as f64
        + halo as f64
        + 3. / view.scale;
    let crop = document.crop;
    let affected = Rect::from_edges(
        (left - pad).floor() as i32,
        (top - pad).floor() as i32,
        (right + pad).ceil() as i32,
        (bottom + pad).ceil() as i32,
    )
    .ok()?
    .intersection(crop)?;
    let mapx = |x: i32| view.x + (x - crop.x) as f64 * view.scale;
    let mapy = |y: i32| view.y + (y - crop.y) as f64 * view.scale;
    let bounds = Rect::from_edges(
        mapx(affected.x).ceil() as i32,
        mapy(affected.y).ceil() as i32,
        mapx(affected.right()).ceil() as i32,
        mapy(affected.bottom()).ceil() as i32,
    )
    .ok()?;
    let mut rgba = vec![0; Frame::byte_len(bounds.width, bounds.height).ok()?];
    let mut snapshot = document.render_snapshot();
    if let Some(old) = snapshot.objects.iter_mut().find(|o| o.id == replacement.id) {
        *old = replacement.clone();
    } else {
        snapshot.objects.push(replacement.clone());
    }
    // Keep path clipping away from tile edges as well as retaining blur input halos.
    let sample_halo = halo + 128 + effect_reach;
    // Compose bounded source tiles, never a new full long screenshot for a pointer move.
    for ty in (affected.y..affected.bottom()).step_by(256) {
        for tx in (affected.x..affected.right()).step_by(256) {
            let tile = Rect::new(tx, ty, 256, 256).ok()?.intersection(affected)?;
            snapshot.crop = Rect::from_edges(
                tile.x.saturating_sub(sample_halo),
                tile.y.saturating_sub(sample_halo),
                tile.right().saturating_add(sample_halo),
                tile.bottom().saturating_add(sample_halo),
            )
            .ok()?
            .intersection(crop)?;
            let frame = flatten(source, &snapshot).ok()?;
            let x0 = (mapx(tile.x).ceil() as i32).max(bounds.x);
            let y0 = (mapy(tile.y).ceil() as i32).max(bounds.y);
            let x1 = (mapx(tile.right()).ceil() as i32).min(bounds.right());
            let y1 = (mapy(tile.bottom()).ceil() as i32).min(bounds.bottom());
            for y in y0..y1 {
                for x in x0..x1 {
                    let sx = (((x as f64 - view.x) / view.scale).floor() as i32 + crop.x)
                        .clamp(tile.x, tile.right() - 1);
                    let sy = (((y as f64 - view.y) / view.scale).floor() as i32 + crop.y)
                        .clamp(tile.y, tile.bottom() - 1);
                    let si = (((sy - snapshot.crop.y) as u32 * frame.width
                        + (sx - snapshot.crop.x) as u32)
                        * 4) as usize;
                    let di = (((y - bounds.y) as u32 * bounds.width + (x - bounds.x) as u32) * 4)
                        as usize;
                    rgba[di..di + 4].copy_from_slice(&[
                        frame.bgra[si + 2],
                        frame.bgra[si + 1],
                        frame.bgra[si],
                        255,
                    ]);
                }
            }
        }
    }
    Some(Draft {
        bounds,
        rgba,
        multiply: false,
    })
}

pub fn draft(object: &Annotation, view: View) -> Option<Draft> {
    if !matches!(object.content, Content::Shape | Content::Highlight)
        && object.tool != crate::editor::Tool::Callout
    {
        return None;
    }
    if !view.scale.is_finite() || view.scale <= 0. {
        return None;
    }
    let lines = if let Some(tip) = object.tail_tip() {
        let (a, b) = object.body_bounds();
        vec![(a, b), (tip, tip)]
    } else {
        segments(object)
    };
    let mut left = f64::INFINITY;
    let mut top = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for (a, b) in &lines {
        for p in [a, b] {
            let x = view.x + (p.x - view.crop.x as f64) * view.scale;
            let y = view.y + (p.y - view.crop.y as f64) * view.scale;
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
        }
    }
    let pad = object.style.width as f64 * view.scale / 2. + 2.;
    let bounds = Rect::from_edges(
        ((left - pad).floor() as i32).max(view.x.round() as i32),
        ((top - pad).floor() as i32).max(view.y.round() as i32),
        ((right + pad).ceil() as i32)
            .min((view.x + view.crop.width as f64 * view.scale).round() as i32),
        ((bottom + pad).ceil() as i32)
            .min((view.y + view.crop.height as f64 * view.scale).round() as i32),
    )
    .ok()?;
    let mut rgba = vec![0; Frame::byte_len(bounds.width, bounds.height).ok()?];
    let mut pixels = tiny_skia::PixmapMut::from_bytes(&mut rgba, bounds.width, bounds.height)?;
    let outline = (object.tool == crate::editor::Tool::Callout).then(|| {
        let mut object = object.clone();
        if let Content::Text { background, .. } = &mut object.content {
            // Geometry-only feedback must not obscure the cached text during a drag.
            background.enabled = false;
        }
        object
    });
    stroke(
        &mut pixels,
        outline.as_ref().unwrap_or(object),
        tiny_skia::Transform::from_row(
            view.scale as f32,
            0.,
            0.,
            view.scale as f32,
            (view.x - view.crop.x as f64 * view.scale - bounds.x as f64) as f32,
            (view.y - view.crop.y as f64 * view.scale - bounds.y as f64) as f32,
        ),
    );
    if grouped_opacity(object.tool) {
        scale_alpha(&mut rgba, object.opacity);
    }
    Some(Draft {
        bounds,
        rgba,
        multiply: matches!(object.content, Content::Highlight),
    })
}

fn stroke(
    pixels: &mut tiny_skia::PixmapMut<'_>,
    object: &Annotation,
    transform: tiny_skia::Transform,
) {
    if let Some(tip) = object.tail_tip() {
        if let Some(path) = crate::callout::perimeter(object.body_bounds(), tip) {
            let mut paint = tiny_skia::Paint::default();
            if let Some(fill) = object.text_background().filter(|b| b.enabled) {
                let [r, g, b] = fill.rgb;
                paint.set_color_rgba8(r, g, b, 255);
                pixels.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
            }
            let border = object.callout_border();
            let [r, g, b] = border.rgb;
            paint.set_color_rgba8(r, g, b, 255);
            pixels.stroke_path(
                &path,
                &paint,
                &tiny_skia::Stroke {
                    width: border.width as f32,
                    line_join: tiny_skia::LineJoin::Round,
                    ..Default::default()
                },
                transform,
                None,
            );
        }
        return;
    }
    let mut builder = tiny_skia::PathBuilder::new();
    use crate::editor::{PathRendering, Pattern, Tool};
    let dot = object.rendering == PathRendering::DotNet
        && matches!(object.tool, Tool::Pen | Tool::Highlighter)
        && object.points.len() == 1;
    let path = if dot {
        let p = object.points[0];
        tiny_skia::PathBuilder::from_circle(p.x as f32, p.y as f32, object.style.width as f32 / 2.)
    } else {
        if matches!(
            object.tool,
            Tool::Rectangle | Tool::FilledRectangle | Tool::Ellipse | Tool::FilledEllipse
        ) {
            let (a, b) = object.bounds();
            let Some(rect) =
                tiny_skia::Rect::from_ltrb(a.x as f32, a.y as f32, b.x as f32, b.y as f32)
            else {
                return;
            };
            if matches!(object.tool, Tool::Ellipse | Tool::FilledEllipse) {
                builder.push_oval(rect);
            } else {
                builder.push_rect(rect);
            }
        } else if matches!(object.tool, Tool::Line | Tool::Arrow) {
            let a = object.points[0];
            let b = object.points[1];
            builder.move_to(a.x as f32, a.y as f32);
            if let Some(c) = object.points.get(2) {
                builder.quad_to(c.x as f32, c.y as f32, b.x as f32, b.y as f32);
            } else {
                builder.line_to(b.x as f32, b.y as f32);
            }
        } else if object.rendering == PathRendering::DotNet
            && matches!(object.tool, Tool::Pen | Tool::Highlighter)
        {
            let Some(first) = object.points.first() else {
                return;
            };
            builder.move_to(first.x as f32, first.y as f32);
            for pair in object.points[1..].windows(2) {
                let current = pair[0];
                let next = pair[1];
                builder.quad_to(
                    current.x as f32,
                    current.y as f32,
                    ((current.x + next.x) / 2.) as f32,
                    ((current.y + next.y) / 2.) as f32,
                );
            }
            if let Some(last) = object.points.last() {
                builder.line_to(last.x as f32, last.y as f32);
            }
        } else {
            let Some(first) = object.points.first() else {
                return;
            };
            builder.move_to(first.x as f32, first.y as f32);
            for point in object.points.iter().skip(1) {
                builder.line_to(point.x as f32, point.y as f32);
            }
        }
        builder.finish()
    };
    let Some(path) = path else {
        return;
    };
    let mut paint = tiny_skia::Paint::default();
    let [r, g, b] = object.style.rgb;
    let opacity = if !grouped_opacity(object.tool) {
        ((object.opacity as u32 * 255 + 50) / 100) as u8
    } else {
        255
    };
    paint.set_color_rgba8(r, g, b, opacity);
    if dot {
        pixels.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
        return;
    }
    if object.tool.filled() {
        pixels.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
        return;
    }
    let width = object.style.width as f32;
    let mut stroke = tiny_skia::Stroke {
        width: object.style.width as f32,
        line_cap: if object.tool == crate::editor::Tool::Highlighter {
            tiny_skia::LineCap::Butt
        } else {
            tiny_skia::LineCap::Round
        },
        line_join: tiny_skia::LineJoin::Round,
        dash: if object.tool.patterned() {
            match object.pattern {
                Pattern::Solid => None,
                Pattern::Dashed => tiny_skia::StrokeDash::new(vec![4. * width, 3. * width], 0.),
                Pattern::Dotted => tiny_skia::StrokeDash::new(vec![0., 3. * width], 0.),
            }
        } else {
            None
        },
        ..Default::default()
    };
    pixels.stroke_path(&path, &paint, &stroke, transform, None);
    if object.tool == Tool::Arrow {
        stroke.dash = None;
        let mut head = tiny_skia::PathBuilder::new();
        for (a, b) in crate::geometry::arrowhead(object) {
            head.move_to(a.x as f32, a.y as f32);
            head.line_to(b.x as f32, b.y as f32);
        }
        if let Some(head) = head.finish() {
            pixels.stroke_path(&head, &paint, &stroke, transform, None);
        }
    }
}

pub fn flatten(source: &Arc<Frame>, document: &Document) -> Result<Arc<Frame>, String> {
    crate::combine::validate_tree(&document.objects)?;
    flatten_tree(source, document, 0, 0)
}

fn flatten_tree(
    source: &Arc<Frame>,
    document: &Document,
    depth: usize,
    retained: usize,
) -> Result<Arc<Frame>, String> {
    if depth > 8 {
        return Err("Image group depth exceeds 8.".into());
    }
    let bytes = Frame::byte_len(document.crop.width, document.crop.height)?;
    let scratch = retained
        .checked_add(bytes * 4)
        .ok_or("Composition budget overflow")?;
    if scratch > crate::image_limits::SCROLL_WORK_LIMIT + crate::image_limits::IMAGE_LIMIT {
        return Err("Composition exceeds the available image working-memory limit.".into());
    }
    let prepared;
    let document = if document
        .objects
        .iter()
        .any(|o| matches!(o.content, Content::Group { .. }))
    {
        let mut d = document.render_snapshot();
        d.objects.clear();
        let mut held = scratch;
        for o in document.objects.iter().filter(|o| o.parent.is_none()) {
            let mut layer = o.clone();
            if let Content::Group { image } = &o.content {
                let v = image.viewport;
                let sample = Rect::from_edges(
                    v.left.floor() as i32,
                    v.top.floor() as i32,
                    v.right.ceil() as i32,
                    v.bottom.ceil() as i32,
                )?;
                let children: Vec<_> = document
                    .objects
                    .iter()
                    .filter(|child| document.is_descendant(child.id, o.id))
                    .map(|child| {
                        let mut child = child.clone();
                        if child.parent == Some(o.id) {
                            child.parent = None
                        }
                        child
                    })
                    .collect();
                let local = Document::from_project(sample, image.source_clip, children)?;
                let raster = flatten_tree(&image.frame, &local, depth + 1, held)?;
                held = held
                    .checked_add(raster.bgra.len())
                    .ok_or("Composition budget overflow")?;
                if held > crate::image_limits::SCROLL_WORK_LIMIT + crate::image_limits::IMAGE_LIMIT
                {
                    return Err(
                        "Composition exceeds the available image working-memory limit.".into(),
                    );
                }
                let raster = if v.left.fract() != 0.
                    || v.top.fract() != 0.
                    || v.right.fract() != 0.
                    || v.bottom.fract() != 0.
                {
                    let mut pixels = raster.bgra.clone();
                    for y in 0..sample.height {
                        for x in 0..sample.width {
                            let px = sample.x as f64 + x as f64;
                            let py = sample.y as f64 + y as f64;
                            let coverage = ((px + 1.).min(v.right) - px.max(v.left)).max(0.)
                                * ((py + 1.).min(v.bottom) - py.max(v.top)).max(0.);
                            let i = ((y * sample.width + x) * 4) as usize;
                            pixels[i + 3] = (pixels[i + 3] as f64 * coverage).round() as u8;
                            if pixels[i + 3] == 0 {
                                pixels[i..i + 4].fill(0)
                            }
                        }
                    }
                    Arc::new(Frame::new(sample.width, sample.height, pixels)?)
                } else {
                    raster
                };
                let t = crate::combine::image_transform(o, v);
                layer.points = Arc::new(vec![
                    t.point(Point::new(sample.x as f64, sample.y as f64)),
                    t.point(Point::new(sample.right() as f64, sample.bottom() as f64)),
                ]);
                layer.content = Content::Image { frame: raster };
            }
            d.objects.push(layer);
        }
        prepared = d;
        &prepared
    } else {
        document
    };
    let full = Rect::new(0, 0, source.width, source.height)?;
    if document.objects.is_empty() && document.crop == full && document.source_clip == Some(full) {
        return Ok(source.clone());
    }
    let crop = document.crop;
    let visible = document
        .source_clip
        .and_then(|r| r.intersection(full))
        .and_then(|r| r.intersection(crop));
    let output = if visible == Some(crop) {
        source.crop(full, crop)?
    } else {
        let mut frame = Frame::new(
            crop.width,
            crop.height,
            vec![255; Frame::byte_len(crop.width, crop.height)?],
        )?;
        if let Some(visible) = visible {
            for y in visible.y..visible.bottom() {
                let from = (y as usize * source.width as usize + visible.x as usize) * 4;
                let to = ((y - crop.y) as usize * frame.width as usize
                    + (visible.x - crop.x) as usize)
                    * 4;
                let len = visible.width as usize * 4;
                frame.bgra[to..to + len].copy_from_slice(&source.bgra[from..from + len]);
            }
        }
        frame
    };
    if document.objects.is_empty() {
        return Ok(Arc::new(output));
    }
    flatten_objects(output, document)
}

fn flatten_objects(output: Frame, document: &Document) -> Result<Arc<Frame>, String> {
    flatten_layers(output, document, true, None)
}
fn flatten_layers(
    mut output: Frame,
    document: &Document,
    effects: bool,
    outline: Option<&crate::glyphs::Mask>,
) -> Result<Arc<Frame>, String> {
    let crop = document.crop;
    let mut pixels = vec![0; output.bgra.len()];
    let mut overlay = tiny_skia::PixmapMut::from_bytes(&mut pixels, output.width, output.height)
        .ok_or("Invalid drawing canvas.")?;
    for object in document.objects.iter().filter(|o| o.parent.is_none()) {
        if effects
            && object
                .effects
                .active(matches!(object.content, Content::Text { .. }))
        {
            composite(&mut output.bgra, overlay.data_mut());
            overlay.data_mut().fill(0);
            effect_into(&mut output, crop, object)?;
            continue;
        }
        if grouped_opacity(object.tool)
            && object.opacity != 100
            && object.content != Content::Highlight
        {
            composite(&mut output.bgra, overlay.data_mut());
            overlay.data_mut().fill(0);
            group_into(&mut output, crop, object)?;
            continue;
        }
        if matches!(object.content, Content::Highlight) {
            composite(&mut output.bgra, overlay.data_mut());
            overlay.data_mut().fill(0);
        }
        match &object.content {
            Content::Image { frame } => {
                let mut rgba = frame.bgra.clone();
                for p in rgba.chunks_exact_mut(4) {
                    p.swap(0, 2);
                    for i in 0..3 {
                        p[i] = ((p[i] as u16 * p[3] as u16 + 127) / 255) as u8;
                    }
                }
                let pixmap = tiny_skia::PixmapRef::from_bytes(&rgba, frame.width, frame.height)
                    .ok_or("Invalid image pixels.")?;
                let (a, b) = object.bounds();
                overlay.draw_pixmap(
                    0,
                    0,
                    pixmap,
                    &tiny_skia::PixmapPaint {
                        opacity: ((object.opacity as u32 * 255 + 50) / 100) as f32 / 255.,
                        quality: tiny_skia::FilterQuality::Bilinear,
                        ..Default::default()
                    },
                    tiny_skia::Transform::from_row(
                        ((b.x - a.x) / frame.width as f64) as f32,
                        0.,
                        0.,
                        ((b.y - a.y) / frame.height as f64) as f32,
                        (a.x - crop.x as f64) as f32,
                        (a.y - crop.y as f64) as f32,
                    ),
                    None,
                );
            }
            Content::Shape | Content::Highlight => stroke(
                &mut overlay,
                object,
                tiny_skia::Transform::from_translate(
                    -document.crop.x as f32,
                    -document.crop.y as f32,
                ),
            ),
            Content::Step { diameter, .. } => {
                let p = object.points[0];
                let mut paint = tiny_skia::Paint::default();
                let [r, g, b] = object.style.rgb;
                paint.set_color_rgba8(r, g, b, 255);
                if let Some(path) = tiny_skia::PathBuilder::from_circle(
                    p.x as f32,
                    p.y as f32,
                    *diameter as f32 / 2.,
                ) {
                    overlay.fill_path(
                        &path,
                        &paint,
                        tiny_skia::FillRule::Winding,
                        tiny_skia::Transform::from_translate(
                            -document.crop.x as f32,
                            -document.crop.y as f32,
                        ),
                        None,
                    );
                    if object.rendering == crate::editor::PathRendering::DotNet {
                        paint.set_color_rgba8(255, 255, 255, 255);
                        let mut stroke = tiny_skia::Stroke::default();
                        stroke.width = (1f32).max(*diameter as f32 / 32.);
                        overlay.stroke_path(
                            &path,
                            &paint,
                            &stroke,
                            tiny_skia::Transform::from_translate(
                                -document.crop.x as f32,
                                -document.crop.y as f32,
                            ),
                            None,
                        );
                    }
                }
            }
            _ => {}
        }
        if !matches!(object.content, Content::Shape) {
            if matches!(object.content, Content::Highlight) {
                for (dst, src) in output
                    .bgra
                    .chunks_exact_mut(4)
                    .zip(overlay.data_mut().chunks_exact(4))
                {
                    blend_pixel(dst, src, true);
                }
            } else {
                composite(&mut output.bgra, overlay.data_mut());
            }
            overlay.data_mut().fill(0);
            match &object.content {
                Content::Text { mask, .. } => {
                    text_decoration(&mut output, document.crop, document.crop, object)?;
                    if object.effects.outline.enabled {
                        let generated;
                        let expanded = if let Some(outline) = outline {
                            outline
                        } else {
                            generated =
                                crate::effects::outlined(mask, object.effects.outline.width)?;
                            &generated
                        };
                        let p = object.text_origin();
                        let radius = object.effects.outline.width as f64;
                        mask_into(
                            &mut output,
                            document.crop,
                            crate::editor::Point::new(p.x - radius, p.y - radius),
                            expanded,
                            object.effects.outline.rgb,
                        );
                    }
                    mask_into(
                        &mut output,
                        document.crop,
                        object.text_origin(),
                        mask,
                        object.style.rgb,
                    );
                }
                Content::Step { mask, .. } => {
                    let p = object.points[0];
                    let [r, g, b] = object.style.rgb;
                    let color = if 299 * r as u32 + 587 * g as u32 + 114 * b as u32 > 145000 {
                        [0, 0, 0]
                    } else {
                        [255, 255, 255]
                    };
                    mask_into(
                        &mut output,
                        document.crop,
                        crate::editor::Point::new(
                            p.x - mask.width as f64 / 2.,
                            p.y - mask.height as f64 / 2.,
                        ),
                        mask,
                        color,
                    );
                }
                Content::Blur { strength } => {
                    blur_into(&mut output, document.crop, object, *strength)?
                }
                _ => {}
            }
        }
    }
    composite(&mut output.bgra, &pixels);
    Ok(Arc::new(output))
}

fn effect_into(output: &mut Frame, crop: Rect, object: &Annotation) -> Result<(), String> {
    let fx = object.effects.validate()?;
    let is_blur = matches!(object.content, Content::Blur { .. });
    if object.opacity == 0 {
        return Ok(());
    }
    let (mut a, mut b) = object.body_bounds();
    if let Some(p) = object.tail_tip() {
        a.x = a.x.min(p.x);
        a.y = a.y.min(p.y);
        b.x = b.x.max(p.x);
        b.y = b.y.max(p.y);
    }
    let reach = fx.reach(matches!(object.content, Content::Text { .. }));
    let pad = reach + object.style.width as i32 + 3;
    let Some(bounds) = Rect::from_edges(
        a.x.floor() as i32 - pad,
        a.y.floor() as i32 - pad,
        b.x.ceil() as i32 + pad,
        b.y.ceil() as i32 + pad,
    )?
    .intersection(crop) else {
        return Ok(());
    };
    let outline = if let Content::Text { mask, .. } = &object.content {
        if fx.outline.enabled {
            Some(crate::effects::outlined(mask, fx.outline.width)?)
        } else {
            None
        }
    } else {
        None
    };
    let image = if let Content::Image { frame } = &object.content {
        let mut rgba = frame.bgra.clone();
        for p in rgba.chunks_exact_mut(4) {
            p.swap(0, 2);
            for c in 0..3 {
                p[c] = ((p[c] as u32 * p[3] as u32 + 127) / 255) as u8;
            }
        }
        Some(rgba)
    } else {
        None
    };
    if let Content::Blur { strength } = object.content {
        blur_into(output, crop, object, strength)?;
    }
    let opacity = (object.opacity as u32 * 255 + 50) / 100;
    for y in (bounds.y.div_euclid(256) * 256..bounds.bottom()).step_by(256) {
        for x in (bounds.x.div_euclid(256) * 256..bounds.right()).step_by(256) {
            let tile = Rect::new(x, y, 256, 256)?.intersection(bounds).unwrap();
            let halo = reach + object.style.width as i32 + 128;
            let region = Rect::from_edges(x - halo, y - halo, x + 256 + halo, y + 256 + halo)?;
            let mut local = object.clone();
            local.opacity = 100;
            local.effects.shadow.enabled = false;
            if matches!(local.content, Content::Highlight) {
                local.content = Content::Shape;
            }
            let mut transparent = Frame::new(
                region.width,
                region.height,
                vec![0; Frame::byte_len(region.width, region.height)?],
            )?;
            let group = if is_blur {
                Arc::new(transparent)
            } else if let (Content::Image { frame }, Some(rgba)) = (&object.content, &image) {
                let mut pixels = tiny_skia::Pixmap::new(region.width, region.height)
                    .ok_or("Cannot draw image effects.")?;
                let source = tiny_skia::PixmapRef::from_bytes(rgba, frame.width, frame.height)
                    .ok_or("Invalid image effects.")?;
                let (a, b) = object.bounds();
                pixels.draw_pixmap(
                    0,
                    0,
                    source,
                    &tiny_skia::PixmapPaint {
                        quality: tiny_skia::FilterQuality::Bilinear,
                        ..Default::default()
                    },
                    tiny_skia::Transform::from_row(
                        ((b.x - a.x) / frame.width as f64) as f32,
                        0.,
                        0.,
                        ((b.y - a.y) / frame.height as f64) as f32,
                        (a.x - region.x as f64) as f32,
                        (a.y - region.y as f64) as f32,
                    ),
                    None,
                );
                composite(&mut transparent.bgra, pixels.data());
                Arc::new(transparent)
            } else {
                let mut d = Document::new(region.width, region.height);
                d.crop = region;
                d.objects.push(local);
                flatten_layers(transparent, &d, false, outline.as_ref())?
            };
            // Path clipping needs a larger guard than the shadow filter's finite support.
            let shadow_region =
                Rect::from_edges(x - reach, y - reach, x + 256 + reach, y + 256 + reach)?;
            let soft = if fx.shadow.enabled {
                let mut mask =
                    vec![0; Frame::byte_len(shadow_region.width, shadow_region.height)? / 4];
                for my in 0..shadow_region.height {
                    for mx in 0..shadow_region.width {
                        let gx = shadow_region.x + mx as i32;
                        let gy = shadow_region.y + my as i32;
                        let i = (((gy - region.y) as u32 * region.width + (gx - region.x) as u32)
                            * 4) as usize;
                        mask[(my * shadow_region.width + mx) as usize] = group.bgra[i + 3];
                    }
                }
                if is_blur {
                    for my in 0..shadow_region.height {
                        for mx in 0..shadow_region.width {
                            let gx = shadow_region.x + mx as i32;
                            let gy = shadow_region.y + my as i32;
                            if gx >= a.x.round() as i32
                                && gx < b.x.round() as i32
                                && gy >= a.y.round() as i32
                                && gy < b.y.round() as i32
                            {
                                mask[(my * shadow_region.width + mx) as usize] = 255;
                            }
                        }
                    }
                }
                Some(crate::effects::soft_mask(
                    mask,
                    shadow_region.width,
                    shadow_region.height,
                    fx.shadow.blur,
                )?)
            } else {
                None
            };
            for gy in tile.y..tile.bottom() {
                for gx in tile.x..tile.right() {
                    let j =
                        (((gy - crop.y) as u32 * output.width + (gx - crop.x) as u32) * 4) as usize;
                    if is_blur
                        && gx >= a.x.round() as i32
                        && gx < b.x.round() as i32
                        && gy >= a.y.round() as i32
                        && gy < b.y.round() as i32
                    {
                        continue;
                    }
                    let mut combined = [0u8; 4];
                    if let Some(soft) = &soft {
                        let sx = gx - fx.shadow.x - shadow_region.x;
                        let sy = gy - fx.shadow.y - shadow_region.y;
                        if sx >= 0
                            && sy >= 0
                            && sx < shadow_region.width as i32
                            && sy < shadow_region.height as i32
                        {
                            let alpha =
                                soft[(sy as u32 * shadow_region.width + sx as u32) as usize] as u32;
                            let [r, g, b] = fx.shadow.rgb;
                            let rgba = [
                                ((r as u32 * alpha + 127) / 255) as u8,
                                ((g as u32 * alpha + 127) / 255) as u8,
                                ((b as u32 * alpha + 127) / 255) as u8,
                                alpha as u8,
                            ];
                            blend_pixel(&mut combined, &rgba, false);
                        }
                    }
                    let i = (((gy - region.y) as u32 * region.width + (gx - region.x) as u32) * 4)
                        as usize;
                    let p = &group.bgra[i..i + 4];
                    let premul = |p: &[u8], opacity: u32| {
                        let a = (p[3] as u32 * opacity + 127) / 255;
                        [
                            ((p[2] as u32 * a + 127) / 255) as u8,
                            ((p[1] as u32 * a + 127) / 255) as u8,
                            ((p[0] as u32 * a + 127) / 255) as u8,
                            a as u8,
                        ]
                    };
                    if matches!(object.content, Content::Highlight) {
                        blend_pixel(
                            &mut output.bgra[j..j + 4],
                            &premul(&combined, opacity),
                            false,
                        );
                        blend_pixel(&mut output.bgra[j..j + 4], &premul(p, opacity), true);
                    } else {
                        blend_pixel(&mut combined, &premul(p, 255), false);
                        blend_pixel(
                            &mut output.bgra[j..j + 4],
                            &premul(&combined, opacity),
                            false,
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

fn composite(output: &mut [u8], pixels: &[u8]) {
    // tiny-skia pixels are premultiplied RGBA; captured/exported pixels are straight BGRA.
    for (dst, src) in output.chunks_exact_mut(4).zip(pixels.chunks_exact(4)) {
        blend_pixel(dst, src, false);
    }
}

fn scale_alpha(rgba: &mut [u8], opacity: u8) {
    let alpha = (opacity as u32 * 255 + 50) / 100;
    for p in rgba.chunks_exact_mut(4) {
        for c in p {
            *c = ((*c as u32 * alpha + 127) / 255) as u8;
        }
    }
}

fn grouped_opacity(tool: crate::editor::Tool) -> bool {
    use crate::editor::Tool;
    matches!(tool, Tool::Arrow | Tool::Text | Tool::Callout | Tool::Step)
}

fn group_into(output: &mut Frame, crop: Rect, object: &Annotation) -> Result<(), String> {
    if object.opacity == 0 {
        return Ok(());
    }
    let (mut a, mut b) = object.body_bounds();
    if let Some(tip) = object.tail_tip() {
        a.x = a.x.min(tip.x);
        a.y = a.y.min(tip.y);
        b.x = b.x.max(tip.x);
        b.y = b.y.max(tip.y);
    }
    let pad = object.style.width as f64 / 2. + 3.;
    let Some(bounds) = Rect::from_edges(
        (a.x - pad).floor() as i32,
        (a.y - pad).floor() as i32,
        (b.x + pad).ceil() as i32,
        (b.y + pad).ceil() as i32,
    )
    .ok()
    .and_then(|r| r.intersection(crop)) else {
        return Ok(());
    };
    // Render one complete object into bounded transparent tiles before applying opacity.
    for y in (bounds.y.div_euclid(256) * 256..bounds.bottom()).step_by(256) {
        for x in (bounds.x.div_euclid(256) * 256..bounds.right()).step_by(256) {
            let tile = Rect::new(x, y, 256, 256)?.intersection(bounds).unwrap();
            let halo = object.style.width as i32 + 3;
            let region = Rect::from_edges(x - halo, y - halo, x + 256 + halo, y + 256 + halo)?;
            let transparent = Frame::new(
                region.width,
                region.height,
                vec![0; Frame::byte_len(region.width, region.height)?],
            )?;
            let mut local = object.clone();
            local.opacity = 100;
            let mut document = Document::new(region.width, region.height);
            document.crop = region;
            document.objects.push(local);
            let group = flatten_objects(transparent, &document)?;
            let alpha = (object.opacity as u32 * 255 + 50) / 100;
            for ty in 0..tile.height {
                for tx in 0..tile.width {
                    let i = ((((tile.y - region.y) as u32 + ty) * region.width
                        + (tile.x - region.x) as u32
                        + tx)
                        * 4) as usize;
                    let p = &group.bgra[i..i + 4];
                    let a = (p[3] as u32 * alpha + 127) / 255;
                    let premul = |c: u8| ((c as u32 * a + 127) / 255) as u8;
                    let src = [premul(p[2]), premul(p[1]), premul(p[0]), a as u8];
                    let j = (((tile.y - crop.y) as u32 + ty) * output.width
                        + (tile.x - crop.x) as u32
                        + tx) as usize
                        * 4;
                    blend_pixel(&mut output.bgra[j..j + 4], &src, false);
                }
            }
        }
    }
    Ok(())
}

pub fn blend_pixel(dst: &mut [u8], src: &[u8], multiply: bool) {
    let alpha = src[3] as u32;
    if alpha == 0 {
        return;
    }
    let background_alpha = dst[3] as u32;
    let back = background_alpha * (255 - alpha);
    let out = alpha * 255 + back;
    for (di, si) in [(0, 2), (1, 1), (2, 0)] {
        let front = if multiply {
            src[si] as u32 * (255 * (255 - background_alpha) + dst[di] as u32 * background_alpha)
        } else {
            src[si] as u32 * 255 * 255
        };
        dst[di] = ((front + dst[di] as u32 * back + out / 2) / out).min(255) as u8;
    }
    dst[3] = ((out + 127) / 255) as u8;
}

fn text_decoration(
    output: &mut Frame,
    crop: Rect,
    source: Rect,
    object: &Annotation,
) -> Result<(), String> {
    let Content::Text { background, .. } = object.content else {
        return Ok(());
    };
    if !background.enabled && object.tail_tip().is_none() {
        return Ok(());
    }
    let (a, b) = object.body_bounds();
    let path = if let Some(tip) = object.tail_tip() {
        crate::callout::perimeter((a, b), tip)
    } else {
        tiny_skia::Rect::from_ltrb(a.x as f32, a.y as f32, b.x as f32, b.y as f32)
            .map(tiny_skia::PathBuilder::from_rect)
    };
    let Some(path) = path else { return Ok(()) };
    let border = object.callout_border();
    let pad = border.width as i32 + 2;
    let r = path.bounds();
    let Ok(bounds) = Rect::from_edges(
        r.left().floor() as i32 - pad,
        r.top().floor() as i32 - pad,
        r.right().ceil() as i32 + pad,
        r.bottom().ceil() as i32 + pad,
    ) else {
        return Ok(());
    };
    let Some(visible) = bounds
        .intersection(crop)
        .and_then(|r| r.intersection(source))
    else {
        return Ok(());
    };
    // Source-anchored tiles keep antialiasing identical when crop clips a curve.
    for ty in ((visible.y / 256) * 256..visible.bottom()).step_by(256) {
        for tx in ((visible.x / 256) * 256..visible.right()).step_by(256) {
            let tile = Rect::new(tx, ty, 256, 256)?.intersection(visible).unwrap();
            let side = (256 + 2 * pad) as u32;
            let mut pixels = tiny_skia::Pixmap::new(side, side).ok_or("Cannot draw text frame.")?;
            let transform =
                tiny_skia::Transform::from_translate(-(tx - pad) as f32, -(ty - pad) as f32);
            let mut paint = tiny_skia::Paint::default();
            if background.enabled {
                let [r, g, b] = background.rgb;
                paint.set_color_rgba8(r, g, b, 255);
                pixels.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
            }
            if object.tail_tip().is_some() {
                let [r, g, b] = border.rgb;
                paint.set_color_rgba8(r, g, b, 255);
                pixels.stroke_path(
                    &path,
                    &paint,
                    &tiny_skia::Stroke {
                        width: border.width as f32,
                        line_join: tiny_skia::LineJoin::Round,
                        ..Default::default()
                    },
                    transform,
                    None,
                );
            }
            for y in tile.y..tile.bottom() {
                let offset =
                    (((y - ty + pad) as u32 * side + (tile.x - tx + pad) as u32) * 4) as usize;
                let dest =
                    (((y - crop.y) as u32 * output.width + (tile.x - crop.x) as u32) * 4) as usize;
                let len = tile.width as usize * 4;
                composite(
                    &mut output.bgra[dest..dest + len],
                    &pixels.data()[offset..offset + len],
                );
            }
        }
    }
    Ok(())
}

fn mask_into(
    output: &mut Frame,
    crop: Rect,
    p: crate::editor::Point,
    mask: &crate::glyphs::Mask,
    color: [u8; 3],
) {
    let ox = p.x.round() as i32 - crop.x;
    let oy = p.y.round() as i32 - crop.y;
    for y in 0..mask.height as i32 {
        for x in 0..mask.width as i32 {
            let dx = ox + x;
            let dy = oy + y;
            if dx < 0 || dy < 0 || dx >= output.width as i32 || dy >= output.height as i32 {
                continue;
            }
            let a = mask.alpha[(y as u32 * mask.width + x as u32) as usize] as u32;
            if a == 0 {
                continue;
            }
            let dst =
                &mut output.bgra[((dy as u32 * output.width + dx as u32) * 4) as usize..][..4];
            let back = dst[3] as u32 * (255 - a);
            let out = a * 255 + back;
            for (di, si) in [(0, 2), (1, 1), (2, 0)] {
                dst[di] =
                    ((color[si] as u32 * a * 255 + dst[di] as u32 * back + out / 2) / out) as u8;
            }
            dst[3] = ((out + 127) / 255) as u8;
        }
    }
}

fn blur_into(
    output: &mut Frame,
    crop: Rect,
    object: &Annotation,
    strength: u32,
) -> Result<(), String> {
    if object.opacity == 0 {
        return Ok(());
    }
    let opacity = (object.opacity as u32 * 255 + 50) / 100;
    let (a, b) = object.bounds();
    let Ok(region) = Rect::from_edges(
        a.x.round() as i32,
        a.y.round() as i32,
        b.x.round() as i32,
        b.y.round() as i32,
    ) else {
        return Ok(());
    };
    let Some(region) = region.intersection(crop) else {
        return Ok(());
    };
    let local = Rect::new(
        region.x - crop.x,
        region.y - crop.y,
        region.width,
        region.height,
    )?;
    let input = output.crop(Rect::new(0, 0, output.width, output.height)?, local)?;
    // Tile with kernel halos; every tile reads the same immutable beneath-image.
    // This bounds the filter's float scratch memory without adding tile seams.
    let halo = blur_halo(strength) as i32;
    for ty in (0..region.height).step_by(256) {
        for tx in (0..region.width).step_by(256) {
            let w = (region.width - tx).min(256);
            let h = (region.height - ty).min(256);
            let l = (tx as i32 - halo).max(0);
            let t = (ty as i32 - halo).max(0);
            let r = (tx as i32 + w as i32 + halo).min(region.width as i32);
            let b = (ty as i32 + h as i32 + halo).min(region.height as i32);
            let tile = input.crop(
                Rect::new(0, 0, input.width, input.height)?,
                Rect::from_edges(l, t, r, b)?,
            )?;
            let mut rgba = tile.bgra;
            for p in rgba.chunks_exact_mut(4) {
                p.swap(0, 2);
                let a = p[3] as u32;
                for c in &mut p[..3] {
                    *c = (*c as u32 * a / 255) as u8;
                }
            }
            let image = image::RgbaImage::from_raw(tile.width, tile.height, rgba)
                .ok_or("Invalid blur tile.")?;
            let filtered = image::imageops::blur(&image, strength as f32);
            for y in 0..h {
                for x in 0..w {
                    let p = filtered.get_pixel(tx + x - l as u32, ty + y - t as u32).0;
                    let index =
                        (((local.y as u32 + ty + y) * output.width + local.x as u32 + tx + x) * 4)
                            as usize;
                    let dst = &mut output.bgra[index..][..4];
                    let alpha = (p[3] as u32 * opacity + 127) / 255;
                    let src = [
                        ((p[0] as u32 * opacity + 127) / 255) as u8,
                        ((p[1] as u32 * opacity + 127) / 255) as u8,
                        ((p[2] as u32 * opacity + 127) / 255) as u8,
                        alpha as u8,
                    ];
                    blend_pixel(dst, &src, false);
                }
            }
        }
    }
    Ok(())
}
fn blur_halo(strength: u32) -> u32 {
    strength * 4 + 2
}

#[cfg(test)]
mod tests {
    #[test]
    fn ordinary_shape_opacity_changes_pixels_and_zero_is_invisible() {
        let source = Arc::new(Frame::new(160, 100, vec![255; 160 * 100 * 4]).unwrap());
        let mut s = crate::interaction::Session::new(160, 100);
        s.choose(Tool::FilledRectangle);
        s.press(Point::new(20., 20.), 2.);
        s.release(Point::new(100., 80.));
        s.set_opacity(50);
        let output = flatten(&source, &s.document).unwrap();
        let i = (40 * 160 + 40) * 4;
        assert_eq!(&output.bgra[i..i + 4], &[127, 127, 255, 255]);
        s.set_opacity(0);
        assert_eq!(flatten(&source, &s.document).unwrap().bgra, source.bgra);
        assert_eq!(s.document.objects.len(), 1);
        assert!(s.document.hit(Point::new(40., 40.), 2.).is_some());
    }
    #[test]
    fn opacity_applies_to_complete_callout_not_each_component() {
        let source = Arc::new(Frame::new(240, 160, vec![255; 240 * 160 * 4]).unwrap());
        let mut s = crate::interaction::Session::new(240, 160);
        s.document
            .add_callout(
                Point::new(40., 30.),
                "Hello",
                24,
                Style::default(),
                crate::editor::Background {
                    enabled: true,
                    rgb: [0, 0, 255],
                },
            )
            .unwrap();
        let opaque = flatten(&source, &s.document).unwrap();
        s.set_opacity(50);
        let translucent = flatten(&source, &s.document).unwrap();
        for (index, (full, half)) in opaque
            .bgra
            .chunks_exact(4)
            .zip(translucent.bgra.chunks_exact(4))
            .enumerate()
        {
            for c in 0..3 {
                let expected = ((full[c] as u32 * 128 + 255 * 127 + 127) / 255) as i32;
                assert!(
                    (half[c] as i32 - expected).abs() <= 1,
                    "group opacity mismatch at {},{}: opaque {:?}, half {:?}, expected {} channel {}",
                    index % 240,
                    index / 240,
                    full,
                    half,
                    expected,
                    c
                );
            }
            assert_eq!(half[3], 255);
        }
    }
    #[test]
    fn image_opacity_preserves_source_alpha_pixels_and_one_history_step() {
        let white = Arc::new(Frame::new(100, 80, vec![255; 100 * 80 * 4]).unwrap());
        let image = Arc::new(Frame::new(2, 1, vec![0, 0, 255, 128, 0, 0, 255, 0]).unwrap());
        let mut d = Document::new(100, 80);
        d.add_image(image.clone()).unwrap();
        let revision = d.revision;
        assert_eq!(d.objects[0].opacity, 100);
        assert!(d.set_opacity(50));
        let output = flatten(&white, &d).unwrap();
        assert_eq!(&output.bgra[..8], &[191, 191, 255, 255, 255, 255, 255, 255]);
        assert_eq!(image.bgra, vec![0, 0, 255, 128, 0, 0, 255, 0]);
        assert!(d.undo());
        assert_eq!(d.revision, revision);
        assert_eq!(d.objects[0].opacity, 100);
        assert!(d.redo());
        assert_eq!(d.objects[0].opacity, 50);
    }
    #[test]
    fn all_annotation_opacity_zero_and_half_match_group_composition() {
        let white = Arc::new(Frame::new(520, 350, vec![255; 520 * 350 * 4]).unwrap());
        for tool in [
            Tool::Pen,
            Tool::Line,
            Tool::Arrow,
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::FilledRectangle,
            Tool::FilledEllipse,
            Tool::Text,
            Tool::Callout,
            Tool::Step,
        ] {
            let mut d = Document::new(520, 350);
            match tool {
                Tool::Text => {
                    d.add_label(
                        Point::new(200., 240.),
                        "Sample text",
                        24,
                        Style::default(),
                        crate::editor::Background {
                            enabled: true,
                            rgb: [0, 0, 255],
                        },
                        false,
                    )
                    .unwrap();
                }
                Tool::Callout => {
                    d.add_callout(
                        Point::new(200., 240.),
                        "Sample text",
                        24,
                        Style::default(),
                        crate::editor::Background {
                            enabled: true,
                            rgb: [0, 0, 255],
                        },
                    )
                    .unwrap();
                }
                Tool::Step => {
                    d.add_step(Point::new(255., 255.), 36, Style::default())
                        .unwrap();
                }
                _ => {
                    d.add_patterned(
                        tool,
                        vec![Point::new(200., 230.), Point::new(420., 310.)],
                        Style::default(),
                        crate::editor::Pattern::Dotted,
                    );
                }
            }
            if tool == Tool::Arrow {
                d.set_curve_midpoint(Point::new(300., 220.));
            }
            let opaque = flatten(&white, &d).unwrap();
            assert!(d.set_opacity(50));
            let half = flatten(&white, &d).unwrap();
            for (index, (a, b)) in opaque
                .bgra
                .chunks_exact(4)
                .zip(half.bgra.chunks_exact(4))
                .enumerate()
            {
                for c in 0..3 {
                    let expected = ((a[c] as u32 * 128 + 255 * 127 + 127) / 255) as i32;
                    assert!(
                        (b[c] as i32 - expected).abs() <= 1,
                        "{:?} at {},{}: {:?} {:?}",
                        tool,
                        index % 520,
                        index / 520,
                        a,
                        b
                    );
                }
            }
            assert!(d.set_opacity(0));
            assert_eq!(flatten(&white, &d).unwrap().bgra, white.bgra);
            assert_eq!(d.objects.len(), 1);
        }
    }
    #[test]
    fn translucent_shape_replacement_keeps_upper_layers_and_matches_final() {
        let source = Arc::new(Frame::new(1000, 600, vec![255; 1000 * 600 * 4]).unwrap());
        let mut d = Document::new(1000, 600);
        d.add(
            Tool::FilledEllipse,
            vec![Point::new(30., 30.), Point::new(180., 180.)],
            Style::default(),
        );
        d.set_opacity(50);
        let id = d.selected;
        d.add(
            Tool::FilledRectangle,
            vec![Point::new(100., 100.), Point::new(250., 120.)],
            Style {
                rgb: [0, 120, 255],
                width: 5,
            },
        );
        d.selected = id;
        let mut replacement = d.objects[0].clone();
        replacement.points = Arc::new(vec![Point::new(30., 30.), Point::new(950., 550.)]);
        let view = View {
            x: 0.,
            y: 0.,
            scale: 1.,
            crop: d.crop,
        };
        let patch = replacement_draft(&source, &d, &replacement, view).unwrap();
        d.resize_shape(replacement.bounds());
        let final_frame = flatten(&source, &d).unwrap();
        for y in 0..patch.bounds.height {
            for x in 0..patch.bounds.width {
                let i = ((y * patch.bounds.width + x) * 4) as usize;
                let j =
                    (((patch.bounds.y as u32 + y) * 1000 + patch.bounds.x as u32 + x) * 4) as usize;
                assert_eq!(
                    &patch.rgba[i..i + 4],
                    &[
                        final_frame.bgra[j + 2],
                        final_frame.bgra[j + 1],
                        final_frame.bgra[j],
                        255
                    ]
                );
            }
        }
    }
    #[test]
    fn patterned_curves_and_shapes_have_matching_draft_export_pixels() {
        use crate::editor::Pattern;
        let white = Arc::new(Frame::new(400, 300, vec![255; 400 * 300 * 4]).unwrap());
        for tool in [
            Tool::Arrow,
            Tool::Ellipse,
            Tool::FilledEllipse,
            Tool::FilledRectangle,
        ] {
            for pattern in [Pattern::Solid, Pattern::Dashed, Pattern::Dotted] {
                let mut d = Document::new(400, 300);
                let points = if tool == Tool::Arrow {
                    vec![
                        Point::new(30., 240.),
                        Point::new(350., 240.),
                        Point::new(190., -80.),
                    ]
                } else {
                    vec![Point::new(30., 30.), Point::new(350., 240.)]
                };
                assert!(d.add_patterned(tool, points, Style::default(), pattern));
                let full = flatten(&white, &d).unwrap();
                let view = View {
                    crop: d.crop,
                    x: 0.,
                    y: 0.,
                    scale: 1.,
                };
                let draft = draft(&d.objects[0], view).unwrap();
                for y in draft.bounds.y..draft.bounds.bottom() {
                    for x in draft.bounds.x..draft.bounds.right() {
                        let i = (((y - draft.bounds.y) as u32 * draft.bounds.width
                            + (x - draft.bounds.x) as u32)
                            * 4) as usize;
                        let mut bgra = [255; 4];
                        blend_pixel(&mut bgra, &draft.rgba[i..i + 4], false);
                        assert_eq!(
                            bgra.as_slice(),
                            pixel(&full, x as usize, y as usize),
                            "{tool:?} {pattern:?} ({x},{y})"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn long_curve_replacement_keeps_display_allocation_bounded() {
        let source = Arc::new(Frame::new(100, 8000, vec![255; 100 * 8000 * 4]).unwrap());
        let mut d = Document::new(100, 8000);
        assert!(d.add(
            Tool::Arrow,
            vec![Point::new(20., 100.), Point::new(20., 7900.)],
            Style::default()
        ));
        let replacement =
            crate::geometry::bend(&d.objects[0], Point::new(60., 4000.), d.crop).unwrap();
        let started = std::time::Instant::now();
        let patch = replacement_draft(
            &source,
            &d,
            &replacement,
            View {
                crop: d.crop,
                x: 0.,
                y: 0.,
                scale: 0.05,
            },
        )
        .unwrap();
        assert!(patch.rgba.len() <= 5 * 400 * 4);
        assert!(patch.rgba.chunks_exact(4).all(|p| p[3] == 255));
        assert_eq!(source.bgra[0..4], [255; 4]);
        eprintln!(
            "long curve replacement {:?}, {} display bytes",
            started.elapsed(),
            patch.rgba.len()
        );
    }
    #[test]
    fn curve_replacement_erases_old_line_and_preserves_layers_above() {
        let white = Arc::new(Frame::new(100, 80, vec![255; 100 * 80 * 4]).unwrap());
        let mut d = Document::new(100, 80);
        d.add(
            Tool::Line,
            vec![Point::new(10., 60.), Point::new(90., 60.)],
            Style::default(),
        );
        let original = d.objects[0].clone();
        d.add(
            Tool::FilledRectangle,
            vec![Point::new(40., 20.), Point::new(60., 40.)],
            Style {
                rgb: [0, 0, 255],
                width: 5,
            },
        );
        let replacement = crate::geometry::bend(&original, Point::new(50., 30.), d.crop).unwrap();
        let view = View {
            crop: d.crop,
            x: 0.,
            y: 0.,
            scale: 1.,
        };
        let patch = replacement_draft(&white, &d, &replacement, view).unwrap();
        let at = |x: u32, y: u32| {
            let i = (((y - patch.bounds.y as u32) * patch.bounds.width + x - patch.bounds.x as u32)
                * 4) as usize;
            patch.rgba[i..i + 4].to_vec()
        };
        assert_eq!(at(30, 60), vec![255; 4], "old straight line must be erased");
        assert_eq!(
            at(50, 30),
            vec![0, 0, 255, 255],
            "blue top layer must cover red curve"
        );
        assert!(patch.rgba.len() <= 100 * 80 * 4);
        let mut final_doc = d.render_snapshot();
        final_doc.objects[0] = replacement;
        let final_frame = flatten(&white, &final_doc).unwrap();
        for y in patch.bounds.y..patch.bounds.bottom() {
            for x in patch.bounds.x..patch.bounds.right() {
                let bgra = pixel(&final_frame, x as usize, y as usize);
                assert_eq!(
                    at(x as u32, y as u32),
                    vec![bgra[2], bgra[1], bgra[0], bgra[3]]
                );
            }
        }
    }
    #[test]
    fn dashed_and_dotted_strokes_have_real_gaps_and_continuous_phase() {
        use crate::editor::Pattern;
        let white = Arc::new(Frame::new(100, 80, vec![255; 100 * 80 * 4]).unwrap());
        for pattern in [Pattern::Dashed, Pattern::Dotted] {
            let mut d = Document::new(100, 80);
            d.add(
                Tool::Pen,
                vec![
                    Point::new(10., 30.),
                    Point::new(17., 30.),
                    Point::new(90., 30.),
                ],
                Style {
                    rgb: [255, 0, 0],
                    width: 4,
                },
            );
            d.objects[0].pattern = pattern;
            let f = flatten(&white, &d).unwrap();
            assert_eq!(pixel(&f, 10, 30), [0, 0, 255, 255]);
            assert_eq!(
                pixel(&f, if pattern == Pattern::Dashed { 32 } else { 16 }, 30),
                [255; 4],
                "{pattern:?}"
            );
            assert_eq!(
                pixel(&f, if pattern == Pattern::Dashed { 40 } else { 22 }, 30),
                [0, 0, 255, 255]
            );
            d.set_crop(Rect::new(20, 20, 50, 30).unwrap());
            assert_eq!(
                flatten(&white, &d).unwrap().bgra,
                f.crop(Rect::new(0, 0, 100, 80).unwrap(), d.crop)
                    .unwrap()
                    .bgra
            );
        }
    }
    #[test]
    fn ellipse_and_filled_shape_pixels_distinguish_boundary_and_interior() {
        let white = Arc::new(Frame::new(100, 80, vec![255; 100 * 80 * 4]).unwrap());
        for tool in [Tool::Ellipse, Tool::FilledEllipse, Tool::FilledRectangle] {
            let mut d = Document::new(100, 80);
            assert!(d.add(
                tool,
                vec![Point::new(80., 60.), Point::new(20., 20.)],
                Style::default()
            ));
            let f = flatten(&white, &d).unwrap();
            assert_eq!(
                pixel(&f, 50, 40),
                if tool.filled() {
                    [0, 0, 255, 255]
                } else {
                    [255; 4]
                },
                "{tool:?}"
            );
            assert_eq!(
                pixel(&f, 22, 22),
                if tool == Tool::FilledRectangle {
                    [0, 0, 255, 255]
                } else {
                    [255; 4]
                }
            );
            assert_eq!(d.hit(Point::new(50., 40.), 0.).is_some(), tool.filled());
            assert_eq!(
                d.hit(Point::new(22., 22.), 0.).is_some(),
                tool == Tool::FilledRectangle
            );
        }
    }
    #[test]
    fn quadratic_render_is_on_curve_not_control_polygon() {
        let white = Arc::new(Frame::new(100, 80, vec![255; 100 * 80 * 4]).unwrap());
        let mut d = Document::new(100, 80);
        d.add(
            Tool::Line,
            vec![
                Point::new(10., 60.),
                Point::new(90., 60.),
                Point::new(50., 0.),
            ],
            Style::default(),
        );
        let f = flatten(&white, &d).unwrap();
        assert_eq!(pixel(&f, 50, 30), [0, 0, 255, 255]);
        assert_eq!(pixel(&f, 50, 60), [255; 4]);
        assert_eq!(pixel(&f, 50, 0), [255; 4]);
    }
    #[test]
    fn highlight_obeys_imported_image_and_blur_layer_order() {
        let mut bytes = Vec::new();
        for _y in 0..80 {
            for x in 0..100 {
                let v = if x < 50 { 0 } else { 255 };
                bytes.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let image = Arc::new(Frame::new(100, 80, bytes).unwrap());
        let mut d = Document::new(100, 80);
        d.add_image(image.clone()).unwrap();
        d.add_highlight(
            vec![Point::new(10., 40.), Point::new(90., 40.)],
            Style {
                rgb: [255, 242, 88],
                width: 24,
            },
            40,
        );
        let highlighted = flatten(&source(), &d).unwrap();
        assert_eq!(pixel(&highlighted, 30, 40), [0, 0, 0, 255]);
        assert_eq!(pixel(&highlighted, 70, 40), [188, 250, 255, 255]);
        d.reorder(crate::editor::Order::Back);
        assert_eq!(flatten(&source(), &d).unwrap().bgra, image.bgra);
        d.undo();
        d.add_blur(Point::new(5., 10.), Point::new(95., 70.), 8);
        let blurred_on_top = flatten(&source(), &d).unwrap();
        let mut reference = Document::new(100, 80);
        reference.add_blur(Point::new(5., 10.), Point::new(95., 70.), 8);
        assert_eq!(
            blurred_on_top.bgra,
            flatten(&highlighted, &reference).unwrap().bgra
        );
        assert!(d.reorder(crate::editor::Order::Backward));
        assert_ne!(flatten(&source(), &d).unwrap().bgra, blurred_on_top.bgra);
        d.undo();
        assert_eq!(flatten(&source(), &d).unwrap().bgra, blurred_on_top.bgra);
        assert_eq!(pixel(&image, 70, 40), [255; 4]);
    }
    #[test]
    fn multiply_handles_transparent_and_partial_alpha_without_dark_fringes() {
        let mut transparent = [17, 31, 62, 0];
        blend_pixel(&mut transparent, &[102, 97, 35, 102], true);
        assert_eq!(transparent, [88, 243, 255, 102]);
        for ab in [0, 77, 128, 255] {
            let mut actual = [200, 150, 100, ab];
            let src = [102, 97, 35, 102];
            blend_pixel(&mut actual, &src, true);
            let a = 102. / 255.;
            let b = ab as f64 / 255.;
            let out = a + b * (1. - a);
            for (di, si) in [(0, 2), (1, 1), (2, 0)] {
                let base = [200., 150., 100.][di] / 255.;
                let fore = src[si] as f64 / 102.;
                let expected = ((a * (1. - b) * fore + a * b * base * fore + (1. - a) * b * base)
                    / out
                    * 255.)
                    .round() as u8;
                assert_eq!(actual[di], expected);
            }
            assert_eq!(actual[3], (out * 255.).round() as u8);
        }
    }
    #[test]
    fn highlight_opacity_endpoints_layer_order_and_crop_preserve_source() {
        let s = source();
        let mut d = Document::new(100, 80);
        d.add(
            Tool::Line,
            vec![Point::new(10., 30.), Point::new(90., 30.)],
            Style {
                rgb: [0, 0, 255],
                width: 5,
            },
        );
        d.add_highlight(
            vec![Point::new(10., 30.), Point::new(90., 30.)],
            Style {
                rgb: [255, 242, 88],
                width: 24,
            },
            0,
        );
        assert_eq!(pixel(&flatten(&s, &d).unwrap(), 50, 30), [255, 0, 0, 255]);
        d.set_opacity(100);
        let full = flatten(&s, &d).unwrap();
        assert_eq!(pixel(&full, 50, 30), [88, 0, 0, 255]);
        d.reorder(crate::editor::Order::Back);
        assert_eq!(pixel(&flatten(&s, &d).unwrap(), 50, 30), [255, 0, 0, 255]);
        d.undo();
        let crop = Rect::new(30, 20, 40, 30).unwrap();
        d.set_crop(crop);
        assert_eq!(
            flatten(&s, &d).unwrap().bgra,
            full.crop(Rect::new(0, 0, 100, 80).unwrap(), crop)
                .unwrap()
                .bgra
        );
        assert_eq!(pixel(&s, 50, 30), [21, 43, 65, 255]);
    }
    #[test]
    fn highlight_draft_matches_final_pixels_and_allocates_only_the_stroke_region() {
        let mut d = Document::new(100, 80);
        d.add_highlight(
            vec![Point::new(10., 20.), Point::new(70., 45.)],
            Style {
                rgb: [255, 242, 88],
                width: 24,
            },
            40,
        );
        let view = View {
            crop: d.crop,
            x: 20.,
            y: 30.,
            scale: 1.,
        };
        let live = draft(&d.objects[0], view).unwrap();
        assert!(live.multiply);
        assert!(live.rgba.len() < 100 * 80 * 4);
        let done = flatten(&source(), &d).unwrap();
        for y in 0..live.bounds.height as usize {
            for x in 0..live.bounds.width as usize {
                let mut actual = [21, 43, 65, 255];
                let p = &live.rgba[(y * live.bounds.width as usize + x) * 4..][..4];
                blend_pixel(&mut actual, p, live.multiply);
                assert_eq!(
                    actual,
                    pixel(
                        &done,
                        live.bounds.x as usize + x - 20,
                        live.bounds.y as usize + y - 30
                    )
                );
            }
        }
    }
    #[test]
    fn highlighter_multiplies_beneath_pixels_without_mutating_source() {
        use crate::{
            editor::{Document, Point, Style, Tool},
            frame::Frame,
        };
        use std::sync::Arc;
        let mut bytes = vec![255; 80 * 60 * 4];
        bytes[(25 * 80 + 30) * 4..][..4].copy_from_slice(&[0, 0, 0, 255]);
        bytes[(25 * 80 + 40) * 4..][..4].copy_from_slice(&[200, 150, 100, 255]);
        let source = Arc::new(Frame::new(80, 60, bytes.clone()).unwrap());
        let mut document = Document::new(80, 60);
        assert!(document.add(
            Tool::Highlighter,
            vec![Point::new(10., 25.), Point::new(60., 25.)],
            Style {
                rgb: [255, 242, 88],
                width: 24
            }
        ));
        let out = super::flatten(&source, &document).unwrap();
        assert_eq!(&out.bgra[(25 * 80 + 30) * 4..][..4], &[0, 0, 0, 255]);
        assert_eq!(&out.bgra[(25 * 80 + 40) * 4..][..4], &[147, 147, 100, 255]);
        assert_eq!(&out.bgra[(25 * 80 + 50) * 4..][..4], &[188, 250, 255, 255]);
        assert_eq!(&out.bgra[..4], &[255; 4]);
        assert_eq!(source.bgra, bytes);
    }

    #[test]
    fn one_highlight_self_overlap_is_not_two_separate_strokes() {
        use crate::{
            editor::{Document, Point, Style, Tool},
            frame::Frame,
        };
        use std::sync::Arc;
        let source = Arc::new(Frame::new(80, 60, vec![255; 80 * 60 * 4]).unwrap());
        let style = Style {
            rgb: [255, 242, 88],
            width: 24,
        };
        let mut document = Document::new(80, 60);
        document.add(
            Tool::Highlighter,
            vec![
                Point::new(10., 25.),
                Point::new(60., 25.),
                Point::new(10., 25.),
            ],
            style,
        );
        let first = super::flatten(&source, &document).unwrap();
        assert_eq!(
            &first.bgra[(25 * 80 + 40) * 4..][..4],
            &[188, 250, 255, 255]
        );
        document.add(
            Tool::Highlighter,
            vec![Point::new(10., 25.), Point::new(60., 25.)],
            style,
        );
        let second = super::flatten(&source, &document).unwrap();
        assert_eq!(
            &second.bgra[(25 * 80 + 40) * 4..][..4],
            &[139, 245, 255, 255]
        );
    }
    #[test]
    fn expanding_after_crop_does_not_reveal_cropped_source_pixels() {
        use crate::editor::Document;
        use crate::frame::{Frame, Rect};
        use std::sync::Arc;
        let source = Arc::new(
            Frame::new(
                3,
                1,
                vec![10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255],
            )
            .unwrap(),
        );
        let mut d = Document::new(3, 1);
        assert!(d.set_crop(Rect::new(0, 0, 1, 1).unwrap()));
        d.expand_canvas(3, 1).unwrap();
        let out = super::flatten(&source, &d).unwrap();
        assert_eq!(&out.bgra[..4], &source.bgra[..4]);
        assert_eq!(&out.bgra[4..], &[255; 8]);
        assert_eq!(
            super::flatten(&source, &d.render_snapshot()).unwrap().bgra,
            out.bgra
        );
        assert!(d.undo());
        assert_eq!(super::flatten(&source, &d).unwrap().width, 1);
        assert!(d.undo());
        assert_eq!(super::flatten(&source, &d).unwrap().bgra, source.bgra);
        assert!(d.redo());
        assert!(d.redo());
        assert_eq!(super::flatten(&source, &d).unwrap().bgra, out.bgra);
    }
    #[test]
    fn expanded_cropped_canvas_keeps_origin_and_allows_text_outside_capture() {
        use crate::editor::{Background, Document, Point, Style};
        use crate::frame::{Frame, Rect};
        use std::sync::Arc;
        let source = Arc::new(
            Frame::new(
                2,
                2,
                vec![
                    10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
                ],
            )
            .unwrap(),
        );
        let mut d = Document::new(2, 2);
        d.set_crop(Rect::new(1, 1, 1, 1).unwrap());
        d.expand_canvas(100, 60).unwrap();
        d.add_label(
            Point::new(20., 20.),
            "Hi",
            12,
            Style::default(),
            Background {
                enabled: true,
                rgb: [0, 255, 0],
            },
            false,
        )
        .unwrap();
        let out = super::flatten(&source, &d).unwrap();
        assert_eq!(&out.bgra[..4], &[100, 110, 120, 255]);
        assert_eq!(&out.bgra[4..8], &[255; 4]);
        let offset = (19 * 100 + 19) * 4;
        assert_eq!(&out.bgra[offset..offset + 4], &[0, 255, 0, 255]);
        assert!(
            out.bgra
                .chunks_exact(4)
                .any(|p| p[2] > 0 && p[1] == 0 && p[0] == 0)
        );
    }
    #[test]
    fn images_compose_in_layer_order_with_alpha_on_expanded_canvas_and_crop() {
        use crate::editor::{Document, Order};
        use crate::frame::{Frame, Rect};
        use std::sync::Arc;
        let base = Arc::new(Frame::new(2, 1, vec![0, 0, 255, 255, 0, 255, 0, 255]).unwrap());
        let image = Arc::new(Frame::new(2, 1, vec![255, 0, 0, 255, 255, 0, 0, 128]).unwrap());
        let mut d = Document::new(2, 1);
        d.add_image(image.clone()).unwrap();
        d.expand_canvas(4, 2).unwrap();
        d.move_selected(1., 0.);
        let out = super::flatten(&base, &d).unwrap();
        assert_eq!((out.width, out.height), (4, 2));
        assert_eq!(
            &out.bgra[..16],
            &[
                0, 0, 255, 255, 255, 0, 0, 255, 255, 127, 127, 255, 255, 255, 255, 255
            ]
        );
        assert_eq!(&out.bgra[16..], &[255; 16]);
        d.add_image(Arc::new(Frame::new(1, 1, vec![0, 0, 0, 255]).unwrap()))
            .unwrap();
        d.move_selected(1., 0.);
        assert_eq!(
            &super::flatten(&base, &d).unwrap().bgra[4..8],
            &[0, 0, 0, 255]
        );
        d.reorder(Order::Back);
        assert_eq!(
            &super::flatten(&base, &d).unwrap().bgra[4..8],
            &[255, 0, 0, 255]
        );
        d.set_crop(Rect::new(1, 0, 2, 1).unwrap());
        assert_eq!(
            super::flatten(&base, &d).unwrap().bgra,
            vec![255, 0, 0, 255, 255, 127, 127, 255]
        );
        assert_eq!(image.bgra, vec![255, 0, 0, 255, 255, 0, 0, 128]);
    }
    #[test]
    fn layer_order_exports_topmost_pixels_and_undo_restores_original_overlap() {
        let source = source();
        let mut d = Document::new(100, 80);
        for rgb in [[255, 0, 0], [0, 255, 0], [0, 0, 255]] {
            assert!(d.add(
                Tool::Line,
                vec![Point::new(10., 20.), Point::new(80., 20.)],
                Style { rgb, width: 5 }
            ));
        }
        d.selected = Some(1);
        let before = flatten(&source, &d).unwrap();
        assert_eq!(pixel(&before, 30, 20), [255, 0, 0, 255]);
        assert!(d.reorder(crate::editor::Order::Front));
        let after = flatten(&source, &d).unwrap();
        assert_eq!(pixel(&after, 30, 20), [0, 0, 255, 255]);
        assert_eq!((after.width, after.height), (100, 80));
        assert_eq!(pixel(&source, 30, 20), [21, 43, 65, 255]);
        assert!(d.undo());
        assert_eq!(flatten(&source, &d).unwrap().bgra, before.bgra);
        assert!(d.redo());
        assert_eq!(flatten(&source, &d).unwrap().bgra, after.bgra);
    }
    #[test]
    fn layer_order_blur_only_filters_objects_below_its_new_position() {
        let source = source();
        let mut d = Document::new(100, 80);
        assert!(d.add(
            Tool::Line,
            vec![Point::new(10., 30.), Point::new(80., 30.)],
            Style::default()
        ));
        assert!(d.add_blur(Point::new(5., 5.), Point::new(90., 70.), 8));
        let blurred = flatten(&source, &d).unwrap();
        assert_ne!(pixel(&blurred, 30, 30), [0, 0, 255, 255]);
        assert!(d.reorder(crate::editor::Order::Back));
        let sharp = flatten(&source, &d).unwrap();
        assert_eq!(pixel(&sharp, 30, 30), [0, 0, 255, 255]);
        assert_eq!(pixel(&sharp, 30, 60), [21, 43, 65, 255]);
        assert!(d.undo());
        assert_eq!(flatten(&source, &d).unwrap().bgra, blurred.bgra);
    }
    #[test]
    fn live_callout_geometry_does_not_cover_cached_text_with_background() {
        let mut d = Document::new(800, 600);
        d.add_callout(
            Point::new(50., 50.),
            "Bubble",
            24,
            Style::default(),
            crate::editor::Background {
                enabled: true,
                rgb: [255, 255, 255],
            },
        )
        .unwrap();
        let view = View {
            crop: d.crop,
            x: 0.,
            y: 0.,
            scale: 1.,
        };
        let live = draft(&d.objects[0], view).unwrap();
        let x = (56 - live.bounds.x) as usize;
        let y = (56 - live.bounds.y) as usize;
        assert_eq!(
            live.rgba[(y * live.bounds.width as usize + x) * 4 + 3],
            0,
            "A geometry draft must not paint fill over the existing text."
        );
        assert!(live.rgba.chunks_exact(4).any(|pixel| pixel[3] > 0));
        assert!(live.rgba.len() < 100_000);
        assert!(d.objects[0].text_background().unwrap().enabled);
    }
    #[test]
    fn callout_crop_hides_tail_without_deleting_content() {
        use crate::editor::{Document, Point, Style};
        let source = std::sync::Arc::new(crate::frame::Frame {
            width: 240,
            height: 200,
            bgra: vec![255; 240 * 200 * 4],
        });
        let mut d = Document::new(240, 200);
        d.add_callout(
            Point::new(40., 40.),
            "Text",
            24,
            Style::default(),
            Default::default(),
        )
        .unwrap();
        let original = d.objects[0].clone();
        let full = super::flatten(&source, &d).unwrap();
        let crop = crate::frame::Rect::new(30, 30, 90, 80).unwrap();
        assert!(d.set_crop(crop));
        let clipped = super::flatten(&source, &d).unwrap();
        let expected = full
            .crop(crate::frame::Rect::new(0, 0, 240, 200).unwrap(), crop)
            .unwrap();
        let differences: Vec<_> = clipped
            .bgra
            .iter()
            .zip(&expected.bgra)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .collect();
        assert!(
            differences.is_empty(),
            "crop differences: count={}, first={:?}",
            differences.len(),
            differences.first()
        );
        assert_eq!(d.objects[0], original);
        d.undo();
        assert_eq!(super::flatten(&source, &d).unwrap().bgra, full.bgra);
    }
    #[test]
    fn text_fill_is_optional_and_callout_has_one_perimeter_in_exports() {
        use crate::editor::{Background, Document, Point, Style};
        let source = std::sync::Arc::new(crate::frame::Frame {
            width: 240,
            height: 200,
            bgra: vec![255; 240 * 200 * 4],
        });
        let mut d = Document::new(240, 200);
        d.add_text(Point::new(40., 40.), "Text", 24, Style::default())
            .unwrap();
        let plain = super::flatten(&source, &d).unwrap();
        let pixel =
            |f: &crate::frame::Frame, x: usize, y: usize| f.bgra[(y * 240 + x) * 4..][..4].to_vec();
        assert_eq!(pixel(&plain, 40, 40), vec![255; 4]);
        d.set_text_background(Background {
            enabled: true,
            rgb: [20, 180, 70],
        });
        let filled = super::flatten(&source, &d).unwrap();
        assert_eq!(pixel(&filled, 40, 40), vec![70, 180, 20, 255]);
        assert_eq!(pixel(&filled, 39, 40), vec![255; 4]);
        assert!(d.undo());
        assert_eq!(super::flatten(&source, &d).unwrap().bgra, plain.bgra);
        let mut d = Document::new(240, 200);
        d.add_callout(
            Point::new(40., 40.),
            "Text",
            24,
            Style::default(),
            Background::default(),
        )
        .unwrap();
        let (a, b) = d.objects[0].body_bounds();
        let cx = (a.x + b.x) / 2.;
        d.set_tail(Point::new(cx, b.y + 40.));
        let outline = super::flatten(&source, &d).unwrap();
        // The gap where the tail joins the body must not contain an internal baseline.
        assert_eq!(pixel(&outline, cx as usize, b.y as usize), vec![255; 4]);
        assert_eq!(
            pixel(&outline, cx as usize, (b.y + 38.) as usize),
            vec![0, 0, 255, 255]
        );
        d.set_text_background(Background {
            enabled: true,
            rgb: [20, 180, 70],
        });
        let filled = super::flatten(&source, &d).unwrap();
        assert_eq!(
            pixel(&filled, cx as usize, (b.y + 10.) as usize),
            vec![70, 180, 20, 255]
        );
        assert_eq!(pixel(&filled, 20, 20), vec![255; 4]);
    }
    use super::*;
    #[test]
    fn blur_halo_covers_the_pinned_filter_kernel() {
        // image 0.25.10 uses a 197-pixel kernel for sigma=30 (radius 98).
        assert!(blur_halo(30) >= 98);
    }
    #[test]
    fn tiled_blur_matches_the_library_at_tile_boundaries() {
        let w = 700u32;
        let h = 90u32;
        let pixels: Vec<u8> = (0..w * h)
            .flat_map(|i| {
                let v = (i.wrapping_mul(2654435761u32) >> 24) as u8;
                [v, v, v, 255]
            })
            .collect();
        let source = Arc::new(Frame::new(w, h, pixels.clone()).unwrap());
        let reference =
            image::imageops::blur(&image::RgbaImage::from_raw(w, h, pixels).unwrap(), 30.);
        let mut d = Document::new(w, h);
        d.add_blur(Point::new(0., 0.), Point::new(w as f64, h as f64), 30);
        let actual = flatten(&source, &d).unwrap();
        for x in 0..w as usize {
            assert_eq!(
                pixel(&actual, x, 40),
                reference.get_pixel(x as u32, 40).0,
                "Tile boundary at {x}"
            );
        }
    }
    #[test]
    fn text_and_steps_render_color_and_leave_original_pixels_unchanged() {
        let s = Arc::new(Frame::new(400, 160, [240, 240, 240, 255].repeat(64000)).unwrap());
        let mut d = Document::new(400, 160);
        d.add_text(
            crate::editor::Point::new(10., 10.),
            "ภาษาไทย\nEnglish",
            24,
            crate::editor::Style::default(),
        )
        .unwrap();
        d.add_step(
            crate::editor::Point::new(250., 50.),
            36,
            crate::editor::Style::default(),
        )
        .unwrap();
        let f = flatten(&s, &d).unwrap();
        assert!(f.bgra.chunks_exact(4).any(|p| p[2] == 255 && p[0] < 200));
        assert_eq!(pixel(&f, 1, 1), pixel(&s, 1, 1));
        assert_eq!(pixel(&s, 250, 50), [240, 240, 240, 255]);
        assert_ne!(pixel(&f, 250, 50), pixel(&s, 250, 50));
    }
    #[test]
    fn gaussian_blur_is_clipped_ordered_and_reversible() {
        let mut pixels = vec![];
        for y in 0..80 {
            for x in 0..100 {
                let v = if (x + y) % 2 == 0 { 0 } else { 255 };
                pixels.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let s = Arc::new(Frame::new(100, 80, pixels).unwrap());
        let mut d = Document::new(100, 80);
        d.add(
            crate::editor::Tool::Line,
            vec![
                crate::editor::Point::new(25., 35.),
                crate::editor::Point::new(65., 35.),
            ],
            crate::editor::Style::default(),
        );
        let beneath = flatten(&s, &d).unwrap();
        d.add_blur(
            crate::editor::Point::new(20., 20.),
            crate::editor::Point::new(70., 60.),
            8,
        );
        let f = flatten(&s, &d).unwrap();
        assert_ne!(pixel(&f, 30, 30), pixel(&beneath, 30, 30));
        assert_ne!(pixel(&f, 40, 35), pixel(&beneath, 40, 35));
        for y in 0..80 {
            for x in 0..100 {
                if !(20..70).contains(&x) || !(20..60).contains(&y) {
                    assert_eq!(pixel(&f, x, y), pixel(&beneath, x, y));
                }
            }
        }
        d.delete();
        assert_eq!(flatten(&s, &d).unwrap().bgra, beneath.bgra);
        d.undo();
        assert_eq!(flatten(&s, &d).unwrap().bgra, f.bgra);
        d.set_crop(Rect::new(30, 30, 25, 20).unwrap());
        assert_eq!(
            (
                flatten(&s, &d).unwrap().width,
                flatten(&s, &d).unwrap().height
            ),
            (25, 20)
        );
        assert_eq!(pixel(&s, 30, 30), [0, 0, 0, 255]);
    }
    use crate::{
        editor::{Point, Style, Tool},
        frame::Rect,
    };
    fn source() -> Arc<Frame> {
        Arc::new(Frame::new(100, 80, [21, 43, 65, 255].repeat(8000)).unwrap())
    }
    fn pixel(f: &Frame, x: usize, y: usize) -> &[u8] {
        &f.bgra[(y * f.width as usize + x) * 4..(y * f.width as usize + x) * 4 + 4]
    }
    #[test]
    fn unedited_frame_is_shared_without_pixel_allocation() {
        let s = source();
        assert!(Arc::ptr_eq(
            &s,
            &flatten(&s, &Document::new(100, 80)).unwrap()
        ));
    }
    #[test]
    fn each_annotation_is_rendered_and_background_is_unchanged() {
        for (tool, x, y) in [
            (Tool::Pen, 30, 20),
            (Tool::Line, 30, 20),
            (Tool::Arrow, 30, 20),
            (Tool::Rectangle, 20, 30),
        ] {
            let s = source();
            let mut d = Document::new(100, 80);
            assert!(d.add(
                tool,
                vec![
                    Point::new(20., 20.),
                    Point::new(60., if tool == Tool::Rectangle { 60. } else { 20. })
                ],
                Style::default()
            ));
            let f = flatten(&s, &d).unwrap();
            assert_eq!(pixel(&f, x, y), [0, 0, 255, 255], "{tool:?}");
            assert_eq!(pixel(&f, 2, 2), [21, 43, 65, 255]);
            assert_eq!(pixel(&s, x, y), [21, 43, 65, 255]);
        }
    }
    #[test]
    fn scaled_text_export_is_rerasterized_at_font_size_and_has_no_selection_handles() {
        let s = source();
        let mut d = Document::new(100, 80);
        d.add_text(Point::new(10., 10.), "A", 12, Style::default())
            .unwrap();
        d.resize_text(24, None, Point::new(10., 10.), false, false)
            .unwrap();
        let mut expected = Document::new(100, 80);
        expected
            .add_text(Point::new(10., 10.), "A", 24, Style::default())
            .unwrap();
        expected.selected = None;
        assert_eq!(
            flatten(&s, &d).unwrap().bgra,
            flatten(&s, &expected).unwrap().bgra
        );
        assert_eq!(pixel(&flatten(&s, &d).unwrap(), 10, 10), [21, 43, 65, 255]);
        let hidden = d.objects[0].clone();
        d.set_crop(Rect::new(60, 40, 20, 20).unwrap());
        assert!(
            flatten(&s, &d)
                .unwrap()
                .bgra
                .chunks_exact(4)
                .all(|p| p == [21, 43, 65, 255])
        );
        d.undo();
        assert_eq!(d.objects[0], hidden);
    }
    #[test]
    fn crop_changes_dimensions_and_clips_without_deleting_objects() {
        let s = source();
        let mut d = Document::new(100, 80);
        d.add(
            Tool::Line,
            vec![Point::new(20., 20.), Point::new(60., 20.)],
            Style::default(),
        );
        d.set_crop(Rect::new(30, 10, 20, 30).unwrap());
        let f = flatten(&s, &d).unwrap();
        assert_eq!((f.width, f.height), (20, 30));
        assert_eq!(pixel(&f, 5, 10), [0, 0, 255, 255]);
        assert_eq!(d.objects.len(), 1);
        d.undo();
        assert_eq!(flatten(&s, &d).unwrap().width, 100);
    }
    #[test]
    fn exported_dib_and_png_contain_composited_pixels() {
        let s = source();
        let mut d = Document::new(100, 80);
        d.add(
            Tool::Arrow,
            vec![Point::new(20., 20.), Point::new(60., 20.)],
            Style {
                rgb: [0, 255, 0],
                width: 5,
            },
        );
        let f = flatten(&s, &d).unwrap();
        assert_eq!(
            &f.dib()[40 + (59 * 100 + 30) * 4..44 + (59 * 100 + 30) * 4],
            [0, 255, 0, 255]
        );
        let mut bytes = Vec::new();
        f.write_png(&mut bytes).unwrap();
        let mut dec = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut out = vec![0; dec.output_buffer_size().unwrap()];
        dec.next_frame(&mut out).unwrap();
        assert_eq!(
            &out[(20 * 100 + 30) * 4..(20 * 100 + 30) * 4 + 4],
            [0, 255, 0, 255]
        );
    }
    #[test]
    fn antialiased_edges_do_not_change_uncovered_alpha() {
        let s = Arc::new(Frame::new(100, 80, [20, 40, 60, 77].repeat(8000)).unwrap());
        let mut d = Document::new(100, 80);
        d.add(
            Tool::Line,
            vec![Point::new(10., 10.), Point::new(70., 60.)],
            Style::default(),
        );
        let f = flatten(&s, &d).unwrap();
        assert_eq!(pixel(&f, 0, 0), [20, 40, 60, 77]);
        assert!(f.bgra.chunks_exact(4).any(|p| p[3] > 77));
    }

    #[test]
    fn live_stroke_has_soft_edges_and_matches_the_finished_stroke_at_native_scale() {
        let mut d = Document::new(100, 80);
        d.add(
            Tool::Line,
            vec![Point::new(10., 10.), Point::new(70., 60.)],
            Style::default(),
        );
        let view = crate::editor::View {
            crop: d.crop,
            x: 20.,
            y: 30.,
            scale: 1.,
        };
        let live = draft(&d.objects[0], view).expect("A live stroke must produce visible pixels.");
        assert!(
            live.rgba.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255),
            "Live stroke needs antialiased edges before release."
        );
        let final_image = flatten(&source(), &d).unwrap();
        for y in 0..live.bounds.height as usize {
            for x in 0..live.bounds.width as usize {
                let p = &live.rgba[(y * live.bounds.width as usize + x) * 4..][..4];
                let bg = [21u32, 43, 65];
                let actual: Vec<u8> = [2, 1, 0]
                    .into_iter()
                    .enumerate()
                    .map(|(i, c)| (p[c] as u32 + (bg[i] * (255 - p[3] as u32) + 127) / 255) as u8)
                    .collect();
                let fx = live.bounds.x as usize + x - 20;
                let fy = live.bounds.y as usize + y - 30;
                assert_eq!(actual, pixel(&final_image, fx, fy)[..3]);
            }
        }
    }

    #[test]
    fn live_arrow_includes_head_but_only_allocates_its_visible_tile() {
        let mut d = Document::new(4000, 3000);
        d.add(
            Tool::Arrow,
            vec![Point::new(100., 100.), Point::new(220., 180.)],
            Style::default(),
        );
        let live = draft(
            &d.objects[0],
            crate::editor::View {
                crop: d.crop,
                x: 0.,
                y: 0.,
                scale: 0.25,
            },
        )
        .unwrap();
        assert!(live.bounds.width < 50 && live.bounds.height < 40);
        assert!(live.rgba.chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
        assert_eq!(
            live.rgba.len(),
            live.bounds.width as usize * live.bounds.height as usize * 4
        );
    }
}

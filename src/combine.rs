use crate::frame::Rect;
use crate::{
    editor::{Annotation, Content, Document, Point, Style, Tool},
    frame::Frame,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Viewport {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}
impl From<Rect> for Viewport {
    fn from(r: Rect) -> Self {
        Self {
            left: r.x as f64,
            top: r.y as f64,
            right: r.right() as f64,
            bottom: r.bottom() as f64,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImageGroup {
    pub frame: Arc<Frame>,
    pub canvas: Rect,
    pub source_clip: Option<Rect>,
    pub viewport: Viewport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    Vertical,
    Horizontal,
    Free,
}

pub struct Arrangement {
    pub canvas: Rect,
    pub positions: Vec<(i32, i32)>,
}

#[derive(Clone)]
pub struct Source {
    pub owner: usize,
    pub identity: u64,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub checked: bool,
}
pub struct Request {
    pub sources: Vec<(usize, u64)>,
    pub layout: Layout,
    pub gap: u32,
    pub canvas: Rect,
}
pub fn request(sources: &[Source], layout: Layout, gap: &str) -> Result<Request, String> {
    let gap = if layout == Layout::Free {
        0
    } else {
        gap.trim()
            .parse::<u32>()
            .map_err(|_| "Gap must be a number from 0 to 500 px.")?
    };
    let checked: Vec<_> = sources.iter().filter(|s| s.checked).collect();
    let sizes: Vec<_> = checked.iter().map(|s| (s.width, s.height)).collect();
    let canvas = arrange(&sizes, layout, gap)?.canvas;
    let sources: Vec<_> = checked.iter().map(|s| (s.owner, s.identity)).collect();
    let unique: HashSet<_> = sources.iter().copied().collect();
    if sources.len() != unique.len() {
        return Err("An image may only be selected once.".into());
    }
    Ok(Request {
        sources,
        layout,
        gap,
        canvas,
    })
}

pub fn arrange(sizes: &[(u32, u32)], layout: Layout, gap: u32) -> Result<Arrangement, String> {
    if !(2..=16).contains(&sizes.len()) || gap > 500 {
        return Err("Choose 2-16 images and a gap from 0 to 500 px.".into());
    }
    let mut positions = Vec::with_capacity(sizes.len());
    let (mut x, mut y, mut width, mut height) = (0u32, 0u32, 0u32, 0u32);
    for (i, &(w, h)) in sizes.iter().enumerate() {
        Rect::new(0, 0, w, h)?;
        if matches!(layout, Layout::Free) {
            x = i as u32 * 24;
            y = x;
        }
        let right = x.checked_add(w).ok_or("Combined width overflow")?;
        let bottom = y.checked_add(h).ok_or("Combined height overflow")?;
        width = width.max(right);
        height = height.max(bottom);
        positions.push((x as i32, y as i32));
        match layout {
            Layout::Vertical => y = bottom.checked_add(gap).ok_or("Combined height overflow")?,
            Layout::Horizontal => x = right.checked_add(gap).ok_or("Combined width overflow")?,
            Layout::Free => {}
        }
    }
    let canvas = Rect::new(0, 0, width, height)?;
    if crate::frame::Frame::byte_len(width, height)? > crate::image_limits::IMAGE_LIMIT {
        return Err("Combined canvas exceeds the 128 MiB pixel limit.".into());
    }
    Ok(Arrangement { canvas, positions })
}

pub fn build(
    sources: &[(Arc<Frame>, Document)],
    layout: Layout,
    gap: u32,
) -> Result<crate::project::Project, String> {
    let sizes: Vec<_> = sources
        .iter()
        .map(|(_, d)| (d.crop.width, d.crop.height))
        .collect();
    let placement = arrange(&sizes, layout, gap)?;
    let mut objects = vec![];
    let mut next = 1u64;
    let mut seen = HashSet::new();
    let mut pixels = 4usize;
    for ((frame, document), &(x, y)) in sources.iter().zip(&placement.positions) {
        for asset in std::iter::once(frame.clone()).chain(document.image_sources()) {
            if seen.insert(Arc::as_ptr(&asset)) {
                pixels += asset.bgra.len();
            }
        }
        if pixels > crate::image_limits::SOURCE_LIMIT
            || objects.len() + document.objects.len() + 1 > 2000
        {
            return Err("Combined source images or object count exceed the limit.".into());
        }
        let mut remap = HashMap::new();
        let parent = next;
        next += 1;
        for o in &document.objects {
            if remap.insert(o.id, next).is_some() {
                return Err("Duplicate source object identity.".into());
            }
            next += 1;
        }
        objects.push(Annotation {
            id: parent,
            parent: None,
            tool: Tool::Image,
            points: Arc::new(vec![
                Point::new(x as f64, y as f64),
                Point::new(
                    (x + document.crop.width as i32) as f64,
                    (y + document.crop.height as i32) as f64,
                ),
            ]),
            style: Style::default(),
            pattern: Default::default(),
            rendering: Default::default(),
            opacity: 100,
            effects: Default::default(),
            content: Content::Group {
                image: Arc::new(ImageGroup {
                    frame: frame.clone(),
                    canvas: document.crop,
                    source_clip: document.source_clip,
                    viewport: document.crop.into(),
                }),
            },
        });
        for o in &document.objects {
            let mut imported = o.clone();
            imported.id = remap[&o.id];
            imported.parent = Some(match o.parent {
                Some(id) => *remap.get(&id).ok_or("Missing source owner")?,
                None => parent,
            });
            objects.push(imported);
        }
    }
    validate_tree(&objects)?;
    let mut document = Document::from_project(placement.canvas, None, objects)?;
    // A newly combined document is independent, not a saved source project.
    document.mark_project_saved(u64::MAX);
    let frame = Arc::new(Frame::new(1, 1, vec![255; 4])?);
    crate::project::validate(&frame, &document)?;
    Ok(crate::project::Project { frame, document })
}

pub fn validate_tree(objects: &[Annotation]) -> Result<(), String> {
    let by_id: HashMap<_, _> = objects.iter().map(|o| (o.id, o)).collect();
    if by_id.len() != objects.len() || objects.len() > 2000 {
        return Err("Duplicate identities or too many objects.".into());
    }
    for object in objects {
        if let Content::Group { image } = &object.content {
            let c = image.canvas;
            Rect::new(c.x, c.y, c.width, c.height)?;
            if Frame::byte_len(c.width, c.height)? > crate::image_limits::IMAGE_LIMIT
                || object.points.len() != 2
                || c.x.abs_diff(0) > 32767
                || c.y.abs_diff(0) > 32767
            {
                return Err("Invalid image group canvas or geometry.".into());
            }
            let v = image.viewport;
            let (a, b) = object.bounds();
            let sx = (b.x - a.x) / (v.right - v.left);
            let sy = (b.y - a.y) / (v.bottom - v.top);
            if [v.left, v.top, v.right, v.bottom, a.x, a.y, b.x, b.y]
                .iter()
                .any(|v| !v.is_finite())
                || v.left < c.x as f64
                || v.top < c.y as f64
                || v.right > c.right() as f64
                || v.bottom > c.bottom() as f64
                || v.right <= v.left
                || v.bottom <= v.top
                || b.x <= a.x
                || b.y <= a.y
                || !sx.is_finite()
                || !sy.is_finite()
                || sx <= 0.
                || sy <= 0.
                || (sx - sy).abs() > sx.max(sy) * 0.000001
                || !representable(image_transform(object, v))
            {
                return Err("Invalid image viewport or nonuniform transform.".into());
            }
            if let Some(clip) = image.source_clip {
                Rect::new(clip.x, clip.y, clip.width, clip.height)?;
                if clip.intersection(Rect::new(0, 0, image.frame.width, image.frame.height)?)
                    != Some(clip)
                {
                    return Err("Invalid group source clip.".into());
                }
            }
        }
        let mut current = object;
        let mut transform = Transform::default();
        let mut depth = usize::from(matches!(object.content, Content::Group { .. }));
        while let Some(parent) = current.parent {
            current = by_id.get(&parent).ok_or("Missing image owner")?;
            let Content::Group { image } = &current.content else {
                return Err("Object owner is not an image group.".into());
            };
            transform = image_transform(current, image.viewport).then(transform);
            if !representable(transform) {
                return Err("Image ownership transform exceeds renderer limits.".into());
            }
            depth += 1;
            if depth > 8 {
                return Err("Image group depth exceeds 8 or ownership contains a cycle.".into());
            }
        }
    }
    Ok(())
}

fn representable(t: Transform) -> bool {
    t.scale > 0.
        && (t.scale as f32) > 0.
        && [t.scale, t.x, t.y, 1. / t.scale]
            .iter()
            .all(|v| (*v as f32).is_finite())
}

impl Document {
    pub fn local_canvas(&self, id: u64) -> Rect {
        self.objects
            .iter()
            .find(|o| o.id == id)
            .and_then(|o| o.parent)
            .and_then(|id| self.objects.iter().find(|o| o.id == id))
            .and_then(|o| {
                if let Content::Group { image } = &o.content {
                    Some(image.canvas)
                } else {
                    None
                }
            })
            .unwrap_or(self.crop)
    }
    pub fn point_visible(&self, id: u64, point: Point) -> bool {
        let r = self.crop;
        if point.x < r.x as f64
            || point.y < r.y as f64
            || point.x > r.right() as f64
            || point.y > r.bottom() as f64
        {
            return false;
        }
        let mut parent = self
            .objects
            .iter()
            .find(|o| o.id == id)
            .and_then(|o| o.parent);
        for _ in 0..8 {
            let Some(o) = parent.and_then(|id| self.objects.iter().find(|o| o.id == id)) else {
                return true;
            };
            let p = self.parent_transform(o.id).inverse(point);
            let (a, b) = o.bounds();
            if p.x < a.x || p.y < a.y || p.x > b.x || p.y > b.y {
                return false;
            }
            parent = o.parent;
        }
        parent.is_none()
    }
    pub fn select_owner(&mut self) -> bool {
        let parent = self
            .objects
            .iter()
            .find(|o| Some(o.id) == self.selected)
            .and_then(|o| o.parent);
        if let Some(parent) = parent {
            self.selected = Some(parent);
            true
        } else {
            false
        }
    }
    pub fn crop_image(&mut self, id: u64, crop: Rect) -> bool {
        let Some(i) = self
            .objects
            .iter()
            .position(|o| o.id == id && o.tool == Tool::Image)
        else {
            return false;
        };
        let o = &self.objects[i];
        let parent = self.parent_transform(id);
        let (a, b) = o.bounds();
        let left = parent.inverse(Point::new(crop.x as f64, crop.y as f64));
        let right = parent.inverse(Point::new(crop.right() as f64, crop.bottom() as f64));
        let p = Point::new(a.x.max(left.x), a.y.max(left.y));
        let q = Point::new(b.x.min(right.x), b.y.min(right.y));
        if q.x <= p.x || q.y <= p.y || (p == a && q == b) {
            return false;
        }
        let mut image = match &o.content {
            Content::Group { image } => (**image).clone(),
            Content::Image { frame } => {
                let canvas = Rect::new(0, 0, frame.width, frame.height).unwrap();
                ImageGroup {
                    frame: frame.clone(),
                    canvas,
                    source_clip: Some(canvas),
                    viewport: canvas.into(),
                }
            }
            _ => return false,
        };
        let t = image_transform(o, image.viewport);
        let start = t.inverse(p);
        let end = t.inverse(q);
        image.viewport = Viewport {
            left: start.x,
            top: start.y,
            right: end.x,
            bottom: end.y,
        };
        self.begin_edit();
        self.objects[i].content = Content::Group {
            image: Arc::new(image),
        };
        self.objects[i].points = Arc::new(vec![p, q]);
        true
    }
    pub fn parent_transform(&self, id: u64) -> Transform {
        let mut t = Transform::default();
        let mut parent = self
            .objects
            .iter()
            .find(|o| o.id == id)
            .and_then(|o| o.parent);
        for _ in 0..8 {
            let Some(owner) = parent.and_then(|id| self.objects.iter().find(|o| o.id == id)) else {
                break;
            };
            let Content::Group { image } = &owner.content else {
                break;
            };
            t = image_transform(owner, image.viewport).then(t);
            parent = owner.parent;
        }
        t
    }
    pub fn is_descendant(&self, id: u64, ancestor: u64) -> bool {
        let mut parent = self
            .objects
            .iter()
            .find(|o| o.id == id)
            .and_then(|o| o.parent);
        for _ in 0..8 {
            if parent == Some(ancestor) {
                return true;
            }
            parent = parent
                .and_then(|id| self.objects.iter().find(|o| o.id == id))
                .and_then(|o| o.parent);
        }
        false
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub scale: f64,
    pub x: f64,
    pub y: f64,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            scale: 1.,
            x: 0.,
            y: 0.,
        }
    }
}
impl Transform {
    pub fn inverse(self, p: Point) -> Point {
        Point::new((p.x - self.x) / self.scale, (p.y - self.y) / self.scale)
    }
    pub fn point(self, p: Point) -> Point {
        Point::new(p.x * self.scale + self.x, p.y * self.scale + self.y)
    }
    fn then(self, inner: Self) -> Self {
        Self {
            scale: self.scale * inner.scale,
            x: self.x + self.scale * inner.x,
            y: self.y + self.scale * inner.y,
        }
    }
}
pub fn image_transform(o: &Annotation, v: Viewport) -> Transform {
    let (a, b) = o.bounds();
    let scale = (b.x - a.x) / (v.right - v.left);
    Transform {
        scale,
        x: a.x - v.left * scale,
        y: a.y - v.top * scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        editor::{Document, Point, Style, Tool},
        frame::Frame,
    };
    use std::sync::Arc;
    #[test]
    fn dialog_request_uses_checked_sources_in_display_order_and_validates_gap() {
        let sources = vec![
            Source {
                owner: 20,
                identity: 2,
                title: "B".into(),
                width: 60,
                height: 40,
                checked: true,
            },
            Source {
                owner: 30,
                identity: 3,
                title: "C".into(),
                width: 600,
                height: 400,
                checked: false,
            },
            Source {
                owner: 10,
                identity: 1,
                title: "A".into(),
                width: 100,
                height: 80,
                checked: true,
            },
        ];
        let r = request(&sources, Layout::Vertical, "16").unwrap();
        assert_eq!(r.sources, vec![(20, 2), (10, 1)]);
        assert_eq!((r.canvas.width, r.canvas.height), (100, 136));
        assert!(request(&sources, Layout::Vertical, "-1").is_err());
        assert!(request(&sources, Layout::Horizontal, "501").is_err());
        let r = request(&sources, Layout::Free, "not used").unwrap();
        assert_eq!((r.canvas.width, r.canvas.height), (124, 104));
        assert_eq!(r.gap, 0);
        assert!(request(&sources[..1], Layout::Vertical, "16").is_err());
    }

    fn pair() -> crate::project::Project {
        let frame = Arc::new(Frame::new(100, 80, vec![255; 32000]).unwrap());
        let mut source = Document::new(100, 80);
        source.add(
            Tool::Line,
            vec![Point::new(10., 10.), Point::new(80., 10.)],
            Style::default(),
        );
        source.add(
            Tool::Rectangle,
            vec![Point::new(20., 20.), Point::new(40., 40.)],
            Style::default(),
        );
        build(
            &[(frame.clone(), source.clone()), (frame, source)],
            Layout::Vertical,
            16,
        )
        .unwrap()
    }

    #[test]
    fn child_hit_follows_owner_move_and_parent_selection_without_mutating_local_points() {
        let mut d = pair().document;
        let parent = d.objects[3].id;
        let child = d.objects[4].id;
        let points = d.objects[4].points.clone();
        assert_eq!(d.hit(Point::new(50., 106.), 1.), Some(child));
        d.selected = Some(child);
        assert!(d.select_owner());
        assert_eq!(d.selected, Some(parent));
        assert!(d.move_selected(50., 10.));
        assert_eq!(d.objects[4].points, points);
        assert_eq!(d.hit(Point::new(100., 116.), 1.), Some(child));
        assert!(d.undo());
        assert_eq!(d.hit(Point::new(50., 106.), 1.), Some(child));
    }

    #[test]
    fn deleting_image_removes_owned_objects_in_one_undo_without_touching_other_image() {
        let mut d = pair().document;
        let original = d.objects.clone();
        d.selected = Some(d.objects[0].id);
        assert!(d.delete());
        assert_eq!(d.objects.len(), 3);
        assert!(d.undo());
        assert_eq!(d.objects, original);
        assert!(!d.can_undo());
        assert!(d.redo());
        assert_eq!(d.objects.len(), 3);
    }

    #[test]
    fn child_layer_order_stays_with_its_siblings() {
        let mut d = pair().document;
        let line = d.objects[1].id;
        d.selected = Some(line);
        assert!(d.reorder(crate::editor::Order::Front));
        assert_eq!(d.objects[2].id, line);
        assert!(!d.can_reorder(crate::editor::Order::Front));
        assert!(d.undo());
        assert_eq!(d.objects[1].id, line);
    }

    #[test]
    fn image_crop_changes_viewport_not_canvas_children_or_original_pixels() {
        let mut d = pair().document;
        let original = d.objects.clone();
        let canvas = d.crop;
        let id = d.objects[3].id;
        assert!(d.crop_image(id, Rect::new(10, 106, 60, 40).unwrap()));
        assert_eq!(d.crop, canvas);
        assert_eq!(d.objects[4], original[4]);
        assert_eq!(
            d.objects[3].bounds(),
            (Point::new(10., 106.), Point::new(70., 146.))
        );
        let Content::Group { image } = &d.objects[3].content else {
            panic!()
        };
        assert_eq!(
            image.viewport,
            Viewport {
                left: 10.,
                top: 10.,
                right: 70.,
                bottom: 50.
            }
        );
        assert!(Arc::ptr_eq(
            &image.frame,
            &match &original[3].content {
                Content::Group { image } => image.frame.clone(),
                _ => panic!(),
            }
        ));
        d.selected = Some(id);
        assert!(d.resize_image((Point::new(10., 106.), Point::new(130., 186.))));
        assert_eq!(d.objects[4], original[4]);
        assert!(d.undo());
        assert!(d.undo());
        assert_eq!(d.objects, original);
    }

    #[test]
    fn identity_combination_matches_source_crop_pixels_and_white_gap() {
        let frame = Arc::new(Frame::new(40, 30, [30, 60, 90, 255].repeat(1200)).unwrap());
        let mut d = Document::new(40, 30);
        d.add(
            Tool::Line,
            vec![Point::new(5., 10.), Point::new(30., 20.)],
            Style::default(),
        );
        d.set_crop(Rect::new(4, 3, 30, 24).unwrap());
        let expected = crate::render::flatten(&frame, &d).unwrap();
        let combined = build(
            &[(frame.clone(), d.clone()), (frame, d)],
            Layout::Vertical,
            2,
        )
        .unwrap();
        let output = crate::render::flatten(&combined.frame, &combined.document).unwrap();
        assert_eq!((output.width, output.height), (30, 50));
        assert_eq!(&output.bgra[..30 * 24 * 4], &expected.bgra);
        assert_eq!(&output.bgra[30 * 24 * 4..30 * 26 * 4], &[255; 240]);
        assert_eq!(&output.bgra[30 * 26 * 4..], &expected.bgra);
    }

    #[test]
    fn imported_shape_handles_resize_in_local_units_after_owner_scaling() {
        let mut s = crate::interaction::Session::new(100, 176);
        s.document = pair().document;
        let owner = s.document.objects[3].id;
        let child = s.document.objects[5].id;
        s.document.selected = Some(owner);
        assert!(
            s.document
                .resize_image((Point::new(0., 96.), Point::new(200., 256.)))
        );
        s.document.selected = Some(child);
        let positions: Vec<_> = s.shape_handles().into_iter().map(|(_, p)| p).collect();
        assert!(positions.contains(&Point::new(40., 136.)));
        s.press(Point::new(80., 176.), 2.);
        assert!(s.release(Point::new(100., 196.)));
        assert_eq!(
            s.document.objects[5].bounds(),
            (Point::new(20., 20.), Point::new(50., 50.))
        );
        assert!(s.document.undo());
        assert_eq!(
            s.document.objects[5].bounds(),
            (Point::new(20., 20.), Point::new(40., 40.))
        );
    }

    #[test]
    fn crop_tool_remembers_selected_image_target_and_cancel_keeps_it_unchanged() {
        let mut s = crate::interaction::Session::new(100, 176);
        s.document = pair().document;
        let canvas = s.document.crop;
        let owner = s.document.objects[3].id;
        s.document.selected = Some(owner);
        s.choose(Tool::Crop);
        s.press(Point::new(10., 106.), 1.);
        s.release(Point::new(70., 146.));
        s.cancel();
        assert_eq!(
            s.document.objects[3].bounds(),
            (Point::new(0., 96.), Point::new(100., 176.))
        );
        s.press(Point::new(10., 106.), 1.);
        s.release(Point::new(70., 146.));
        assert!(s.confirm_crop());
        assert_eq!(s.document.crop, canvas);
        assert_eq!(
            s.document.objects[3].bounds(),
            (Point::new(10., 106.), Point::new(70., 146.))
        );
    }

    #[test]
    fn owned_curve_bends_at_its_visible_midpoint_and_undo_keeps_owner_unchanged() {
        let mut s = crate::interaction::Session::new(100, 176);
        s.document = pair().document;
        let id = s.document.objects[4].id;
        let owner = s.document.objects[3].clone();
        s.document.selected = Some(id);
        assert_eq!(s.curve_handle_position(), Some(Point::new(45., 106.)));
        s.press(Point::new(45., 106.), 2.);
        assert!(s.release(Point::new(45., 126.)));
        assert_eq!(s.document.objects[4].points[2], Point::new(45., 50.));
        assert_eq!(s.document.objects[3], owner);
        assert!(s.document.undo());
        assert_eq!(s.document.objects[4].points.len(), 2);
    }

    #[test]
    fn nested_ownership_depth_is_bounded_and_hidden_children_cannot_be_hit() {
        let mut p = pair();
        let original = p.document.objects.clone();
        assert!(
            p.document
                .crop_image(original[3].id, Rect::new(10, 126, 40, 40).unwrap())
        );
        assert_eq!(p.document.hit(Point::new(50., 106.), 3.), None);
        assert!(
            !p.document
                .point_visible(original[4].id, Point::new(50., 106.))
        );
        let frame = Arc::new(Frame::new(1, 1, vec![255; 4]).unwrap());
        for _ in 1..8 {
            p = build(
                &[(p.frame, p.document), (frame.clone(), Document::new(1, 1))],
                Layout::Horizontal,
                0,
            )
            .unwrap();
        }
        assert!(
            build(
                &[(p.frame, p.document), (frame, Document::new(1, 1))],
                Layout::Horizontal,
                0
            )
            .is_err()
        );
    }

    #[test]
    fn group_opacity_applies_once_and_local_blur_does_not_sample_other_images() {
        let frame = Arc::new(Frame::new(40, 40, [255, 0, 0, 255].repeat(1600)).unwrap());
        let mut source = Document::new(40, 40);
        source.add(
            Tool::FilledRectangle,
            vec![Point::new(0., 0.), Point::new(30., 30.)],
            Style::default(),
        );
        source.add_blur(Point::new(0., 0.), Point::new(40., 40.), 8);
        let plain = Document::new(40, 40);
        let red = Arc::new(Frame::new(40, 40, [0, 0, 255, 255].repeat(1600)).unwrap());
        let mut p = build(
            &[(red, plain), (frame.clone(), source.clone())],
            Layout::Horizontal,
            0,
        )
        .unwrap();
        p.document.selected = Some(p.document.objects[1].id);
        assert!(p.document.move_selected(-40., 0.));
        let expected = crate::render::flatten(&frame, &source).unwrap();
        let output = crate::render::flatten(&p.frame, &p.document).unwrap();
        for y in 0..40 {
            let i = y * 80 * 4;
            assert_eq!(
                &output.bgra[i..i + 160],
                &expected.bgra[y * 160..(y + 1) * 160]
            );
        }
        assert!(p.document.set_opacity(50));
        let output = crate::render::flatten(&p.frame, &p.document).unwrap();
        let i = (35 * 80 + 35) * 4;
        let e = (35 * 40 + 35) * 4;
        assert!((output.bgra[i] as i32 - expected.bgra[e] as i32 / 2).abs() <= 1);
        assert!((output.bgra[i + 2] as i32 - (127 + expected.bgra[e + 2] as i32 / 2)).abs() <= 1);
    }

    #[test]
    fn new_stroke_cannot_exceed_total_point_budget_or_create_history_on_failure() {
        let mut d = Document::new(100, 80);
        let points: Vec<_> = (0..100_000)
            .map(|i| Point::new((i % 100) as f64, (i % 80) as f64))
            .collect();
        for _ in 0..10 {
            assert!(d.add(Tool::Pen, points.clone(), Style::default()));
        }
        let revision = d.revision;
        assert!(!d.add(
            Tool::Pen,
            vec![Point::new(0., 0.), Point::new(20., 20.)],
            Style::default()
        ));
        assert_eq!(d.revision, revision);
        assert_eq!(d.objects.len(), 10);
    }

    #[test]
    fn combining_keeps_sources_independent_and_starts_without_history() {
        let frame = Arc::new(Frame::new(100, 80, vec![255; 32000]).unwrap());
        let mut source = Document::new(100, 80);
        source.add(
            Tool::Line,
            vec![Point::new(10., 10.), Point::new(80., 60.)],
            Style::default(),
        );
        source.mark_project_saved(source.revision);
        let before = source.objects.clone();
        let result = build(
            &[
                (frame.clone(), source.clone()),
                (frame.clone(), source.clone()),
            ],
            Layout::Vertical,
            16,
        )
        .unwrap();
        let mut d = result.document;
        assert_eq!((d.crop.width, d.crop.height), (100, 176));
        assert_eq!(
            d.objects.len(),
            4,
            "two editable images and their original lines"
        );
        assert_eq!((result.frame.width, result.frame.height), (1, 1));
        assert_eq!(d.selected, None);
        assert!(!d.can_undo());
        assert!(!d.can_redo());
        assert!(!d.is_project_saved());
        let ids: std::collections::HashSet<_> = d.objects.iter().map(|o| o.id).collect();
        assert_eq!(ids.len(), 4);
        let child = d.objects.iter().find(|o| o.tool == Tool::Line).unwrap().id;
        d.selected = Some(child);
        assert!(d.move_selected(5., 0.));
        assert_eq!(source.objects, before);
        assert!(source.is_project_saved());
        assert!(source.can_undo());
        assert!(d.undo());
        let assets = d.image_sources();
        assert_eq!(assets.len(), 1);
        assert!(Arc::ptr_eq(&assets[0], &frame));
    }

    #[test]
    fn native_layouts_preserve_size_order_and_have_no_trailing_gap() {
        let sizes = [(100, 80), (60, 120), (90, 40)];
        for (mode, dimensions, positions) in [
            (
                Layout::Vertical,
                (100, 272),
                vec![(0, 0), (0, 96), (0, 232)],
            ),
            (
                Layout::Horizontal,
                (282, 120),
                vec![(0, 0), (116, 0), (192, 0)],
            ),
            (Layout::Free, (138, 144), vec![(0, 0), (24, 24), (48, 48)]),
        ] {
            let result = arrange(&sizes, mode, 16).unwrap();
            assert_eq!((result.canvas.width, result.canvas.height), dimensions);
            assert_eq!(result.positions, positions);
        }
    }

    #[test]
    fn invalid_or_excessive_combination_is_rejected_before_allocation() {
        for sizes in [
            vec![],
            vec![(10, 10)],
            vec![(0, 10), (20, 10)],
            vec![(32767, 1025), (1, 1)],
        ] {
            assert!(arrange(&sizes, Layout::Vertical, 16).is_err());
        }
        assert!(arrange(&[(20, 10), (20, 10)], Layout::Horizontal, 501).is_err());
        let zero = arrange(&[(20, 10), (20, 10)], Layout::Vertical, 0).unwrap();
        assert_eq!((zero.canvas.width, zero.canvas.height), (20, 20));
    }
}

use crate::frame::Rect;
use std::sync::Arc;

#[cfg(test)]
mod project_state_tests {
    use super::*;

    #[test]
    fn png_kept_does_not_save_editable_project_and_undo_restores_saved_state() {
        let mut d = Document::new(100, 80);
        d.mark_kept(0);
        assert!(!d.is_project_saved());
        d.mark_project_saved(0);
        d.add(
            Tool::Line,
            vec![Point::new(1., 2.), Point::new(20., 30.)],
            Style::default(),
        );
        assert!(!d.is_project_saved());
        assert!(d.undo());
        assert!(d.is_project_saved());
        assert!(d.redo());
        assert!(!d.is_project_saved());
    }

    #[test]
    fn reopening_clears_history_selection_and_uses_fresh_noncolliding_ids() {
        let mut d = Document::new(100, 80);
        d.add(
            Tool::Line,
            vec![Point::new(1., 2.), Point::new(20., 30.)],
            Style::default(),
        );
        d.objects[0].id = 700;
        let mut reopened =
            Document::from_project(d.crop, d.source_clip, d.objects.clone()).unwrap();
        assert_eq!(reopened.objects, d.objects);
        assert_eq!(reopened.selected, None);
        assert!(!reopened.undo());
        assert!(!reopened.redo());
        assert!(reopened.is_project_saved());
        assert!(!reopened.is_kept());
        reopened.add(
            Tool::Rectangle,
            vec![Point::new(3., 4.), Point::new(12., 14.)],
            Style::default(),
        );
        assert!(reopened.objects[1].id > 700);
        assert!(!reopened.is_project_saved());
        assert!(reopened.undo());
        assert!(reopened.is_project_saved());
    }
}

#[cfg(test)]
mod effect_tests {
    use super::*;
    use crate::effects::Effects;
    #[test]
    fn effects_apply_atomically_and_survive_geometry_history() {
        let mut d = Document::new(100, 80);
        d.add(
            Tool::FilledRectangle,
            vec![Point::new(10., 10.), Point::new(40., 40.)],
            Style::default(),
        );
        let initial = d.objects[0].clone();
        let mut effects = Effects::default();
        effects.shadow.enabled = true;
        effects.shadow.x = -5;
        assert!(d.set_effects(effects).unwrap());
        let edited = d.objects[0].clone();
        assert!(!d.set_effects(effects).unwrap());
        effects.shadow.blur = 33;
        assert!(d.set_effects(effects).is_err());
        assert_eq!(d.objects[0], edited);
        assert!(d.undo());
        assert_eq!(d.objects[0], initial);
        assert!(d.redo());
        assert_eq!(d.objects[0], edited);
        assert!(d.resize_shape((Point::new(10., 10.), Point::new(50., 50.))));
        assert_eq!(d.objects[0].effects, edited.effects);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tool {
    Select,
    Crop,
    Pen,
    Highlighter,
    Line,
    Arrow,
    Rectangle,
    Ellipse,
    FilledRectangle,
    FilledEllipse,
    Text,
    Callout,
    Step,
    Blur,
    Image,
}
impl Tool {
    pub fn opacity_supported(self) -> bool {
        !matches!(self, Self::Select | Self::Crop)
    }
    pub fn patterned(self) -> bool {
        matches!(
            self,
            Self::Pen | Self::Line | Self::Arrow | Self::Rectangle | Self::Ellipse
        )
    }
    pub fn filled(self) -> bool {
        matches!(self, Self::FilledRectangle | Self::FilledEllipse)
    }
}

pub fn dotnet_step_radius(number: u32, source_size: u32) -> f64 {
    16f64
        .max(source_size as f64 * 3.)
        .max(number.to_string().len() as f64 * source_size as f64 * 1.6)
}

pub fn dotnet_step_diameter(number: u32, source_size: u32) -> u32 {
    (dotnet_step_radius(number, source_size) * 2.).ceil() as u32
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Pattern {
    #[default]
    Solid,
    Dashed,
    Dotted,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PathRendering {
    #[default]
    Native,
    DotNet,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Front,
    Forward,
    Backward,
    Back,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    pub rgb: [u8; 3],
    pub width: u32,
}
impl Default for Style {
    fn default() -> Self {
        Self {
            rgb: [255, 0, 0],
            width: 5,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    pub id: u64,
    pub parent: Option<u64>,
    pub tool: Tool,
    pub points: Arc<Vec<Point>>,
    pub style: Style,
    pub pattern: Pattern,
    pub rendering: PathRendering,
    pub content: Content,
    pub opacity: u8,
    pub effects: crate::effects::Effects,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Background {
    pub enabled: bool,
    pub rgb: [u8; 3],
}
impl Default for Background {
    fn default() -> Self {
        Self {
            enabled: false,
            rgb: [255, 255, 255],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BubbleFrame {
    pub width: f64,
    pub min_height: f64,
    pub padding: f64,
    pub border_rgb: [u8; 3],
    pub border_width: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TextLayout {
    #[default]
    Native,
    DotNet {
        source_size: f64,
        bubble: Option<BubbleFrame>,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    Group {
        image: Arc<crate::combine::ImageGroup>,
    },
    Image {
        frame: Arc<crate::frame::Frame>,
    },
    Shape,
    Highlight,
    Text {
        text: Arc<str>,
        size: u32,
        wrap_width: Option<u32>,
        mask: Arc<crate::glyphs::Mask>,
        background: Background,
        tail: Option<Point>,
        layout: TextLayout,
    },
    Step {
        number: u32,
        diameter: u32,
        mask: Arc<crate::glyphs::Mask>,
    },
    Blur {
        strength: u32,
    },
}
impl Annotation {
    pub fn curve_midpoint(&self) -> Option<Point> {
        matches!(self.tool, Tool::Line | Tool::Arrow)
            .then(|| crate::geometry::curve_point(self, 0.5))
    }
    pub fn tail_tip(&self) -> Option<Point> {
        if let Content::Text {
            tail: Some(tip), ..
        } = self.content
        {
            let mut tip = Point::new(self.points[0].x + tip.x, self.points[0].y + tip.y);
            if matches!(
                self.content,
                Content::Text {
                    layout: TextLayout::DotNet {
                        bubble: Some(_),
                        ..
                    },
                    ..
                }
            ) {
                let (a, b) = self.body_bounds();
                let outside = [
                    (a.x - tip.x) / (b.x - a.x),
                    (tip.x - b.x) / (b.x - a.x),
                    (a.y - tip.y) / (b.y - a.y),
                    (tip.y - b.y) / (b.y - a.y),
                ];
                if outside.into_iter().fold(f64::NEG_INFINITY, f64::max) < 0. {
                    let distances = [tip.x - a.x, b.x - tip.x, tip.y - a.y, b.y - tip.y];
                    let edge = distances
                        .iter()
                        .enumerate()
                        .min_by(|a, b| a.1.total_cmp(b.1))
                        .map(|(i, _)| i)
                        .unwrap();
                    match edge {
                        0 => tip.x = a.x - 16.,
                        1 => tip.x = b.x + 16.,
                        2 => tip.y = a.y - 16.,
                        _ => tip.y = b.y + 16.,
                    }
                }
            }
            Some(tip)
        } else {
            None
        }
    }
    pub fn text_background(&self) -> Option<Background> {
        if let Content::Text { background, .. } = self.content {
            Some(background)
        } else {
            None
        }
    }
    pub fn text_origin(&self) -> Point {
        let p = self.points[0];
        if let Content::Text {
            layout:
                TextLayout::DotNet {
                    bubble: Some(frame),
                    ..
                },
            ..
        } = self.content
        {
            Point::new(p.x + frame.padding, p.y + frame.padding)
        } else {
            p
        }
    }
    pub fn callout_border(&self) -> Style {
        if let Content::Text {
            layout:
                TextLayout::DotNet {
                    bubble: Some(frame),
                    ..
                },
            ..
        } = self.content
        {
            Style {
                rgb: frame.border_rgb,
                width: frame.border_width.round().max(1.) as u32,
            }
        } else {
            self.style
        }
    }
    pub fn body_bounds(&self) -> (Point, Point) {
        if let Content::Text {
            layout:
                TextLayout::DotNet {
                    bubble: Some(frame),
                    ..
                },
            mask,
            ..
        } = &self.content
        {
            let p = self.points[0];
            let height = frame
                .min_height
                .max(mask.height as f64 + frame.padding * 2.);
            return (p, Point::new(p.x + frame.width, p.y + height));
        }
        let (a, b) = self.bounds();
        if self.tool == Tool::Callout {
            (
                Point::new(a.x - 8., a.y - 8.),
                Point::new(b.x + 8., b.y + 8.),
            )
        } else {
            (a, b)
        }
    }
    pub fn bounds(&self) -> (Point, Point) {
        if matches!(self.tool, Tool::Line | Tool::Arrow) {
            let (mut a, mut b) = if self.points.len() == 3 {
                crate::geometry::curve_bounds(self)
            } else {
                let p = self.points[0];
                let q = self.points[1];
                (
                    Point::new(p.x.min(q.x), p.y.min(q.y)),
                    Point::new(p.x.max(q.x), p.y.max(q.y)),
                )
            };
            if self.tool == Tool::Arrow {
                for (_, p) in crate::geometry::arrowhead(self) {
                    a.x = a.x.min(p.x);
                    a.y = a.y.min(p.y);
                    b.x = b.x.max(p.x);
                    b.y = b.y.max(p.y);
                }
            }
            return (a, b);
        }
        let p = self.points[0];
        match &self.content {
            Content::Text {
                mask: _,
                layout:
                    TextLayout::DotNet {
                        bubble: Some(_), ..
                    },
                ..
            } => self.body_bounds(),
            Content::Text { mask, .. } => (
                p,
                Point::new(p.x + mask.width as f64, p.y + mask.height as f64),
            ),
            Content::Step { diameter, .. } => {
                let r = *diameter as f64 / 2.;
                (Point::new(p.x - r, p.y - r), Point::new(p.x + r, p.y + r))
            }
            _ => (
                Point::new(
                    self.points
                        .iter()
                        .map(|p| p.x)
                        .fold(f64::INFINITY, f64::min),
                    self.points
                        .iter()
                        .map(|p| p.y)
                        .fold(f64::INFINITY, f64::min),
                ),
                Point::new(
                    self.points
                        .iter()
                        .map(|p| p.x)
                        .fold(f64::NEG_INFINITY, f64::max),
                    self.points
                        .iter()
                        .map(|p| p.y)
                        .fold(f64::NEG_INFINITY, f64::max),
                ),
            ),
        }
    }
    pub fn parameter(&self) -> u32 {
        match self.content {
            Content::Image { .. } | Content::Group { .. } => 0,
            Content::Shape | Content::Highlight => self.style.width,
            Content::Text { size, .. } => size,
            Content::Step { diameter, .. } => {
                if self.rendering == PathRendering::DotNet {
                    self.style.width
                } else {
                    diameter
                }
            }
            Content::Blur { strength } => strength,
        }
    }
}
#[derive(Clone)]
struct Snapshot {
    crop: Rect,
    source_clip: Option<Rect>,
    objects: Vec<Annotation>,
    revision: u64,
}
#[derive(Clone)]
pub struct Document {
    pub crop: Rect,
    pub source_clip: Option<Rect>,
    pub objects: Vec<Annotation>,
    pub selected: Option<u64>,
    pub revision: u64,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    next: u64,
    kept: Option<u64>,
    project_saved: Option<u64>,
}
fn source_budget_allows(existing: usize, incoming: usize, shared: bool) -> bool {
    existing
        .checked_add(if shared { 0 } else { incoming })
        .is_some_and(|total| total <= crate::image_limits::SOURCE_LIMIT)
}
impl Document {
    pub fn add_image(&mut self, frame: Arc<crate::frame::Frame>) -> Result<bool, String> {
        let sources = self.image_sources();
        let bytes: usize = sources.iter().map(|f| f.bgra.len()).sum();
        if !source_budget_allows(
            bytes,
            frame.bgra.len(),
            sources.iter().any(|f| Arc::ptr_eq(f, &frame)),
        ) {
            return Err(
                "Image memory limit reached. Close an older editor or use smaller images.".into(),
            );
        }
        let width = self.crop.width.max(frame.width);
        let height = self.crop.height.max(frame.height);
        let crop = self.expansion(width, height)?;
        let a = Point::new(self.crop.x as f64, self.crop.y as f64);
        let b = Point::new(a.x + frame.width as f64, a.y + frame.height as f64);
        if !self.insert(
            Tool::Image,
            vec![a, b],
            Style::default(),
            Content::Image { frame },
        ) {
            return Err("Cannot add another image to this document.".into());
        }
        self.crop = crop;
        Ok(true)
    }
    fn expansion(&self, width: u32, height: u32) -> Result<Rect, String> {
        if width < self.crop.width || height < self.crop.height {
            return Err("Canvas can only expand. Use Crop to make it smaller.".into());
        }
        if crate::frame::Frame::byte_len(width, height)? > crate::image_limits::IMAGE_LIMIT {
            return Err("Canvas exceeds the 128 MiB pixel limit.".into());
        }
        let rect = Rect::new(self.crop.x, self.crop.y, width, height)?;
        if rect.right() > 32767 || rect.bottom() > 32767 {
            return Err("Canvas is outside the supported coordinates.".into());
        }
        Ok(rect)
    }
    pub fn expand_canvas(&mut self, width: u32, height: u32) -> Result<bool, String> {
        let crop = self.expansion(width, height)?;
        if crop == self.crop {
            return Ok(false);
        }
        self.begin_edit();
        self.crop = crop;
        Ok(true)
    }
    pub fn image_sources(&self) -> Vec<Arc<crate::frame::Frame>> {
        let mut seen = std::collections::HashSet::new();
        self.objects
            .iter()
            .chain(self.undo.iter().chain(&self.redo).flat_map(|s| &s.objects))
            .filter_map(|o| match &o.content {
                Content::Image { frame } if seen.insert(Arc::as_ptr(frame)) => Some(frame.clone()),
                Content::Group { image } if seen.insert(Arc::as_ptr(&image.frame)) => {
                    Some(image.frame.clone())
                }
                _ => None,
            })
            .collect()
    }
    pub fn resize_image(&mut self, bounds: (Point, Point)) -> bool {
        let Some(i) = self
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected)
        else {
            return false;
        };
        let (native_width, native_height) = match &self.objects[i].content {
            Content::Image { frame } => (frame.width as f64, frame.height as f64),
            Content::Group { image } => (
                image.viewport.right - image.viewport.left,
                image.viewport.bottom - image.viewport.top,
            ),
            _ => return false,
        };
        let (a, b) = bounds;
        let width = b.x - a.x;
        let height = b.y - a.y;
        if [a.x, a.y, b.x, b.y]
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 32767.)
            || width < 1.
            || height < 1.
            || width > 32767.
            || height > 32767.
            || (width / native_width - height / native_height).abs() > 0.00001
            || bounds == self.objects[i].bounds()
        {
            return false;
        }
        self.begin_edit();
        self.objects[i].points = Arc::new(vec![a, b]);
        true
    }
    pub fn resize_shape(&mut self, bounds: (Point, Point)) -> bool {
        let Some(i) = self
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected && crate::shape_frame::supported(o.tool))
        else {
            return false;
        };
        if !crate::shape_frame::valid_bounds(bounds) || self.objects[i].bounds() == bounds {
            return false;
        }
        self.begin_edit();
        self.objects[i].points = Arc::new(vec![bounds.0, bounds.1]);
        true
    }
    fn order_target(&self, order: Order) -> Option<(usize, usize)> {
        let from = self
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected)?;
        let siblings: Vec<_> = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.parent == self.objects[from].parent)
            .map(|(i, _)| i)
            .collect();
        let position = siblings.iter().position(|i| *i == from)?;
        let to = match order {
            Order::Front => *siblings.last()?,
            Order::Forward => siblings[(position + 1).min(siblings.len() - 1)],
            Order::Backward => siblings[position.saturating_sub(1)],
            Order::Back => siblings[0],
        };
        (from != to).then_some((from, to))
    }
    pub fn can_reorder(&self, order: Order) -> bool {
        self.order_target(order).is_some()
    }
    pub fn reorder(&mut self, order: Order) -> bool {
        let Some((from, to)) = self.order_target(order) else {
            return false;
        };
        self.begin_edit();
        let object = self.objects.remove(from);
        self.objects.insert(to, object);
        true
    }
    pub fn set_text_background(&mut self, background: Background) -> bool {
        let Some(i) = self.objects.iter().position(|o| {
            Some(o.id) == self.selected && o.text_background().is_some_and(|b| b != background)
        }) else {
            return false;
        };
        self.begin_edit();
        if let Content::Text { background: b, .. } = &mut self.objects[i].content {
            *b = background;
        }
        true
    }
    pub fn set_callout_border(&mut self, style: Style) -> bool {
        if !(1..=16).contains(&style.width) {
            return false;
        }
        let Some(i) = self.objects.iter().position(|o| {
            Some(o.id) == self.selected
                && o.tool == Tool::Callout
                && matches!(
                    o.content,
                    Content::Text {
                        layout: TextLayout::DotNet {
                            bubble: Some(_),
                            ..
                        },
                        ..
                    }
                )
                && o.callout_border() != style
        }) else {
            return false;
        };
        self.begin_edit();
        if let Content::Text {
            layout:
                TextLayout::DotNet {
                    bubble: Some(frame),
                    ..
                },
            ..
        } = &mut self.objects[i].content
        {
            frame.border_rgb = style.rgb;
            frame.border_width = style.width as f64;
        }
        true
    }
    pub fn set_tail(&mut self, tip: Point) -> bool {
        if !tip.x.is_finite() || !tip.y.is_finite() || tip.x.abs() > 32767. || tip.y.abs() > 32767.
        {
            return false;
        }
        let Some(i) = self
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected && o.tool == Tool::Callout)
        else {
            return false;
        };
        let p = self.objects[i].points[0];
        let relative = Point::new(tip.x - p.x, tip.y - p.y);
        if relative.x.abs() > 32767.
            || relative.y.abs() > 32767.
            || self.objects[i].tail_tip() == Some(tip)
        {
            return false;
        }
        self.begin_edit();
        if let Content::Text { tail, .. } = &mut self.objects[i].content {
            *tail = Some(relative);
        }
        true
    }
    pub fn add_callout(
        &mut self,
        p: Point,
        text: &str,
        size: u32,
        style: Style,
        background: Background,
    ) -> Result<bool, String> {
        self.add_label(p, text, size, style, background, true)
    }
    pub fn resize_text(
        &mut self,
        size: u32,
        width: Option<u32>,
        anchor: Point,
        right: bool,
        bottom: bool,
    ) -> Result<bool, String> {
        let Some(i) = self.objects.iter().position(|o| {
            Some(o.id) == self.selected && matches!(o.tool, Tool::Text | Tool::Callout)
        }) else {
            return Ok(false);
        };
        let Content::Text {
            text,
            size: old_size,
            wrap_width,
            mask,
            background,
            tail,
            layout,
        } = &self.objects[i].content
        else {
            return Ok(false);
        };
        if !(8..=256).contains(&size)
            || width.is_some_and(|w| !(24..=32767).contains(&w))
            || !anchor.x.is_finite()
            || !anchor.y.is_finite()
        {
            return Err("Invalid text frame.".into());
        }
        let (layout, wrap_width, tail, mask, object_width, object_height) = match *layout {
            TextLayout::Native => {
                let mask = if size == *old_size && width == *wrap_width {
                    mask.clone()
                } else {
                    Arc::new(crate::glyphs::wrapped(text, size, width)?)
                };
                (
                    TextLayout::Native,
                    width,
                    *tail,
                    mask.clone(),
                    mask.width as f64,
                    mask.height as f64,
                )
            }
            TextLayout::DotNet {
                source_size: _,
                bubble: None,
            } => {
                let mask = if size == *old_size && width == *wrap_width {
                    mask.clone()
                } else {
                    Arc::new(crate::glyphs::dotnet_text(text, size, width)?)
                };
                (
                    TextLayout::DotNet {
                        source_size: size as f64 / 4.,
                        bubble: None,
                    },
                    width,
                    None,
                    mask.clone(),
                    mask.width as f64,
                    mask.height as f64,
                )
            }
            TextLayout::DotNet {
                source_size: _,
                bubble: Some(mut frame),
            } => {
                let ratio = size as f64 / *old_size as f64;
                let corner = size != *old_size;
                frame.width = width.map(f64::from).unwrap_or(frame.width);
                if corner {
                    frame.min_height *= ratio;
                }
                frame.padding = 14f64.min(frame.width / 5.);
                let inner = Some((frame.width - frame.padding * 2.).max(1.).round() as u32);
                let mask = Arc::new(crate::glyphs::dotnet_text(text, size, inner)?);
                let tail = tail.map(|p| {
                    if corner {
                        Point::new(p.x * ratio, p.y * ratio)
                    } else {
                        p
                    }
                });
                let height = frame
                    .min_height
                    .max(mask.height as f64 + frame.padding * 2.);
                (
                    TextLayout::DotNet {
                        source_size: size as f64 / 4.,
                        bubble: Some(frame),
                    },
                    inner,
                    tail,
                    mask,
                    frame.width,
                    height,
                )
            }
        };
        let p = Point::new(
            anchor.x - if right { object_width } else { 0. },
            anchor.y - if bottom { object_height } else { 0. },
        );
        if p.x.abs() > 32767. || p.y.abs() > 32767. {
            return Err("Text frame is outside the supported coordinates.".into());
        }
        let content = Content::Text {
            text: text.clone(),
            size,
            wrap_width,
            mask,
            background: *background,
            tail,
            layout,
        };
        if content == self.objects[i].content && p == self.objects[i].points[0] {
            return Ok(false);
        }
        if !self.permits_mask(&content) {
            return Err("Text memory limit reached. Use shorter text or fewer edits.".into());
        }
        self.begin_edit();
        self.objects[i].content = content;
        self.objects[i].points = Arc::new(vec![p]);
        Ok(true)
    }
    pub fn render_snapshot(&self) -> Self {
        Self {
            crop: self.crop,
            source_clip: self.source_clip,
            objects: self.objects.clone(),
            selected: None,
            revision: self.revision,
            undo: vec![],
            redo: vec![],
            next: self.next,
            kept: None,
            project_saved: None,
        }
    }
    fn permits_mask(&self, content: &Content) -> bool {
        let mut seen = std::collections::HashSet::new();
        let mut bytes = 0;
        for c in std::iter::once(content)
            .chain(self.objects.iter().map(|o| &o.content))
            .chain(
                self.undo
                    .iter()
                    .chain(&self.redo)
                    .flat_map(|s| s.objects.iter().map(|o| &o.content)),
            )
        {
            if let Content::Text { mask, .. } | Content::Step { mask, .. } = c {
                if seen.insert(Arc::as_ptr(mask)) {
                    bytes += mask.alpha.len();
                }
            }
        }
        bytes <= 1024 * 1024
    }
    fn insert(&mut self, tool: Tool, points: Vec<Point>, style: Style, content: Content) -> bool {
        if !self.permits_mask(&content)
            || self.objects.len() >= 2000
            || self.objects.iter().map(|o| o.points.len()).sum::<usize>() + points.len() > 1_000_000
            || points.iter().any(|p| {
                !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 32767. || p.y.abs() > 32767.
            })
        {
            return false;
        }
        self.begin_edit();
        let id = self.revision;
        self.objects.push(Annotation {
            id,
            parent: None,
            tool,
            points: Arc::new(points),
            style,
            rendering: PathRendering::Native,
            content,
            opacity: 100,
            effects: crate::effects::Effects::default(),
            pattern: Pattern::Solid,
        });
        self.selected = Some(id);
        true
    }
    pub fn add_text(
        &mut self,
        p: Point,
        text: &str,
        size: u32,
        style: Style,
    ) -> Result<bool, String> {
        self.add_label(p, text, size, style, Background::default(), false)
    }
    pub fn add_label(
        &mut self,
        p: Point,
        text: &str,
        size: u32,
        style: Style,
        background: Background,
        callout: bool,
    ) -> Result<bool, String> {
        if text.trim().is_empty() || text.chars().count() > 4096 || !(8..=128).contains(&size) {
            return Ok(false);
        }
        let mask = Arc::new(crate::glyphs::raster(text, size)?);
        let tail = callout.then_some(Point::new(16., mask.height as f64 + 38.));
        let content = Content::Text {
            text: Arc::from(text),
            size,
            wrap_width: None,
            mask,
            background,
            tail,
            layout: TextLayout::Native,
        };
        if !self.permits_mask(&content) {
            return Err("Text memory limit reached. Use shorter text or fewer edits.".into());
        }
        Ok(self.insert(
            if callout { Tool::Callout } else { Tool::Text },
            vec![p],
            style,
            content,
        ))
    }
    pub fn edit_text(&mut self, text: &str, size: u32) -> Result<bool, String> {
        if text.trim().is_empty() || text.chars().count() > 4096 || !(8..=256).contains(&size) {
            return Ok(false);
        }
        let Some(i) = self.objects.iter().position(|o| {
            Some(o.id) == self.selected && matches!(o.tool, Tool::Text | Tool::Callout)
        }) else {
            return Ok(false);
        };
        if let Content::Text {
            text: old,
            size: old_size,
            ..
        } = &self.objects[i].content
        {
            if old.as_ref() == text && *old_size == size {
                return Ok(false);
            }
        }
        let width = match &self.objects[i].content {
            Content::Text { wrap_width, .. } => *wrap_width,
            _ => None,
        };
        let layout = if let Content::Text { layout, .. } = self.objects[i].content {
            layout
        } else {
            TextLayout::Native
        };
        let mask = Arc::new(match layout {
            TextLayout::Native => crate::glyphs::wrapped(text, size, width)?,
            TextLayout::DotNet { .. } => crate::glyphs::dotnet_text(text, size, width)?,
        });
        let background = self.objects[i].text_background().unwrap();
        let tail = if let Content::Text { tail, .. } = self.objects[i].content {
            tail
        } else {
            None
        };
        let layout = match layout {
            TextLayout::Native => TextLayout::Native,
            TextLayout::DotNet { bubble, .. } => TextLayout::DotNet {
                source_size: size as f64 / 4.,
                bubble,
            },
        };
        let content = Content::Text {
            text: Arc::from(text),
            size,
            wrap_width: width,
            mask,
            background,
            tail,
            layout,
        };
        if !self.permits_mask(&content) {
            return Err("Text memory limit reached. Use shorter text or fewer edits.".into());
        }
        self.begin_edit();
        self.objects[i].content = content;
        Ok(true)
    }
    pub fn add_step(&mut self, p: Point, diameter: u32, style: Style) -> Result<bool, String> {
        if !(20..=120).contains(&diameter) {
            return Ok(false);
        }
        let number = self.step_numbers().into_iter().max().unwrap_or(0) + 1;
        let mask = Arc::new(crate::glyphs::step(number, diameter)?);
        if !self.permits_mask(&Content::Step {
            number,
            diameter,
            mask: mask.clone(),
        }) {
            return Err("Annotation memory limit reached.".into());
        }
        Ok(self.insert(
            Tool::Step,
            vec![p],
            style,
            Content::Step {
                number,
                diameter,
                mask,
            },
        ))
    }
    pub fn step_numbers(&self) -> Vec<u32> {
        self.objects
            .iter()
            .filter_map(|o| {
                if let Content::Step { number, .. } = o.content {
                    Some(number)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn add_blur(&mut self, a: Point, b: Point, strength: u32) -> bool {
        if !(1..=64).contains(&strength) || (a.x - b.x).abs() < 1. || (a.y - b.y).abs() < 1. {
            return false;
        }
        self.insert(
            Tool::Blur,
            vec![a, b],
            Style::default(),
            Content::Blur { strength },
        )
    }
    pub fn set_parameter(&mut self, value: u32) -> Result<bool, String> {
        let Some(i) = self
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected)
        else {
            return Ok(false);
        };
        if self.objects[i].parameter() == value {
            return Ok(false);
        }
        if let Content::Step { number, .. } = self.objects[i].content {
            if self.objects[i].rendering == PathRendering::DotNet {
                if !(2..=32).contains(&value) {
                    return Ok(false);
                }
                let diameter = dotnet_step_diameter(number, value);
                let content = Content::Step {
                    number,
                    diameter,
                    mask: Arc::new(crate::glyphs::dotnet_step(number, value)?),
                };
                if !self.permits_mask(&content) {
                    return Err("Annotation memory limit reached.".into());
                }
                self.begin_edit();
                self.objects[i].style.width = value;
                self.objects[i].content = content;
                return Ok(true);
            }
        }
        let content = match &self.objects[i].content {
            Content::Text { text, .. } => {
                let text = text.clone();
                return self.edit_text(&text, value);
            }
            Content::Step { number, .. } if (20..=120).contains(&value) => Content::Step {
                number: *number,
                diameter: value,
                mask: Arc::new(crate::glyphs::step(*number, value)?),
            },
            Content::Blur { .. } if (1..=64).contains(&value) => Content::Blur { strength: value },
            Content::Shape | Content::Highlight if (1..=40).contains(&value) => {
                let mut style = self.objects[i].style;
                style.width = value;
                return Ok(self.restyle(style));
            }
            _ => return Ok(false),
        };
        if !self.permits_mask(&content) {
            return Err("Annotation memory limit reached.".into());
        }
        self.begin_edit();
        self.objects[i].content = content;
        Ok(true)
    }
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            crop: Rect::new(0, 0, width, height).unwrap(),
            source_clip: Some(Rect::new(0, 0, width, height).unwrap()),
            objects: vec![],
            selected: None,
            revision: 0,
            undo: vec![],
            redo: vec![],
            next: 1,
            kept: None,
            project_saved: None,
        }
    }
    pub fn from_project(
        crop: Rect,
        source_clip: Option<Rect>,
        objects: Vec<Annotation>,
    ) -> Result<Self, String> {
        crate::combine::validate_tree(&objects)?;
        let next = objects
            .iter()
            .map(|o| o.id)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .filter(|v| *v < u64::MAX)
            .ok_or("Project object identity overflow")?;
        Ok(Self {
            crop,
            source_clip,
            objects,
            selected: None,
            revision: 0,
            undo: vec![],
            redo: vec![],
            next,
            kept: None,
            project_saved: Some(0),
        })
    }
    pub fn mark_project_saved(&mut self, revision: u64) {
        self.project_saved = Some(revision);
    }
    pub fn is_project_saved(&self) -> bool {
        self.project_saved == Some(self.revision)
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            crop: self.crop,
            source_clip: self.source_clip,
            objects: self.objects.clone(),
            revision: self.revision,
        }
    }
    pub(crate) fn begin_edit(&mut self) {
        self.undo.push(self.snapshot());
        if self.undo.len() > 64 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.revision = self.next;
        self.next += 1;
    }
    fn restore(&mut self, s: Snapshot) {
        self.crop = s.crop;
        self.source_clip = s.source_clip;
        self.objects = s.objects;
        self.revision = s.revision;
        if !self.objects.iter().any(|o| Some(o.id) == self.selected) {
            self.selected = None;
        }
    }
    pub fn add(&mut self, tool: Tool, points: Vec<Point>, style: Style) -> bool {
        if !matches!(
            tool,
            Tool::Pen
                | Tool::Highlighter
                | Tool::Line
                | Tool::Arrow
                | Tool::Rectangle
                | Tool::Ellipse
                | Tool::FilledRectangle
                | Tool::FilledEllipse
        ) || !(1..=40).contains(&style.width)
            || points.len() < 2
            || points.len() > 100_000
            || self.objects.len() >= 2000
            || self.objects.iter().map(|o| o.points.len()).sum::<usize>() + points.len() > 1_000_000
            || points.iter().any(|p| {
                !p.x.is_finite() || !p.y.is_finite() || p.x.abs() > 32767. || p.y.abs() > 32767.
            })
        {
            return false;
        }
        if points.iter().all(|p| distance(points[0], *p) < 1.) {
            return false;
        }
        if matches!(
            tool,
            Tool::Rectangle | Tool::Ellipse | Tool::FilledRectangle | Tool::FilledEllipse
        ) && ((points[0].x - points[1].x).abs() < 1. || (points[0].y - points[1].y).abs() < 1.)
        {
            return false;
        }
        if matches!(tool, Tool::Line | Tool::Arrow)
            && (points.len() > 3 || distance(points[0], points[1]) < 1.)
        {
            return false;
        }
        self.begin_edit();
        let id = self.revision;
        self.objects.push(Annotation {
            id,
            parent: None,
            tool,
            points: Arc::new(points),
            style,
            rendering: PathRendering::Native,
            content: if tool == Tool::Highlighter {
                Content::Highlight
            } else {
                Content::Shape
            },
            pattern: Pattern::Solid,
            opacity: if tool == Tool::Highlighter { 40 } else { 100 },
            effects: crate::effects::Effects::default(),
        });
        self.selected = Some(id);
        true
    }
    pub fn undo(&mut self) -> bool {
        let Some(s) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.snapshot());
        self.restore(s);
        true
    }
    pub fn set_effects(&mut self, effects: crate::effects::Effects) -> Result<bool, String> {
        let mut effects = effects.validate()?;
        let Some(index) = self
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected)
        else {
            return Ok(false);
        };
        if !matches!(self.objects[index].tool, Tool::Text | Tool::Callout) {
            effects.outline = self.objects[index].effects.outline;
        }
        if self.objects[index].effects == effects {
            return Ok(false);
        }
        self.begin_edit();
        self.objects[index].effects = effects;
        Ok(true)
    }
    pub fn set_opacity(&mut self, opacity: u8) -> bool {
        if opacity > 100 {
            return false;
        }
        let Some(index) = self.objects.iter().position(|o| {
            Some(o.id) == self.selected && o.tool.opacity_supported() && o.opacity != opacity
        }) else {
            return false;
        };
        self.begin_edit();
        self.objects[index].opacity = opacity;
        true
    }
    pub fn add_patterned(
        &mut self,
        tool: Tool,
        points: Vec<Point>,
        style: Style,
        pattern: Pattern,
    ) -> bool {
        if !self.add(tool, points, style) {
            return false;
        }
        if tool.patterned() {
            self.objects.last_mut().unwrap().pattern = pattern;
        }
        true
    }
    pub fn set_pattern(&mut self, pattern: Pattern) -> bool {
        let Some(i) = self.objects.iter().position(|o| {
            Some(o.id) == self.selected && o.tool.patterned() && o.pattern != pattern
        }) else {
            return false;
        };
        self.begin_edit();
        self.objects[i].pattern = pattern;
        true
    }
    pub fn set_curve_midpoint(&mut self, midpoint: Point) -> bool {
        let Some(i) = self
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected)
        else {
            return false;
        };
        let Some(proposal) = crate::geometry::bend(
            &self.objects[i],
            midpoint,
            self.local_canvas(self.objects[i].id),
        ) else {
            return false;
        };
        if proposal.points == self.objects[i].points {
            return false;
        }
        if self.objects.iter().map(|o| o.points.len()).sum::<usize>() - self.objects[i].points.len()
            + proposal.points.len()
            > 1_000_000
        {
            return false;
        }
        self.begin_edit();
        self.objects[i] = proposal;
        true
    }
    pub fn reset_curve(&mut self) -> bool {
        let Some(i) = self.objects.iter().position(|o| {
            Some(o.id) == self.selected
                && matches!(o.tool, Tool::Line | Tool::Arrow)
                && o.points.len() == 3
        }) else {
            return false;
        };
        let points = self.objects[i].points[..2].to_vec();
        self.begin_edit();
        self.objects[i].points = Arc::new(points);
        true
    }
    pub fn add_highlight(&mut self, points: Vec<Point>, style: Style, opacity: u8) -> bool {
        if opacity > 100 || !self.add(Tool::Highlighter, points, style) {
            return false;
        }
        self.objects.last_mut().unwrap().opacity = opacity;
        true
    }
    pub fn redo(&mut self) -> bool {
        let Some(s) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.snapshot());
        self.restore(s);
        true
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn delete(&mut self) -> bool {
        let Some(i) = self
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected)
        else {
            return false;
        };
        self.begin_edit();
        let id = self.objects[i].id;
        let remove: std::collections::HashSet<_> = self
            .objects
            .iter()
            .filter(|o| o.id == id || self.is_descendant(o.id, id))
            .map(|o| o.id)
            .collect();
        self.objects.retain(|o| !remove.contains(&o.id));
        self.selected = None;
        true
    }
    pub fn move_selected(&mut self, dx: f64, dy: f64) -> bool {
        if !dx.is_finite() || !dy.is_finite() || (dx.abs() < 0.001 && dy.abs() < 0.001) {
            return false;
        }
        let Some(i) = self
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected)
        else {
            return false;
        };
        let scale = self.parent_transform(self.objects[i].id).scale;
        let (dx, dy) = (dx / scale, dy / scale);
        let points: Vec<_> = self.objects[i]
            .points
            .iter()
            .map(|p| Point::new(p.x + dx, p.y + dy))
            .collect();
        if points
            .iter()
            .any(|p| p.x.abs() > 32767. || p.y.abs() > 32767.)
        {
            return false;
        }
        self.begin_edit();
        self.objects[i].points = Arc::new(points);
        true
    }
    pub fn restyle(&mut self, style: Style) -> bool {
        if !(1..=40).contains(&style.width) {
            return false;
        }
        let Some(i) = self
            .objects
            .iter()
            .position(|o| Some(o.id) == self.selected)
        else {
            return false;
        };
        if matches!(
            self.objects[i].content,
            Content::Image { .. } | Content::Group { .. }
        ) || self.objects[i].style == style
        {
            return false;
        }
        self.begin_edit();
        self.objects[i].style = style;
        true
    }
    pub fn set_crop(&mut self, rect: Rect) -> bool {
        let Some(crop) = self.crop.intersection(rect) else {
            return false;
        };
        if crop == self.crop {
            return false;
        }
        self.begin_edit();
        self.crop = crop;
        self.source_clip = self.source_clip.and_then(|r| r.intersection(crop));
        true
    }
    pub fn hit(&self, point: Point, tolerance: f64) -> Option<u64> {
        self.hit_in(point, tolerance, None)
    }
    fn hit_in(&self, point: Point, tolerance: f64, parent: Option<u64>) -> Option<u64> {
        for o in self.objects.iter().rev().filter(|o| o.parent == parent) {
            if let Content::Group { image } = &o.content {
                let (a, b) = o.bounds();
                if point.x >= a.x && point.x < b.x && point.y >= a.y && point.y < b.y {
                    let transform = crate::combine::image_transform(o, image.viewport);
                    return self
                        .hit_in(
                            transform.inverse(point),
                            tolerance / transform.scale,
                            Some(o.id),
                        )
                        .or(Some(o.id));
                }
                continue;
            }
            if self.hit_object(o, point, tolerance) {
                return Some(o.id);
            }
        }
        None
    }
    fn hit_object(&self, object: &Annotation, point: Point, tolerance: f64) -> bool {
        std::iter::once(object)
            .find(|o| {
                if let Some(tip) = o.tail_tip() {
                    let inside_text = {
                        let (a, b) = o.bounds();
                        point.x >= a.x && point.x <= b.x && point.y >= a.y && point.y <= b.y
                    };
                    return inside_text
                        || crate::callout::hit(
                            o.body_bounds(),
                            tip,
                            point,
                            tolerance + o.style.width as f64 / 2.,
                            o.text_background().is_some_and(|b| b.enabled),
                        );
                }
                if let Content::Step { diameter, .. } = o.content {
                    return distance(point, o.points[0]) <= diameter as f64 / 2. + tolerance;
                }
                if !matches!(o.content, Content::Shape | Content::Highlight) {
                    let (a, b) = o.bounds();
                    return point.x >= a.x - tolerance
                        && point.x <= b.x + tolerance
                        && point.y >= a.y - tolerance
                        && point.y <= b.y + tolerance;
                }
                let t = tolerance + o.style.width as f64 / 2.;
                if o.tool.filled() {
                    let (a, b) = o.bounds();
                    if o.tool == Tool::FilledRectangle {
                        return point.x >= a.x
                            && point.x <= b.x
                            && point.y >= a.y
                            && point.y <= b.y;
                    }
                    let rx = (b.x - a.x) / 2.;
                    let ry = (b.y - a.y) / 2.;
                    return ((point.x - (a.x + b.x) / 2.) / rx).powi(2)
                        + ((point.y - (a.y + b.y) / 2.) / ry).powi(2)
                        <= 1.;
                }
                segments(o)
                    .iter()
                    .any(|(a, b)| segment_distance(point, *a, *b) <= t)
            })
            .is_some()
    }
    pub fn mark_kept(&mut self, revision: u64) {
        self.kept = Some(revision);
    }
    pub fn is_kept(&self) -> bool {
        self.kept == Some(self.revision)
    }
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
fn segment_distance(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let n = dx * dx + dy * dy;
    let t = if n == 0. {
        0.
    } else {
        ((p.x - a.x) * dx + (p.y - a.y) * dy) / n
    }
    .clamp(0., 1.);
    distance(p, Point::new(a.x + t * dx, a.y + t * dy))
}
pub fn segments(o: &Annotation) -> Vec<(Point, Point)> {
    if !matches!(o.content, Content::Shape | Content::Highlight) {
        return vec![];
    }
    let p = &o.points;
    if p.len() < 2 {
        return if p.len() == 1
            && o.rendering == PathRendering::DotNet
            && matches!(o.tool, Tool::Pen | Tool::Highlighter)
        {
            vec![(p[0], p[0])]
        } else {
            vec![]
        };
    }
    let a = p[0];
    let b = p[p.len() - 1];
    if matches!(o.tool, Tool::Rectangle | Tool::FilledRectangle) {
        let c = Point::new(b.x, a.y);
        let d = Point::new(a.x, b.y);
        vec![(a, c), (c, b), (b, d), (d, a)]
    } else {
        let path = crate::geometry::polyline(o);
        let mut lines: Vec<_> = path.windows(2).map(|p| (p[0], p[1])).collect();
        if o.tool == Tool::Arrow {
            lines.extend(crate::geometry::arrowhead(o));
        }
        lines
    }
}

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub crop: Rect,
    pub x: f64,
    pub y: f64,
    pub scale: f64,
}
impl View {
    pub fn point(self, x: f64, y: f64, clamp: bool) -> Option<Point> {
        if !self.scale.is_finite() || self.scale <= 0. || !x.is_finite() || !y.is_finite() {
            return None;
        }
        let x = (x - self.x) / self.scale;
        let y = (y - self.y) / self.scale;
        if !clamp && (x < 0. || y < 0. || x > self.crop.width as f64 || y > self.crop.height as f64)
        {
            return None;
        }
        Some(Point::new(
            self.crop.x as f64 + x.clamp(0., self.crop.width as f64),
            self.crop.y as f64 + y.clamp(0., self.crop.height as f64),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curved_arrow_bounds_include_the_tangent_oriented_head() {
        let mut d = Document::new(200, 200);
        d.add(
            Tool::Arrow,
            vec![
                Point::new(20., 80.),
                Point::new(180., 80.),
                Point::new(180., 0.),
            ],
            Style::default(),
        );
        let (a, b) = d.objects[0].bounds();
        assert!(a.x <= 20. && a.y <= 40.);
        assert!(
            b.x > 180.,
            "vertical tangent head extends right of endpoint"
        );
        assert_eq!(d.hit(Point::new(186., 67.), 1.), Some(d.objects[0].id));
    }
    #[test]
    fn pattern_and_curve_edits_round_trip_without_mutating_original_points() {
        let mut d = Document::new(100, 80);
        d.add(
            Tool::Arrow,
            vec![Point::new(10., 60.), Point::new(90., 60.)],
            Style::default(),
        );
        let original = d.objects[0].points.clone();
        assert!(d.set_pattern(Pattern::Dashed));
        assert!(d.set_curve_midpoint(Point::new(50., 30.)));
        assert_eq!(d.objects[0].points[2], Point::new(50., 0.));
        assert_eq!(original.len(), 2);
        assert!(d.move_selected(5., 5.));
        assert_eq!(d.objects[0].curve_midpoint(), Some(Point::new(55., 35.)));
        assert!(d.undo());
        assert!(d.undo());
        assert_eq!(d.objects[0].pattern, Pattern::Dashed);
        assert_eq!(d.objects[0].points.len(), 2);
        assert!(d.redo());
        assert!(d.reset_curve());
        assert_eq!(d.objects[0].pattern, Pattern::Dashed);
        let rev = d.revision;
        assert!(!d.reset_curve());
        assert!(!d.set_curve_midpoint(Point::new(f64::NAN, 0.)));
        assert_eq!(d.revision, rev);
        assert!(d.undo());
        assert_eq!(d.objects[0].curve_midpoint(), Some(Point::new(50., 30.)));
    }
    #[test]
    fn shapes_accept_both_drag_directions_and_reject_degenerate_area() {
        for tool in [Tool::Ellipse, Tool::FilledRectangle, Tool::FilledEllipse] {
            let mut d = Document::new(100, 80);
            assert!(
                d.add(
                    tool,
                    vec![Point::new(80., 60.), Point::new(20., 20.)],
                    Style::default()
                ),
                "{tool:?}"
            );
            assert_eq!(
                d.objects[0].bounds(),
                (Point::new(20., 20.), Point::new(80., 60.))
            );
            let revision = d.revision;
            assert!(!d.add(
                tool,
                vec![Point::new(10., 10.), Point::new(10., 50.)],
                Style::default()
            ));
            assert_eq!(d.revision, revision);
        }
    }
    #[test]
    fn quadratic_bounds_and_hit_follow_curve_not_control_polygon() {
        let mut d = Document::new(200, 200);
        assert!(d.add(
            Tool::Line,
            vec![
                Point::new(20., 80.),
                Point::new(180., 80.),
                Point::new(100., 0.)
            ],
            Style::default()
        ));
        let o = &d.objects[0];
        assert_eq!(o.bounds(), (Point::new(20., 40.), Point::new(180., 80.)));
        assert_eq!(d.hit(Point::new(100., 40.), 0.), Some(o.id));
        assert_eq!(d.hit(Point::new(100., 80.), 0.), None);
        assert_eq!(d.hit(Point::new(100., 0.), 0.), None);
    }
    #[test]
    fn highlight_and_blur_opacity_edits_round_trip_history() {
        let mut d = Document::new(80, 60);
        assert!(d.add(
            Tool::Highlighter,
            vec![Point::new(10., 25.), Point::new(60., 25.)],
            Style::default()
        ));
        let points = d.objects[0].points.clone();
        d.mark_kept(d.revision);
        assert!(d.set_opacity(70));
        assert_eq!(d.objects[0].content, Content::Highlight);
        assert_eq!(d.objects[0].opacity, 70);
        assert!(Arc::ptr_eq(&points, &d.objects[0].points));
        assert!(!d.is_kept());
        let revision = d.revision;
        assert!(!d.set_opacity(101));
        assert!(!d.set_opacity(70));
        assert_eq!(d.revision, revision);
        assert!(d.undo());
        assert!(d.is_kept());
        assert_eq!(d.objects[0].opacity, 40);
        assert!(d.redo());
        assert_eq!(d.objects[0].opacity, 70);
        assert!(d.set_opacity(0));
        assert!(d.hit(Point::new(30., 25.), 2.).is_some());
        assert!(d.move_selected(3., 2.));
        assert!(d.delete());
        assert!(d.undo());
        assert_eq!(d.objects[0].opacity, 0);
        d.add(
            Tool::Pen,
            vec![Point::new(10., 10.), Point::new(20., 20.)],
            Style::default(),
        );
        assert!(d.set_opacity(60));
        assert_eq!(d.objects.last().unwrap().opacity, 60);
        assert!(d.add_blur(Point::new(10., 10.), Point::new(30., 30.), 8));
        assert!(d.set_opacity(60));
        assert_eq!(d.objects.last().unwrap().opacity, 60);
        assert!(d.undo());
        assert_eq!(d.objects.last().unwrap().opacity, 100);
    }
    #[test]
    fn image_budget_allows_larger_sources_and_counts_redo_sources_once() {
        use crate::frame::Frame;
        let a = Arc::new(Frame::new(2048, 2048, vec![255; 16 * 1024 * 1024]).unwrap());
        let b = Arc::new(Frame::new(2048, 2048, vec![0; 16 * 1024 * 1024]).unwrap());
        let tiny = Arc::new(Frame::new(1, 1, vec![255; 4]).unwrap());
        let mut d = Document::new(10, 10);
        d.add_image(a.clone()).unwrap();
        for _ in 0..10 {
            d.move_selected(1., 0.);
        }
        assert_eq!(d.image_sources().len(), 1);
        d.add_image(b).unwrap();
        d.delete();
        let rev = d.revision;
        d.add_image(tiny.clone()).unwrap();
        assert_ne!(d.revision, rev);
        assert_eq!(d.image_sources().len(), 3);
        d.undo();
        d.undo();
        assert!(d.can_redo());
        d.add_image(tiny.clone()).unwrap();
        assert_eq!(
            d.image_sources().len(),
            3,
            "redo source is not counted twice"
        );
        assert!(Arc::ptr_eq(&d.image_sources()[0], &a));
    }

    #[test]
    fn image_source_budget_uses_checked_aggregate_and_deduplicates_shared_frames() {
        let limit = crate::image_limits::SOURCE_LIMIT;
        assert!(source_budget_allows(limit - 1, 1, false));
        assert!(!source_budget_allows(limit, 1, false));
        assert!(source_budget_allows(limit, usize::MAX, true));
        assert!(!source_budget_allows(usize::MAX, 1, false));
    }
    #[test]
    fn image_resize_is_reversible_and_keeps_original_pixels() {
        let frame = Arc::new(crate::frame::Frame::new(40, 20, vec![255; 40 * 20 * 4]).unwrap());
        let mut d = Document::new(100, 80);
        d.add_image(frame.clone()).unwrap();
        assert!(d.resize_image((Point::new(10., 20.), Point::new(90., 60.))));
        assert_eq!(
            d.objects[0].bounds(),
            (Point::new(10., 20.), Point::new(90., 60.))
        );
        assert!(
            !d.resize_image((Point::new(10., 20.), Point::new(90., 65.))),
            "reject distortion"
        );
        assert!(d.undo());
        assert_eq!(
            d.objects[0].bounds(),
            (Point::new(0., 0.), Point::new(40., 20.))
        );
        assert!(d.redo());
        assert!(Arc::ptr_eq(&d.image_sources()[0], &frame));
        let rev = d.revision;
        assert!(!d.restyle(Style {
            rgb: [1, 2, 3],
            width: 4
        }));
        assert_eq!(d.revision, rev);
    }
    #[test]
    fn image_insert_expands_canvas_in_one_undo_without_copying_source_history() {
        let frame = Arc::new(crate::frame::Frame::new(120, 40, vec![255; 120 * 40 * 4]).unwrap());
        let mut d = Document::new(100, 80);
        assert!(d.add_image(frame.clone()).unwrap());
        assert_eq!(d.crop, Rect::new(0, 0, 120, 80).unwrap());
        assert_eq!(d.objects.len(), 1);
        assert_eq!(d.hit(Point::new(110., 20.), 0.), d.selected);
        let sources = d.image_sources();
        assert_eq!(sources.len(), 1);
        assert!(Arc::ptr_eq(&sources[0], &frame));
        assert!(d.move_selected(10., 20.));
        assert!(d.delete());
        assert_eq!(d.image_sources().len(), 1, "history retains shared pixels");
        assert!(d.undo());
        assert!(d.undo());
        assert!(d.undo());
        assert!(d.objects.is_empty());
        assert_eq!(d.crop, Rect::new(0, 0, 100, 80).unwrap());
        assert!(d.redo());
        assert_eq!(d.objects.len(), 1);
    }
    #[test]
    fn canvas_expansion_rejects_shrink_and_oversize_without_editing_history() {
        let mut d = Document::new(100, 80);
        assert!(d.expand_canvas(160, 120).unwrap());
        assert_eq!(d.crop, Rect::new(0, 0, 160, 120).unwrap());
        let revision = d.revision;
        assert!(!d.expand_canvas(160, 120).unwrap());
        assert!(d.expand_canvas(80, 120).is_err());
        assert!(d.expand_canvas(32767, 32767).is_err());
        assert_eq!(d.revision, revision);
        assert!(d.undo());
        assert_eq!(d.crop, Rect::new(0, 0, 100, 80).unwrap());
        assert!(!d.can_undo());
    }
    fn layer_fixture() -> Document {
        let mut d = Document::new(100, 80);
        for rgb in [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0]] {
            assert!(d.add(
                Tool::Line,
                vec![Point::new(10., 20.), Point::new(80., 20.)],
                Style { rgb, width: 5 },
            ));
        }
        d
    }
    #[test]
    fn layer_order_moves_one_neighbor_or_to_stack_end_without_changing_objects() {
        for (order, expected) in [
            (Order::Front, [1, 3, 4, 2]),
            (Order::Forward, [1, 3, 2, 4]),
            (Order::Backward, [2, 1, 3, 4]),
            (Order::Back, [2, 1, 3, 4]),
        ] {
            let mut d = layer_fixture();
            let before = d.objects.clone();
            d.selected = Some(2);
            assert!(d.can_reorder(order));
            assert!(d.reorder(order));
            assert_eq!(d.objects.iter().map(|o| o.id).collect::<Vec<_>>(), expected);
            assert_eq!(d.selected, Some(2));
            for o in &d.objects {
                let original = &before[o.id as usize - 1];
                assert_eq!(o, original);
                assert!(Arc::ptr_eq(&o.points, &original.points));
            }
        }
        let mut d = layer_fixture();
        d.selected = Some(3);
        assert!(d.reorder(Order::Back));
        assert_eq!(
            d.objects.iter().map(|o| o.id).collect::<Vec<_>>(),
            [3, 1, 2, 4]
        );
    }
    #[test]
    fn layer_order_boundaries_and_invalid_selection_preserve_history_and_revision() {
        for (selection, orders) in [
            (
                None,
                vec![Order::Front, Order::Forward, Order::Backward, Order::Back],
            ),
            (
                Some(999),
                vec![Order::Front, Order::Forward, Order::Backward, Order::Back],
            ),
            (Some(4), vec![Order::Front, Order::Forward]),
            (Some(1), vec![Order::Back, Order::Backward]),
        ] {
            let mut d = layer_fixture();
            d.selected = selection;
            let before = d.objects.clone();
            let revision = d.revision;
            let history = (d.undo.len(), d.redo.len());
            for order in orders {
                assert!(!d.can_reorder(order));
                assert!(!d.reorder(order));
                assert_eq!(d.objects, before);
                assert_eq!(d.selected, selection);
                assert_eq!(d.revision, revision);
                assert_eq!((d.undo.len(), d.redo.len()), history);
            }
        }
        for count in [0, 1] {
            let mut d = layer_fixture();
            d.objects.truncate(count);
            d.selected = d.objects.first().map(|o| o.id);
            for order in [Order::Front, Order::Forward, Order::Backward, Order::Back] {
                assert!(!d.can_reorder(order));
                assert!(!d.reorder(order));
            }
        }
    }
    #[test]
    fn layer_order_is_one_undo_step_and_restores_saved_state_crop_and_topmost_hit() {
        let mut d = layer_fixture();
        let crop = Rect::new(5, 5, 80, 60).unwrap();
        assert!(d.set_crop(crop));
        d.selected = Some(2);
        d.mark_kept(d.revision);
        let history = d.undo.len();
        assert!(d.reorder(Order::Front));
        assert_eq!(d.undo.len(), history + 1);
        assert!(!d.is_kept());
        assert_eq!(d.crop, crop);
        assert_eq!(d.hit(Point::new(30., 20.), 0.), Some(2));
        assert!(!d.can_reorder(Order::Front));
        assert!(d.can_reorder(Order::Back));
        assert!(d.undo());
        assert!(d.is_kept());
        assert_eq!(d.selected, Some(2));
        assert_eq!(d.crop, crop);
        assert_eq!(d.hit(Point::new(30., 20.), 0.), Some(4));
        assert!(d.redo());
        assert!(!d.is_kept());
        assert_eq!(d.hit(Point::new(30., 20.), 0.), Some(2));
        assert!(d.undo());
        assert!(d.can_redo());
        assert!(d.reorder(Order::Back));
        assert!(!d.can_redo());
    }
    #[test]
    fn layer_order_shares_callout_mask_and_preserves_text_background_and_tail() {
        let mut d = layer_fixture();
        assert!(
            d.add_callout(
                Point::new(20., 20.),
                "Bubble",
                24,
                Style::default(),
                Background {
                    enabled: true,
                    rgb: [20, 100, 180]
                }
            )
            .unwrap()
        );
        assert!(d.set_tail(Point::new(8., 60.)));
        let original = d.objects.last().unwrap().clone();
        assert!(d.reorder(Order::Back));
        assert_eq!(d.objects[0], original);
        let (Content::Text { mask: a, .. }, Content::Text { mask: b, .. }) =
            (&original.content, &d.objects[0].content)
        else {
            panic!("Callout lost text")
        };
        assert!(Arc::ptr_eq(a, b));
        assert!(d.undo());
        assert_eq!(d.objects.last().unwrap(), &original);
    }
    #[test]
    fn text_background_is_independent_shared_and_reversible() {
        let mut d = Document::new(800, 600);
        d.add_text(Point::new(60., 60.), "Text", 24, Style::default())
            .unwrap();
        let original = d.objects[0].clone();
        let Content::Text {
            mask, background, ..
        } = &original.content
        else {
            panic!()
        };
        assert!(!background.enabled);
        let fill = Background {
            enabled: true,
            rgb: [10, 190, 40],
        };
        assert!(d.set_text_background(fill));
        let Content::Text { mask: current, .. } = &d.objects[0].content else {
            panic!()
        };
        assert!(Arc::ptr_eq(mask, current));
        assert!(d.edit_text("Changed text", 32).unwrap());
        assert!(
            matches!(d.objects[0].content, Content::Text {background, ..} if background == fill)
        );
        assert!(d.undo());
        assert!(d.undo());
        assert_eq!(d.objects[0], original);
        assert!(d.redo());
        assert!(d.set_text_background(Background {
            enabled: false,
            ..fill
        }));
        assert!(
            matches!(d.objects[0].content, Content::Text {background, ..} if !background.enabled && background.rgb == fill.rgb)
        );
        let revision = d.revision;
        assert!(!d.set_text_background(Background {
            enabled: false,
            ..fill
        }));
        assert_eq!(d.revision, revision);
    }
    #[test]
    fn callout_tail_is_relative_bounded_and_does_not_rerasterize_text() {
        let mut d = Document::new(800, 600);
        assert!(
            d.add_callout(
                Point::new(80., 90.),
                "Bubble",
                24,
                Style::default(),
                Background::default()
            )
            .unwrap()
        );
        let original = d.objects[0].clone();
        assert_eq!(original.tool, Tool::Callout);
        assert!(d.set_tail(Point::new(20., 180.)));
        let moved_tail = d.objects[0].clone();
        let Content::Text { mask: a, .. } = &original.content else {
            panic!()
        };
        let Content::Text { mask: b, tail, .. } = &moved_tail.content else {
            panic!()
        };
        assert!(Arc::ptr_eq(a, b));
        assert_eq!(*tail, Some(Point::new(-60., 90.)));
        assert!(!d.set_tail(Point::new(f64::NAN, 0.)));
        assert!(!d.set_tail(Point::new(40000., 0.)));
        assert!(d.move_selected(20., 30.));
        assert_eq!(d.objects[0].tail_tip(), Some(Point::new(40., 210.)));
        assert!(d.undo());
        assert_eq!(d.objects[0], moved_tail);
        assert!(d.undo());
        assert_eq!(d.objects[0], original);
        assert!(d.edit_text("Updated", 30).unwrap());
        assert_eq!(d.objects[0].tail_tip(), original.tail_tip());
    }
    #[test]
    fn text_side_resize_preserves_size_input_anchor_and_one_history_step() {
        let mut d = Document::new(800, 600);
        let text = "English words in a longer line\nภาษาไทย";
        d.add_text(Point::new(20., 40.), text, 24, Style::default())
            .unwrap();
        let before = d.objects[0].clone();
        assert!(
            d.resize_text(24, Some(140), Point::new(300., 40.), true, false)
                .unwrap()
        );
        let resized = d.objects[0].clone();
        if let Content::Text { text: stored, .. } = &resized.content {
            assert_eq!(stored.as_ref(), text);
        } else {
            panic!("Text content was lost");
        }
        assert_eq!(resized.parameter(), 24);
        assert_eq!(resized.bounds().1.x, 300.);
        assert_eq!(resized.bounds().0.y, 40.);
        assert_eq!(resized.bounds().1.x - resized.bounds().0.x, 140.);
        assert!(resized.bounds().1.y > before.bounds().1.y);
        assert!(d.undo());
        assert_eq!(d.objects[0], before);
        assert!(d.redo());
        assert_eq!(d.objects[0], resized);
        d.edit_text("English words in a longer line\nchanged", 24)
            .unwrap();
        assert_eq!(d.objects[0].bounds().1.x - d.objects[0].bounds().0.x, 140.);
        d.set_parameter(30).unwrap();
        assert_eq!(d.objects[0].parameter(), 30);
        assert_eq!(d.objects[0].bounds().1.x - d.objects[0].bounds().0.x, 140.);
    }
    #[test]
    fn text_corner_resize_fixes_opposite_corner_and_rejects_invalid_layout() {
        for callout in [false, true] {
            let mut d = Document::new(800, 600);
            d.add_label(
                Point::new(40., 40.),
                "ภาษาไทย\nEnglish",
                24,
                Style::default(),
                Background {
                    enabled: true,
                    rgb: [30, 60, 90],
                },
                callout,
            )
            .unwrap();
            let before = d.objects[0].clone();
            let anchor = before.bounds().1;
            assert!(d.resize_text(48, None, anchor, true, true).unwrap());
            assert_eq!(d.objects[0].bounds().1, anchor);
            assert_eq!(d.objects[0].parameter(), 48);
            assert_eq!(d.objects[0].text_background(), before.text_background());
            let old_origin = before.points[0];
            let new_origin = d.objects[0].points[0];
            if let Some(tip) = before.tail_tip() {
                assert_eq!(
                    d.objects[0].tail_tip(),
                    Some(Point::new(
                        tip.x + new_origin.x - old_origin.x,
                        tip.y + new_origin.y - old_origin.y,
                    ))
                );
            }
            let valid = d.objects[0].clone();
            let revision = d.revision;
            assert!(!d.resize_text(48, None, anchor, true, true).unwrap());
            assert!(d.resize_text(48, Some(23), anchor, true, true).is_err());
            assert!(d.resize_text(129, None, anchor, true, true).is_err());
            assert!(
                d.resize_text(48, None, Point::new(f64::NAN, 0.), true, true)
                    .is_err()
            );
            assert_eq!(d.objects[0], valid);
            assert_eq!(d.revision, revision);
            d.undo();
            assert_eq!(d.objects[0], before);
        }
    }
    #[test]
    fn oversized_resize_preserves_object_history_and_selection() {
        let mut d = Document::new(800, 600);
        d.add_text(
            Point::new(40., 40.),
            &"line\n".repeat(60),
            8,
            Style::default(),
        )
        .unwrap();
        let before = d.objects[0].clone();
        let revision = d.revision;
        let selected = d.selected;
        assert!(
            d.resize_text(128, Some(32767), Point::new(40., 40.), false, false)
                .is_err()
        );
        assert_eq!(d.objects[0], before);
        assert_eq!(d.revision, revision);
        assert_eq!(d.selected, selected);
        d.undo();
        assert!(d.objects.is_empty());
    }
    #[test]
    fn text_memory_limit_is_reported_instead_of_silently_losing_input() {
        let mut d = Document::new(200, 200);
        d.insert(
            Tool::Text,
            vec![Point::new(1., 1.)],
            Style::default(),
            Content::Text {
                text: Arc::from("A"),
                size: 24,
                wrap_width: None,
                background: Default::default(),
                tail: None,
                layout: TextLayout::Native,
                mask: Arc::new(crate::glyphs::Mask {
                    width: 1024,
                    height: 1024,
                    alpha: vec![0; 1024 * 1024],
                }),
            },
        );
        let revision = d.revision;
        assert!(
            d.add_text(Point::new(10., 10.), "New text", 24, Style::default())
                .is_err()
        );
        assert_eq!(d.revision, revision);
    }
    #[test]
    fn long_step_numbers_fit_inside_the_smallest_circle() {
        let mut d = Document::new(200, 200);
        d.insert(
            Tool::Step,
            vec![Point::new(20., 20.)],
            Style::default(),
            Content::Step {
                number: 1999,
                diameter: 36,
                mask: Arc::new(crate::glyphs::raster("1999", 18).unwrap()),
            },
        );
        d.add_step(Point::new(80., 80.), 20, Style::default())
            .unwrap();
        if let Content::Step { mask, number, .. } = &d.objects[1].content {
            assert_eq!(*number, 2000);
            assert!(mask.width <= 16 && mask.height <= 16);
        } else {
            panic!("Expected a step");
        }
    }
    #[test]
    fn circular_steps_do_not_select_empty_bounding_box_corners() {
        let mut d = Document::new(200, 200);
        d.add_step(Point::new(80., 80.), 36, Style::default())
            .unwrap();
        assert_eq!(d.hit(Point::new(80., 80.), 0.), Some(d.objects[0].id));
        assert_eq!(d.hit(Point::new(97., 97.), 0.), None);
    }
    #[test]
    fn composition_snapshot_shares_content_without_copying_history() {
        let mut d = Document::new(100, 100);
        line(&mut d);
        line(&mut d);
        let snapshot = d.render_snapshot();
        assert_eq!(snapshot.objects, d.objects);
        assert!(Arc::ptr_eq(
            &snapshot.objects[0].points,
            &d.objects[0].points
        ));
        assert!(!snapshot.can_undo());
        assert!(!snapshot.can_redo());
    }
    #[test]
    fn glyph_memory_budget_rejects_excess_without_mutating_history() {
        let mut d = Document::new(100, 100);
        let mask = Arc::new(crate::glyphs::Mask {
            width: 2048,
            height: 1024,
            alpha: vec![0; 2048 * 1024],
        });
        assert!(!d.insert(
            Tool::Text,
            vec![Point::new(1., 1.)],
            Style::default(),
            Content::Text {
                text: Arc::from("A"),
                size: 24,
                wrap_width: None,
                background: Default::default(),
                tail: None,
                layout: TextLayout::Native,
                mask
            }
        ));
        assert_eq!(d.revision, 0);
        assert!(!d.can_undo());
    }
    #[test]
    fn text_rejects_empty_input_and_edit_is_one_undo_step() {
        let mut d = Document::new(600, 300);
        assert!(
            !d.add_text(Point::new(10., 10.), " \n ", 24, Style::default())
                .unwrap()
        );
        assert_eq!(d.revision, 0);
        assert!(
            d.add_text(
                Point::new(10., 10.),
                "ภาษาไทย\nEnglish",
                24,
                Style::default()
            )
            .unwrap()
        );
        let original = d.objects[0].clone();
        assert!(d.edit_text("แก้ไข\nEdited", 32).unwrap());
        assert!(d.undo());
        assert_eq!(d.objects[0], original);
        assert!(d.redo());
        assert_eq!(d.hit(Point::new(15., 40.), 0.), Some(original.id));
        assert!(
            !d.add_text(
                Point::new(10., 10.),
                &"x".repeat(4097),
                24,
                Style::default()
            )
            .unwrap()
        );
        assert!(
            !d.add_text(Point::new(10., 10.), "bad", 129, Style::default())
                .unwrap()
        );
    }
    #[test]
    fn steps_resume_from_current_maximum_including_cropped_objects() {
        let mut d = Document::new(200, 200);
        for x in [20., 80., 160.] {
            assert!(
                d.add_step(Point::new(x, 50.), 36, Style::default())
                    .unwrap()
            );
        }
        assert_eq!(d.step_numbers(), [1, 2, 3]);
        assert!(d.delete());
        assert!(
            d.add_step(Point::new(160., 100.), 36, Style::default())
                .unwrap()
        );
        assert_eq!(d.step_numbers(), [1, 2, 3]);
        d.selected = Some(d.objects[1].id);
        d.delete();
        d.set_crop(Rect::new(0, 0, 60, 180).unwrap());
        assert!(
            d.add_step(Point::new(30., 100.), 36, Style::default())
                .unwrap()
        );
        assert_eq!(d.step_numbers(), [1, 3, 4]);
        assert!(d.undo());
        assert_eq!(d.step_numbers(), [1, 3]);
        assert!(d.redo());
        assert_eq!(d.step_numbers(), [1, 3, 4]);
    }
    #[test]
    fn blur_interior_is_selectable_and_move_delete_are_reversible() {
        let mut d = Document::new(200, 200);
        assert!(!d.add_blur(Point::new(10., 10.), Point::new(10., 20.), 8));
        assert!(!d.add_blur(Point::new(10., 10.), Point::new(70., 70.), 65));
        assert!(d.add_blur(Point::new(10., 10.), Point::new(70., 70.), 64));
        d.undo();
        assert!(d.add_blur(Point::new(10., 10.), Point::new(70., 70.), 8));
        let original = d.objects[0].clone();
        assert_eq!(d.hit(Point::new(40., 40.), 0.), Some(original.id));
        assert!(d.move_selected(10., 5.));
        assert!(d.delete());
        assert!(d.undo());
        assert!(d.undo());
        assert_eq!(d.objects[0], original);
    }
    fn line(doc: &mut Document) {
        assert!(doc.add(
            Tool::Line,
            vec![Point::new(10., 10.), Point::new(80., 10.)],
            Style::default()
        ));
    }
    #[test]
    fn drawing_and_deletion_round_trip_history() {
        let mut d = Document::new(100, 100);
        line(&mut d);
        let id = d.objects[0].id;
        d.selected = Some(id);
        assert!(d.delete());
        assert!(d.objects.is_empty());
        assert!(d.undo());
        assert_eq!(d.objects[0].id, id);
        assert!(d.undo());
        assert!(d.objects.is_empty());
        assert!(d.redo());
        assert_eq!(d.objects[0].points[1], Point::new(80., 10.));
    }
    #[test]
    fn empty_gestures_and_invalid_styles_do_not_dirty_document() {
        let mut d = Document::new(100, 100);
        assert!(!d.add(Tool::Line, vec![Point::new(1., 1.); 2], Style::default()));
        assert!(!d.add(
            Tool::Pen,
            vec![Point::new(f64::NAN, 1.), Point::new(5., 5.)],
            Style::default()
        ));
        assert!(!d.add(
            Tool::Select,
            vec![Point::new(1., 1.), Point::new(5., 5.)],
            Style::default()
        ));
        assert!(!d.add(
            Tool::Line,
            vec![Point::new(1., 1.), Point::new(5., 5.)],
            Style {
                width: 0,
                ..Style::default()
            }
        ));
        assert!(!d.can_undo());
        assert_eq!(d.revision, 0);
    }
    #[test]
    fn move_and_style_changes_undo_without_copying_stroke_points() {
        let mut d = Document::new(100, 100);
        line(&mut d);
        let original = d.objects[0].points.clone();
        d.selected = Some(d.objects[0].id);
        assert!(d.move_selected(3., 4.));
        assert_eq!(d.objects[0].points[0], Point::new(13., 14.));
        assert!(d.restyle(Style {
            rgb: [0, 0, 255],
            width: 2
        }));
        assert!(d.undo());
        assert_eq!(d.objects[0].style, Style::default());
        assert!(d.undo());
        assert!(Arc::ptr_eq(&d.objects[0].points, &original));
    }
    #[test]
    fn crop_is_clamped_and_undo_restores_hidden_objects() {
        let mut d = Document::new(100, 100);
        line(&mut d);
        assert!(d.set_crop(Rect::new(-10, 5, 40, 25).unwrap()));
        assert_eq!(d.crop, Rect::new(0, 5, 30, 25).unwrap());
        assert_eq!(d.objects.len(), 1);
        assert!(!d.set_crop(Rect::new(90, 90, 5, 5).unwrap()));
        assert!(d.undo());
        assert_eq!(d.crop, Rect::new(0, 0, 100, 100).unwrap());
        assert_eq!(d.objects.len(), 1);
    }
    #[test]
    fn new_edit_after_undo_discards_redo() {
        let mut d = Document::new(100, 100);
        line(&mut d);
        line(&mut d);
        assert!(d.undo());
        line(&mut d);
        assert!(!d.redo());
    }
    #[test]
    fn late_save_does_not_mark_newer_edits_saved() {
        let mut d = Document::new(100, 100);
        line(&mut d);
        let saved = d.revision;
        line(&mut d);
        d.mark_kept(saved);
        assert!(!d.is_kept());
        assert!(d.undo());
        assert!(d.is_kept());
        assert!(d.redo());
        assert!(!d.is_kept());
    }
    #[test]
    fn hit_testing_prefers_topmost_and_does_not_select_rectangle_interior() {
        let mut d = Document::new(100, 100);
        line(&mut d);
        line(&mut d);
        assert_eq!(d.hit(Point::new(30., 11.), 2.), Some(d.objects[1].id));
        assert!(d.add(
            Tool::Rectangle,
            vec![Point::new(20., 20.), Point::new(80., 80.)],
            Style::default()
        ));
        assert_eq!(d.hit(Point::new(50., 50.), 2.), None);
        assert_eq!(d.hit(Point::new(20., 50.), 2.), Some(d.objects[2].id));
    }
    #[test]
    fn history_is_bounded_and_documents_are_independent() {
        let mut a = Document::new(100, 100);
        let b = Document::new(100, 100);
        for _ in 0..70 {
            line(&mut a);
        }
        let mut count = 0;
        while a.undo() {
            count += 1;
        }
        assert_eq!(count, 64);
        assert!(b.objects.is_empty());
        assert!(!b.can_undo());
    }
    #[test]
    fn display_mapping_uses_crop_origin_and_rejects_letterbox() {
        let v = View {
            crop: Rect::new(20, 10, 100, 60).unwrap(),
            x: 30.,
            y: 40.,
            scale: 0.5,
        };
        assert_eq!(v.point(35., 45., false), Some(Point::new(30., 20.)));
        assert_eq!(v.point(29., 45., false), None);
        assert_eq!(v.point(90., 80., true), Some(Point::new(120., 70.)));
    }
}

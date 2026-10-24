use crate::{
    combine::{ImageGroup, Viewport},
    editor::{
        Annotation, Background as TextBackground, BubbleFrame, Content, Document, PathRendering,
        Pattern, Point, Style, TextLayout, Tool,
    },
    effects::Effects,
    frame::{Frame, Rect},
    project::Project,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    path::Path,
    sync::Arc,
};

mod json_guard;

const INPUT_LIMIT: usize = 96 * 1024 * 1024;
const PIXELS: usize = crate::image_io::SOURCE_LIMIT;

#[derive(Clone, Debug)]
pub struct Failure {
    issues: Vec<(Location, Reason)>,
    total: usize,
}
#[derive(Clone, Copy, Debug)]
enum Location {
    Document,
    Asset(usize),
    Object(usize),
}
#[derive(Clone, Copy, Debug)]
enum Reason {
    Read,
    Json,
    Version,
    Bounds,
    Limit,
    Asset,
    Geometry,
    Style,
    Tool,
    Ownership,
}
impl Failure {
    fn new(location: Location, reason: Reason) -> Self {
        Self {
            issues: vec![(location, reason)],
            total: 1,
        }
    }
    fn push(&mut self, location: Location, reason: Reason) {
        self.total += 1;
        if self.issues.len() < 10 {
            self.issues.push((location, reason));
        }
    }
    pub fn message(&self, thai: bool) -> String {
        let mut out = if thai {
            "ไม่นำเข้าไฟล์ .neosnap นี้ ไม่มีงานถูกเปิดบางส่วนและต้นฉบับไม่เปลี่ยน สามารถแก้ต่อในรุ่น .NET ได้\n"
                .to_string()
        } else {
            "Nothing was imported from this .neosnap file. The original is unchanged; continue editing it in .NET.\n".to_string()
        };
        for (location, reason) in &self.issues {
            let label = match location {
                Location::Document => {
                    if thai {
                        "ไฟล์งาน".into()
                    } else {
                        "Document".into()
                    }
                }
                Location::Asset(i) => format!("{} {}", if thai { "ภาพ" } else { "Asset" }, i + 1),
                Location::Object(i) => format!("{} {}", if thai { "วัตถุ" } else { "Object" }, i + 1),
            };
            let (en, th) = match reason {
                Reason::Read => ("Cannot read the source file.", "อ่านไฟล์ต้นฉบับไม่ได้"),
                Reason::Json => (
                    "Invalid, duplicate, unknown or excessively nested JSON fields.",
                    "โครงสร้าง JSON ไม่ถูกต้อง มีฟิลด์ซ้ำ ไม่รู้จัก หรือซ้อนลึกเกินไป",
                ),
                Reason::Version => (
                    "Only neo-snap format version 1 is supported.",
                    "รองรับรูปแบบ neo-snap เวอร์ชัน 1 เท่านั้น",
                ),
                Reason::Bounds => (
                    "Invalid or unrepresentable canvas/crop.",
                    "ขนาดพื้นที่งานหรือครอปไม่ถูกต้องหรือแปลงไม่ได้",
                ),
                Reason::Limit => (
                    "Source, image memory, object or point limit exceeded.",
                    "เกินขีดจำกัดไฟล์ หน่วยความจำภาพ วัตถุ หรือจุดเส้น",
                ),
                Reason::Asset => (
                    "Missing, duplicate, invalid or non-embedded PNG asset.",
                    "ภาพ PNG หาย ซ้ำ ไม่ถูกต้อง หรือไม่ได้ฝังในไฟล์",
                ),
                Reason::Geometry => (
                    "Geometry or clipping cannot be preserved.",
                    "ไม่สามารถรักษาตำแหน่ง รูปร่าง หรือการครอปเดิมได้",
                ),
                Reason::Style => (
                    "Stored style/effect values cannot be preserved.",
                    "ไม่สามารถรักษาค่าสี เส้น ความโปร่งใส หรือเอฟเฟกต์เดิมได้",
                ),
                Reason::Tool => (
                    "The file contains an unknown or unsupported tool.",
                    "ไฟล์มีเครื่องมือที่ไม่รู้จักหรือยังไม่รองรับ",
                ),
                Reason::Ownership => (
                    "Image ownership or original layer order cannot be preserved.",
                    "ไม่สามารถรักษาความสัมพันธ์ภาพและวัตถุหรือลำดับเลเยอร์เดิมได้",
                ),
            };
            out.push_str(&format!("\n{label}: {}", if thai { th } else { en }));
        }
        if self.total > self.issues.len() {
            out.push_str(&format!(
                "\n{} {}",
                if thai {
                    "รายการเพิ่มเติม:"
                } else {
                    "Additional issues:"
                },
                self.total - self.issues.len()
            ));
        }
        out
    }
}
fn error(reason: Reason) -> Failure {
    Failure::new(Location::Document, reason)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Source {
    format: String,
    version: u32,
    width: u32,
    height: u32,
    background: Background,
    crop_rect: SourceRect,
    assets: Vec<Asset>,
    objects: Vec<Object>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Asset {
    id: String,
    data_url: String,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Background {
    Asset(String),
    Empty(()),
}
impl Background {
    fn as_ref(&self) -> Option<&String> {
        match self {
            Self::Asset(id) => Some(id),
            Self::Empty(()) => None,
        }
    }
}
#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
impl SourceRect {
    fn valid_stored_clip(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|n| n.is_finite())
            && self.width >= 1.
            && self.height >= 1.
    }

    fn rect(self) -> Result<Rect, Reason> {
        if [self.x, self.y, self.width, self.height]
            .iter()
            .any(|n| !n.is_finite() || n.fract() != 0. || *n < 0. || *n > 32767.)
        {
            return Err(Reason::Geometry);
        }
        Rect::new(
            self.x as i32,
            self.y as i32,
            self.width as u32,
            self.height as u32,
        )
        .map_err(|_| Reason::Geometry)
    }
    fn viewport(self, width: u32, height: u32) -> Result<Viewport, Reason> {
        let right = self.x + self.width;
        let bottom = self.y + self.height;
        if [self.x, self.y, self.width, self.height, right, bottom]
            .iter()
            .any(|n| !n.is_finite())
            || self.x < 0.
            || self.y < 0.
            || self.width <= 0.
            || self.height <= 0.
            || right > width as f64 + 0.000001
            || bottom > height as f64 + 0.000001
        {
            return Err(Reason::Geometry);
        }
        Ok(Viewport {
            left: self.x,
            top: self.y,
            right,
            bottom,
        })
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Object {
    tool: String,
    color: String,
    size: f64,
    opacity: f64,
    x1: Option<f64>,
    y1: Option<f64>,
    x2: Option<f64>,
    y2: Option<f64>,
    points: Option<Vec<Point>>,
    curve: Option<Point>,
    stroke: Option<String>,
    shadow: Option<Shadow>,
    outline: Option<Outline>,
    blur: Option<f64>,
    bubble_fill: Option<String>,
    bubble_border: Option<String>,
    bubble_border_width: Option<f64>,
    clip_rect: Option<SourceRect>,
    owner: Option<String>,
    child_id: Option<String>,
    id: Option<String>,
    asset_id: Option<String>,
    source_crop: Option<SourceRect>,
    number: Option<u32>,
    text: Option<String>,
    bubble_width: Option<f64>,
    bubble_height: Option<f64>,
    tail: Option<Point>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Shadow {
    enabled: bool,
    color: String,
    blur: f64,
    x: f64,
    y: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Outline {
    enabled: bool,
    color: String,
    width: f64,
}

pub fn load(path: &Path, budget: usize) -> Result<Project, Failure> {
    let f = std::fs::File::open(path).map_err(|_| error(Reason::Read))?;
    if f.metadata().map_err(|_| error(Reason::Read))?.len() > INPUT_LIMIT as u64 {
        return Err(error(Reason::Limit));
    }
    let mut bytes = vec![];
    f.take(INPUT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error(Reason::Read))?;
    decode(&bytes, budget)
}

pub fn decode(bytes: &[u8], budget: usize) -> Result<Project, Failure> {
    if bytes.len() > INPUT_LIMIT {
        return Err(error(Reason::Limit));
    }
    json_guard::check(bytes).map_err(|_| error(Reason::Json))?;
    let source: Source = serde_json::from_slice(bytes).map_err(|_| error(Reason::Json))?;
    if source.format != "neo-snap" || source.version != 1 {
        return Err(error(Reason::Version));
    }
    if source.assets.len() > 100 || source.objects.len() > 2000 {
        return Err(error(Reason::Limit));
    }
    let canvas = Rect::new(0, 0, source.width, source.height).map_err(|_| error(Reason::Bounds))?;
    if Frame::byte_len(canvas.width, canvas.height).map_err(|_| error(Reason::Limit))? > PIXELS {
        return Err(error(Reason::Limit));
    }
    let crop = source.crop_rect.rect().map_err(|_| error(Reason::Bounds))?;
    if canvas.intersection(crop) != Some(crop) {
        return Err(error(Reason::Bounds));
    }
    let assets = read_assets(&source.assets, budget)?;
    let frame = match source.background.as_ref() {
        Some(id) => {
            let f = assets.get(id).ok_or_else(|| error(Reason::Asset))?;
            if f.width != canvas.width || f.height != canvas.height {
                return Err(error(Reason::Bounds));
            }
            f.clone()
        }
        None => Arc::new(Frame::new(1, 1, vec![255; 4]).map_err(|_| error(Reason::Bounds))?),
    };
    let mut failures = Failure {
        issues: vec![],
        total: 0,
    };
    let mut objects = vec![];
    let mut owner_scales = HashMap::new();
    let mut point_count = 0usize;
    for (i, item) in source.objects.iter().enumerate() {
        point_count = point_count.saturating_add(item.points.as_ref().map_or(2, Vec::len));
        let owner_scale = match item.owner.as_ref() {
            Some(owner) => owner_scales.get(owner).copied().ok_or(Reason::Ownership),
            None => Ok(1.),
        };
        match owner_scale.and_then(|scale| convert(item, (i + 1) as u64, &assets, canvas, scale)) {
            Ok(o) => {
                if o.tool == Tool::Image {
                    let Content::Group { image } = &o.content else {
                        unreachable!()
                    };
                    owner_scales.insert(
                        item.id.as_ref().unwrap().clone(),
                        crate::combine::image_transform(&o, image.viewport).scale,
                    );
                }
                objects.push(o)
            }
            Err(reason) => failures.push(Location::Object(i), reason),
        }
    }
    if point_count > 1_000_000 {
        failures.push(Location::Document, Reason::Limit);
    }
    if failures.total > 0 {
        return Err(failures);
    }
    attach_owners(&source.objects, &mut objects)?;
    let mut document =
        Document::from_project(crop, source.background.as_ref().map(|_| canvas), objects)
            .map_err(|_| error(Reason::Ownership))?;
    document.mark_project_saved(u64::MAX);
    crate::project::validate(&frame, &document).map_err(|_| error(Reason::Limit))?;
    let retained = std::iter::once(frame.clone()).chain(document.image_sources());
    let mut seen = HashSet::new();
    let mut total = 0usize;
    for f in retained {
        if seen.insert(Arc::as_ptr(&f)) {
            total = total.saturating_add(f.bgra.len());
        }
    }
    if total > budget.min(PIXELS) {
        return Err(error(Reason::Limit));
    }
    Ok(Project { frame, document })
}

fn read_assets(source: &[Asset], budget: usize) -> Result<HashMap<String, Arc<Frame>>, Failure> {
    let mut assets = HashMap::new();
    let mut total = 0usize;
    for (i, a) in source.iter().enumerate() {
        let bad = |r| Failure::new(Location::Asset(i), r);
        if a.id.is_empty() || a.id.len() > 100 || assets.contains_key(&a.id) {
            return Err(bad(Reason::Asset));
        }
        let data = a
            .data_url
            .strip_prefix("data:image/png;base64,")
            .ok_or_else(|| bad(Reason::Asset))?;
        if data.len() > PIXELS.div_ceil(3) * 4 {
            return Err(bad(Reason::Limit));
        }
        let png = STANDARD.decode(data).map_err(|_| bad(Reason::Asset))?;
        if png.len() < 33 || &png[..8] != b"\x89PNG\r\n\x1a\n" || &png[12..16] != b"IHDR" {
            return Err(bad(Reason::Asset));
        }
        let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
        let needed = Frame::byte_len(width, height).map_err(|_| bad(Reason::Limit))?;
        total = total
            .checked_add(needed)
            .ok_or_else(|| bad(Reason::Limit))?;
        if total > budget.min(PIXELS) {
            return Err(bad(Reason::Limit));
        }
        let f = crate::image_io::decode(&png, needed).map_err(|_| bad(Reason::Asset))?;
        if f.width != width || f.height != height {
            return Err(bad(Reason::Asset));
        }
        assets.insert(a.id.clone(), Arc::new(f));
    }
    Ok(assets)
}
fn rgb(color: &str) -> Result<[u8; 3], Reason> {
    if color.len() != 7
        || !color.starts_with('#')
        || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
    {
        return Err(Reason::Style);
    }
    Ok([
        u8::from_str_radix(&color[1..3], 16).unwrap(),
        u8::from_str_radix(&color[3..5], 16).unwrap(),
        u8::from_str_radix(&color[5..7], 16).unwrap(),
    ])
}
fn scaled_integer(n: f64, scale: f64, min: i32, max: i32) -> Result<i32, Reason> {
    if !n.is_finite() || !scale.is_finite() || scale <= 0. {
        return Err(Reason::Style);
    }
    let value = n / scale;
    let rounded = value.round();
    if (value - rounded).abs() > 1e-6 * value.abs().max(1.)
        || rounded < min as f64
        || rounded > max as f64
    {
        Err(Reason::Style)
    } else {
        Ok(rounded as i32)
    }
}
fn point(x: Option<f64>, y: Option<f64>) -> Result<Point, Reason> {
    let p = Point::new(x.ok_or(Reason::Geometry)?, y.ok_or(Reason::Geometry)?);
    if valid_point(&p) {
        Ok(p)
    } else {
        Err(Reason::Geometry)
    }
}
fn valid_point(p: &Point) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.x.abs() <= 32767. && p.y.abs() <= 32767.
}
fn effects(o: &Object, scale: f64, text: bool) -> Result<Effects, Reason> {
    let mut e = Effects::default();
    if let Some(s) = &o.shadow {
        e.shadow.enabled = s.enabled;
        e.shadow.rgb = rgb(&s.color)?;
        e.shadow.blur = scaled_integer(s.blur, scale, 0, 32)? as u32;
        e.shadow.x = scaled_integer(s.x, scale, -32, 32)?;
        e.shadow.y = scaled_integer(s.y, scale, -32, 32)?;
    }
    if let Some(s) = &o.outline {
        e.outline.rgb = rgb(&s.color)?;
        if !s.width.is_finite() || !(0.0..=32767.).contains(&s.width) || (s.enabled && !text) {
            return Err(Reason::Style);
        }
        if text {
            e.outline.enabled = s.enabled;
            e.outline.width = scaled_integer(s.width, scale, 1, 16)? as u32;
        }
    }
    Ok(e)
}
fn convert(
    o: &Object,
    id: u64,
    assets: &HashMap<String, Arc<Frame>>,
    canvas: Rect,
    scale: f64,
) -> Result<Annotation, Reason> {
    let tool = match o.tool.as_str() {
        "pen" => Tool::Pen,
        "highlight" => Tool::Highlighter,
        "line" => Tool::Line,
        "arrow" => Tool::Arrow,
        "box" => Tool::Rectangle,
        "block" => Tool::FilledRectangle,
        "ellipse" => Tool::Ellipse,
        "ellipseFill" => Tool::FilledEllipse,
        "number" => Tool::Step,
        "blur" => Tool::Blur,
        "image" => Tool::Image,
        "text" => Tool::Text,
        "bubble" => Tool::Callout,
        _ => return Err(Reason::Tool),
    };
    if !o.extra.is_empty()
        || o.child_id
            .as_ref()
            .is_some_and(|s| s.is_empty() || s.len() > 100)
    {
        return Err(Reason::Style);
    }
    if o.blur.is_some_and(|n| !n.is_finite() || n <= 0.)
        || o.bubble_width
            .is_some_and(|n| !n.is_finite() || !(0.0..=32767.).contains(&n))
        || o.bubble_height
            .is_some_and(|n| !n.is_finite() || !(0.0..=32767.).contains(&n))
        || o.bubble_fill.as_deref().is_some_and(|c| rgb(c).is_err())
        || o.bubble_border.as_deref().is_some_and(|c| rgb(c).is_err())
        || o.bubble_border_width
            .is_some_and(|n| !n.is_finite() || !(0.0..=32767.).contains(&n))
    {
        return Err(Reason::Style);
    }
    if tool != Tool::Image && (o.id.is_some() || o.asset_id.is_some() || o.source_crop.is_some()) {
        return Err(Reason::Style);
    }
    let text_tool = matches!(tool, Tool::Text | Tool::Callout);
    let width_range = if tool == Tool::Step { (2, 32) } else { (1, 40) };
    let style = Style {
        rgb: rgb(&o.color)?,
        width: if text_tool {
            5
        } else {
            scaled_integer(o.size, scale, width_range.0, width_range.1)? as u32
        },
    };
    if !o.opacity.is_finite()
        || !(0.0..=1.).contains(&o.opacity)
        || (o.opacity * 100. - (o.opacity * 100.).round()).abs() > 1e-7
    {
        return Err(Reason::Style);
    }
    let pattern = if text_tool {
        match o.stroke.as_deref().unwrap_or("solid") {
            "solid" | "dashed" | "dotted" => Pattern::Solid,
            _ => return Err(Reason::Style),
        }
    } else {
        match o.stroke.as_deref().unwrap_or("solid") {
            "solid" => Pattern::Solid,
            "dashed" => Pattern::Dashed,
            "dotted" => Pattern::Dotted,
            _ => return Err(Reason::Style),
        }
    };
    if !tool.patterned() && pattern != Pattern::Solid && !text_tool {
        return Err(Reason::Style);
    }
    let mut points = if matches!(tool, Tool::Pen | Tool::Highlighter) {
        let p = o.points.as_ref().ok_or(Reason::Geometry)?;
        if p.is_empty() || p.len() > 100_000 || p.iter().any(|p| !valid_point(p)) {
            return Err(Reason::Geometry);
        }
        p.clone()
    } else if matches!(tool, Tool::Step | Tool::Text | Tool::Callout) {
        vec![point(o.x1, o.y1)?]
    } else {
        vec![point(o.x1, o.y1)?, point(o.x2, o.y2)?]
    };
    if let Some(c) = o.curve {
        if !matches!(tool, Tool::Line | Tool::Arrow) || !valid_point(&c) {
            return Err(Reason::Geometry);
        }
        points.push(c);
    }
    if o.points.is_some() && !matches!(tool, Tool::Pen | Tool::Highlighter) {
        return Err(Reason::Geometry);
    }
    let content = if tool == Tool::Image {
        let frame = assets
            .get(o.asset_id.as_ref().ok_or(Reason::Asset)?)
            .ok_or(Reason::Asset)?
            .clone();
        if o.id
            .as_ref()
            .is_none_or(|id| id.is_empty() || id.len() > 100)
            || o.owner.is_some()
        {
            return Err(Reason::Ownership);
        }
        let full = Rect::new(0, 0, frame.width, frame.height).map_err(|_| Reason::Geometry)?;
        let crop = o
            .source_crop
            .ok_or(Reason::Geometry)?
            .viewport(frame.width, frame.height)?;
        if points[1].x <= points[0].x || points[1].y <= points[0].y {
            return Err(Reason::Geometry);
        }
        let sx = (points[1].x - points[0].x) / (crop.right - crop.left);
        let sy = (points[1].y - points[0].y) / (crop.bottom - crop.top);
        if (sx - sy).abs() > 1e-9 * sx.max(sy) {
            return Err(Reason::Geometry);
        }
        Content::Group {
            image: Arc::new(ImageGroup {
                frame,
                canvas: full,
                source_clip: Some(full),
                viewport: crop,
            }),
        }
    } else if tool == Tool::Highlighter {
        Content::Highlight
    } else if tool == Tool::Step {
        let number = o.number.ok_or(Reason::Geometry)?;
        if !(1..=1_000_000).contains(&number) {
            return Err(Reason::Geometry);
        }
        let diameter = crate::editor::dotnet_step_diameter(number, style.width);
        Content::Step {
            number,
            diameter,
            mask: Arc::new(
                crate::glyphs::dotnet_step(number, style.width).map_err(|_| Reason::Limit)?,
            ),
        }
    } else if tool == Tool::Blur {
        Content::Blur {
            strength: scaled_integer(o.blur.ok_or(Reason::Style)?, scale, 1, 64)? as u32,
        }
    } else if text_tool {
        let text = o.text.as_deref().ok_or(Reason::Tool)?;
        if text.trim().is_empty() || text.chars().count() > 4096 {
            return Err(Reason::Limit);
        }
        let source_size = o.size / scale;
        if !source_size.is_finite() || !(0.0..=64.).contains(&source_size) {
            return Err(Reason::Style);
        }
        let size = (source_size * 4.).max(12.).round() as u32;
        let bubble = if tool == Tool::Callout {
            let width = match o.bubble_width.unwrap_or(260.) {
                0. => 260.,
                n => n / scale,
            };
            let min_height = o.bubble_height.unwrap_or(0.) / scale;
            if !width.is_finite()
                || !min_height.is_finite()
                || !(20.0..=32767.).contains(&width)
                || !(0.0..=32767.).contains(&min_height)
            {
                return Err(Reason::Geometry);
            }
            let padding = 14f64.min(width / 5.);
            let border_width = match o.bubble_border_width.unwrap_or(5.) {
                0. => 5.,
                n => n / scale,
            };
            if !border_width.is_finite() || !(1.0..=16.).contains(&border_width) {
                return Err(Reason::Style);
            }
            Some(BubbleFrame {
                width,
                min_height,
                padding,
                border_rgb: rgb(o.bubble_border.as_deref().unwrap_or(&o.color))?,
                border_width,
            })
        } else {
            None
        };
        let wrap_width = bubble.map(|b| (b.width - b.padding * 2.).max(1.).round() as u32);
        let mask = crate::glyphs::dotnet_text(text, size, wrap_width).map_err(|_| Reason::Limit)?;
        let tail = if let Some(frame) = bubble {
            let height = frame
                .min_height
                .max(mask.height as f64 + frame.padding * 2.);
            Some(match o.tail {
                Some(tail) => Point::new(tail.x / scale, tail.y / scale),
                None => {
                    let legacy = 24f64.min(frame.width / 4.);
                    Point::new(legacy, height + 16.)
                }
            })
        } else {
            if o.tail.is_some() {
                return Err(Reason::Geometry);
            }
            None
        };
        if tail.as_ref().is_some_and(|p| !valid_point(p)) {
            return Err(Reason::Geometry);
        }
        Content::Text {
            text: Arc::from(text),
            size,
            font: crate::editor::FontFamily::default(),
            wrap_width,
            mask: Arc::new(mask),
            background: TextBackground {
                enabled: bubble.is_some(),
                rgb: rgb(o.bubble_fill.as_deref().unwrap_or("#ffffff"))?,
            },
            tail,
            layout: TextLayout::DotNet {
                source_size,
                bubble,
            },
        }
    } else {
        Content::Shape
    };
    if (tool == Tool::Step) != o.number.is_some() {
        return Err(Reason::Style);
    }
    if matches!(
        tool,
        Tool::Rectangle | Tool::FilledRectangle | Tool::Ellipse | Tool::FilledEllipse | Tool::Blur
    ) && (points[0].x == points[1].x || points[0].y == points[1].y)
    {
        return Err(Reason::Geometry);
    }
    if let Some(clip) = o.clip_rect {
        if o.owner.is_some() {
            // The .NET renderer clips owned annotations to their image bounds and
            // ignores the stored clip rectangle, which can contain fractional data.
            if !clip.valid_stored_clip() {
                return Err(Reason::Geometry);
            }
        } else {
            let clip = clip.rect()?;
            // Non-workspace clips need ownership conversion, not a discarded clip.
            if clip != canvas {
                return Err(Reason::Geometry);
            }
        }
    }
    let rendering = if matches!(
        tool,
        Tool::Pen
            | Tool::Highlighter
            | Tool::Arrow
            | Tool::Step
            | Tool::Blur
            | Tool::Text
            | Tool::Callout
    ) {
        PathRendering::DotNet
    } else {
        PathRendering::Native
    };
    Ok(Annotation {
        id,
        parent: None,
        tool,
        points: Arc::new(points),
        style,
        pattern,
        rendering,
        opacity: (o.opacity * 100.).round() as u8,
        effects: effects(o, scale, text_tool)?,
        content,
    })
}

fn attach_owners(source: &[Object], objects: &mut [Annotation]) -> Result<(), Failure> {
    let mut images = HashMap::new();
    for (i, o) in source.iter().enumerate().filter(|(_, o)| o.tool == "image") {
        if images.insert(o.id.as_ref().unwrap().as_str(), i).is_some() {
            return Err(Failure::new(Location::Object(i), Reason::Ownership));
        }
    }
    let mut active = None;
    for (i, o) in source.iter().enumerate() {
        if let Some(owner) = &o.owner {
            let index = *images
                .get(owner.as_str())
                .ok_or_else(|| Failure::new(Location::Object(i), Reason::Ownership))?;
            if index >= i || (active != Some(index) && i != index + 1) {
                return Err(Failure::new(Location::Object(i), Reason::Ownership));
            }
            let parent = &objects[index];
            if parent.opacity != 100 || parent.effects.shadow.enabled {
                return Err(Failure::new(Location::Object(index), Reason::Ownership));
            }
            let Content::Group { image } = &parent.content else {
                unreachable!()
            };
            let t = crate::combine::image_transform(parent, image.viewport);
            let mut local = objects[i].clone();
            local.points = Arc::new(local.points.iter().map(|p| t.inverse(*p)).collect());
            local.parent = Some(parent.id);
            objects[i] = local;
            active = Some(index);
        } else {
            active = if o.tool == "image" { Some(i) } else { None };
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "neosnap_tests.rs"]
mod tests;

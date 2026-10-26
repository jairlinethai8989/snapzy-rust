use crate::{
    editor::{
        Annotation, Background, Content, Document, FontFamily, PathRendering, Pattern, Point,
        Style, Tool,
    },
    effects::Effects,
    frame::{Frame, Rect},
    glyphs::Mask,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::{Cursor, Read, Write},
    sync::Arc,
};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

const LIMIT: usize = 384 * 1024 * 1024;
const PIXELS: usize = crate::image_limits::SOURCE_LIMIT;
const MANIFEST_LIMIT: usize = 32 * 1024 * 1024;
const MASK_LIMIT: usize = 1024 * 1024;

pub struct Project {
    pub frame: Arc<Frame>,
    pub document: Document,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    schema: u32,
    writer: String,
    base: u64,
    crop: Rect,
    source_clip: Option<Rect>,
    images: Vec<Asset>,
    masks: Vec<Asset>,
    objects: Vec<Object>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Asset {
    id: u64,
    width: u32,
    height: u32,
    length: usize,
    sha256: String,
}
#[derive(Default)]
struct Owner {
    present: bool,
    value: Option<u64>,
}
impl Serialize for Owner {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Owner {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self {
            present: true,
            value: Option::<u64>::deserialize(deserializer)?,
        })
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Object {
    id: u64,
    #[serde(default)]
    parent: Owner,
    tool: Tool,
    points: Vec<Point>,
    style: Style,
    pattern: Pattern,
    #[serde(default)]
    rendering: PathRendering,
    opacity: u8,
    effects: Effects,
    content: Payload,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Payload {
    Group {
        asset: u64,
        canvas: Rect,
        source_clip: Option<Rect>,
        viewport: crate::combine::Viewport,
    },
    Image {
        asset: u64,
    },
    Shape,
    Highlight,
    Text {
        text: String,
        size: u32,
        #[serde(default)]
        font: Option<FontFamily>,
        wrap_width: Option<u32>,
        mask: u64,
        background: Background,
        tail: Option<Point>,
        #[serde(default)]
        layout: crate::editor::TextLayout,
    },
    Step {
        number: u32,
        diameter: u32,
        mask: u64,
    },
    Blur {
        strength: u32,
    },
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_point(p: &Point) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.x.abs() <= 32767. && p.y.abs() <= 32767.
}
fn valid_rect(r: Rect) -> Result<(), String> {
    Rect::new(r.x, r.y, r.width, r.height)?;
    if r.x.abs_diff(0) > 32767 || r.y.abs_diff(0) > 32767 {
        return Err("Invalid project bounds".into());
    }
    Ok(())
}
pub(crate) fn validate(frame: &Arc<Frame>, d: &Document) -> Result<(), String> {
    crate::combine::validate_tree(&d.objects)?;
    valid_rect(d.crop)?;
    if Frame::byte_len(d.crop.width, d.crop.height)? > crate::image_limits::IMAGE_LIMIT {
        return Err("Canvas exceeds 128 MiB".into());
    }
    if let Some(r) = d.source_clip {
        valid_rect(r)?;
        if r.intersection(Rect::new(0, 0, frame.width, frame.height)?) != Some(r) {
            return Err("Invalid source clip".into());
        }
    }
    if d.objects.len() > 2000 {
        return Err("Too many project objects".into());
    }
    let mut ids = HashSet::new();
    let mut images = HashSet::new();
    let mut masks = HashSet::new();
    let mut pixels = 0usize;
    let mut coverage = 0usize;
    let mut points = 0usize;
    for f in std::iter::once(frame).chain(d.objects.iter().filter_map(|o| match &o.content {
        Content::Image { frame } => Some(frame),
        Content::Group { image } => Some(&image.frame),
        _ => None,
    })) {
        if images.insert(Arc::as_ptr(f)) {
            if Frame::byte_len(f.width, f.height)? != f.bgra.len() {
                return Err("Incomplete source pixels".into());
            }
            pixels += f.bgra.len();
            if pixels > PIXELS {
                return Err("Project source images exceed 256 MiB".into());
            }
        }
    }
    for o in &d.objects {
        if !ids.insert(o.id)
            || o.id >= u64::MAX - 1
            || o.points.is_empty()
            || o.points.iter().any(|p| !valid_point(p))
            || !(1..=40).contains(&o.style.width)
            || o.opacity > 100
        {
            return Err("Invalid project object".into());
        }
        o.effects.validate()?;
        points += o.points.len();
        if points > 1_000_000 {
            return Err("Project has too many points".into());
        }
        let count = o.points.len();
        let legal = match (&o.content, o.tool) {
            (Content::Shape, Tool::Pen) | (Content::Highlight, Tool::Highlighter) => {
                match o.rendering {
                    PathRendering::Native => (2..=100_000).contains(&count),
                    PathRendering::DotNet => (1..=100_000).contains(&count),
                }
            }
            (Content::Shape, Tool::Line | Tool::Arrow) => (2..=3).contains(&count),
            (
                Content::Shape,
                Tool::Rectangle | Tool::Ellipse | Tool::FilledRectangle | Tool::FilledEllipse,
            )
            | (Content::Image { .. }, Tool::Image)
            | (Content::Group { .. }, Tool::Image)
            | (Content::Blur { .. }, Tool::Blur) => {
                count == 2 && o.points[0].x != o.points[1].x && o.points[0].y != o.points[1].y
            }
            (
                Content::Text {
                    text,
                    size,
                    wrap_width,
                    tail,
                    layout,
                    ..
                },
                Tool::Text | Tool::Callout,
            ) => {
                let layout = match layout {
                    crate::editor::TextLayout::Native => {
                        o.rendering == PathRendering::Native
                            && (8..=256).contains(size)
                            && wrap_width.is_none_or(|w| (24..=32767).contains(&w))
                    }
                    crate::editor::TextLayout::DotNet {
                        source_size,
                        bubble,
                    } => {
                        let source_ok = source_size.is_finite()
                            && (0.0..=64.).contains(source_size)
                            && *size == (*source_size * 4.).max(12.).round() as u32;
                        let bubble_ok = match (o.tool, bubble) {
                            (Tool::Text, None) => {
                                wrap_width.is_none_or(|w| (24..=32767).contains(&w))
                            }
                            (Tool::Callout, Some(frame)) => {
                                [
                                    frame.width,
                                    frame.min_height,
                                    frame.padding,
                                    frame.border_width,
                                ]
                                .iter()
                                .all(|v| v.is_finite())
                                    && (20.0..=32767.).contains(&frame.width)
                                    && (0.0..=32767.).contains(&frame.min_height)
                                    && frame.padding > 0.
                                    && frame.padding <= 14.
                                    && (1.0..=16.).contains(&frame.border_width)
                                    && *wrap_width
                                        == Some((frame.width - frame.padding * 2.).max(1.).round()
                                            as u32)
                            }
                            _ => false,
                        };
                        o.rendering == PathRendering::DotNet && source_ok && bubble_ok
                    }
                };
                count == 1
                    && !text.trim().is_empty()
                    && text.chars().count() <= 4096
                    && layout
                    && tail.as_ref().is_none_or(valid_point)
                    && (tail.is_some() == (o.tool == Tool::Callout))
            }
            (
                Content::Step {
                    number, diameter, ..
                },
                Tool::Step,
            ) => {
                count == 1
                    && *number > 0
                    && *number < u32::MAX
                    && match o.rendering {
                        PathRendering::Native => (20..=120).contains(diameter),
                        PathRendering::DotNet => {
                            (2..=32).contains(&o.style.width)
                                && *diameter
                                    == crate::editor::dotnet_step_diameter(*number, o.style.width)
                        }
                    }
            }
            _ => false,
        };
        let rendering_legal = o.rendering == PathRendering::Native
            || matches!(
                o.tool,
                Tool::Pen
                    | Tool::Highlighter
                    | Tool::Arrow
                    | Tool::Step
                    | Tool::Blur
                    | Tool::Text
                    | Tool::Callout
            );
        if !legal || !rendering_legal || (!o.tool.patterned() && o.pattern != Pattern::Solid) {
            return Err("Invalid tool geometry or content".into());
        }
        if let Content::Blur { strength } = o.content {
            if !(1..=64).contains(&strength) {
                return Err("Invalid blur settings".into());
            }
        }
        if let Content::Text { mask, .. } | Content::Step { mask, .. } = &o.content {
            if Frame::byte_len(mask.width, mask.height)? / 4 != mask.alpha.len() {
                return Err("Incomplete glyph coverage".into());
            }
            if masks.insert(Arc::as_ptr(mask)) {
                coverage += mask.alpha.len();
            }
            if coverage > MASK_LIMIT {
                return Err("Project glyphs exceed 1 MiB".into());
            }
        }
    }
    Ok(())
}
pub fn encode(frame: &Arc<Frame>, document: &Document) -> Result<Vec<u8>, String> {
    validate(frame, document)?;
    let mut frames = vec![frame.clone()];
    let mut masks: Vec<Arc<Mask>> = vec![];
    let mut objects = vec![];
    for o in &document.objects {
        let content = match &o.content {
            Content::Group { image } => {
                let id = frames
                    .iter()
                    .position(|f| Arc::ptr_eq(f, &image.frame))
                    .unwrap_or_else(|| {
                        frames.push(image.frame.clone());
                        frames.len() - 1
                    });
                Payload::Group {
                    asset: id as u64,
                    canvas: image.canvas,
                    source_clip: image.source_clip,
                    viewport: image.viewport,
                }
            }
            Content::Image { frame } => {
                let id = frames
                    .iter()
                    .position(|f| Arc::ptr_eq(f, frame))
                    .unwrap_or_else(|| {
                        frames.push(frame.clone());
                        frames.len() - 1
                    });
                Payload::Image { asset: id as u64 }
            }
            Content::Shape => Payload::Shape,
            Content::Highlight => Payload::Highlight,
            Content::Blur { strength } => Payload::Blur {
                strength: *strength,
            },
            Content::Text { mask, .. } | Content::Step { mask, .. } => {
                let id = masks
                    .iter()
                    .position(|m| Arc::ptr_eq(m, mask))
                    .unwrap_or_else(|| {
                        masks.push(mask.clone());
                        masks.len() - 1
                    }) as u64;
                match &o.content {
                    Content::Text {
                        text,
                        size,
                        font,
                        wrap_width,
                        background,
                        tail,
                        layout,
                        ..
                    } => Payload::Text {
                        text: text.to_string(),
                        size: *size,
                        font: Some(*font),
                        wrap_width: *wrap_width,
                        mask: id,
                        background: *background,
                        tail: *tail,
                        layout: *layout,
                    },
                    Content::Step {
                        number, diameter, ..
                    } => Payload::Step {
                        number: *number,
                        diameter: *diameter,
                        mask: id,
                    },
                    _ => unreachable!(),
                }
            }
        };
        objects.push(Object {
            id: o.id,
            parent: Owner {
                present: true,
                value: o.parent,
            },
            tool: o.tool,
            points: o.points.as_ref().clone(),
            style: o.style,
            pattern: o.pattern,
            rendering: o.rendering,
            opacity: o.opacity,
            effects: o.effects,
            content,
        });
    }
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut total = 0usize;
    let mut images = vec![];
    let mut glyphs = vec![];
    for (id, f) in frames.iter().enumerate() {
        let mut bytes = vec![];
        f.write_png(&mut bytes)?;
        total += bytes.len();
        if total > LIMIT {
            return Err("Project exceeds 384 MiB".into());
        }
        archive
            .start_file(format!("images/{id}.png"), options)
            .map_err(|e| e.to_string())?;
        archive.write_all(&bytes).map_err(|e| e.to_string())?;
        images.push(Asset {
            id: id as u64,
            width: f.width,
            height: f.height,
            length: bytes.len(),
            sha256: digest(&bytes),
        });
    }
    for (id, m) in masks.iter().enumerate() {
        let mut bytes = vec![];
        let mut png = png::Encoder::new(&mut bytes, m.width, m.height);
        png.set_color(png::ColorType::Grayscale);
        png.set_depth(png::BitDepth::Eight);
        let mut w = png.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&m.alpha).map_err(|e| e.to_string())?;
        w.finish().map_err(|e| e.to_string())?;
        total += bytes.len();
        if total > LIMIT {
            return Err("Project exceeds 384 MiB".into());
        }
        archive
            .start_file(format!("masks/{id}.png"), options)
            .map_err(|e| e.to_string())?;
        archive.write_all(&bytes).map_err(|e| e.to_string())?;
        glyphs.push(Asset {
            id: id as u64,
            width: m.width,
            height: m.height,
            length: bytes.len(),
            sha256: digest(&bytes),
        });
    }
    let manifest = Manifest {
        format: "snapzy-rust-project".into(),
        schema: 5,
        writer: env!("CARGO_PKG_VERSION").into(),
        base: 0,
        crop: document.crop,
        source_clip: document.source_clip,
        images,
        masks: glyphs,
        objects,
    };
    let bytes = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
    if bytes.len() > MANIFEST_LIMIT || total + bytes.len() > LIMIT {
        return Err("Project metadata exceeds its limit".into());
    }
    archive
        .start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    archive.write_all(&bytes).map_err(|e| e.to_string())?;
    let bytes = archive.finish().map_err(|e| e.to_string())?.into_inner();
    if bytes.len() > LIMIT {
        return Err("Project exceeds 384 MiB".into());
    }
    Ok(bytes)
}

// Bound archive metadata before the library allocates its central-directory index.
fn preflight(bytes: &[u8]) -> Result<usize, String> {
    if bytes.len() < 22 || bytes.len() > LIMIT {
        return Err("Invalid project file size".into());
    }
    let p = bytes.len() - 22;
    let e = &bytes[p..];
    let u16at = |i| u16::from_le_bytes([e[i], e[i + 1]]);
    let u32at = |i| u32::from_le_bytes([e[i], e[i + 1], e[i + 2], e[i + 3]]);
    if e[..4] != [80, 75, 5, 6]
        || u16at(4) != 0
        || u16at(6) != 0
        || u16at(8) != u16at(10)
        || u16at(20) != 0
        || !(1..=4101).contains(&u16at(10))
        || u32at(12) as usize + u32at(16) as usize != p
    {
        return Err("Unsupported or invalid project archive".into());
    }
    Ok(u16at(10) as usize)
}
fn asset_bytes(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    path: &str,
    a: &Asset,
) -> Result<Vec<u8>, String> {
    let mut f = archive.by_name(path).map_err(|e| e.to_string())?;
    if f.size() != a.length as u64 || a.length > LIMIT {
        return Err("Asset length mismatch".into());
    }
    let mut bytes = Vec::new();
    (&mut f)
        .take(a.length as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() != a.length || digest(&bytes) != a.sha256 {
        return Err("Project asset integrity check failed".into());
    }
    Ok(bytes)
}
fn pixels(bytes: &[u8], a: &Asset, mask: bool) -> Result<Vec<u8>, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_limits(png::Limits { bytes: PIXELS });
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    let color = if mask {
        png::ColorType::Grayscale
    } else {
        png::ColorType::Rgba
    };
    let len = Frame::byte_len(a.width, a.height)? / if mask { 4 } else { 1 };
    if info.width != a.width
        || info.height != a.height
        || info.color_type != color
        || info.bit_depth != png::BitDepth::Eight
        || info.animation_control.is_some()
        || len > if mask { MASK_LIMIT } else { PIXELS }
    {
        return Err("Invalid project PNG dimensions or format".into());
    }
    let mut out = vec![0; len];
    let info = reader.next_frame(&mut out).map_err(|e| e.to_string())?;
    if info.buffer_size() != len {
        return Err("Incomplete project PNG".into());
    }
    reader.finish().map_err(|e| e.to_string())?;
    if !mask {
        for p in out.chunks_exact_mut(4) {
            p.swap(0, 2);
        }
    }
    Ok(out)
}
#[cfg(test)]
pub fn decode(bytes: &[u8]) -> Result<Project, String> {
    decode_budget(bytes, PIXELS)
}
fn decode_budget(bytes: &[u8], budget: usize) -> Result<Project, String> {
    let count = preflight(bytes)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if archive.len() != count
        || archive.offset() != 0
        || archive.raw_zip64_extensible_data_sector().is_some()
    {
        return Err("Duplicate entries or unsupported ZIP features".into());
    }
    let mut names = HashSet::new();
    let mut total = 0u64;
    for i in 0..archive.len() {
        let f = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = f.name();
        let legal = name == "manifest.json"
            || ["images/", "masks/"].iter().any(|prefix| {
                name.strip_prefix(prefix)
                    .and_then(|n| n.strip_suffix(".png"))
                    .is_some_and(|id| id.parse::<u64>().is_ok_and(|n| n.to_string() == id))
            });
        total = total.checked_add(f.size()).ok_or("Archive size overflow")?;
        if !legal
            || !names.insert(name.to_string())
            || f.encrypted()
            || f.is_symlink()
            || !matches!(
                f.compression(),
                CompressionMethod::Stored | CompressionMethod::Deflated
            )
            || total > LIMIT as u64
        {
            return Err("Unsafe or oversized project entry".into());
        }
        // The library reads payloads; check schema-1 local headers against its directory metadata.
        let p = f.header_start() as usize;
        let h = bytes.get(p..p + 30).ok_or("Truncated local ZIP header")?;
        let u16at = |i| u16::from_le_bytes([h[i], h[i + 1]]);
        let u32at = |i| u32::from_le_bytes([h[i], h[i + 1], h[i + 2], h[i + 3]]);
        if h[..4] != [80, 75, 3, 4]
            || u16at(4) >= 45
            || u16at(6) & !0x800 != 0
            || u16at(28) != 0
            || u32at(14) != f.crc32()
            || u32at(18) as u64 != f.compressed_size()
            || u32at(22) as u64 != f.size()
            || bytes.get(p + 30..p + 30 + u16at(26) as usize) != Some(f.name_raw())
            || u16at(8)
                != if f.compression() == CompressionMethod::Stored {
                    0
                } else {
                    8
                }
        {
            return Err("Inconsistent or unsupported local ZIP metadata".into());
        }
    }
    let mut f = archive
        .by_name("manifest.json")
        .map_err(|e| e.to_string())?;
    if f.size() > MANIFEST_LIMIT as u64 {
        return Err("Project manifest is too large".into());
    }
    let mut json = Vec::new();
    (&mut f)
        .take(MANIFEST_LIMIT as u64 + 1)
        .read_to_end(&mut json)
        .map_err(|e| e.to_string())?;
    drop(f);
    let m: Manifest = serde_json::from_slice(&json).map_err(|e| e.to_string())?;
    if m.format != "snapzy-rust-project"
        || !matches!(m.schema, 1..=5)
        || m.objects.len() > 2000
        || m.images.len() + m.masks.len() + 1 != count
    {
        return Err("Unsupported project version or invalid manifest".into());
    }
    if m.objects.iter().any(|o| {
        (m.schema == 1 && (o.parent.present || matches!(o.content, Payload::Group { .. })))
            || (matches!(m.schema, 2..=5) && !o.parent.present)
            || (m.schema < 3 && o.rendering != PathRendering::Native)
            || (m.schema < 4
                && matches!(
                    o.content,
                    Payload::Text {
                        layout: crate::editor::TextLayout::DotNet { .. },
                        ..
                    }
                ))
    }) {
        return Err("Invalid ownership metadata for the project schema.".into());
    }
    let mut frames = HashMap::new();
    let mut masks = HashMap::new();
    let mut used = HashSet::new();
    let mut memory = 0usize;
    for a in &m.images {
        memory = memory
            .checked_add(Frame::byte_len(a.width, a.height)?)
            .ok_or("Image budget overflow")?;
        if memory > budget.min(PIXELS) || frames.contains_key(&a.id) {
            return Err("Project source budget or duplicate ID".into());
        }
        let path = format!("images/{}.png", a.id);
        let bytes = asset_bytes(&mut archive, &path, a)?;
        frames.insert(
            a.id,
            Arc::new(Frame::new(a.width, a.height, pixels(&bytes, a, false)?)?),
        );
    }
    memory = 0;
    for a in &m.masks {
        memory = memory
            .checked_add(Frame::byte_len(a.width, a.height)? / 4)
            .ok_or("Glyph budget overflow")?;
        if memory > MASK_LIMIT || masks.contains_key(&a.id) {
            return Err("Project glyph budget or duplicate ID".into());
        }
        let path = format!("masks/{}.png", a.id);
        let bytes = asset_bytes(&mut archive, &path, a)?;
        masks.insert(
            a.id,
            Arc::new(Mask {
                width: a.width,
                height: a.height,
                alpha: pixels(&bytes, a, true)?,
            }),
        );
    }
    let frame = frames.get(&m.base).ok_or("Missing source image")?.clone();
    used.insert((false, m.base));
    let mut objects = vec![];
    for o in m.objects {
        let content = match o.content {
            Payload::Group {
                asset,
                canvas,
                source_clip,
                viewport,
            } => {
                used.insert((false, asset));
                Content::Group {
                    image: Arc::new(crate::combine::ImageGroup {
                        frame: frames
                            .get(&asset)
                            .ok_or("Missing group source image")?
                            .clone(),
                        canvas,
                        source_clip,
                        viewport,
                    }),
                }
            }
            Payload::Shape => Content::Shape,
            Payload::Highlight => Content::Highlight,
            Payload::Blur { strength } => Content::Blur { strength },
            Payload::Image { asset } => {
                used.insert((false, asset));
                Content::Image {
                    frame: frames.get(&asset).ok_or("Missing image")?.clone(),
                }
            }
            Payload::Text {
                text,
                size,
                font,
                wrap_width,
                mask,
                background,
                tail,
                layout,
            } => {
                used.insert((true, mask));
                Content::Text {
                    text: Arc::from(text),
                    size,
                    font: font.unwrap_or(if m.schema < 5 {
                        FontFamily::SegoeUi
                    } else {
                        FontFamily::default()
                    }),
                    wrap_width,
                    mask: masks.get(&mask).ok_or("Missing glyph")?.clone(),
                    background,
                    tail,
                    layout,
                }
            }
            Payload::Step {
                number,
                diameter,
                mask,
            } => {
                used.insert((true, mask));
                Content::Step {
                    number,
                    diameter,
                    mask: masks.get(&mask).ok_or("Missing step glyph")?.clone(),
                }
            }
        };
        objects.push(Annotation {
            id: o.id,
            parent: o.parent.value,
            tool: o.tool,
            points: Arc::new(o.points),
            style: o.style,
            pattern: o.pattern,
            rendering: o.rendering,
            opacity: o.opacity,
            effects: o.effects,
            content,
        });
    }
    if used.len() != frames.len() + masks.len() {
        return Err("Unreferenced project assets".into());
    }
    let document = Document::from_project(m.crop, m.source_clip, objects)?;
    validate(&frame, &document)?;
    Ok(Project { frame, document })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{Point, Style, Tool};
    #[test]
    fn nested_combination_round_trips_editable_ownership_and_pixels() {
        let f = Arc::new(Frame::new(40, 30, [30, 60, 90, 255].repeat(1200)).unwrap());
        let mut d = Document::new(40, 30);
        d.add(
            Tool::Arrow,
            vec![Point::new(4., 10.), Point::new(30., 20.)],
            Style::default(),
        );
        let first = crate::combine::build(
            &[(f.clone(), d.clone()), (f.clone(), d.clone())],
            crate::combine::Layout::Vertical,
            2,
        )
        .unwrap();
        let nested = crate::combine::build(
            &[(first.frame, first.document), (f, d)],
            crate::combine::Layout::Horizontal,
            2,
        )
        .unwrap();
        let expected = crate::render::flatten(&nested.frame, &nested.document).unwrap();
        let bytes = encode(&nested.frame, &nested.document).unwrap();
        let mut loaded = decode(&bytes).unwrap();
        assert_eq!(loaded.document.objects, nested.document.objects);
        assert_eq!(
            crate::render::flatten(&loaded.frame, &loaded.document).unwrap(),
            expected
        );
        loaded.document.selected = loaded
            .document
            .objects
            .iter()
            .find(|o| o.tool == Tool::Arrow)
            .map(|o| o.id);
        assert!(loaded.document.move_selected(3., 2.));
        assert!(!loaded.document.is_project_saved());
        assert!(loaded.document.undo());
        assert!(loaded.document.is_project_saved());
        assert_eq!(
            crate::render::flatten(&loaded.frame, &loaded.document).unwrap(),
            expected
        );
    }
    #[test]
    fn schema_one_reader_preserves_old_pixels_and_rejects_new_ownership() {
        let (f, d) = fixture();
        let bytes = encode(&f, &d).unwrap();
        let old = rewrite(&bytes, |m| {
            m["schema"] = serde_json::json!(1);
            for object in m["objects"].as_array_mut().unwrap() {
                object.as_object_mut().unwrap().remove("parent");
                object.as_object_mut().unwrap().remove("rendering");
                if object["content"]["kind"] == "Text" {
                    object["content"].as_object_mut().unwrap().remove("layout");
                }
            }
        });
        let loaded = decode(&old).unwrap();
        assert_eq!(
            crate::render::flatten(&loaded.frame, &loaded.document).unwrap(),
            crate::render::flatten(&f, &d).unwrap()
        );
        assert!(
            decode(&rewrite(&old, |m| m["objects"][0]["parent"] =
                serde_json::json!(null)))
            .is_err()
        );
    }

    #[test]
    fn schema_two_defaults_to_native_rendering_and_cannot_claim_dotnet_geometry() {
        let (f, d) = fixture();
        let bytes = encode(&f, &d).unwrap();
        let old = rewrite(&bytes, |m| {
            m["schema"] = serde_json::json!(2);
            for object in m["objects"].as_array_mut().unwrap() {
                object.as_object_mut().unwrap().remove("rendering");
                if object["content"]["kind"] == "Text" {
                    object["content"].as_object_mut().unwrap().remove("layout");
                }
            }
        });
        assert!(decode(&old).is_ok());
        let invalid = rewrite(&old, |m| {
            m["objects"][0]["rendering"] = serde_json::json!("DotNet");
        });
        assert!(decode(&invalid).is_err());
    }

    #[test]
    fn schema_three_native_text_defaults_its_missing_layout_without_pixel_changes() {
        let (f, d) = fixture();
        let expected = crate::render::flatten(&f, &d).unwrap();
        let old = rewrite(&encode(&f, &d).unwrap(), |m| {
            m["schema"] = serde_json::json!(3);
            for object in m["objects"].as_array_mut().unwrap() {
                if object["content"]["kind"] == "Text" {
                    object["content"].as_object_mut().unwrap().remove("layout");
                }
            }
        });
        let loaded = decode(&old).unwrap();
        assert_eq!(
            crate::render::flatten(&loaded.frame, &loaded.document).unwrap(),
            expected
        );
        assert!(
            loaded
                .document
                .objects
                .iter()
                .filter_map(|o| match o.content {
                    Content::Text { layout, .. } => Some(layout),
                    _ => None,
                })
                .all(|layout| layout == crate::editor::TextLayout::Native)
        );
    }

    #[test]
    fn ownership_cycles_missing_owners_and_illegal_viewports_are_rejected() {
        let f = Arc::new(Frame::new(20, 20, vec![255; 1600]).unwrap());
        let d = Document::new(20, 20);
        let p = crate::combine::build(
            &[(f.clone(), d.clone()), (f, d)],
            crate::combine::Layout::Vertical,
            0,
        )
        .unwrap();
        let bytes = encode(&p.frame, &p.document).unwrap();
        for changed in [
            rewrite(&bytes, |m| {
                m["objects"][0].as_object_mut().unwrap().remove("parent");
            }),
            rewrite(&bytes, |m| m["objects"][0]["parent"] = serde_json::json!(1)),
            rewrite(&bytes, |m| {
                m["objects"][0]["parent"] = serde_json::json!(999)
            }),
            rewrite(&bytes, |m| {
                m["objects"][0]["content"]["viewport"]["right"] = serde_json::json!(0)
            }),
            rewrite(&bytes, |m| {
                m["objects"][0]["content"]["viewport"]["left"] = serde_json::json!(-1)
            }),
            rewrite(&bytes, |m| {
                m["objects"][0]["content"]["viewport"]["right"] = serde_json::json!(1e-310);
                m["objects"][0]["content"]["viewport"]["bottom"] = serde_json::json!(1e-310);
            }),
            rewrite(&bytes, |m| {
                m["objects"][0]["content"]["canvas"]["x"] = serde_json::json!(50000);
                m["objects"][0]["content"]["viewport"]["left"] = serde_json::json!(50000);
                m["objects"][0]["content"]["viewport"]["right"] = serde_json::json!(50020);
            }),
            rewrite(&bytes, |m| {
                m["objects"][0]["points"][1]["y"] = serde_json::json!(30)
            }),
        ] {
            assert!(decode(&changed).is_err());
        }
    }
    #[test]
    fn project_geometry_retains_full_f64_precision() {
        let f = Arc::new(Frame::new(100, 100, [255; 4].repeat(10000)).unwrap());
        let mut d = Document::new(100, 100);
        let p = Point::new(51.248178375505404, -36.573994842753436);
        assert!(d.add(
            Tool::Line,
            vec![p, Point::new(97.45365320034685, 52.314008204106244)],
            Style::default()
        ));
        let loaded = decode(&encode(&f, &d).unwrap()).unwrap();
        assert_eq!(loaded.document.objects[0].points, d.objects[0].points);
    }
    #[test]
    fn round_trip_retains_source_alpha_curves_and_exact_composed_pixels() {
        let frame = Arc::new(Frame::new(20, 20, [19, 28, 37, 0].repeat(400)).unwrap());
        let mut d = Document::new(20, 20);
        d.add(
            Tool::Arrow,
            vec![Point::new(-2., 3.), Point::new(15., 17.)],
            Style::default(),
        );
        d.objects[0].points = Arc::new(vec![
            Point::new(-2., 3.),
            Point::new(15., 17.),
            Point::new(2.25, -1.5),
        ]);
        let bytes = encode(&frame, &d).unwrap();
        let loaded = decode(&bytes).unwrap();
        assert_eq!(loaded.frame, frame);
        assert_eq!(loaded.document.objects, d.objects);
        assert_eq!(
            crate::render::flatten(&frame, &d).unwrap(),
            crate::render::flatten(&loaded.frame, &loaded.document).unwrap()
        );
        assert!(loaded.document.is_project_saved());
    }
    #[test]
    fn project_round_trip_retains_text_font_and_older_schema_defaults_to_segoe_ui() {
        let frame = Arc::new(Frame::new(120, 80, [255; 4].repeat(120 * 80)).unwrap());
        let mut document = Document::new(120, 80);
        document
            .add_text(Point::new(8., 12.), "ภาษาไทย", 24, Style::default())
            .unwrap();
        document
            .set_text_font(crate::editor::FontFamily::Sarabun)
            .unwrap();

        let encoded = encode(&frame, &document).unwrap();
        let loaded = decode(&encoded).unwrap();
        assert_eq!(
            loaded.document.objects[0].text_font(),
            Some(crate::editor::FontFamily::Sarabun)
        );

        let old = rewrite(&encoded, |manifest| {
            manifest["schema"] = serde_json::json!(4);
            manifest["objects"][0]["content"]
                .as_object_mut()
                .unwrap()
                .remove("font");
        });
        let old_loaded = decode(&old).unwrap();
        assert_eq!(
            old_loaded.document.objects[0].text_font(),
            Some(crate::editor::FontFamily::SegoeUi)
        );
    }
    fn fixture() -> (Arc<Frame>, Document) {
        let f = Arc::new(Frame::new(160, 120, [220, 230, 240, 255].repeat(160 * 120)).unwrap());
        let mut d = Document::new(160, 120);
        for tool in [
            Tool::Pen,
            Tool::Highlighter,
            Tool::Line,
            Tool::Arrow,
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::FilledRectangle,
            Tool::FilledEllipse,
        ] {
            assert!(d.add(
                tool,
                vec![Point::new(5., 6.), Point::new(75., 60.)],
                Style::default()
            ));
            if tool.patterned() {
                d.objects.last_mut().unwrap().pattern = Pattern::Dotted;
            }
            d.objects.last_mut().unwrap().opacity = 57;
        }
        d.add_text(
            Point::new(3., 5.),
            "ไทย English\nสองบรรทัด",
            14,
            Style::default(),
        )
        .unwrap();
        d.add_label(
            Point::new(10., 20.),
            "พูด",
            12,
            Style::default(),
            Background {
                enabled: true,
                rgb: [20, 60, 90],
            },
            true,
        )
        .unwrap();
        d.add_step(Point::new(70., 70.), 36, Style::default())
            .unwrap();
        d.add_blur(Point::new(20., 25.), Point::new(40., 45.), 8);
        d.add_image(f.clone()).unwrap();
        for o in &mut d.objects {
            o.effects.shadow.enabled = true;
            o.effects.outline.enabled = true;
        }
        d.crop = Rect::new(-10, -4, 180, 130).unwrap();
        (f, d)
    }
    #[test]
    fn every_tool_reopens_editable_with_shared_sources_and_glyphs() {
        let (f, d) = fixture();
        let p = decode(&encode(&f, &d).unwrap()).unwrap();
        assert_eq!(p.document.crop, d.crop);
        assert_eq!(p.document.source_clip, d.source_clip);
        assert_eq!(p.document.objects, d.objects);
        assert_eq!(
            crate::render::flatten(&f, &d).unwrap(),
            crate::render::flatten(&p.frame, &p.document).unwrap()
        );
        let Content::Image { frame } = &p.document.objects.last().unwrap().content else {
            panic!("missing image")
        };
        assert!(Arc::ptr_eq(frame, &p.frame));
    }
    #[test]
    fn combining_every_tool_preserves_source_pixels_and_editable_nested_work() {
        let (f, d) = fixture();
        let expected = crate::render::flatten(&f, &d).unwrap();
        let original = d.objects.clone();
        for layout in [
            crate::combine::Layout::Vertical,
            crate::combine::Layout::Horizontal,
        ] {
            let combined =
                crate::combine::build(&[(f.clone(), d.clone()), (f.clone(), d.clone())], layout, 7)
                    .unwrap();
            let loaded = decode(&encode(&combined.frame, &combined.document).unwrap()).unwrap();
            let actual = crate::render::flatten(&loaded.frame, &loaded.document).unwrap();
            for (ox, oy) in [
                (0, 0),
                match layout {
                    crate::combine::Layout::Vertical => (0, expected.height + 7),
                    _ => (expected.width + 7, 0),
                },
            ] {
                for y in 0..expected.height {
                    let start = (((y + oy) * actual.width + ox) * 4) as usize;
                    let row = (y * expected.width * 4) as usize;
                    assert_eq!(
                        &actual.bgra[start..start + expected.width as usize * 4],
                        &expected.bgra[row..row + expected.width as usize * 4],
                        "layout {layout:?}, row {y}"
                    );
                }
            }
            assert_eq!(loaded.document.objects.len(), original.len() * 2 + 2);
            assert_eq!(d.objects, original);
            let nested = crate::combine::build(
                &[
                    (loaded.frame.clone(), loaded.document.clone()),
                    (f.clone(), d.clone()),
                ],
                crate::combine::Layout::Vertical,
                0,
            )
            .unwrap();
            let round_trip = decode(&encode(&nested.frame, &nested.document).unwrap()).unwrap();
            assert_eq!(round_trip.document.objects, nested.document.objects);
            assert_eq!(
                crate::render::flatten(&round_trip.frame, &round_trip.document).unwrap(),
                crate::render::flatten(&nested.frame, &nested.document).unwrap()
            );
        }
    }
    fn rewrite(bytes: &[u8], mutate: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
        let mut r = ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut entries = vec![];
        let mut change = Some(mutate);
        for i in 0..r.len() {
            let mut file = r.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut bytes = vec![];
            file.read_to_end(&mut bytes).unwrap();
            if name == "manifest.json" {
                let mut json = serde_json::from_slice(&bytes).unwrap();
                change.take().unwrap()(&mut json);
                bytes = serde_json::to_vec(&json).unwrap();
            }
            entries.push((name, bytes));
        }
        let mut w = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            w.start_file(
                name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
            w.write_all(&bytes).unwrap();
        }
        w.finish().unwrap().into_inner()
    }
    #[test]
    fn malformed_metadata_and_payloads_are_rejected_without_panicking() {
        let (f, d) = fixture();
        let bytes = encode(&f, &d).unwrap();
        for mutation in [
            ("schema", serde_json::json!(6)),
            ("format", serde_json::json!("neo-snap")),
            ("surprise", serde_json::json!(true)),
        ] {
            assert!(
                decode(&rewrite(&bytes, |m| {
                    m[mutation.0] = mutation.1;
                }))
                .is_err()
            );
        }
        for changed in [
            rewrite(&bytes, |m| {
                m["objects"][0]["points"] = serde_json::json!([])
            }),
            rewrite(&bytes, |m| {
                m["objects"][0]["opacity"] = serde_json::json!(101)
            }),
            rewrite(&bytes, |m| {
                m["objects"][0]["id"] = serde_json::json!(u64::MAX)
            }),
            rewrite(&bytes, |m| {
                m["objects"][1]["id"] = m["objects"][0]["id"].clone()
            }),
            rewrite(&bytes, |m| {
                m["images"][0]["width"] = serde_json::json!(32767)
            }),
            rewrite(&bytes, |m| {
                m["images"][0]["sha256"] = serde_json::json!("bad")
            }),
            bytes[..bytes.len() - 1].to_vec(),
        ] {
            assert!(decode(&changed).is_err());
        }
    }
    #[test]
    fn unsafe_names_duplicate_entries_and_mismatched_local_headers_are_rejected() {
        let (f, d) = fixture();
        let bytes = encode(&f, &d).unwrap();
        let mut bad = bytes.clone();
        bad[30] = b'.';
        assert!(decode(&bad).is_err());
        let mut bad = bytes.clone();
        bad[18] ^= 1;
        assert!(decode(&bad).is_err());
        let mut bad = bytes.clone();
        let n = bad.len();
        bad[n - 12..n - 10].copy_from_slice(&4102u16.to_le_bytes());
        assert!(decode(&bad).is_err());
    }
    #[test]
    fn atomic_replace_preserves_old_file_on_write_failure_and_overwrites_on_success() {
        let root = std::env::temp_dir().join(format!(
            "snapzy-project-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("Thai-งาน.snapzy");
        std::fs::write(&path, b"original").unwrap();
        assert!(
            atomic_write(&path, |f| {
                f.write_all(b"partial").map_err(|e| e.to_string())?;
                Err("injected write failure".into())
            })
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        atomic_write(&path, |f| {
            f.write_all(b"replacement").map_err(|e| e.to_string())
        })
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"replacement");
        use std::os::windows::fs::OpenOptionsExt;
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        assert!(
            atomic_write(&path, |f| f
                .write_all(b"must not replace")
                .map_err(|e| e.to_string()))
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"replacement");
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        drop(locked);
        std::fs::remove_file(&path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn source_reservation_and_invalid_objects_reject_without_partial_publication() {
        let (f, d) = fixture();
        let bytes = encode(&f, &d).unwrap();
        assert!(decode_budget(&bytes, f.bgra.len() - 1).is_err());
        assert!(decode_budget(&bytes, f.bgra.len()).is_ok());
        for value in [0, 41] {
            assert!(
                decode(&rewrite(&bytes, |m| m["objects"][0]["style"]["width"] =
                    serde_json::json!(value)))
                .is_err()
            );
        }
        assert!(
            decode(&rewrite(&bytes, |m| m["objects"][0]["effects"]["shadow"]
                ["blur"] = serde_json::json!(33)))
            .is_err()
        );
        assert!(
            decode(&rewrite(&bytes, |m| m["objects"][0]["tool"] =
                serde_json::json!("Select")))
            .is_err()
        );
        assert!(
            decode(&rewrite(&bytes, |m| m["objects"][0]["points"][0]["x"] =
                serde_json::json!(32768)))
            .is_err()
        );
        assert!(
            decode(&rewrite(&bytes, |m| m["objects"][8]["content"]["text"] =
                serde_json::json!("x".repeat(4097))))
            .is_err()
        );
    }
    #[test]
    fn corrupted_zip_payload_crc_and_truncated_headers_are_rejected() {
        let (f, d) = fixture();
        let bytes = encode(&f, &d).unwrap();
        let mut bad = bytes.clone();
        bad[60] ^= 32;
        assert!(decode(&bad).is_err());
        for length in [0, 1, 21, 22, 30, bytes.len() / 2] {
            assert!(decode(&bytes[..length]).is_err());
        }
        let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(
            "../outside.png",
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(b"unsafe").unwrap();
        assert!(decode(&zip.finish().unwrap().into_inner()).is_err());
    }
    #[test]
    fn project_save_open_timing_fixtures_preserve_native_pixels() {
        for (name, width, height) in [("plain", 160, 120), ("large-scroll", 600, 6000)] {
            let f = Arc::new(
                Frame::new(
                    width,
                    height,
                    [200, 230, 240, 255].repeat((width * height) as usize),
                )
                .unwrap(),
            );
            let d = Document::new(width, height);
            let timer = std::time::Instant::now();
            let bytes = encode(&f, &d).unwrap();
            let saved = timer.elapsed();
            let timer = std::time::Instant::now();
            let p = decode(&bytes).unwrap();
            let opened = timer.elapsed();
            assert_eq!(p.frame, f);
            eprintln!(
                "project-timing {name} source-bytes={} archive-bytes={} encode-ms={:.2} decode-ms={:.2}",
                f.bgra.len(),
                bytes.len(),
                saved.as_secs_f64() * 1000.,
                opened.as_secs_f64() * 1000.
            );
        }
    }

    #[test]
    fn project_roundtrip_accepts_sources_larger_than_the_previous_32_mib_limit() {
        let width = 3000;
        let height = 3000;
        let frame = Arc::new(
            Frame::new(
                width,
                height,
                [17, 83, 191, 255].repeat(width as usize * height as usize),
            )
            .unwrap(),
        );
        assert!(frame.bgra.len() > 32 * 1024 * 1024);
        let document = Document::new(width, height);
        let archive = encode(&frame, &document).unwrap();
        let reopened = decode(&archive).unwrap();
        assert_eq!(*reopened.frame, *frame);
        assert_eq!(reopened.document.crop, document.crop);
    }
}

pub fn load(path: &std::path::Path, budget: usize) -> Result<Project, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > LIMIT as u64 {
        return Err("Project exceeds 384 MiB".into());
    }
    let mut bytes = vec![];
    file.take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    decode_budget(&bytes, budget)
}
pub fn save(path: &std::path::Path, frame: &Arc<Frame>, document: &Document) -> Result<(), String> {
    let bytes = encode(frame, document)?;
    atomic_write(path, |f| f.write_all(&bytes).map_err(|e| e.to_string()))
}
fn atomic_write(
    path: &std::path::Path,
    write: impl FnOnce(&mut std::fs::File) -> Result<(), String>,
) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let name = path
        .file_name()
        .ok_or("Choose a project filename")?
        .to_string_lossy();
    let temp = path.with_file_name(format!(".{name}.{}-{stamp}.tmp", std::process::id()));
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        write(&mut f)?;
        f.flush().map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        drop(f);
        let from: Vec<u16> = temp.as_os_str().encode_wide().chain([0]).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        if unsafe {
            MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

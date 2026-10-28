use crate::{
    editor::{Background, FontFamily, Order, Pattern, Point, Tool},
    effects::Effects,
    frame::{Frame, Rect},
    interaction::Session,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

const MESSAGE_LIMIT: usize = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditorCommand {
    Choose {
        tool: Tool,
    },
    SetColor {
        rgb: [u8; 3],
    },
    SetParameter {
        value: u32,
    },
    SetOpacity {
        value: u32,
    },
    SetPattern {
        pattern: Pattern,
    },
    SetFont {
        font: FontFamily,
    },
    SetTextBackground {
        background: Background,
    },
    ExpandCanvas {
        width: u32,
        height: u32,
    },
    ResizeCanvas {
        width: u32,
        height: u32,
    },
    SetEffects {
        effects: Effects,
    },
    AddText {
        point: Point,
        text: String,
        callout: bool,
    },
    DuplicateArea {
        start: Point,
        end: Point,
    },
    DuplicateSelection,
    Press {
        point: Point,
        tolerance: f64,
    },
    Move {
        point: Point,
    },
    Release {
        point: Point,
    },
    ConfirmCrop,
    Cancel,
    Delete,
    Undo,
    Redo,
    ResetCurve,
    Reorder {
        order: ServiceOrder,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceOrder {
    Front,
    Forward,
    Backward,
    Back,
}

impl From<ServiceOrder> for Order {
    fn from(value: ServiceOrder) -> Self {
        match value {
            ServiceOrder::Front => Self::Front,
            ServiceOrder::Forward => Self::Forward,
            ServiceOrder::Backward => Self::Backward,
            ServiceOrder::Back => Self::Back,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct EditorView {
    pub width: u32,
    pub height: u32,
    pub revision: u64,
    pub tool: Tool,
    pub color: [u8; 3],
    pub parameter: u32,
    pub opacity: u8,
    pub pattern: Pattern,
    pub font: FontFamily,
    pub text_background: Background,
    pub selected: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub can_delete: bool,
    pub crop_ready: bool,
    pub effects: Effects,
    pub outline_supported: bool,
    pub dirty: bool,
    pub selection_overlay: Option<SelectionOverlay>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionKind {
    Image,
    Shape,
    Region,
    Line,
    Stroke,
    Text,
    Step,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HandleRole {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    Start,
    End,
    Curve,
    Tail,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SelectionBounds {
    pub start: Point,
    pub end: Point,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct SelectionHandle {
    pub role: HandleRole,
    pub point: Point,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SelectionOverlay {
    pub kind: SelectionKind,
    pub bounds: Option<SelectionBounds>,
    pub handles: Vec<SelectionHandle>,
    pub movable: bool,
}

pub struct EditorService {
    source: Arc<Frame>,
    session: Session,
    project_path: Option<PathBuf>,
}

impl EditorService {
    fn ensure_duplication_idle(&self) -> Result<(), String> {
        if self.session.gesture.is_some()
            || self.session.resize.is_some()
            || self.session.shape_resize.is_some()
            || self.session.tail_drag.is_some()
            || self.session.curve_drag.is_some()
            || self.session.line_endpoint_drag.is_some()
            || self.session.path_resize.is_some()
            || self.session.step_resize.is_some()
        {
            return Err("Finish or cancel the current edit before duplicating.".into());
        }
        Ok(())
    }

    pub fn open_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > crate::image_limits::INPUT_FILE_LIMIT {
            return Err("The image file is too large.".into());
        }
        let source = Arc::new(crate::image_io::decode(
            bytes,
            crate::image_limits::IMAGE_LIMIT,
        )?);
        let session = Session::new(source.width, source.height);
        Ok(Self {
            source,
            session,
            project_path: None,
        })
    }

    pub fn open_path(path: &Path) -> Result<Self, String> {
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        if metadata.len() > crate::image_limits::INPUT_FILE_LIMIT as u64 {
            return Err("The image file is too large.".into());
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        Self::open_bytes(&bytes)
    }

    pub fn open_project(path: &Path) -> Result<Self, String> {
        let project = crate::project::load(path, crate::image_limits::SOURCE_LIMIT)?;
        let mut session = Session::new(project.frame.width, project.frame.height);
        session.document = project.document;
        Ok(Self {
            source: project.frame,
            session,
            project_path: Some(path.to_path_buf()),
        })
    }

    pub fn snapshot_project(&self, path: &Path) -> Result<(), String> {
        self.ensure_duplication_idle()?;
        crate::project::save(path, &self.source, &self.session.document)
    }

    pub fn combine_projects(
        paths: &[PathBuf],
        layout: crate::combine::Layout,
        gap: u32,
    ) -> Result<Self, String> {
        if !(2..=16).contains(&paths.len()) {
            return Err("Choose 2-16 images to combine.".into());
        }
        let mut sources = Vec::with_capacity(paths.len());
        for path in paths {
            let project = crate::project::load(path, crate::image_limits::SOURCE_LIMIT)?;
            sources.push((project.frame, project.document));
        }
        let project = crate::combine::build(&sources, layout, gap)?;
        let mut session = Session::new(project.document.crop.width, project.document.crop.height);
        session.document = project.document;
        Ok(Self {
            source: project.frame,
            session,
            project_path: None,
        })
    }

    pub fn save_project(&mut self, path: &Path) -> Result<(), String> {
        crate::project::save(path, &self.source, &self.session.document)?;
        self.session
            .document
            .mark_project_saved(self.session.document.revision);
        self.project_path = Some(path.to_path_buf());
        Ok(())
    }

    pub fn view(&self) -> EditorView {
        let document = &self.session.document;
        EditorView {
            width: document.crop.width,
            height: document.crop.height,
            revision: document.revision,
            tool: self.session.tool,
            color: self.session.style.rgb,
            parameter: self.session.parameter(),
            opacity: self.session.opacity(),
            pattern: self.session.pattern(),
            font: self.session.font(),
            text_background: self.session.text_background(),
            selected: document.selected.is_some(),
            can_undo: document.can_undo(),
            can_redo: document.can_redo(),
            can_delete: document.selected.is_some(),
            crop_ready: self.session.crop_draft().is_some(),
            effects: self.session.effects(),
            outline_supported: matches!(self.session.effective_tool(), Tool::Text | Tool::Callout),
            dirty: !self.session.document.is_project_saved(),
            selection_overlay: self
                .selection_overlay()
                .map(|overlay| visible_overlay(overlay, document.crop)),
        }
    }

    fn selection_overlay(&self) -> Option<SelectionOverlay> {
        let selected = self.session.document.selected?;
        let object = self
            .session
            .document
            .objects
            .iter()
            .find(|object| object.id == selected)?;
        let transform = self.session.selected_transform();
        let bounds = |(start, end): (Point, Point)| SelectionBounds {
            start: transform.point(start),
            end: transform.point(end),
        };
        let moved_bounds = |mut visible: (Point, Point)| {
            if let Some((dx, dy)) = self.session.selection_move_delta_local() {
                visible.0.x += dx;
                visible.0.y += dy;
                visible.1.x += dx;
                visible.1.y += dy;
            }
            visible
        };
        let rectangular = |kind, visible: (Point, Point)| {
            let visible = bounds(moved_bounds(visible));
            SelectionOverlay {
                kind,
                bounds: Some(visible),
                handles: rectangular_handles((visible.start, visible.end)),
                movable: true,
            }
        };
        let framed = |kind, handles: Vec<(crate::shape_frame::Handle, Point)>| {
            if handles.is_empty() {
                return None;
            }
            let start = Point::new(
                handles
                    .iter()
                    .map(|(_, point)| point.x)
                    .fold(f64::INFINITY, f64::min),
                handles
                    .iter()
                    .map(|(_, point)| point.y)
                    .fold(f64::INFINITY, f64::min),
            );
            let end = Point::new(
                handles
                    .iter()
                    .map(|(_, point)| point.x)
                    .fold(f64::NEG_INFINITY, f64::max),
                handles
                    .iter()
                    .map(|(_, point)| point.y)
                    .fold(f64::NEG_INFINITY, f64::max),
            );
            Some(SelectionOverlay {
                kind,
                bounds: Some(SelectionBounds { start, end }),
                handles: handles
                    .into_iter()
                    .map(|(role, point)| SelectionHandle {
                        role: shape_handle_role(role),
                        point,
                    })
                    .collect(),
                movable: true,
            })
        };
        match object.tool {
            Tool::Image => Some(rectangular(
                SelectionKind::Image,
                self.session
                    .resize
                    .as_ref()
                    .filter(|resize| resize.original.id == object.id)
                    .map(crate::text_frame::Resize::bounds)
                    .unwrap_or_else(|| object.bounds()),
            )),
            Tool::Rectangle | Tool::Ellipse | Tool::FilledRectangle | Tool::FilledEllipse => {
                framed(SelectionKind::Shape, self.session.shape_handles())
            }
            Tool::Blur => framed(SelectionKind::Region, self.session.shape_handles()),
            Tool::Pen | Tool::Highlighter => {
                framed(SelectionKind::Stroke, self.session.path_handles())
            }
            Tool::Line | Tool::Arrow => {
                let endpoint_draft = self.session.line_endpoint_draft();
                let curve_draft = self.session.curve_draft();
                let visible = endpoint_draft
                    .as_ref()
                    .or(curve_draft.as_ref())
                    .unwrap_or(object);
                let mut start = *visible.points.first()?;
                let mut end = visible.points[1];
                let mut curve = visible.curve_midpoint()?;
                if let Some((dx, dy)) = self.session.selection_move_delta_local() {
                    for point in [&mut start, &mut end, &mut curve] {
                        point.x += dx;
                        point.y += dy;
                    }
                }
                let start = transform.point(start);
                let end = transform.point(end);
                let curve = transform.point(curve);
                Some(SelectionOverlay {
                    kind: SelectionKind::Line,
                    bounds: Some(SelectionBounds { start, end }),
                    handles: vec![
                        SelectionHandle {
                            role: HandleRole::Start,
                            point: start,
                        },
                        SelectionHandle {
                            role: HandleRole::End,
                            point: end,
                        },
                        SelectionHandle {
                            role: HandleRole::Curve,
                            point: curve,
                        },
                    ],
                    movable: true,
                })
            }
            Tool::Text | Tool::Callout => {
                let visible = bounds(moved_bounds(
                    self.session
                        .resize
                        .as_ref()
                        .filter(|resize| resize.original.id == object.id)
                        .map(crate::text_frame::Resize::bounds)
                        .unwrap_or_else(|| object.bounds()),
                ));
                let mut handles = crate::text_frame::positions((visible.start, visible.end))
                    .into_iter()
                    .filter(|(role, _)| {
                        !matches!(
                            role,
                            crate::text_frame::Handle::Top | crate::text_frame::Handle::Bottom
                        )
                    })
                    .map(|(role, point)| SelectionHandle {
                        role: text_handle_role(role),
                        point,
                    })
                    .collect::<Vec<_>>();
                let mut tail = object.tail_tip();
                if let (Some(point), Some(drag)) = (&mut tail, &self.session.tail_drag) {
                    point.x += drag.end.x - drag.start.x;
                    point.y += drag.end.y - drag.start.y;
                }
                if let (Some(point), Some((dx, dy))) =
                    (&mut tail, self.session.selection_move_delta_local())
                {
                    point.x += dx;
                    point.y += dy;
                }
                if let Some(point) = tail.map(|point| transform.point(point)) {
                    handles.push(SelectionHandle {
                        role: HandleRole::Tail,
                        point,
                    });
                }
                Some(SelectionOverlay {
                    kind: SelectionKind::Text,
                    bounds: Some(visible),
                    handles,
                    movable: true,
                })
            }
            Tool::Step => framed(SelectionKind::Step, self.session.step_handles()),
            _ => None,
        }
    }

    pub fn apply(&mut self, command: EditorCommand) -> Result<EditorView, String> {
        match command {
            EditorCommand::Choose { tool } => self.session.choose(tool),
            EditorCommand::SetColor { rgb } => {
                self.session.style.rgb = rgb;
                self.session.document.restyle(self.session.style);
            }
            EditorCommand::SetParameter { value } => {
                self.session.set_parameter(value)?;
            }
            EditorCommand::SetOpacity { value } => {
                self.session.set_opacity(value);
            }
            EditorCommand::SetPattern { pattern } => {
                self.session.set_pattern(pattern);
            }
            EditorCommand::SetFont { font } => {
                self.session.set_font(font)?;
            }
            EditorCommand::SetTextBackground { background } => {
                self.session.set_background(background);
            }
            EditorCommand::ExpandCanvas { width, height } => {
                self.session.cancel();
                self.session.document.expand_canvas(width, height)?;
            }
            EditorCommand::ResizeCanvas { width, height } => {
                self.session.cancel();
                self.session.document.resize_canvas(width, height)?;
            }
            EditorCommand::SetEffects { effects } => {
                self.session.set_effects(effects)?;
            }
            EditorCommand::AddText {
                point,
                text,
                callout,
            } => {
                let point = self.to_document_point(point);
                if !valid_point(point) {
                    return Err("Invalid text position.".into());
                }
                let size = self.session.text_size;
                let style = self.session.style;
                let opacity = self.session.opacity();
                let background = self.session.text_background();
                let changed = self.session.document.add_label_with_font(
                    point,
                    &text,
                    size,
                    style,
                    background,
                    callout,
                    self.session.font(),
                )?;
                if !changed {
                    return Err("Enter 1-4096 nonblank characters.".into());
                }
                self.session.apply_creation_opacity(true, opacity);
                let selected = self.session.document.selected;
                self.session.choose(Tool::Select);
                self.session.document.selected = selected;
            }
            EditorCommand::DuplicateArea { start, end } => {
                self.ensure_duplication_idle()?;
                if !valid_point(start) || !valid_point(end) {
                    return Err("Invalid duplicate area.".into());
                }
                let canvas = Rect::new(
                    0,
                    0,
                    self.session.document.crop.width,
                    self.session.document.crop.height,
                )?;
                let requested = Rect::from_edges(
                    start.x.min(end.x).floor() as i32,
                    start.y.min(end.y).floor() as i32,
                    start.x.max(end.x).ceil() as i32,
                    start.y.max(end.y).ceil() as i32,
                )?;
                let area = canvas
                    .intersection(requested)
                    .ok_or("Select a non-empty image region.")?;
                let rendered =
                    crate::render::flatten(&self.source, &self.session.document.render_snapshot())?;
                let frame = Arc::new(rendered.crop(canvas, area)?);
                let crop = self.session.document.crop;
                let x =
                    duplicate_area_axis(area.x as f64, area.right() as f64, canvas.right() as f64);
                let y = duplicate_area_axis(
                    area.y as f64,
                    area.bottom() as f64,
                    canvas.bottom() as f64,
                );
                self.session.document.add_duplicated_image_at(
                    frame,
                    Point::new(crop.x as f64 + x, crop.y as f64 + y),
                )?;
                let selected = self.session.document.selected;
                self.session.choose(Tool::Select);
                self.session.document.selected = selected;
            }
            EditorCommand::DuplicateSelection => {
                self.ensure_duplication_idle()?;
                self.session.cancel();
                if self.session.document.duplicate_selected()? {
                    let selected = self.session.document.selected;
                    self.session.choose(Tool::Select);
                    self.session.document.selected = selected;
                }
            }
            EditorCommand::Press { point, tolerance } => {
                let point = self.to_document_point(point);
                if !valid_point(point)
                    || !tolerance.is_finite()
                    || !(0.5..=100.).contains(&tolerance)
                {
                    return Err("Invalid pointer input.".into());
                }
                self.session.press(point, tolerance);
            }
            EditorCommand::Move { point } => {
                let point = self.to_document_point(point);
                if !valid_point(point) {
                    return Err("Invalid pointer input.".into());
                }
                self.session.update(point);
            }
            EditorCommand::Release { point } => {
                let point = self.to_document_point(point);
                if !valid_point(point) {
                    return Err("Invalid pointer input.".into());
                }
                self.session.release(point);
            }
            EditorCommand::ConfirmCrop => {
                self.session.confirm_crop();
            }
            EditorCommand::Cancel => self.session.cancel(),
            EditorCommand::Delete => {
                self.session.cancel();
                self.session.document.delete();
            }
            EditorCommand::Undo => {
                self.session.cancel();
                self.session.document.undo();
            }
            EditorCommand::Redo => {
                self.session.cancel();
                self.session.document.redo();
            }
            EditorCommand::ResetCurve => {
                self.session.cancel();
                self.session.document.reset_curve();
            }
            EditorCommand::Reorder { order } => {
                self.session.cancel();
                self.session.document.reorder(order.into());
            }
        }
        if let Some(error) = self.session.error.take() {
            return Err(error);
        }
        Ok(self.view())
    }

    fn to_document_point(&self, point: Point) -> Point {
        Point::new(
            point.x + self.session.document.crop.x as f64,
            point.y + self.session.document.crop.y as f64,
        )
    }

    pub fn preview_png(&self) -> Result<Vec<u8>, String> {
        let frame = crate::render::flatten(&self.source, &self.session.document.render_snapshot())?;
        let mut bytes = Vec::new();
        frame.write_png(&mut bytes)?;
        Ok(bytes)
    }

    pub fn write_preview(&self, path: &Path) -> Result<(), String> {
        let bytes = self.preview_png()?;
        atomic_write(path, &bytes)
    }
}

fn shape_handle_role(handle: crate::shape_frame::Handle) -> HandleRole {
    use crate::shape_frame::Handle;
    match handle {
        Handle::TopLeft => HandleRole::TopLeft,
        Handle::Top => HandleRole::Top,
        Handle::TopRight => HandleRole::TopRight,
        Handle::Right => HandleRole::Right,
        Handle::BottomRight => HandleRole::BottomRight,
        Handle::Bottom => HandleRole::Bottom,
        Handle::BottomLeft => HandleRole::BottomLeft,
        Handle::Left => HandleRole::Left,
    }
}

fn text_handle_role(handle: crate::text_frame::Handle) -> HandleRole {
    use crate::text_frame::Handle;
    match handle {
        Handle::TopLeft => HandleRole::TopLeft,
        Handle::TopRight => HandleRole::TopRight,
        Handle::BottomRight => HandleRole::BottomRight,
        Handle::BottomLeft => HandleRole::BottomLeft,
        Handle::Left => HandleRole::Left,
        Handle::Right => HandleRole::Right,
        Handle::Top => HandleRole::Top,
        Handle::Bottom => HandleRole::Bottom,
    }
}

fn rectangular_handles(bounds: (Point, Point)) -> Vec<SelectionHandle> {
    crate::shape_frame::positions(bounds)
        .into_iter()
        .map(|(role, point)| SelectionHandle {
            role: shape_handle_role(role),
            point,
        })
        .collect()
}

fn valid_point(point: Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && (-32768.0..=32768.0).contains(&point.x)
        && (-32768.0..=32768.0).contains(&point.y)
}

fn visible_overlay(mut overlay: SelectionOverlay, crop: Rect) -> SelectionOverlay {
    let origin = Point::new(crop.x as f64, crop.y as f64);
    let visible = |point: Point| Point::new(point.x - origin.x, point.y - origin.y);
    if let Some(bounds) = &mut overlay.bounds {
        bounds.start = visible(bounds.start);
        bounds.end = visible(bounds.end);
    }
    for handle in &mut overlay.handles {
        handle.point = visible(handle.point);
    }
    overlay
}

fn duplicate_area_axis(min: f64, max: f64, canvas_max: f64) -> f64 {
    if min + 16. < canvas_max {
        min + 16.
    } else if max - 16. > 0. {
        min - 16.
    } else {
        min
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    std::fs::rename(temporary, path).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum ServiceRequest {
    Open {
        path: PathBuf,
    },
    OpenProject {
        path: PathBuf,
    },
    SnapshotProject {
        path: PathBuf,
    },
    CombineProjects {
        paths: Vec<PathBuf>,
        layout: CombineLayout,
        gap: u32,
    },
    SaveProject {
        path: PathBuf,
    },
    Apply {
        command: EditorCommand,
    },
    Preview {
        path: PathBuf,
    },
    View,
    Shutdown,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum CombineLayout {
    Vertical,
    Horizontal,
    Free,
}

impl From<CombineLayout> for crate::combine::Layout {
    fn from(value: CombineLayout) -> Self {
        match value {
            CombineLayout::Vertical => Self::Vertical,
            CombineLayout::Horizontal => Self::Horizontal,
            CombineLayout::Free => Self::Free,
        }
    }
}

#[derive(Serialize)]
struct ServiceResponse {
    ok: bool,
    view: Option<EditorView>,
    error: Option<String>,
}

impl ServiceResponse {
    fn success(view: Option<EditorView>) -> Self {
        Self {
            ok: true,
            view,
            error: None,
        }
    }

    fn failure(error: String) -> Self {
        Self {
            ok: false,
            view: None,
            error: Some(error),
        }
    }
}

pub fn run_stdio() -> Result<(), String> {
    let input = std::io::stdin();
    let mut output = std::io::stdout().lock();
    let mut service: Option<EditorService> = None;
    for line in input.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.len() > MESSAGE_LIMIT {
            write_response(
                &mut output,
                ServiceResponse::failure("Editor request is too large.".into()),
            )?;
            continue;
        }
        let request = match serde_json::from_str::<ServiceRequest>(&line) {
            Ok(request) => request,
            Err(error) => {
                write_response(&mut output, ServiceResponse::failure(error.to_string()))?;
                continue;
            }
        };
        let shutdown = matches!(request, ServiceRequest::Shutdown);
        let result = match request {
            ServiceRequest::Open { path } => EditorService::open_path(&path).map(|opened| {
                let view = opened.view();
                service = Some(opened);
                Some(view)
            }),
            ServiceRequest::OpenProject { path } => {
                EditorService::open_project(&path).map(|opened| {
                    let view = opened.view();
                    service = Some(opened);
                    Some(view)
                })
            }
            ServiceRequest::SnapshotProject { path } => service
                .as_ref()
                .ok_or_else(|| "Open an image before combining.".into())
                .and_then(|editor| editor.snapshot_project(&path).map(|_| Some(editor.view()))),
            ServiceRequest::CombineProjects { paths, layout, gap } => {
                EditorService::combine_projects(&paths, layout.into(), gap).map(|combined| {
                    let view = combined.view();
                    service = Some(combined);
                    Some(view)
                })
            }
            ServiceRequest::SaveProject { path } => service
                .as_mut()
                .ok_or_else(|| "Open an image before saving a project.".into())
                .and_then(|editor| editor.save_project(&path).map(|_| Some(editor.view()))),
            ServiceRequest::Apply { command } => service
                .as_mut()
                .ok_or_else(|| "Open an image before editing.".into())
                .and_then(|editor| editor.apply(command).map(Some)),
            ServiceRequest::Preview { path } => service
                .as_ref()
                .ok_or_else(|| "Open an image before editing.".into())
                .and_then(|editor| editor.write_preview(&path).map(|_| Some(editor.view()))),
            ServiceRequest::View => service
                .as_ref()
                .map(|editor| Some(editor.view()))
                .ok_or_else(|| "Open an image before editing.".into()),
            ServiceRequest::Shutdown => Ok(None),
        };
        write_response(
            &mut output,
            match result {
                Ok(view) => ServiceResponse::success(view),
                Err(error) => ServiceResponse::failure(error),
            },
        )?;
        if shutdown {
            return Ok(());
        }
    }
    Ok(())
}

fn write_response(output: &mut impl Write, response: ServiceResponse) -> Result<(), String> {
    serde_json::to_writer(&mut *output, &response).map_err(|e| e.to_string())?;
    output.write_all(b"\n").map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{Annotation, Content, Document, PathRendering, Style};

    fn blank_service() -> EditorService {
        let mut png = Vec::new();
        Frame::new(96, 72, vec![255; 96 * 72 * 4])
            .unwrap()
            .write_png(&mut png)
            .unwrap();
        EditorService::open_bytes(&png).unwrap()
    }

    #[test]
    fn combine_snapshots_creates_editable_new_work_without_saving_sources() {
        let mut first = blank_service();
        let second = blank_service();
        draw(
            &mut first,
            Tool::Line,
            Point::new(10., 10.),
            Point::new(50., 40.),
        );
        let first_before = first.view();
        let second_before = second.view();
        let directory =
            std::env::temp_dir().join(format!("snapzy-combine-test-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let a = directory.join("a.snapzy");
        let b = directory.join("b.snapzy");
        first.snapshot_project(&a).unwrap();
        second.snapshot_project(&b).unwrap();
        let mut combined =
            EditorService::combine_projects(&[a, b], crate::combine::Layout::Vertical, 16).unwrap();
        assert_eq!((combined.view().width, combined.view().height), (96, 160));
        assert_eq!(combined.session.document.objects.len(), 3);
        assert!(combined.view().dirty);
        assert!(!combined.view().can_undo);
        assert_eq!(first.view(), first_before);
        assert_eq!(second.view(), second_before);
        combined
            .apply(EditorCommand::Choose { tool: Tool::Select })
            .unwrap();
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn expand_canvas_is_one_undoable_edit_and_keeps_objects() {
        let mut service = blank_service();
        draw(
            &mut service,
            Tool::Line,
            Point::new(10., 10.),
            Point::new(50., 40.),
        );
        let object_count = service.session.document.objects.len();
        let view = service
            .apply(EditorCommand::ExpandCanvas {
                width: 160,
                height: 120,
            })
            .unwrap();
        assert_eq!((view.width, view.height), (160, 120));
        assert!(view.can_undo);
        assert_eq!(service.session.document.objects.len(), object_count);

        let undone = service.apply(EditorCommand::Undo).unwrap();
        assert_eq!((undone.width, undone.height), (96, 72));
        assert_eq!(service.session.document.objects.len(), object_count);
    }

    #[test]
    fn resize_canvas_can_reduce_and_undo_restores_the_previous_workspace() {
        let mut service = blank_service();
        draw(
            &mut service,
            Tool::Line,
            Point::new(10., 10.),
            Point::new(50., 40.),
        );
        let object_count = service.session.document.objects.len();
        let view = service
            .apply(EditorCommand::ResizeCanvas {
                width: 64,
                height: 48,
            })
            .unwrap();
        assert_eq!((view.width, view.height), (64, 48));
        assert!(view.can_undo);
        assert_eq!(service.session.document.objects.len(), object_count);

        let undone = service.apply(EditorCommand::Undo).unwrap();
        assert_eq!((undone.width, undone.height), (96, 72));
        assert_eq!(service.session.document.objects.len(), object_count);
    }

    #[test]
    fn service_applies_font_and_background_to_new_text_and_exposes_them() {
        let mut service = blank_service();
        service
            .apply(EditorCommand::SetFont {
                font: crate::editor::FontFamily::Kanit,
            })
            .unwrap();
        let background = crate::editor::Background {
            enabled: true,
            rgb: [245, 206, 58],
        };
        service
            .apply(EditorCommand::SetTextBackground { background })
            .unwrap();
        let view = service
            .apply(EditorCommand::AddText {
                point: Point::new(8., 12.),
                text: "ภาษาไทย".into(),
                callout: false,
            })
            .unwrap();

        assert_eq!(view.font, crate::editor::FontFamily::Kanit);
        assert_eq!(view.text_background, background);
        assert_eq!(
            service.session.document.objects[0].text_font(),
            Some(crate::editor::FontFamily::Kanit)
        );
        assert_eq!(
            service.session.document.objects[0].text_background(),
            Some(background)
        );
    }

    fn draw(service: &mut EditorService, tool: Tool, start: Point, end: Point) -> EditorView {
        service.apply(EditorCommand::Choose { tool }).unwrap();
        service
            .apply(EditorCommand::Press {
                point: start,
                tolerance: 5.,
            })
            .unwrap();
        service.apply(EditorCommand::Move { point: end }).unwrap();
        service
            .apply(EditorCommand::Release { point: end })
            .unwrap()
    }

    #[test]
    fn drawn_shape_view_exposes_eight_resize_handles() {
        let mut service = blank_service();
        let view = draw(
            &mut service,
            Tool::Rectangle,
            Point::new(8., 10.),
            Point::new(48., 38.),
        );

        let overlay = view.selection_overlay.expect("shape must stay active");
        assert_eq!(overlay.kind, SelectionKind::Shape);
        assert_eq!(overlay.handles.len(), 8);
    }

    #[test]
    fn effects_are_exposed_applied_and_undoable_through_the_service() {
        let mut service = blank_service();
        draw(
            &mut service,
            Tool::Rectangle,
            Point::new(8., 10.),
            Point::new(48., 38.),
        );
        let mut effects = crate::effects::Effects::default();
        effects.shadow.enabled = true;
        effects.shadow.rgb = [12, 34, 56];
        effects.shadow.x = -4;

        let view = service
            .apply(EditorCommand::SetEffects { effects })
            .unwrap();
        assert_eq!(view.effects, effects);
        assert!(!view.outline_supported);

        let undone = service.apply(EditorCommand::Undo).unwrap();
        assert_eq!(undone.effects, crate::effects::Effects::default());

        service
            .apply(EditorCommand::Choose { tool: Tool::Text })
            .unwrap();
        let text = service
            .apply(EditorCommand::AddText {
                point: Point::new(10., 12.),
                text: "SnapZy".into(),
                callout: false,
            })
            .unwrap();
        assert!(text.outline_supported);
    }

    #[test]
    fn shape_overlay_tracks_resize_before_pointer_release() {
        let mut service = blank_service();
        draw(
            &mut service,
            Tool::Rectangle,
            Point::new(8., 10.),
            Point::new(48., 38.),
        );
        service
            .apply(EditorCommand::Press {
                point: Point::new(48., 38.),
                tolerance: 5.,
            })
            .unwrap();
        let view = service
            .apply(EditorCommand::Move {
                point: Point::new(60., 50.),
            })
            .unwrap();

        let overlay = view.selection_overlay.expect("shape must stay active");
        assert_eq!(overlay.bounds.unwrap().end, Point::new(60., 50.));
    }

    #[test]
    fn drawn_blur_view_exposes_eight_resize_handles() {
        let mut service = blank_service();
        let view = draw(
            &mut service,
            Tool::Blur,
            Point::new(8., 10.),
            Point::new(48., 38.),
        );

        let overlay = view.selection_overlay.expect("blur must stay active");
        assert_eq!(overlay.kind, SelectionKind::Region);
        assert_eq!(overlay.handles.len(), 8);
    }

    #[test]
    fn drawn_freehand_view_exposes_a_bounded_eight_handle_overlay() {
        let mut service = blank_service();
        let view = draw(
            &mut service,
            Tool::Pen,
            Point::new(8., 10.),
            Point::new(48., 38.),
        );

        let overlay = view.selection_overlay.expect("stroke must stay active");
        assert_eq!(overlay.kind, SelectionKind::Stroke);
        assert_eq!(overlay.handles.len(), 8);
    }

    #[test]
    fn text_and_callout_views_expose_semantic_handles() {
        let mut text = blank_service();
        text.apply(EditorCommand::Choose { tool: Tool::Text })
            .unwrap();
        let text_view = text
            .apply(EditorCommand::AddText {
                point: Point::new(10., 12.),
                text: "SnapZy".into(),
                callout: false,
            })
            .unwrap();
        let text_overlay = text_view.selection_overlay.expect("text must stay active");
        assert_eq!(text_view.tool, Tool::Select);
        assert_eq!(text_overlay.kind, SelectionKind::Text);
        assert_eq!(text_overlay.handles.len(), 6);

        let mut callout = blank_service();
        callout
            .apply(EditorCommand::Choose {
                tool: Tool::Callout,
            })
            .unwrap();
        let callout_view = callout
            .apply(EditorCommand::AddText {
                point: Point::new(10., 12.),
                text: "SnapZy".into(),
                callout: true,
            })
            .unwrap();
        let callout_overlay = callout_view
            .selection_overlay
            .expect("callout must stay active");
        assert_eq!(callout_view.tool, Tool::Select);
        assert_eq!(callout_overlay.kind, SelectionKind::Text);
        assert_eq!(callout_overlay.handles.len(), 7);
        assert!(callout_overlay
            .handles
            .iter()
            .any(|handle| handle.role == HandleRole::Tail));
    }

    #[test]
    fn drawn_step_view_exposes_uniform_resize_handles() {
        let mut service = blank_service();
        let view = draw(
            &mut service,
            Tool::Step,
            Point::new(28., 26.),
            Point::new(28., 26.),
        );

        let overlay = view.selection_overlay.expect("step must stay active");
        assert_eq!(overlay.kind, SelectionKind::Step);
        assert_eq!(overlay.handles.len(), 4);
    }

    #[test]
    fn duplicate_area_view_exposes_an_active_image_with_eight_handles() {
        let mut png = Vec::new();
        Frame::new(96, 72, vec![255; 96 * 72 * 4])
            .unwrap()
            .write_png(&mut png)
            .unwrap();
        let mut service = EditorService::open_bytes(&png).unwrap();

        let view = service
            .apply(EditorCommand::DuplicateArea {
                start: Point::new(8., 10.),
                end: Point::new(40., 34.),
            })
            .unwrap();

        let overlay = view.selection_overlay.expect("duplicate must stay active");
        assert_eq!(overlay.kind, SelectionKind::Image);
        assert!(overlay.movable);
        assert_eq!(overlay.handles.len(), 8);
        for role in [
            HandleRole::TopLeft,
            HandleRole::Top,
            HandleRole::TopRight,
            HandleRole::Right,
            HandleRole::BottomRight,
            HandleRole::Bottom,
            HandleRole::BottomLeft,
            HandleRole::Left,
        ] {
            assert!(overlay.handles.iter().any(|handle| handle.role == role));
        }
    }

    #[test]
    fn duplicate_image_overlay_tracks_free_side_resize_before_release() {
        let mut service = blank_service();
        service
            .apply(EditorCommand::DuplicateArea {
                start: Point::new(8., 10.),
                end: Point::new(40., 34.),
            })
            .unwrap();
        service
            .apply(EditorCommand::Choose { tool: Tool::Select })
            .unwrap();
        service.session.document.selected = service.session.document.objects.last().map(|o| o.id);
        let initial = service.view().selection_overlay.unwrap();
        let initial_bounds = initial.bounds.unwrap();
        let right = initial
            .handles
            .iter()
            .find(|handle| handle.role == HandleRole::Right)
            .unwrap()
            .point;
        service
            .apply(EditorCommand::Press {
                point: right,
                tolerance: 5.,
            })
            .unwrap();
        let view = service
            .apply(EditorCommand::Move {
                point: Point::new(right.x + 16., right.y),
            })
            .unwrap();

        let overlay = view.selection_overlay.expect("image must stay active");
        let bounds = overlay.bounds.unwrap();
        assert_eq!(bounds.start, initial_bounds.start);
        assert_eq!(bounds.end.x, initial_bounds.end.x + 16.);
        assert_eq!(bounds.end.y, initial_bounds.end.y);
    }

    #[test]
    fn selected_line_view_exposes_endpoints_and_curve_handle() {
        let mut png = Vec::new();
        Frame::new(96, 72, vec![255; 96 * 72 * 4])
            .unwrap()
            .write_png(&mut png)
            .unwrap();
        let mut service = EditorService::open_bytes(&png).unwrap();
        service.session.document = Document::from_project(
            Rect::new(0, 0, 96, 72).unwrap(),
            Some(Rect::new(0, 0, 96, 72).unwrap()),
            vec![Annotation {
                id: 1,
                parent: None,
                tool: Tool::Line,
                points: Arc::new(vec![Point::new(10., 20.), Point::new(70., 50.)]),
                style: Style::default(),
                pattern: Pattern::Solid,
                rendering: PathRendering::Native,
                content: Content::Shape,
                opacity: 100,
                effects: Default::default(),
            }],
        )
        .unwrap();
        service.session.document.selected = Some(1);
        service.session.tool = Tool::Select;

        let overlay = service
            .view()
            .selection_overlay
            .expect("selected line must expose its controls");
        assert_eq!(overlay.kind, SelectionKind::Line);
        assert_eq!(overlay.handles.len(), 3);
        assert!(overlay
            .handles
            .iter()
            .any(|handle| handle.role == HandleRole::Start));
        assert!(overlay
            .handles
            .iter()
            .any(|handle| handle.role == HandleRole::End));
        assert!(overlay
            .handles
            .iter()
            .any(|handle| handle.role == HandleRole::Curve));
    }

    #[test]
    fn reset_curve_command_restores_a_selected_line_and_is_undoable() {
        let mut service = blank_service();
        draw(
            &mut service,
            Tool::Line,
            Point::new(10., 20.),
            Point::new(70., 40.),
        );
        service
            .apply(EditorCommand::Press {
                point: Point::new(40., 30.),
                tolerance: 5.,
            })
            .unwrap();
        service
            .apply(EditorCommand::Release {
                point: Point::new(40., 60.),
            })
            .unwrap();
        assert_eq!(service.session.document.objects[0].points.len(), 3);

        service.apply(EditorCommand::ResetCurve).unwrap();
        assert_eq!(service.session.document.objects[0].points.len(), 2);
        service.apply(EditorCommand::Undo).unwrap();
        assert_eq!(service.session.document.objects[0].points.len(), 3);
    }

    #[test]
    fn failed_duplicate_area_keeps_tool_selection_revision_and_history_unchanged() {
        let mut png = Vec::new();
        Frame::new(64, 48, vec![255; 64 * 48 * 4])
            .unwrap()
            .write_png(&mut png)
            .unwrap();
        let mut service = EditorService::open_bytes(&png).unwrap();
        let objects: Vec<_> = (1..=2000)
            .map(|id| Annotation {
                id,
                parent: None,
                tool: Tool::Line,
                points: Arc::new(vec![Point::new(1., 1.), Point::new(2., 2.)]),
                style: Style::default(),
                pattern: Pattern::Solid,
                rendering: PathRendering::Native,
                content: Content::Shape,
                opacity: 100,
                effects: Default::default(),
            })
            .collect();
        service.session.document = Document::from_project(
            Rect::new(0, 0, 64, 48).unwrap(),
            Some(Rect::new(0, 0, 64, 48).unwrap()),
            objects,
        )
        .unwrap();
        service.session.tool = Tool::Line;
        service.session.document.selected = Some(2000);
        let before = service.view();

        assert!(service
            .apply(EditorCommand::DuplicateArea {
                start: Point::new(4., 4.),
                end: Point::new(20., 20.),
            })
            .is_err());

        assert_eq!(service.view(), before);
    }
}

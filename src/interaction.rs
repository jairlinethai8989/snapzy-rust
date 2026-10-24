use crate::editor::{Annotation, Background, Document, FontFamily, Pattern, Point, Style, Tool};
use crate::frame::Rect;

#[cfg(test)]
mod effect_tests {
    use super::*;
    use crate::effects::Effects;
    #[test]
    fn effects_defaults_are_independent_of_selected_objects() {
        let mut s = Session::new(100, 80);
        s.choose(Tool::Pen);
        let mut fx = Effects::default();
        fx.shadow.enabled = true;
        s.set_effects(fx).unwrap();
        s.press(Point::new(10., 10.), 3.);
        assert!(s.release(Point::new(70., 20.)));
        assert_eq!(s.document.objects[0].effects, fx);
        s.choose(Tool::Select);
        s.press(Point::new(40., 15.), 3.);
        s.release(Point::new(40., 15.));
        s.set_effects(Effects::default()).unwrap();
        assert!(!s.document.objects[0].effects.shadow.enabled);
        s.choose(Tool::Pen);
        s.document.selected = None;
        assert_eq!(s.effects(), fx);
        s.press(Point::new(10., 40.), 3.);
        s.release(Point::new(70., 50.));
        assert_eq!(s.document.objects[1].effects, fx);
        assert!(s.document.undo());
        assert_eq!(s.document.objects.len(), 1);
    }
}

pub struct Gesture {
    pub start: Point,
    pub points: Vec<Point>,
    pub end: Point,
    pub tool: Tool,
    pub ready: bool,
}
pub struct Session {
    pub document: Document,
    pub tool: Tool,
    pub style: Style,
    normal_style: Style,
    highlighter_style: Style,
    style_is_highlighter: bool,
    highlight_opacity: u8,
    normal_opacity: u8,
    effect_defaults: crate::effects::Effects,
    outline_shape: Tool,
    filled_shape: Tool,
    pub gesture: Option<Gesture>,
    pub resize: Option<crate::text_frame::Resize>,
    pub shape_resize: Option<crate::shape_frame::Resize>,
    pub tail_drag: Option<TailDrag>,
    pub curve_drag: Option<TailDrag>,
    pub line_endpoint_drag: Option<LineEndpointDrag>,
    pub path_resize: Option<PathResize>,
    pub step_resize: Option<StepResize>,
    pub pattern: Pattern,
    pub background: Background,
    pub text_font: FontFamily,
    pub text_size: u32,
    pub step_diameter: u32,
    pub blur_strength: u32,
    pub error: Option<String>,
    pub crop_target: Option<u64>,
}
pub struct TailDrag {
    pub start: Point,
    pub end: Point,
}
pub struct LineEndpointDrag {
    pub original: crate::editor::Annotation,
    pub index: usize,
    pub start: Point,
    pub end: Point,
}
impl LineEndpointDrag {
    fn draft(&self) -> Option<crate::editor::Annotation> {
        if self.start == self.end || ![self.end.x, self.end.y].iter().all(|v| v.is_finite()) {
            return None;
        }
        let mut points = self.original.points.as_ref().clone();
        if self.index > 1 || points.len() < 2 {
            return None;
        }
        points[self.index] = self.end;
        if (points[0].x - points[1].x).hypot(points[0].y - points[1].y) < 1. {
            return None;
        }
        let mut draft = self.original.clone();
        draft.points = std::sync::Arc::new(points);
        Some(draft)
    }
}
pub struct PathResize {
    pub original: crate::editor::Annotation,
    pub handle: crate::shape_frame::Handle,
    pub start: Point,
    pub end: Point,
}
impl PathResize {
    fn draft(&self) -> Option<crate::editor::Annotation> {
        if !matches!(self.original.tool, Tool::Pen | Tool::Highlighter) {
            return None;
        }
        let old = self.original.bounds();
        let new = crate::shape_frame::resized_bounds(
            old,
            self.handle,
            self.end.x - self.start.x,
            self.end.y - self.start.y,
        )?;
        let old_width = old.1.x - old.0.x;
        let old_height = old.1.y - old.0.y;
        if old_width < 1. || old_height < 1. {
            return None;
        }
        let scale_x = (new.1.x - new.0.x) / old_width;
        let scale_y = (new.1.y - new.0.y) / old_height;
        let points = self
            .original
            .points
            .iter()
            .map(|point| {
                Point::new(
                    new.0.x + (point.x - old.0.x) * scale_x,
                    new.0.y + (point.y - old.0.y) * scale_y,
                )
            })
            .collect();
        let mut draft = self.original.clone();
        draft.points = std::sync::Arc::new(points);
        Some(draft)
    }
}
pub struct StepResize {
    pub original: crate::editor::Annotation,
    pub handle: crate::shape_frame::Handle,
    pub start: Point,
    pub end: Point,
}
impl StepResize {
    fn proposal(&self) -> Option<(Point, u32)> {
        use crate::shape_frame::Handle;
        if self.original.tool != Tool::Step
            || !matches!(
                self.handle,
                Handle::TopLeft | Handle::TopRight | Handle::BottomRight | Handle::BottomLeft
            )
            || self.start == self.end
        {
            return None;
        }
        let (a, b) = self.original.bounds();
        let (corner, anchor) = match self.handle {
            Handle::TopLeft => (a, b),
            Handle::TopRight => (Point::new(b.x, a.y), Point::new(a.x, b.y)),
            Handle::BottomRight => (b, a),
            Handle::BottomLeft => (Point::new(a.x, b.y), Point::new(b.x, a.y)),
            _ => return None,
        };
        let original_vector = Point::new(corner.x - anchor.x, corner.y - anchor.y);
        let target_vector = Point::new(self.end.x - anchor.x, self.end.y - anchor.y);
        let denominator =
            original_vector.x * original_vector.x + original_vector.y * original_vector.y;
        if denominator <= 0. || !target_vector.x.is_finite() || !target_vector.y.is_finite() {
            return None;
        }
        let scale = ((target_vector.x * original_vector.x + target_vector.y * original_vector.y)
            / denominator)
            .max(0.);
        let original_diameter = self.original.parameter();
        let diameter = (original_diameter as f64 * scale).round().clamp(20., 120.) as u32;
        if diameter == original_diameter {
            return None;
        }
        let applied = diameter as f64 / original_diameter as f64;
        let moved = Point::new(
            anchor.x + original_vector.x * applied,
            anchor.y + original_vector.y * applied,
        );
        Some((
            Point::new((anchor.x + moved.x) / 2., (anchor.y + moved.y) / 2.),
            diameter,
        ))
    }
}
impl Session {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            document: Document::new(w, h),
            tool: Tool::Select,
            style: Style::default(),
            normal_style: Style::default(),
            highlighter_style: Style {
                rgb: [255, 242, 88],
                width: 24,
            },
            highlight_opacity: 40,
            normal_opacity: 100,
            effect_defaults: Default::default(),
            outline_shape: Tool::Rectangle,
            filled_shape: Tool::FilledRectangle,
            style_is_highlighter: false,
            gesture: None,
            resize: None,
            shape_resize: None,
            tail_drag: None,
            curve_drag: None,
            line_endpoint_drag: None,
            path_resize: None,
            step_resize: None,
            pattern: Pattern::Solid,
            background: Background::default(),
            text_font: FontFamily::default(),
            text_size: 24,
            step_diameter: 36,
            blur_strength: 8,
            error: None,
            crop_target: None,
        }
    }
    pub fn press(&mut self, p: Point, tolerance: f64) {
        self.cancel();
        let local = self.selected_transform().inverse(p);
        if let Some(handle) = self.step_handle(p, tolerance) {
            let original = self
                .document
                .objects
                .iter()
                .find(|o| Some(o.id) == self.document.selected)
                .unwrap()
                .clone();
            self.step_resize = Some(StepResize {
                original,
                handle,
                start: local,
                end: local,
            });
            return;
        }
        if let Some(handle) = self.path_handle(p, tolerance) {
            let original = self
                .document
                .objects
                .iter()
                .find(|o| Some(o.id) == self.document.selected)
                .unwrap()
                .clone();
            self.path_resize = Some(PathResize {
                original,
                handle,
                start: local,
                end: local,
            });
            return;
        }
        if let Some(handle) = self.shape_handle(p, tolerance) {
            let original = self
                .document
                .objects
                .iter()
                .find(|o| Some(o.id) == self.document.selected)
                .unwrap()
                .clone();
            self.shape_resize = Some(crate::shape_frame::Resize {
                original,
                handle,
                start: local,
                end: local,
            });
            return;
        }
        if let Some(index) = self.line_endpoint_handle(p, tolerance) {
            let original = self
                .document
                .objects
                .iter()
                .find(|o| Some(o.id) == self.document.selected)
                .unwrap()
                .clone();
            self.line_endpoint_drag = Some(LineEndpointDrag {
                original,
                index,
                start: local,
                end: local,
            });
            return;
        }
        if self.curve_handle(p, tolerance) {
            self.curve_drag = Some(TailDrag {
                start: local,
                end: local,
            });
            return;
        }
        if self.tail_handle(p, tolerance) {
            self.tail_drag = Some(TailDrag {
                start: local,
                end: local,
            });
            return;
        }
        if let Some(handle) = self.resize_handle(p, tolerance) {
            let original = self
                .document
                .objects
                .iter()
                .find(|o| Some(o.id) == self.document.selected)
                .unwrap()
                .clone();
            self.resize = Some(crate::text_frame::Resize {
                original,
                handle,
                start: local,
                end: local,
            });
            return;
        }
        if self.tool == Tool::Select {
            self.document.selected = self.document.hit(p, tolerance);
            if self.document.selected.is_none() {
                return;
            }
            self.sync_selected_style();
        }
        self.gesture = Some(Gesture {
            start: p,
            points: vec![p],
            end: p,
            tool: self.tool,
            ready: false,
        });
    }
    pub fn update(&mut self, p: Point) -> bool {
        let p = if self.shape_resize.is_some()
            || self.curve_drag.is_some()
            || self.line_endpoint_drag.is_some()
            || self.path_resize.is_some()
            || self.step_resize.is_some()
            || self.tail_drag.is_some()
            || self.resize.is_some()
        {
            self.selected_transform().inverse(p)
        } else {
            p
        };
        if let Some(r) = &mut self.shape_resize {
            if r.end == p {
                return false;
            }
            r.end = p;
            return true;
        }
        if let Some(r) = &mut self.path_resize {
            if r.end == p {
                return false;
            }
            r.end = p;
            return true;
        }
        if let Some(r) = &mut self.step_resize {
            if r.end == p {
                return false;
            }
            r.end = p;
            return true;
        }
        if let Some(d) = &mut self.curve_drag {
            if d.end == p {
                return false;
            }
            d.end = p;
            return true;
        }
        if let Some(d) = &mut self.line_endpoint_drag {
            if d.end == p {
                return false;
            }
            d.end = p;
            return true;
        }
        if let Some(d) = &mut self.tail_drag {
            if d.end == p {
                return false;
            }
            d.end = p;
            return true;
        }
        if let Some(r) = &mut self.resize {
            if r.end == p {
                return false;
            }
            r.end = p;
            return true;
        }
        if let Some(g) = &mut self.gesture {
            if g.ready || g.end == p {
                return false;
            }
            g.end = p;
            if matches!(g.tool, Tool::Pen | Tool::Highlighter)
                && g.points.len() < 100_000
                && g.points
                    .last()
                    .is_some_and(|q| (q.x - p.x).hypot(q.y - p.y) >= 0.5)
            {
                g.points.push(p);
            }
            return true;
        }
        false
    }
    pub fn release(&mut self, p: Point) -> bool {
        self.update(p);
        if let Some(r) = self.shape_resize.take() {
            return r
                .draft()
                .is_some_and(|o| self.document.resize_shape(o.bounds()));
        }
        if let Some(resize) = self.path_resize.take() {
            return resize
                .draft()
                .is_some_and(|draft| self.document.set_path_points(draft.points));
        }
        if let Some(resize) = self.step_resize.take() {
            return match resize.proposal() {
                Some((center, diameter)) => match self.document.resize_step(center, diameter) {
                    Ok(changed) => changed,
                    Err(error) => {
                        self.error = Some(error);
                        false
                    }
                },
                None => false,
            };
        }
        if let Some(proposal) = self.curve_draft() {
            self.curve_drag = None;
            return self
                .document
                .set_curve_midpoint(proposal.curve_midpoint().unwrap());
        }
        if self.curve_drag.take().is_some() {
            return false;
        }
        if let Some(drag) = self.line_endpoint_drag.take() {
            return drag
                .draft()
                .is_some_and(|draft| self.document.set_line_points(draft.points));
        }
        if let Some(d) = self.tail_drag.take() {
            if d.start == d.end {
                return false;
            }
            let Some(tip) = self
                .document
                .objects
                .iter()
                .find(|o| Some(o.id) == self.document.selected)
                .and_then(|o| o.tail_tip())
            else {
                return false;
            };
            return self.document.set_tail(Point::new(
                tip.x + d.end.x - d.start.x,
                tip.y + d.end.y - d.start.y,
            ));
        }
        if let Some(r) = self.resize.take() {
            if r.original.tool == Tool::Image {
                let preserve_aspect = !matches!(
                    r.handle,
                    crate::text_frame::Handle::Left
                        | crate::text_frame::Handle::Right
                        | crate::text_frame::Handle::Top
                        | crate::text_frame::Handle::Bottom
                );
                return if preserve_aspect {
                    self.document.resize_image(r.bounds())
                } else {
                    self.document.resize_image_free(r.bounds())
                };
            }
            let Some(p) = r.proposal() else {
                return false;
            };
            return match self
                .document
                .resize_text(p.size, p.width, p.anchor, p.right, p.bottom)
            {
                Ok(changed) => changed,
                Err(e) => {
                    self.error = Some(e);
                    false
                }
            };
        }
        let Some(mut g) = self.gesture.take() else {
            return false;
        };
        if g.tool == Tool::Crop {
            g.ready = true;
            self.gesture = Some(g);
            return false;
        }
        if g.tool == Tool::Select {
            return self
                .document
                .move_selected(g.end.x - g.start.x, g.end.y - g.start.y);
        }
        if g.tool == Tool::Step {
            let opacity = self.opacity();
            let changed = match self
                .document
                .add_step(g.start, self.step_diameter, self.style)
            {
                Ok(changed) => changed,
                Err(e) => {
                    self.error = Some(e);
                    false
                }
            };
            self.apply_creation_opacity(changed, opacity);
            return changed;
        }
        if g.tool == Tool::Blur {
            let changed = self.document.add_blur(g.start, g.end, self.blur_strength);
            self.apply_creation_opacity(changed, 100);
            return changed;
        }
        let points = if matches!(g.tool, Tool::Pen | Tool::Highlighter) {
            g.points
        } else {
            vec![g.start, g.end]
        };
        if g.tool == Tool::Highlighter {
            let opacity = self.opacity();
            let changed = self.document.add_highlight(points, self.style, opacity);
            self.apply_creation_opacity(changed, opacity);
            changed
        } else {
            let opacity = self.opacity();
            let changed = self
                .document
                .add_patterned(g.tool, points, self.style, self.pattern());
            self.apply_creation_opacity(changed, opacity);
            changed
        }
    }
    pub fn apply_creation_opacity(&mut self, changed: bool, opacity: u8) {
        if changed {
            if let Some(o) = self.document.objects.last_mut() {
                if o.tool.opacity_supported() {
                    o.opacity = opacity.min(100);
                }
                o.effects = self.effect_defaults;
            }
        }
    }
    pub fn confirm_crop(&mut self) -> bool {
        if !self
            .gesture
            .as_ref()
            .is_some_and(|g| g.ready && g.tool == Tool::Crop)
        {
            return false;
        }
        let crop = self.crop_draft();
        let target = self.crop_target;
        self.cancel();
        crop.is_some_and(|r| {
            if let Some(id) = target {
                self.document.crop_image(id, r)
            } else {
                self.document.set_crop(r)
            }
        })
    }
    pub fn cancel(&mut self) {
        self.gesture = None;
        self.resize = None;
        self.shape_resize = None;
        self.tail_drag = None;
        self.curve_drag = None;
        self.line_endpoint_drag = None;
        self.path_resize = None;
        self.step_resize = None;
    }
    pub fn step_handles(&self) -> Vec<(crate::shape_frame::Handle, Point)> {
        use crate::shape_frame::Handle;
        if !(self.tool == Tool::Select || self.tool == Tool::Step) {
            return vec![];
        }
        let Some(object) =
            self.document.objects.iter().find(|object| {
                Some(object.id) == self.document.selected && object.tool == Tool::Step
            })
        else {
            return vec![];
        };
        let transform = self.selected_transform();
        let mut visible = self
            .step_resize
            .as_ref()
            .and_then(StepResize::proposal)
            .map(|(center, diameter)| {
                let radius = diameter as f64 / 2.;
                (
                    Point::new(center.x - radius, center.y - radius),
                    Point::new(center.x + radius, center.y + radius),
                )
            })
            .unwrap_or_else(|| object.bounds());
        if let Some((dx, dy)) = self.selection_move_delta_local() {
            visible.0.x += dx;
            visible.0.y += dy;
            visible.1.x += dx;
            visible.1.y += dy;
        }
        crate::shape_frame::positions(visible)
            .into_iter()
            .filter(|(handle, _)| {
                matches!(
                    handle,
                    Handle::TopLeft | Handle::TopRight | Handle::BottomRight | Handle::BottomLeft
                )
            })
            .map(|(handle, point)| (handle, transform.point(point)))
            .collect()
    }
    pub fn step_handle(&self, point: Point, radius: f64) -> Option<crate::shape_frame::Handle> {
        if !radius.is_finite() || radius < 0. {
            return None;
        }
        self.step_handles()
            .into_iter()
            .find(|(_, handle)| {
                (handle.x - point.x).abs() <= radius && (handle.y - point.y).abs() <= radius
            })
            .map(|(handle, _)| handle)
    }
    pub fn path_draft(&self) -> Option<crate::editor::Annotation> {
        self.path_resize.as_ref()?.draft()
    }
    pub fn path_handles(&self) -> Vec<(crate::shape_frame::Handle, Point)> {
        if !(self.tool == Tool::Select || matches!(self.tool, Tool::Pen | Tool::Highlighter))
            || self
                .gesture
                .as_ref()
                .is_some_and(|gesture| gesture.tool != Tool::Select)
        {
            return vec![];
        }
        let Some(object) = self.document.objects.iter().find(|object| {
            Some(object.id) == self.document.selected
                && matches!(object.tool, Tool::Pen | Tool::Highlighter)
        }) else {
            return vec![];
        };
        let draft = self.path_draft();
        let visible = draft.as_ref().unwrap_or(object);
        let (mut start, mut end) = visible.bounds();
        if end.x - start.x < 1. || end.y - start.y < 1. {
            return vec![];
        }
        if let Some((dx, dy)) = self.selection_move_delta_local() {
            start.x += dx;
            start.y += dy;
            end.x += dx;
            end.y += dy;
        }
        let transform = self.selected_transform();
        crate::shape_frame::positions((start, end))
            .into_iter()
            .map(|(role, point)| (role, transform.point(point)))
            .collect()
    }
    pub fn path_handle(&self, point: Point, radius: f64) -> Option<crate::shape_frame::Handle> {
        if !radius.is_finite() || radius < 0. {
            return None;
        }
        self.path_handles()
            .into_iter()
            .find(|(_, handle)| {
                (handle.x - point.x).abs() <= radius && (handle.y - point.y).abs() <= radius
            })
            .map(|(role, _)| role)
    }
    pub fn line_endpoint_draft(&self) -> Option<crate::editor::Annotation> {
        self.line_endpoint_drag.as_ref()?.draft()
    }
    pub fn line_endpoint_handle(&self, point: Point, radius: f64) -> Option<usize> {
        if !matches!(self.tool, Tool::Select | Tool::Line | Tool::Arrow)
            || !radius.is_finite()
            || radius < 0.
        {
            return None;
        }
        let object = self.document.objects.iter().find(|object| {
            Some(object.id) == self.document.selected
                && matches!(object.tool, Tool::Line | Tool::Arrow)
        })?;
        let transform = self.selected_transform();
        object.points[..2]
            .iter()
            .map(|point| transform.point(*point))
            .enumerate()
            .find(|(_, handle)| {
                (handle.x - point.x).abs() <= radius && (handle.y - point.y).abs() <= radius
            })
            .map(|(index, _)| index)
    }
    pub fn curve_handle_position(&self) -> Option<Point> {
        if !matches!(self.tool, Tool::Select | Tool::Line | Tool::Arrow)
            || self
                .gesture
                .as_ref()
                .is_some_and(|g| g.tool != Tool::Select)
        {
            return None;
        }
        let object = self
            .document
            .objects
            .iter()
            .find(|o| Some(o.id) == self.document.selected)?;
        let midpoint = if let Some(draft) = self.curve_draft() {
            draft.curve_midpoint()?
        } else {
            let mut midpoint = object.curve_midpoint()?;
            if let Some(g) = &self.gesture {
                midpoint.x += (g.end.x - g.start.x) / self.selected_transform().scale;
                midpoint.y += (g.end.y - g.start.y) / self.selected_transform().scale;
            }
            midpoint
        };
        let midpoint = self.selected_transform().point(midpoint);
        self.document
            .point_visible(object.id, midpoint)
            .then_some(midpoint)
    }
    pub fn shape_draft(&self) -> Option<Annotation> {
        self.shape_resize.as_ref()?.draft()
    }
    pub fn shape_handles(&self) -> Vec<(crate::shape_frame::Handle, Point)> {
        if !(self.tool == Tool::Select || crate::shape_frame::supported(self.tool))
            || self
                .gesture
                .as_ref()
                .is_some_and(|g| g.tool != Tool::Select)
        {
            return vec![];
        }
        let Some(o) = self.document.objects.iter().find(|o| {
            Some(o.id) == self.document.selected && crate::shape_frame::supported(o.tool)
        }) else {
            return vec![];
        };
        let draft = self.shape_draft();
        let (mut a, mut b) = draft.as_ref().unwrap_or(o).bounds();
        if let Some(g) = &self.gesture {
            let dx = (g.end.x - g.start.x) / self.selected_transform().scale;
            let dy = (g.end.y - g.start.y) / self.selected_transform().scale;
            a.x += dx;
            b.x += dx;
            a.y += dy;
            b.y += dy;
        }
        crate::shape_frame::positions((a, b))
            .into_iter()
            .map(|(h, p)| (h, self.selected_transform().point(p)))
            .filter(|(_, p)| self.document.point_visible(o.id, *p))
            .collect()
    }
    pub fn shape_handle(&self, p: Point, radius: f64) -> Option<crate::shape_frame::Handle> {
        if !radius.is_finite() || radius < 0. {
            return None;
        }
        self.shape_handles()
            .into_iter()
            .find(|(_, h)| (h.x - p.x).abs() <= radius && (h.y - p.y).abs() <= radius)
            .map(|(h, _)| h)
    }
    pub fn curve_handle(&self, p: Point, radius: f64) -> bool {
        radius.is_finite()
            && radius >= 0.
            && self
                .curve_handle_position()
                .is_some_and(|m| (m.x - p.x).abs() <= radius && (m.y - p.y).abs() <= radius)
    }
    pub fn curve_draft(&self) -> Option<Annotation> {
        let d = self.curve_drag.as_ref()?;
        let o = self
            .document
            .objects
            .iter()
            .find(|o| Some(o.id) == self.document.selected)?;
        let m = o.curve_midpoint()?;
        crate::geometry::bend(
            o,
            Point::new(m.x + d.end.x - d.start.x, m.y + d.end.y - d.start.y),
            self.document.local_canvas(o.id),
        )
    }
    pub fn pattern(&self) -> Pattern {
        self.document
            .objects
            .iter()
            .find(|o| Some(o.id) == self.document.selected && o.tool.patterned())
            .map(|o| o.pattern)
            .unwrap_or(self.pattern)
    }
    pub fn set_pattern(&mut self, value: Pattern) -> bool {
        if !self.effective_tool().patterned() {
            return false;
        }
        self.cancel();
        if self.document.selected.is_none() {
            self.pattern = value;
        }
        self.document.set_pattern(value)
    }
    pub fn tail_handle(&self, p: Point, radius: f64) -> bool {
        self.tool == Tool::Select
            && radius.is_finite()
            && radius >= 0.
            && self
                .document
                .objects
                .iter()
                .find(|o| Some(o.id) == self.document.selected)
                .and_then(|o| o.tail_tip())
                .map(|tip| self.selected_transform().point(tip))
                .is_some_and(|tip| {
                    self.document
                        .selected
                        .is_some_and(|id| self.document.point_visible(id, tip))
                        && (p.x - tip.x).abs() <= radius
                        && (p.y - tip.y).abs() <= radius
                })
    }
    pub fn text_background(&self) -> Background {
        self.document
            .objects
            .iter()
            .find(|o| Some(o.id) == self.document.selected)
            .and_then(|o| o.text_background())
            .unwrap_or(self.background)
    }
    pub fn font(&self) -> FontFamily {
        self.document
            .objects
            .iter()
            .find(|object| Some(object.id) == self.document.selected)
            .and_then(|object| object.text_font())
            .unwrap_or(self.text_font)
    }
    pub fn set_font(&mut self, font: FontFamily) -> Result<bool, String> {
        self.cancel();
        if self.document.selected.is_none() {
            self.text_font = font;
            return Ok(false);
        }
        self.document.set_text_font(font)
    }
    pub fn set_background(&mut self, background: Background) {
        self.cancel();
        self.background = background;
        self.document.set_text_background(background);
    }
    pub fn callout_border(&self) -> Style {
        self.document
            .objects
            .iter()
            .find(|o| Some(o.id) == self.document.selected && o.tool == Tool::Callout)
            .map(Annotation::callout_border)
            .unwrap_or(self.style)
    }
    pub fn has_independent_callout_border(&self) -> bool {
        self.document.objects.iter().any(|o| {
            Some(o.id) == self.document.selected
                && matches!(
                    o.content,
                    crate::editor::Content::Text {
                        layout: crate::editor::TextLayout::DotNet {
                            bubble: Some(_),
                            ..
                        },
                        ..
                    }
                )
        })
    }
    pub fn set_callout_border(&mut self, style: Style) -> bool {
        self.cancel();
        self.document.set_callout_border(style)
    }
    pub fn resize_handle(&self, point: Point, radius: f64) -> Option<crate::text_frame::Handle> {
        if self.tool != Tool::Select {
            return None;
        }
        let o = self.document.objects.iter().find(|o| {
            Some(o.id) == self.document.selected
                && matches!(o.tool, Tool::Text | Tool::Callout | Tool::Image)
        })?;
        if !self.document.point_visible(o.id, point) {
            return None;
        }
        let transform = self.selected_transform();
        let point = transform.inverse(point);
        let radius = radius / transform.scale;
        if o.tool != Tool::Image {
            return crate::text_frame::hit(o.bounds(), point, radius);
        }
        crate::text_frame::positions(o.bounds())
            .into_iter()
            .filter(|(handle, _)| {
                o.tool == Tool::Image
                    || !matches!(
                        handle,
                        crate::text_frame::Handle::Top | crate::text_frame::Handle::Bottom
                    )
            })
            .find(|(_, p)| {
                radius.is_finite()
                    && radius >= 0.
                    && (p.x - point.x).abs() <= radius
                    && (p.y - point.y).abs() <= radius
            })
            .map(|(h, _)| h)
    }
    pub fn choose(&mut self, tool: Tool) {
        self.cancel();
        self.crop_target = if tool == Tool::Crop {
            self.document
                .objects
                .iter()
                .find(|o| Some(o.id) == self.document.selected && o.tool == Tool::Image)
                .map(|o| o.id)
        } else {
            None
        };
        match tool {
            Tool::Rectangle | Tool::Ellipse => self.outline_shape = tool,
            Tool::FilledRectangle | Tool::FilledEllipse => self.filled_shape = tool,
            _ => {}
        }
        // Selection may disappear after Delete/Undo; retain which defaults own the style.
        if self.tool == Tool::Select && self.document.selected.is_some() {
            // Browsing an existing object must not replace future drawing defaults.
        } else if self.style_is_highlighter {
            self.highlighter_style = self.style;
            self.highlight_opacity = self.opacity();
        } else {
            self.normal_style = self.style;
            self.normal_opacity = self.opacity();
        }
        self.style = if tool == Tool::Highlighter {
            self.highlighter_style
        } else {
            self.normal_style
        };
        self.tool = tool;
        self.style_is_highlighter = tool == Tool::Highlighter;
        self.document.selected = None;
    }
    pub fn selected_transform(&self) -> crate::combine::Transform {
        self.document
            .selected
            .map(|id| self.document.parent_transform(id))
            .unwrap_or_default()
    }
    pub fn selection_move_delta_local(&self) -> Option<(f64, f64)> {
        let gesture = self
            .gesture
            .as_ref()
            .filter(|gesture| gesture.tool == Tool::Select && gesture.start != gesture.end)?;
        let scale = self.selected_transform().scale;
        Some((
            (gesture.end.x - gesture.start.x) / scale,
            (gesture.end.y - gesture.start.y) / scale,
        ))
    }
    pub fn shape_choices(&self) -> [Tool; 2] {
        [self.outline_shape, self.filled_shape]
    }
    pub fn sync_selected_style(&mut self) {
        if let Some(object) = self
            .document
            .objects
            .iter()
            .find(|o| Some(o.id) == self.document.selected)
        {
            self.style = object.style;
            self.style_is_highlighter = object.tool == Tool::Highlighter;
        }
    }
    pub fn effective_tool(&self) -> Tool {
        self.document
            .objects
            .iter()
            .find(|o| Some(o.id) == self.document.selected)
            .map(|o| o.tool)
            .unwrap_or(self.tool)
    }
    pub fn effects(&self) -> crate::effects::Effects {
        self.document
            .objects
            .iter()
            .find(|o| Some(o.id) == self.document.selected)
            .map(|o| o.effects)
            .unwrap_or(self.effect_defaults)
    }
    pub fn set_effects(&mut self, effects: crate::effects::Effects) -> Result<bool, String> {
        let effects = effects.validate()?;
        if self.tool == Tool::Crop
            || (self.tool == Tool::Select && self.document.selected.is_none())
        {
            return Ok(false);
        }
        self.cancel();
        if self.tool != Tool::Select {
            self.effect_defaults.shadow = effects.shadow;
            if matches!(self.effective_tool(), Tool::Text | Tool::Callout) {
                self.effect_defaults.outline = effects.outline;
            }
        }
        self.document.set_effects(effects)
    }
    pub fn creation_effects(&self) -> crate::effects::Effects {
        self.effect_defaults
    }
    pub fn opacity(&self) -> u8 {
        self.document
            .objects
            .iter()
            .find(|o| Some(o.id) == self.document.selected)
            .filter(|o| o.tool.opacity_supported())
            .map(|o| o.opacity)
            .unwrap_or(if self.tool == Tool::Highlighter {
                self.highlight_opacity
            } else {
                self.normal_opacity
            })
    }
    pub fn set_opacity(&mut self, value: u32) -> bool {
        if value > 100 || !self.effective_tool().opacity_supported() {
            return false;
        }
        self.cancel();
        if self.tool != Tool::Select {
            if self.tool == Tool::Highlighter {
                self.highlight_opacity = value as u8;
            } else {
                self.normal_opacity = value as u8;
            }
        }
        self.document.set_opacity(value as u8)
    }
    pub fn parameter(&self) -> u32 {
        if let Some(o) = self
            .document
            .objects
            .iter()
            .find(|o| Some(o.id) == self.document.selected)
        {
            return o.parameter();
        }
        match self.tool {
            Tool::Text | Tool::Callout => self.text_size,
            Tool::Step => self.step_diameter,
            Tool::Blur => self.blur_strength,
            _ => self.style.width,
        }
    }
    pub fn set_parameter(&mut self, value: u32) -> Result<bool, String> {
        let tool = self.effective_tool();
        let valid = match tool {
            Tool::Text | Tool::Callout => (8..=256).contains(&value),
            Tool::Step => (20..=120).contains(&value),
            Tool::Blur => (1..=64).contains(&value),
            Tool::Select
            | Tool::Crop
            | Tool::Image
            | Tool::FilledRectangle
            | Tool::FilledEllipse => false,
            _ => (1..=40).contains(&value),
        };
        if !valid {
            return Ok(false);
        }
        match tool {
            Tool::Text | Tool::Callout => self.text_size = value,
            Tool::Step => self.step_diameter = value,
            Tool::Blur => self.blur_strength = value,
            _ => self.style.width = value,
        }
        self.document.set_parameter(value)
    }
    pub fn crop_draft(&self) -> Option<Rect> {
        let g = self.gesture.as_ref()?;
        if g.tool != Tool::Crop {
            return None;
        }
        Rect::from_edges(
            g.start.x.min(g.end.x).round() as i32,
            g.start.y.min(g.end.y).round() as i32,
            g.start.x.max(g.end.x).round() as i32,
            g.start.y.max(g.end.y).round() as i32,
        )
        .ok()
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn shape_choices_are_independent_and_scoped_to_an_editor() {
        let mut s = Session::new(100, 100);
        assert_eq!(s.shape_choices(), [Tool::Rectangle, Tool::FilledRectangle]);
        s.choose(Tool::Ellipse);
        s.choose(Tool::FilledEllipse);
        s.choose(Tool::Pen);
        assert_eq!(s.shape_choices(), [Tool::Ellipse, Tool::FilledEllipse]);
        s.choose(Tool::Rectangle);
        assert_eq!(s.shape_choices(), [Tool::Rectangle, Tool::FilledEllipse]);
        assert_eq!(s.document.revision, 0);
        assert!(!s.document.can_undo());
        assert_eq!(
            Session::new(100, 100).shape_choices(),
            [Tool::Rectangle, Tool::FilledRectangle]
        );
    }

    #[test]
    fn direct_handle_hides_during_new_stroke_and_tracks_only_latest_selection() {
        let mut s = Session::new(100, 100);
        s.choose(Tool::Line);
        s.press(Point::new(10., 20.), 2.);
        s.release(Point::new(90., 20.));
        assert_eq!(s.curve_handle_position(), Some(Point::new(50., 20.)));
        s.press(Point::new(10., 80.), 2.);
        assert_eq!(s.curve_handle_position(), None);
        s.update(Point::new(90., 80.));
        assert_eq!(s.curve_handle_position(), None);
        s.cancel();
        assert_eq!(s.curve_handle_position(), Some(Point::new(50., 20.)));
        s.press(Point::new(10., 80.), 2.);
        s.release(Point::new(90., 80.));
        assert!(!s.curve_handle(Point::new(50., 20.), 3.));
        assert!(s.curve_handle(Point::new(50., 80.), 3.));
        s.choose(Tool::Pen);
        assert_eq!(s.curve_handle_position(), None);
    }

    #[test]
    fn direct_bend_cancel_reset_noop_and_tool_change_preserve_history_rules() {
        let mut s = Session::new(100, 100);
        s.choose(Tool::Arrow);
        s.press(Point::new(10., 60.), 2.);
        s.release(Point::new(90., 60.));
        let revision = s.document.revision;
        s.press(Point::new(50., 60.), 3.);
        assert!(!s.release(Point::new(50., 60.)));
        assert_eq!(s.document.revision, revision);
        s.press(Point::new(50., 60.), 3.);
        s.update(Point::new(50., 30.));
        assert_eq!(s.curve_handle_position(), Some(Point::new(50., 30.)));
        s.cancel();
        assert_eq!(s.document.revision, revision);
        assert_eq!(s.curve_handle_position(), Some(Point::new(50., 60.)));
        s.press(Point::new(50., 60.), 3.);
        s.update(Point::new(50., 30.));
        s.choose(Tool::Rectangle);
        assert_eq!(s.document.revision, revision);
        assert!(s.curve_drag.is_none());
        s.choose(Tool::Arrow);
        s.press(Point::new(10., 80.), 2.);
        s.release(Point::new(90., 80.));
        s.press(Point::new(50., 80.), 3.);
        s.release(Point::new(50., 50.));
        assert!(s.document.reset_curve());
        assert_eq!(s.tool, Tool::Arrow);
        assert!(s.curve_handle(Point::new(50., 80.), 3.));
        assert!(s.document.undo());
        assert!(s.document.redo());
        assert!(s.document.selected.is_some());
        assert_eq!(s.curve_handle_position(), Some(Point::new(50., 80.)));
        assert!(s.document.undo());
        assert!(s.document.undo());
        assert!(s.document.undo());
        assert_eq!(s.document.selected, None);
        assert_eq!(s.curve_handle_position(), None);
    }

    #[test]
    fn degenerate_new_line_does_not_expose_a_handle() {
        let mut s = Session::new(100, 100);
        s.choose(Tool::Line);
        s.press(Point::new(10., 20.), 2.);
        assert!(!s.release(Point::new(10., 20.)));
        assert_eq!(s.curve_handle_position(), None);
        assert!(!s.curve_handle(Point::new(10., 20.), f64::NAN));
    }

    #[test]
    fn direct_bend_is_available_without_select_and_keeps_drawing_mode() {
        for tool in [Tool::Line, Tool::Arrow] {
            let mut s = Session::new(100, 100);
            s.choose(tool);
            s.press(Point::new(10., 60.), 2.);
            assert!(s.release(Point::new(90., 60.)));
            assert!(s.curve_handle(Point::new(50., 60.), 3.));
            s.press(Point::new(50., 60.), 3.);
            assert!(s.curve_drag.is_some());
            assert!(s.release(Point::new(50., 30.)));
            assert_eq!(s.document.objects.len(), 1);
            assert_eq!(s.tool, tool);
            assert_eq!(
                s.document.objects[0].curve_midpoint(),
                Some(Point::new(50., 30.))
            );
            assert!(s.document.undo());
            assert_eq!(s.document.objects[0].points.len(), 2);
            assert!(s.document.redo());
            s.press(Point::new(10., 80.), 3.);
            assert!(s.release(Point::new(90., 80.)));
            assert_eq!(s.document.objects.len(), 2);
        }
    }
    #[test]
    fn hidden_curve_handle_outside_crop_cannot_start_a_drag() {
        let mut s = Session::new(100, 80);
        s.document.add(
            Tool::Line,
            vec![Point::new(10., 60.), Point::new(90., 60.)],
            Style::default(),
        );
        s.document.move_selected(-100., 0.);
        assert!(!s.curve_handle(Point::new(-50., 60.), 5.));
    }
    #[test]
    fn pattern_selection_and_highlighter_do_not_replace_next_normal_defaults() {
        let mut s = Session::new(100, 80);
        s.choose(Tool::Pen);
        s.set_pattern(crate::editor::Pattern::Dashed);
        s.press(Point::new(10., 20.), 2.);
        s.release(Point::new(90., 20.));
        assert_eq!(
            s.document.objects[0].pattern,
            crate::editor::Pattern::Dashed
        );
        s.choose(Tool::Highlighter);
        assert!(!s.set_pattern(crate::editor::Pattern::Dotted));
        s.press(Point::new(10., 40.), 2.);
        s.release(Point::new(90., 40.));
        assert_eq!(s.document.objects[1].pattern, crate::editor::Pattern::Solid);
        s.choose(Tool::Select);
        s.press(Point::new(30., 20.), 1.);
        s.release(Point::new(30., 20.));
        assert!(s.set_pattern(crate::editor::Pattern::Dotted));
        s.style = Style {
            rgb: [0, 0, 255],
            width: 2,
        };
        s.document.restyle(s.style);
        s.choose(Tool::Line);
        assert_eq!(s.style, Style::default());
        assert_eq!(s.pattern(), crate::editor::Pattern::Dashed);
        s.press(Point::new(10., 60.), 2.);
        s.release(Point::new(90., 60.));
        assert_eq!(
            s.document.objects[2].pattern,
            crate::editor::Pattern::Dashed
        );
    }
    #[test]
    fn dragging_selected_line_midpoint_bends_without_moving_endpoints() {
        let mut s = Session::new(100, 80);
        s.choose(Tool::Line);
        s.press(Point::new(10., 60.), 2.);
        s.release(Point::new(90., 60.));
        s.choose(Tool::Select);
        s.press(Point::new(30., 60.), 2.);
        s.release(Point::new(30., 60.));
        let before = s.document.revision;
        s.press(Point::new(50., 60.), 2.);
        s.update(Point::new(50., 30.));
        assert_eq!(s.document.revision, before, "drag must remain uncommitted");
        assert!(s.release(Point::new(50., 30.)));
        assert_eq!(s.document.objects[0].points[0], Point::new(10., 60.));
        assert_eq!(s.document.objects[0].points[1], Point::new(90., 60.));
        assert_eq!(
            s.document.objects[0].curve_midpoint(),
            Some(Point::new(50., 30.))
        );
        assert!(s.document.undo());
        assert_eq!(
            s.document.objects[0].curve_midpoint(),
            Some(Point::new(50., 60.))
        );
        assert!(s.document.redo());
        assert_eq!(
            s.document.objects[0].curve_midpoint(),
            Some(Point::new(50., 30.))
        );
    }

    #[test]
    fn line_endpoint_drag_preserves_curve_and_does_not_draw_a_new_line() {
        let mut s = Session::new(200, 120);
        s.choose(Tool::Line);
        s.press(Point::new(10., 20.), 3.);
        assert!(s.release(Point::new(110., 60.)));
        s.press(Point::new(60., 40.), 3.);
        assert!(s.release(Point::new(60., 80.)));
        let curved = s.document.objects[0].clone();
        assert_eq!(curved.points.len(), 3);

        s.press(Point::new(10., 20.), 3.);
        s.update(Point::new(24., 30.));
        assert_eq!(s.document.objects.len(), 1);
        assert!(s.release(Point::new(24., 30.)));
        assert_eq!(s.document.objects.len(), 1);
        assert_eq!(s.document.objects[0].points[0], Point::new(24., 30.));
        assert_eq!(s.document.objects[0].points[1], curved.points[1]);
        assert_eq!(s.document.objects[0].points[2], curved.points[2]);

        assert!(s.document.undo());
        assert_eq!(s.document.objects[0], curved);
    }
    #[test]
    fn curve_drag_cancel_and_no_motion_preserve_committed_revision() {
        let mut s = Session::new(100, 80);
        s.document.add(
            Tool::Arrow,
            vec![
                Point::new(10., 60.),
                Point::new(90., 60.),
                Point::new(50., 0.),
            ],
            Style::default(),
        );
        let before = s.document.objects[0].clone();
        let revision = s.document.revision;
        s.press(Point::new(50., 30.), 3.);
        s.release(Point::new(50., 30.));
        assert_eq!(s.document.revision, revision);
        s.press(Point::new(50., 30.), 3.);
        s.update(Point::new(50., 45.));
        s.cancel();
        assert_eq!(s.document.objects[0], before);
        assert_eq!(s.document.revision, revision);
    }
    #[test]
    fn deleting_selection_keeps_its_style_family_for_the_next_tool() {
        for tool in [Tool::Line, Tool::Highlighter] {
            let mut s = Session::new(100, 80);
            s.choose(tool);
            s.press(Point::new(10., 20.), 2.);
            s.release(Point::new(80., 20.));
            s.choose(Tool::Select);
            s.press(Point::new(40., 20.), 2.);
            s.release(Point::new(40., 20.));
            let custom = Style {
                rgb: [0, 160, 80],
                width: 2,
            };
            s.style = custom;
            s.document.restyle(custom);
            s.document.delete();
            s.document.undo();
            assert_eq!(s.document.selected, None);
            s.choose(Tool::Pen);
            assert_eq!(
                s.style,
                if tool == Tool::Highlighter {
                    Style::default()
                } else {
                    custom
                }
            );
            if tool == Tool::Highlighter {
                s.choose(Tool::Highlighter);
                assert_eq!(s.style, custom);
            }
        }
    }
    #[test]
    fn highlighter_cancellation_and_style_switch_do_not_change_pen_defaults() {
        let mut s = Session::new(100, 80);
        s.choose(Tool::Pen);
        s.style = Style {
            rgb: [12, 34, 56],
            width: 7,
        };
        s.choose(Tool::Highlighter);
        assert_eq!(s.parameter(), 24);
        assert_eq!(s.opacity(), 40);
        assert!(!s.set_opacity(101));
        s.press(Point::new(10., 20.), 2.);
        assert!(!s.release(Point::new(10., 20.)));
        s.press(Point::new(10., 20.), 2.);
        s.update(Point::new(80., 20.));
        s.cancel();
        assert!(!s.release(Point::new(80., 20.)));
        assert!(s.document.objects.is_empty());
        assert!(!s.document.can_undo());
        s.set_parameter(18).unwrap();
        s.set_opacity(65);
        s.choose(Tool::Pen);
        assert_eq!(
            s.style,
            Style {
                rgb: [12, 34, 56],
                width: 7
            }
        );
        s.choose(Tool::Highlighter);
        assert_eq!(s.parameter(), 18);
        assert_eq!(s.opacity(), 65);
    }
    #[test]
    fn next_highlight_commits_the_opacity_shown_after_undo() {
        let mut s = Session::new(100, 80);
        s.choose(Tool::Highlighter);
        s.press(Point::new(10., 20.), 2.);
        assert!(s.release(Point::new(80., 20.)));
        assert!(s.set_opacity(70));
        assert!(s.document.undo());
        assert_eq!(s.opacity(), 40);
        s.press(Point::new(10., 50.), 2.);
        assert!(s.release(Point::new(80., 50.)));
        assert_eq!(
            s.document.objects[1].content,
            crate::editor::Content::Highlight
        );
        assert_eq!(s.document.objects[1].opacity, 40);
    }
    #[test]
    fn highlighter_command_draws_with_independent_defaults() {
        use crate::editor::{Point, Style, Tool};
        let tool = crate::editor_controls::tool(633)
            .expect("Highlighter command must reach the drawing session");
        let mut session = super::Session::new(80, 60);
        session.choose(tool);
        session.press(Point::new(10., 25.), 2.);
        session.update(Point::new(30., 25.));
        assert!(session.release(Point::new(60., 25.)));
        let object = &session.document.objects[0];
        assert_eq!(object.tool, tool);
        assert_eq!(object.style.rgb, [255, 242, 88]);
        assert_eq!(object.style.width, 24);
        assert_eq!(object.points.len(), 3);
        session.choose(Tool::Pen);
        assert_eq!(session.style, Style::default());
        assert!(session.document.undo());
        assert!(session.document.objects.is_empty());
        assert!(session.document.redo());
        assert_eq!(session.document.objects[0].style.width, 24);
    }

    #[test]
    fn freehand_path_resize_scales_points_but_keeps_stroke_style() {
        let mut s = Session::new(160, 100);
        s.choose(Tool::Pen);
        s.press(Point::new(10., 10.), 3.);
        s.update(Point::new(30., 30.));
        assert!(s.release(Point::new(50., 10.)));
        let original = s.document.objects[0].clone();
        assert_eq!(s.path_handles().len(), 8);

        s.press(Point::new(50., 20.), 3.);
        s.update(Point::new(90., 20.));
        assert_eq!(s.document.objects[0], original);
        assert!(s.release(Point::new(90., 20.)));
        assert_eq!(
            s.document.objects[0].points.as_ref(),
            &[
                Point::new(10., 10.),
                Point::new(50., 30.),
                Point::new(90., 10.)
            ]
        );
        assert_eq!(s.document.objects[0].style, original.style);
        assert!(s.document.undo());
        assert_eq!(s.document.objects[0], original);
    }
    use super::*;
    #[test]
    fn shape_corner_resizes_immediately_in_drawing_mode() {
        for tool in [
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::FilledRectangle,
            Tool::FilledEllipse,
        ] {
            let mut s = Session::new(240, 180);
            s.choose(tool);
            s.press(Point::new(20., 30.), 3.);
            assert!(s.release(Point::new(100., 90.)));
            let original = s.document.objects[0].clone();
            let revision = s.document.revision;
            s.press(Point::new(100., 90.), 3.);
            s.update(Point::new(160., 120.));
            assert_eq!(s.document.revision, revision);
            assert!(s.release(Point::new(160., 120.)));
            assert_eq!(
                s.document.objects.len(),
                1,
                "handle must not draw another shape"
            );
            assert_eq!(
                s.document.objects[0].bounds(),
                (Point::new(20., 30.), Point::new(160., 120.))
            );
            assert_eq!(s.document.objects[0].id, original.id);
            assert_eq!(s.document.objects[0].style, original.style);
            assert_eq!(s.tool, tool);
            assert!(s.document.undo());
            assert_eq!(s.document.objects[0], original);
            assert!(s.document.redo());
            assert_eq!(s.document.objects[0].bounds().1, Point::new(160., 120.));
        }
    }
    #[test]
    fn ordinary_opacity_default_and_selected_edit_are_independent_of_highlighter() {
        let mut s = Session::new(240, 180);
        s.choose(Tool::Rectangle);
        assert_eq!(s.opacity(), 100);
        s.set_opacity(65);
        s.press(Point::new(20., 30.), 3.);
        s.release(Point::new(100., 90.));
        assert_eq!(s.opacity(), 65);
        assert!(s.set_opacity(25));
        assert_eq!(s.opacity(), 25);
        assert!(s.document.undo());
        assert_eq!(s.opacity(), 65);
        s.choose(Tool::Highlighter);
        assert_eq!(s.opacity(), 40);
        assert!(!s.set_opacity(101));
    }
    #[test]
    fn shape_all_handles_anchor_cancel_clamp_and_preserve_drawing() {
        use crate::shape_frame::{Handle, positions};
        let a = Point::new(40., 40.);
        let b = Point::new(120., 100.);
        for tool in [
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::FilledRectangle,
            Tool::FilledEllipse,
        ] {
            for (handle, point) in positions((a, b)) {
                let mut s = Session::new(240, 180);
                s.choose(tool);
                s.press(b, 2.);
                s.release(a);
                assert_eq!(s.shape_handles().len(), 8);
                assert_eq!(s.shape_handle(point, 2.), Some(handle));
                let original = s.document.objects[0].clone();
                let revision = s.document.revision;
                let end = Point::new(point.x + 12., point.y + 15.);
                s.press(point, 2.);
                s.update(end);
                s.cancel();
                assert!(!s.release(end));
                assert_eq!(s.document.revision, revision);
                assert_eq!(s.document.objects[0], original);
                s.press(point, 2.);
                assert!(!s.release(point));
                assert_eq!(s.document.revision, revision);
                s.press(point, 2.);
                assert!(s.release(end));
                let (x, y) = s.document.objects[0].bounds();
                assert_eq!(
                    x.x,
                    if matches!(handle, Handle::TopLeft | Handle::BottomLeft | Handle::Left) {
                        a.x + 12.
                    } else {
                        a.x
                    }
                );
                assert_eq!(
                    x.y,
                    if matches!(handle, Handle::TopLeft | Handle::TopRight | Handle::Top) {
                        a.y + 15.
                    } else {
                        a.y
                    }
                );
                assert_eq!(
                    y.x,
                    if matches!(
                        handle,
                        Handle::TopRight | Handle::BottomRight | Handle::Right
                    ) {
                        b.x + 12.
                    } else {
                        b.x
                    }
                );
                assert_eq!(
                    y.y,
                    if matches!(
                        handle,
                        Handle::BottomLeft | Handle::BottomRight | Handle::Bottom
                    ) {
                        b.y + 15.
                    } else {
                        b.y
                    }
                );
                assert!(s.document.undo());
                assert_eq!(s.document.objects[0], original);
                s.press(Point::new(180., 120.), 2.);
                s.update(Point::new(220., 160.));
                assert!(s.shape_handles().is_empty());
                assert!(s.release(Point::new(220., 160.)));
                assert_eq!(s.document.objects.len(), 2);
                s.choose(Tool::Select);
                s.press(Point::new(40., 70.), 2.);
                s.release(Point::new(40., 70.));
                assert_eq!(s.document.selected, Some(original.id));
                s.press(a, 2.);
                assert!(s.release(Point::new(200., 200.)));
                assert_eq!(s.document.objects[0].bounds(), (Point::new(119., 99.), b));
            }
        }
    }

    #[test]
    fn blur_region_uses_eight_shape_handles_and_one_step_history() {
        let mut s = Session::new(200, 120);
        s.choose(Tool::Blur);
        s.press(Point::new(20., 20.), 3.);
        assert!(s.release(Point::new(80., 60.)));
        assert_eq!(s.shape_handles().len(), 8);

        let revision = s.document.revision;
        s.press(Point::new(80., 40.), 3.);
        s.update(Point::new(110., 40.));
        assert_eq!(s.document.revision, revision);
        assert!(s.release(Point::new(110., 40.)));
        assert_eq!(
            s.document.objects[0].bounds(),
            (Point::new(20., 20.), Point::new(110., 60.))
        );
        assert!(s.document.undo());
        assert_eq!(
            s.document.objects[0].bounds(),
            (Point::new(20., 20.), Point::new(80., 60.))
        );
    }
    #[test]
    fn hidden_shape_handles_and_invalid_geometry_do_not_mutate_document() {
        let mut s = Session::new(240, 180);
        s.choose(Tool::Rectangle);
        s.press(Point::new(20., 30.), 2.);
        s.release(Point::new(120., 100.));
        s.document.set_crop(Rect::new(50, 50, 100, 100).unwrap());
        assert_eq!(s.shape_handle(Point::new(20., 30.), 5.), None);
        assert_eq!(
            s.shape_handle(Point::new(120., 100.), 5.),
            Some(crate::shape_frame::Handle::BottomRight)
        );
        assert_eq!(s.shape_handle(Point::new(120., 100.), f64::NAN), None);
        let old = s.document.objects[0].clone();
        let revision = s.document.revision;
        s.press(Point::new(120., 100.), 2.);
        assert!(!s.release(Point::new(f64::NAN, 90.)));
        s.press(Point::new(120., 100.), 2.);
        assert!(!s.release(Point::new(40000., 90.)));
        assert_eq!(s.document.objects[0], old);
        assert_eq!(s.document.revision, revision);
        s.choose(Tool::Pen);
        assert!(s.shape_handles().is_empty());
    }
    #[test]
    fn opacity_selection_does_not_replace_drawing_defaults_and_undo_controls_next_mark() {
        let mut s = Session::new(240, 180);
        s.choose(Tool::FilledRectangle);
        s.set_opacity(65);
        s.press(Point::new(20., 30.), 2.);
        s.release(Point::new(100., 90.));
        assert!(s.set_opacity(25));
        assert!(s.document.undo());
        assert_eq!(s.opacity(), 65);
        s.press(Point::new(150., 100.), 2.);
        s.release(Point::new(200., 150.));
        assert_eq!(s.document.objects[1].opacity, 65);
        s.choose(Tool::Select);
        s.press(Point::new(50., 50.), 2.);
        s.release(Point::new(50., 50.));
        assert!(s.set_opacity(10));
        s.choose(Tool::Pen);
        assert_eq!(s.opacity(), 65);
        s.choose(Tool::Blur);
        let revision = s.document.revision;
        assert!(!s.set_opacity(20));
        assert_eq!(s.document.revision, revision);
    }
    #[test]
    fn image_gesture_moves_resizes_on_release_and_cancels_without_history() {
        use crate::frame::Frame;
        use std::sync::Arc;
        let mut s = Session::new(160, 80);
        s.document
            .add_image(Arc::new(Frame::new(40, 20, vec![255; 3200]).unwrap()))
            .unwrap();
        s.press(Point::new(10., 10.), 4.);
        s.release(Point::new(70., 40.));
        assert_eq!(
            s.document.objects[0].bounds(),
            (Point::new(60., 30.), Point::new(100., 50.))
        );
        let rev = s.document.revision;
        s.press(Point::new(100., 50.), 4.);
        s.update(Point::new(140., 70.));
        assert_eq!(s.document.revision, rev);
        assert_eq!(
            s.resize.as_ref().unwrap().bounds(),
            (Point::new(60., 30.), Point::new(140., 70.))
        );
        s.cancel();
        assert!(!s.release(Point::new(140., 70.)));
        assert_eq!(s.document.revision, rev);
        s.press(Point::new(100., 50.), 4.);
        assert!(s.release(Point::new(140., 70.)));
        assert_eq!(
            s.document.objects[0].bounds(),
            (Point::new(60., 30.), Point::new(140., 70.))
        );
        assert_eq!(
            s.resize_handle(Point::new(100., 30.), 4.),
            Some(crate::text_frame::Handle::Top),
            "image top and bottom handles must allow intentional height changes"
        );
        s.press(Point::new(100., 30.), 4.);
        assert!(s.release(Point::new(100., 40.)));
        assert_eq!(
            s.document.objects[0].bounds(),
            (Point::new(60., 40.), Point::new(140., 70.))
        );
        assert!(s.document.undo());
        assert_eq!(
            s.document.objects[0].bounds(),
            (Point::new(60., 30.), Point::new(140., 70.))
        );
        assert_eq!(
            s.resize_handle(Point::new(60., 50.), 4.),
            Some(crate::text_frame::Handle::Left),
            "image side handles must allow intentional width changes"
        );
        s.press(Point::new(60., 50.), 4.);
        assert!(s.release(Point::new(80., 50.)));
        assert_eq!(
            s.document.objects[0].bounds(),
            (Point::new(80., 30.), Point::new(140., 70.)),
            "a side handle changes only one axis"
        );
        s.document.undo();
        assert_eq!(
            s.document.objects[0].bounds(),
            (Point::new(60., 30.), Point::new(140., 70.))
        );
    }
    #[test]
    fn background_toggle_keeps_color_for_new_text_without_editing_shapes() {
        let mut s = Session::new(800, 600);
        let chosen = Background {
            enabled: true,
            rgb: [30, 60, 90],
        };
        s.set_background(chosen);
        assert_eq!(s.document.revision, 0);
        s.document
            .add_label(
                Point::new(30., 30.),
                "First",
                24,
                s.style,
                s.text_background(),
                false,
            )
            .unwrap();
        let original = s.document.objects[0].clone();
        s.set_background(Background {
            enabled: false,
            ..chosen
        });
        assert_eq!(
            s.text_background(),
            Background {
                enabled: false,
                ..chosen
            }
        );
        assert!(s.document.undo());
        assert_eq!(s.document.objects[0], original);
        s.choose(Tool::Text);
        assert_eq!(
            s.text_background(),
            Background {
                enabled: false,
                ..chosen
            }
        );
        s.document.add(
            Tool::Line,
            vec![Point::new(0., 0.), Point::new(10., 10.)],
            s.style,
        );
        let revision = s.document.revision;
        s.set_background(chosen);
        assert_eq!(s.document.revision, revision);
    }
    #[test]
    fn callout_tail_drag_has_priority_one_undo_and_cancellation() {
        let mut s = Session::new(800, 600);
        s.document
            .add_callout(
                Point::new(50., 50.),
                "Bubble",
                24,
                Style::default(),
                Default::default(),
            )
            .unwrap();
        let corner = s.document.objects[0].bounds().1;
        s.document.set_tail(corner);
        let original = s.document.objects[0].clone();
        s.press(corner, 5.);
        assert!(s.update(Point::new(260., 200.)));
        assert_eq!(s.document.objects[0], original);
        s.cancel();
        assert_eq!(s.document.objects[0], original);
        s.press(corner, 5.);
        assert!(s.release(Point::new(260., 200.)));
        assert_eq!(
            s.document.objects[0].tail_tip(),
            Some(Point::new(260., 200.))
        );
        assert_eq!(s.document.objects[0].bounds(), original.bounds());
        s.document.undo();
        assert_eq!(s.document.objects[0], original);
        let revision = s.document.revision;
        s.press(corner, 5.);
        assert!(!s.release(corner));
        assert_eq!(s.document.revision, revision);
    }
    #[test]
    fn callout_wrap_keeps_tail_relative_and_fill_independent() {
        let mut s = Session::new(800, 600);
        s.document
            .add_callout(
                Point::new(50., 50.),
                "Words in a long text line",
                24,
                Style::default(),
                crate::editor::Background {
                    enabled: true,
                    rgb: [25, 80, 180],
                },
            )
            .unwrap();
        let original = s.document.objects[0].clone();
        let (a, b) = original.bounds();
        let start = Point::new(b.x, (a.y + b.y) / 2.);
        s.press(start, 5.);
        assert!(s.release(Point::new(a.x + 120., start.y)));
        assert_eq!(s.document.objects[0].bounds().1.x, a.x + 120.);
        assert_eq!(s.document.objects[0].tail_tip(), original.tail_tip());
        assert_eq!(
            s.document.objects[0].text_background(),
            original.text_background()
        );
        s.document.undo();
        assert_eq!(s.document.objects[0], original);
    }
    #[test]
    fn selected_text_handle_drag_reflows_only_on_release_and_is_one_undo() {
        let mut s = Session::new(800, 600);
        s.document
            .add_text(
                Point::new(40., 40.),
                "English words in a longer line",
                24,
                Style::default(),
            )
            .unwrap();
        let before = s.document.objects[0].clone();
        let revision = s.document.revision;
        let (a, b) = before.bounds();
        let start = Point::new(b.x, (a.y + b.y) / 2.);
        s.press(start, 5.);
        assert!(s.update(Point::new(a.x + 120., start.y)));
        assert!(!s.update(Point::new(a.x + 120., start.y)));
        assert_eq!(s.document.objects[0], before);
        assert_eq!(s.document.revision, revision);
        assert!(s.release(Point::new(a.x + 120., start.y)));
        assert_eq!(s.document.objects[0].parameter(), 24);
        assert_eq!(s.document.objects[0].bounds().0, a);
        assert_eq!(s.document.objects[0].bounds().1.x, a.x + 120.);
        assert!(s.document.objects[0].bounds().1.y > b.y);
        s.document.undo();
        assert_eq!(s.document.objects[0], before);
    }
    #[test]
    fn text_corner_drag_scaling_can_cancel_and_does_not_commit_no_motion() {
        let mut s = Session::new(800, 600);
        s.document
            .add_text(Point::new(40., 40.), "Text", 24, Style::default())
            .unwrap();
        let before = s.document.objects[0].clone();
        let (a, b) = before.bounds();
        let end = Point::new(a.x + 2. * (b.x - a.x), a.y + 2. * (b.y - a.y));
        s.press(b, 5.);
        s.update(end);
        s.cancel();
        assert!(!s.release(end));
        assert_eq!(s.document.objects[0], before);
        s.press(b, 5.);
        assert!(!s.release(b));
        s.press(b, 5.);
        assert!(s.release(end));
        assert_eq!(s.document.objects[0].parameter(), 48);
        assert_eq!(s.document.objects[0].bounds().0, a);
        s.document.undo();
        assert_eq!(s.document.objects[0], before);
    }
    #[test]
    fn steps_commit_on_click_and_blur_commits_only_on_release() {
        let mut s = Session::new(300, 200);
        s.choose(Tool::Step);
        s.press(Point::new(30., 30.), 2.);
        assert!(s.document.objects.is_empty());
        assert!(s.release(Point::new(30., 30.)));
        assert_eq!(s.document.step_numbers(), [1]);
        s.choose(Tool::Blur);
        s.press(Point::new(80., 30.), 2.);
        s.update(Point::new(200., 100.));
        assert_eq!(s.document.objects.len(), 1);
        assert!(s.release(Point::new(200., 100.)));
        assert_eq!(s.document.objects[1].tool, Tool::Blur);
        s.document.undo();
        assert_eq!(s.document.step_numbers(), [1]);
    }

    #[test]
    fn step_corner_resize_stays_circular_and_anchors_the_opposite_corner() {
        let mut s = Session::new(160, 100);
        s.choose(Tool::Step);
        s.press(Point::new(50., 50.), 3.);
        assert!(s.release(Point::new(50., 50.)));
        let original = s.document.objects[0].clone();
        assert_eq!(s.step_handles().len(), 4);

        s.press(Point::new(68., 68.), 3.);
        s.update(Point::new(86., 86.));
        assert_eq!(s.document.objects[0], original);
        assert!(s.release(Point::new(86., 86.)));
        assert_eq!(
            s.document.objects[0].bounds(),
            (Point::new(32., 32.), Point::new(86., 86.))
        );
        assert_eq!(s.document.objects[0].parameter(), 54);
        assert!(s.document.undo());
        assert_eq!(s.document.objects[0], original);
    }
    #[test]
    fn idle_or_unchanged_pointer_does_not_request_a_redraw() {
        let mut s = Session::new(100, 80);
        assert!(
            !s.update(Point::new(20., 20.)),
            "Hovering must not repaint the editor."
        );
        s.choose(Tool::Line);
        s.press(Point::new(10., 10.), 5.);
        assert!(!s.update(Point::new(10., 10.)));
        assert!(s.update(Point::new(20., 20.)));
        assert!(!s.update(Point::new(20., 20.)));
        s.choose(Tool::Crop);
        s.press(Point::new(10., 10.), 5.);
        s.release(Point::new(30., 30.));
        assert!(
            !s.update(Point::new(50., 50.)),
            "A pending crop must not follow a hovering mouse."
        );
    }
    #[test]
    fn drawing_is_one_undo_step_and_cancellation_is_not_an_edit() {
        let mut s = Session::new(100, 80);
        s.choose(Tool::Pen);
        s.press(Point::new(10., 10.), 2.);
        s.update(Point::new(30., 10.));
        assert!(!s.document.can_undo());
        assert!(s.release(Point::new(40., 20.)));
        assert_eq!(s.document.objects[0].points.len(), 3);
        assert!(s.document.undo());
        assert!(!s.document.can_undo());
        s.press(Point::new(20., 20.), 2.);
        s.update(Point::new(50., 50.));
        s.cancel();
        assert!(!s.document.can_undo());
        assert!(s.document.objects.is_empty());
    }
    #[test]
    fn move_is_committed_only_on_release() {
        let mut s = Session::new(100, 80);
        s.document.add(
            Tool::Line,
            vec![Point::new(10., 10.), Point::new(70., 10.)],
            Style::default(),
        );
        s.press(Point::new(20., 10.), 2.);
        s.update(Point::new(25., 20.));
        assert_eq!(s.document.objects[0].points[0], Point::new(10., 10.));
        assert!(s.release(Point::new(25., 20.)));
        assert_eq!(s.document.objects[0].points[0], Point::new(15., 20.));
    }
    #[test]
    fn crop_waits_for_confirmation_and_cancel_restores_canvas() {
        let mut s = Session::new(100, 80);
        s.choose(Tool::Crop);
        s.press(Point::new(70., 60.), 2.);
        s.release(Point::new(20., 10.));
        assert_eq!(s.crop_draft(), Some(Rect::new(20, 10, 50, 50).unwrap()));
        assert_eq!(s.document.crop.width, 100);
        s.cancel();
        assert!(!s.confirm_crop());
        assert!(!s.document.can_undo());
        s.press(Point::new(70., 60.), 2.);
        s.release(Point::new(20., 10.));
        assert!(s.confirm_crop());
        assert_eq!(s.document.crop.width, 50);
        assert!(s.document.undo());
        assert_eq!(s.document.crop.width, 100);
    }
}

use crate::capture::Candidate;
use crate::frame::{Frame, Rect};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use uiautomation::{
    UIAutomation, UIElement, UITreeWalker,
    patterns::UIScrollPattern,
    types::{ControlType, Handle, Rect as UiRect},
};
use windows::{Win32::UI::Accessibility::IUIAutomation2, core::Interface};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{ClientToScreen, ScreenToClient},
    UI::WindowsAndMessaging::{
        CWP_SKIPDISABLED, CWP_SKIPINVISIBLE, CWP_SKIPTRANSPARENT, ChildWindowFromPointEx,
        GetClientRect, GetParent, GetScrollInfo, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindow, IsWindowVisible, PostMessageW, SB_VERT, SCROLLINFO,
        SIF_ALL,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaneElement {
    pub bounds: Rect,
    pub vertically_scrollable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaneCandidate {
    pub window: usize,
    pub process: u32,
    pub dpi: u32,
    pub title: String,
    pub bounds: Rect,
}

#[derive(Clone, Debug)]
pub struct DetectedPane {
    pub pane: PaneCandidate,
    pub target: Candidate,
    pub source: Source,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Sheet,
    Native,
    Accessibility,
    Window,
}

#[derive(Clone)]
pub struct Request {
    pub owner: usize,
    pub notifier: usize,
    pub notify_message: u32,
    pub generation: u64,
    pub point: (i32, i32),
    pub desktop: Rect,
    pub windows: Vec<Candidate>,
    pub snapshot: Option<Arc<Frame>>,
}

pub struct DetectionResult {
    pub owner: usize,
    pub generation: u64,
    pub finished: bool,
    pub timed_out: bool,
    pub result: Option<DetectedPane>,
}

#[derive(Clone)]
pub struct Service {
    pending: Arc<(Mutex<Option<Queued>>, Condvar)>,
    active: Arc<Mutex<Option<(usize, Arc<AtomicBool>)>>>,
}

struct Queued {
    request: Request,
    cancelled: Arc<AtomicBool>,
}

impl Service {
    pub fn new() -> (Self, mpsc::Receiver<DetectionResult>) {
        let pending = Arc::new((Mutex::new(None::<Queued>), Condvar::new()));
        let worker_pending = pending.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let mut cache_owner = 0;
            let mut sheets = HashMap::new();
            loop {
                let Queued { request, cancelled } = {
                    let (lock, wake) = &*worker_pending;
                    let mut pending = lock.lock().unwrap_or_else(|e| e.into_inner());
                    while pending.is_none() {
                        pending = wake.wait(pending).unwrap_or_else(|e| e.into_inner());
                    }
                    pending.take().unwrap()
                };
                if cancelled.load(Ordering::Acquire) {
                    continue;
                }
                let started = std::time::Instant::now();
                crate::trace::record("scroll.detect.started", started);
                if crate::settings::is_test_run() {
                    let delay = std::env::var("SNAPZY_TEST_UIA_HANDOFF_DELAY_MS")
                        .ok()
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(0)
                        .min(2_000);
                    while started.elapsed() < Duration::from_millis(delay)
                        && !cancelled.load(Ordering::Acquire)
                    {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    if cancelled.load(Ordering::Acquire) {
                        continue;
                    }
                }
                if cache_owner != request.owner {
                    cache_owner = request.owner;
                    sheets.clear();
                }
                let (result, timed_out) = detect(&request, &cancelled, &mut sheets);
                crate::trace::record(
                    match result.as_ref().map(|r| r.source) {
                        Some(Source::Sheet) => "scroll.detect.sheet",
                        Some(Source::Native) => "scroll.detect.native",
                        Some(Source::Accessibility) => "scroll.detect.accessibility",
                        Some(Source::Window) => "scroll.detect.window-fallback",
                        None => "scroll.detect.no-match",
                    },
                    started,
                );
                crate::trace::record(
                    if timed_out {
                        "scroll.detect.timeout"
                    } else {
                        "scroll.detect.finished"
                    },
                    started,
                );
                if cancelled.load(Ordering::Acquire) {
                    continue;
                }
                let _ = sender.send(DetectionResult {
                    owner: request.owner,
                    generation: request.generation,
                    finished: true,
                    timed_out,
                    result,
                });
                unsafe { PostMessageW(request.notifier as HWND, request.notify_message, 0, 0) };
            }
        });
        (
            Self {
                pending,
                active: Arc::new(Mutex::new(None)),
            },
            receiver,
        )
    }

    pub fn submit(&self, request: Request) {
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((_, old)) = active.replace((request.owner, cancelled.clone())) {
            old.store(true, Ordering::Release);
        }
        let (lock, wake) = &*self.pending;
        *lock.lock().unwrap_or_else(|e| e.into_inner()) = Some(Queued { request, cancelled });
        wake.notify_one();
    }

    pub fn cancel(&self, owner: usize) {
        let active = self.active.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((current, cancelled)) = active.as_ref() {
            if *current == owner {
                cancelled.store(true, Ordering::Release);
            }
        }
    }
}

pub fn select_nearest_pane(
    ancestors_nearest_first: &[PaneElement],
    target: &PaneCandidate,
    desktop: Rect,
    point: (i32, i32),
) -> Option<PaneCandidate> {
    ancestors_nearest_first
        .iter()
        .filter(|element| element.vertically_scrollable)
        .find_map(|element| {
            let bounds = element
                .bounds
                .intersection(target.bounds)?
                .intersection(desktop)?;
            (bounds.width >= 40 && bounds.height >= 40 && bounds.contains(point.0, point.1)).then(
                || PaneCandidate {
                    bounds,
                    ..target.clone()
                },
            )
        })
}

fn detect(
    request: &Request,
    cancelled: &AtomicBool,
    sheets: &mut HashMap<(usize, i32, i32, u32, u32), Option<Rect>>,
) -> (Option<DetectedPane>, bool) {
    let Some(hit_target) = request
        .windows
        .iter()
        .find(|candidate| candidate.rect.contains(request.point.0, request.point.1))
    else {
        return (None, false);
    };
    let Some(target_client) = client_bounds(hit_target.window)
        .and_then(|r| r.intersection(request.desktop))
        .and_then(|r| {
            crate::capture::monitor_at(request.point.0, request.point.1)
                .ok()
                .and_then(|m| r.intersection(m))
        })
    else {
        return (None, false);
    };
    if !target_client.contains(request.point.0, request.point.1) {
        return (None, false);
    }
    let key = (
        hit_target.window,
        target_client.x,
        target_client.y,
        target_client.width,
        target_client.height,
    );
    let sheet = sheets.entry(key).or_insert_with(|| {
        crate::sheet_viewport::find_in_desktop(
            request.snapshot.as_ref()?,
            request.desktop,
            target_client,
        )
    });
    if let Some(bounds) = sheet.filter(|r| r.contains(request.point.0, request.point.1)) {
        return (
            Some(make_detected(hit_target, bounds, Source::Sheet)),
            false,
        );
    }
    let native_path = native_path_at(hit_target.window, request.point);
    if let Some(native) = detect_native(&native_path, hit_target, target_client, request.desktop) {
        return (Some(native), false);
    }
    let probe = crate::scroll_probe::run(request, cancelled, || {});
    let uia = probe.pane.and_then(|(window, bounds)| {
        let target = request.windows.iter().find(|w| w.window == window)?;
        if bounds.width < 40
            || bounds.height < 40
            || !bounds.contains(request.point.0, request.point.1)
            || bounds.intersection(request.desktop) != Some(bounds)
            || bounds.intersection(client_bounds(window)?) != Some(bounds)
        {
            return None;
        }
        let bounds = bounds.intersection(target_client)?;
        let detected = make_detected(target, bounds, Source::Accessibility);
        is_current(&detected).then_some(detected)
    });
    (
        uia.or_else(|| {
            (target_client.width >= 80 && target_client.height >= 100)
                .then(|| make_detected(hit_target, target_client, Source::Window))
        }),
        probe.timed_out,
    )
}

fn make_detected(target: &Candidate, bounds: Rect, source: Source) -> DetectedPane {
    DetectedPane {
        pane: PaneCandidate {
            window: target.window,
            process: process_of(target.window),
            dpi: dpi_for(target.window),
            title: window_title(target.window),
            bounds,
        },
        target: target.clone(),
        source,
    }
}

fn native_path_at(window: usize, point: (i32, i32)) -> Vec<HWND> {
    let mut path = Vec::new();
    let mut current = window as HWND;
    for _ in 0..32 {
        let mut local = POINT {
            x: point.0,
            y: point.1,
        };
        unsafe {
            ScreenToClient(current, &mut local);
        }
        let child = unsafe {
            ChildWindowFromPointEx(
                current,
                local,
                CWP_SKIPINVISIBLE | CWP_SKIPDISABLED | CWP_SKIPTRANSPARENT,
            )
        };
        if child.is_null() || child == current {
            break;
        }
        current = child;
    }
    for _ in 0..32 {
        if current.is_null() {
            break;
        }
        path.push(current);
        current = unsafe { GetParent(current) };
    }
    path
}

pub(crate) fn detect_uia(request: &Request, on_ready: impl FnOnce()) -> Option<DetectedPane> {
    let automation = UIAutomation::new()
        .map_err(|e| crate::trace::test_note(&format!("uia.init error={e}")))
        .ok()?;
    if let Ok(automation2) = automation.as_ref().cast::<IUIAutomation2>() {
        unsafe {
            let _ = automation2.SetConnectionTimeout(1_500);
            let _ = automation2.SetTransactionTimeout(2_000);
        }
    }
    let target = request
        .windows
        .iter()
        .find(|w| w.rect.contains(request.point.0, request.point.1))?;
    let element = automation
        .element_from_handle(Handle::from(target.window as isize))
        .map_err(|e| crate::trace::test_note(&format!("uia.root error={e}")))
        .ok()?;
    on_ready();
    if crate::settings::is_test_run() {
        if let Ok(delay) = std::env::var("SNAPZY_TEST_UIA_HANDOFF_DELAY_MS") {
            if let Ok(delay) = delay.parse::<u64>() {
                std::thread::sleep(std::time::Duration::from_millis(delay.min(2_000)));
            }
        }
    }
    let process = process_of(target.window);
    let target_client = client_bounds(target.window)?;
    let target_pane = PaneCandidate {
        window: target.window,
        process,
        dpi: dpi_for(target.window),
        title: window_title(target.window),
        bounds: target_client,
    };
    let walker = automation.get_raw_view_walker().ok()?;
    let mut budget = TreeBudget {
        started: Instant::now(),
        remaining: 250,
    };
    let (scroll, document) = search_tree(&element, &walker, request.point, &mut budget, 0);
    crate::trace::test_note(&format!(
        "uia.search scroll={scroll:?} document={document:?} remaining={} elapsed={:?}",
        budget.remaining,
        budget.started.elapsed()
    ));
    let bounds = if let Some(bounds) = scroll {
        select_nearest_pane(
            &[PaneElement {
                bounds,
                vertically_scrollable: true,
            }],
            &target_pane,
            request.desktop,
            request.point,
        )?
        .bounds
    } else {
        document?
            .intersection(target_pane.bounds)?
            .intersection(request.desktop)?
    };
    Some(make_detected(target, bounds, Source::Accessibility))
}

struct TreeBudget {
    started: Instant,
    remaining: usize,
}

// Cold accessibility providers can spend more than 250 ms on the root alone.
// The supervising process still enforces the four-second hard deadline.
const TREE_SEARCH_DEADLINE: Duration = Duration::from_millis(1_500);

fn search_tree(
    element: &UIElement,
    walker: &UITreeWalker,
    point: (i32, i32),
    budget: &mut TreeBudget,
    depth: usize,
) -> (Option<Rect>, Option<Rect>) {
    if budget.remaining == 0 || depth > 20 || budget.started.elapsed() > TREE_SEARCH_DEADLINE {
        crate::trace::test_note(&format!(
            "uia.budget depth={depth} remaining={} elapsed={:?}",
            budget.remaining,
            budget.started.elapsed()
        ));
        return (None, None);
    }
    budget.remaining -= 1;
    let Some(bounds) = element
        .get_bounding_rectangle()
        .ok()
        .and_then(rect_from_uia)
        .filter(|r| r.contains(point.0, point.1))
    else {
        return (None, None);
    };
    let offscreen = element.is_offscreen().unwrap_or(true);
    crate::trace::test_note(&format!(
        "uia.node depth={depth} bounds={bounds:?} offscreen={offscreen}"
    ));
    if offscreen {
        return (None, None);
    }
    let kind = element.get_control_type().ok();
    let mut best = (bounds.width >= 80
        && bounds.height >= 100
        && element
            .get_pattern::<UIScrollPattern>()
            .and_then(|p| p.is_vertically_scrollable())
            .unwrap_or(false))
    .then_some(bounds);
    let mut document =
        (kind == Some(ControlType::Document) && bounds.width >= 200 && bounds.height >= 200)
            .then_some(bounds);
    let mut child = walker.get_first_child(element).ok();
    while let Some(current) = child {
        if budget.remaining == 0 || budget.started.elapsed() > TREE_SEARCH_DEADLINE {
            break;
        }
        let (nested, nested_document) = search_tree(&current, walker, point, budget, depth + 1);
        if let Some(nested) = nested {
            if best.is_none_or(|b| {
                nested.width as u64 * (nested.height as u64) < b.width as u64 * (b.height as u64)
            }) {
                best = Some(nested);
            }
        }
        if document.is_none() {
            document = nested_document;
        }
        child = walker.get_next_sibling(&current).ok();
    }
    if matches!(kind, Some(ControlType::DataGrid | ControlType::Table)) {
        if let Some(body) = best {
            if bounds.width as u64 * 10 <= body.width as u64 * 12
                && bounds.height as u64 <= body.height as u64 * 3
            {
                best = Some(bounds);
            }
        }
    }
    (best, document)
}

fn detect_native(
    path: &[HWND],
    target: &Candidate,
    target_client: Rect,
    desktop: Rect,
) -> Option<DetectedPane> {
    for &current in path {
        let mut scroll: SCROLLINFO = unsafe { std::mem::zeroed() };
        scroll.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
        scroll.fMask = SIF_ALL;
        if unsafe { GetScrollInfo(current, SB_VERT, &mut scroll) } != 0
            && scroll.nMax > scroll.nMin
            && scroll.nPage > 0
            && (scroll.nMax - scroll.nMin + 1) > scroll.nPage as i32
        {
            let mut client = unsafe { std::mem::zeroed::<RECT>() };
            if unsafe { GetClientRect(current, &mut client) } != 0 {
                let mut origin = POINT {
                    x: client.left,
                    y: client.top,
                };
                if unsafe { ClientToScreen(current, &mut origin) } != 0 {
                    if let Some(bounds) = Rect::from_edges(
                        origin.x,
                        origin.y,
                        origin.x + client.right - client.left,
                        origin.y + client.bottom - client.top,
                    )
                    .ok()
                    .and_then(|r| r.intersection(target_client))
                    .and_then(|r| r.intersection(desktop))
                    .filter(|r| r.width >= 40 && r.height >= 40)
                    {
                        return Some(DetectedPane {
                            pane: PaneCandidate {
                                window: target.window,
                                process: process_of(target.window),
                                dpi: dpi_for(target.window),
                                title: window_title(target.window),
                                bounds,
                            },
                            target: target.clone(),
                            source: Source::Native,
                        });
                    }
                }
            }
        }
    }
    None
}

fn rect_from_uia(rect: UiRect) -> Option<Rect> {
    Rect::from_edges(
        rect.get_left(),
        rect.get_top(),
        rect.get_right(),
        rect.get_bottom(),
    )
    .ok()
}

fn client_bounds(window: usize) -> Option<Rect> {
    let hwnd = window as HWND;
    let mut client = unsafe { std::mem::zeroed::<RECT>() };
    if unsafe { IsWindow(hwnd) } == 0
        || unsafe { IsWindowVisible(hwnd) } == 0
        || unsafe { GetClientRect(hwnd, &mut client) } == 0
    {
        return None;
    }
    let mut origin = POINT {
        x: client.left,
        y: client.top,
    };
    if unsafe { ClientToScreen(hwnd, &mut origin) } == 0 {
        return None;
    }
    Rect::from_edges(
        origin.x,
        origin.y,
        origin.x + client.right - client.left,
        origin.y + client.bottom - client.top,
    )
    .ok()
}

fn process_of(window: usize) -> u32 {
    let mut process = 0;
    unsafe {
        GetWindowThreadProcessId(window as HWND, &mut process);
    }
    process
}

fn dpi_for(window: usize) -> u32 {
    unsafe { windows_sys::Win32::UI::HiDpi::GetDpiForWindow(window as _) }
}

fn window_title(window: usize) -> String {
    let hwnd = window as HWND;
    let length = unsafe { GetWindowTextLengthW(hwnd) }.clamp(0, 512) as usize;
    let mut title = vec![0u16; length + 1];
    let read = unsafe { GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32) };
    String::from_utf16_lossy(&title[..read.max(0) as usize])
}

pub fn is_current(detected: &DetectedPane) -> bool {
    let pane = &detected.pane;
    let process = process_of(pane.window);
    let dpi = dpi_for(pane.window);
    if process != pane.process
        || dpi != pane.dpi
        || crate::capture::current_window_rect(pane.window) != Some(detected.target.rect)
    {
        return false;
    }
    let client = client_bounds(pane.window);
    client.and_then(|client| client.intersection(pane.bounds)) == Some(pane.bounds)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    AwaitingClick {
        generation: u64,
    },
    Detecting {
        generation: u64,
    },
    Preview {
        generation: u64,
        candidate: PaneCandidate,
    },
    NoMatch {
        generation: u64,
    },
    Cancelled,
    Started {
        generation: u64,
    },
}

impl Default for State {
    fn default() -> Self {
        Self::AwaitingClick { generation: 0 }
    }
}

impl State {
    pub fn begin_hover(&mut self) -> Option<u64> {
        let generation = match self {
            Self::AwaitingClick { generation }
            | Self::Detecting { generation }
            | Self::Preview { generation, .. }
            | Self::NoMatch { generation } => generation.checked_add(1)?,
            Self::Cancelled | Self::Started { .. } => return None,
        };
        *self = Self::Detecting { generation };
        Some(generation)
    }

    pub fn confirm_at(
        &mut self,
        point: (i32, i32),
        pressed: &PaneCandidate,
    ) -> Option<PaneCandidate> {
        match self {
            Self::Preview { candidate, .. }
                if candidate == pressed && candidate.bounds.contains(point.0, point.1) =>
            {
                self.confirm()
            }
            _ => None,
        }
    }

    pub fn detection_finished(&mut self, generation: u64, candidate: Option<PaneCandidate>) {
        if !matches!(self, Self::Detecting { generation: current } | Self::Preview { generation: current, .. } if *current == generation)
        {
            return;
        }
        *self = match candidate {
            Some(candidate) => Self::Preview {
                generation,
                candidate,
            },
            None => Self::NoMatch { generation },
        };
    }

    pub fn cancel(&mut self) {
        *self = Self::Cancelled;
    }

    pub fn confirm(&mut self) -> Option<PaneCandidate> {
        match self {
            Self::Preview {
                generation,
                candidate,
            } => {
                let generation = *generation;
                let candidate = candidate.clone();
                *self = Self::Started { generation };
                Some(candidate)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, width: u32, height: u32) -> Rect {
        Rect::new(x, y, width, height).unwrap()
    }

    fn target(bounds: Rect) -> PaneCandidate {
        PaneCandidate {
            window: 42,
            process: 7,
            dpi: 96,
            title: "Spreadsheet".into(),
            bounds,
        }
    }

    #[test]
    fn hover_replaces_pending_generation_and_rejects_stale_completion() {
        let mut state = State::default();
        let first = state.begin_hover().unwrap();
        let second = state.begin_hover().unwrap();
        assert!(second > first);
        state.detection_finished(first, Some(target(rect(0, 0, 300, 400))));
        assert_eq!(state, State::Detecting { generation: second });
        let current = target(rect(400, 0, 300, 400));
        state.detection_finished(second, Some(current.clone()));
        assert_eq!(state.confirm_at((450, 50), &current), Some(current));
        assert!(state.begin_hover().is_none());
    }

    #[test]
    fn hover_click_requires_the_same_frame_and_release_inside_it() {
        let mut state = State::default();
        let generation = state.begin_hover().unwrap();
        let current = target(rect(20, 20, 300, 400));
        state.detection_finished(generation, Some(current.clone()));
        assert!(state.confirm_at((0, 0), &current).is_none());
        let other = target(rect(30, 20, 300, 400));
        assert!(state.confirm_at((50, 50), &other).is_none());
        assert_eq!(state.confirm_at((50, 50), &current), Some(current.clone()));
        assert!(state.confirm_at((50, 50), &current).is_none());
        state.cancel();
        assert!(state.begin_hover().is_none());
    }

    #[test]
    fn retained_hover_frame_can_be_refined_but_a_held_old_frame_cannot_confirm_it() {
        let mut state = State::default();
        let generation = state.begin_hover().unwrap();
        let parent = target(rect(0, 0, 600, 500));
        state.detection_finished(generation, Some(parent.clone()));
        let nested = target(rect(100, 100, 300, 300));
        state.detection_finished(generation, Some(nested.clone()));
        assert_eq!(state.confirm_at((150, 150), &parent), None);
        assert_eq!(state.confirm_at((150, 150), &nested), Some(nested));
    }

    #[test]
    fn selects_nearest_vertical_pane_and_clips_to_target_and_desktop() {
        let desktop = rect(0, 0, 1000, 800);
        let target = target(rect(100, 100, 700, 600));
        let ancestors = [
            PaneElement {
                bounds: rect(140, 150, 500, 400),
                vertically_scrollable: false,
            },
            PaneElement {
                bounds: rect(80, 90, 650, 500),
                vertically_scrollable: true,
            },
            PaneElement {
                bounds: rect(100, 100, 700, 600),
                vertically_scrollable: true,
            },
        ];
        let found = select_nearest_pane(&ancestors, &target, desktop, (120, 120)).unwrap();
        assert_eq!(found.bounds, rect(100, 100, 630, 490));
        assert_eq!(found.window, target.window);
        assert_eq!(found.process, target.process);
    }

    #[test]
    fn ignores_horizontal_only_and_too_small_panes() {
        let target = target(rect(100, 100, 700, 600));
        let panes = [
            PaneElement {
                bounds: rect(200, 200, 10, 500),
                vertically_scrollable: true,
            },
            PaneElement {
                bounds: rect(100, 100, 700, 600),
                vertically_scrollable: false,
            },
        ];
        assert!(select_nearest_pane(&panes, &target, rect(0, 0, 1000, 800), (120, 120)).is_none());
    }

    #[test]
    fn rejects_scrollable_bounds_that_do_not_contain_the_clicked_point() {
        let target = target(rect(100, 100, 700, 600));
        let panes = [PaneElement {
            bounds: rect(300, 300, 200, 200),
            vertically_scrollable: true,
        }];
        assert!(select_nearest_pane(&panes, &target, rect(0, 0, 1000, 800), (120, 120)).is_none());
    }

    #[test]
    fn no_match_can_be_retried_with_a_new_generation() {
        let mut state = State::default();
        let first = state.begin_hover().unwrap();
        state.detection_finished(first, None);
        assert_eq!(state, State::NoMatch { generation: first });
        let retry = state.begin_hover().unwrap();
        assert!(retry > first);
        assert_eq!(state, State::Detecting { generation: retry });
    }

    #[test]
    fn a_click_before_the_hover_is_ready_does_not_start_capture() {
        let mut state = State::default();
        let first = state.begin_hover().unwrap();
        assert_eq!(state.confirm(), None);
        assert_eq!(state, State::Detecting { generation: first });

        let current = target(rect(10, 10, 200, 300));
        state.detection_finished(first, Some(current.clone()));
        assert_eq!(
            state,
            State::Preview {
                generation: first,
                candidate: current
            }
        );
    }

    #[test]
    fn cancel_invalidates_detection_and_confirmation_consumes_candidate_once() {
        let mut state = State::default();
        let generation = state.begin_hover().unwrap();
        state.cancel();
        state.detection_finished(generation, Some(target(rect(10, 10, 200, 300))));
        assert_eq!(state, State::Cancelled);

        let mut state = State::default();
        let generation = state.begin_hover().unwrap();
        let candidate = target(rect(10, 10, 200, 300));
        state.detection_finished(generation, Some(candidate.clone()));
        assert_eq!(state.confirm(), Some(candidate));
        assert_eq!(state.confirm(), None);
        assert_eq!(state, State::Started { generation });
    }
}

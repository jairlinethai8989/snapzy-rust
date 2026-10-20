use crate::{
    capture::{self, Candidate},
    frame::{Frame, Rect},
    scroll::{Outcome, Settler, Stitcher},
    scroll_auto::{Action, Stepper},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, POINT, RECT},
    Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute},
    UI::{
        HiDpi::GetDpiForWindow,
        Input::KeyboardAndMouse::GetAsyncKeyState,
        WindowsAndMessaging::{
            EnumWindows, GA_ROOT, GetAncestor, GetForegroundWindow, GetWindowRect,
            GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, SMTO_ABORTIFHUNG,
            SMTO_BLOCK, SMTO_ERRORONEXIT, SendMessageTimeoutW, WM_MOUSEWHEEL, WindowFromPoint,
        },
    },
};

#[derive(Clone, Copy, Debug)]
pub enum Status {
    Settling,
    Finishing,
    FinalUnconfirmed,
    Unstable,
    Capturing,
    Ambiguous,
    TargetChanged,
    Limit,
    CaptureFailed,
    Paused,
    InputBlocked,
    NoNewContent,
}
pub enum Event {
    Progress(Status, u32),
    Done(Result<Frame, String>, Option<Status>),
}
pub struct Job {
    pub window: usize,
    pub region: Rect,
    pub rx: Receiver<Event>,
    control: Arc<AtomicU8>,
}
struct Target {
    candidate: Candidate,
    dpi: u32,
    process: u32,
}
impl Job {
    pub fn start(
        window: usize,
        region: Rect,
        target: Candidate,
        budget: usize,
        automatic: bool,
    ) -> Self {
        let (tx, rx) = mpsc::sync_channel(1);
        let control = Arc::new(AtomicU8::new(0));
        let worker_control = control.clone();
        let dpi = unsafe { GetDpiForWindow(target.window as HWND) };
        let process = process_of(target.window);
        let target = Target {
            candidate: target,
            dpi,
            process,
        };
        std::thread::spawn(move || {
            run(
                window,
                region,
                target,
                budget,
                automatic,
                worker_control,
                tx,
            )
        });
        Self {
            window,
            region,
            rx,
            control,
        }
    }
    pub fn stop(&self) {
        self.control.store(1, Ordering::Release);
    }
    pub fn paused(&self) -> bool {
        self.control.load(Ordering::Acquire) == 3
    }
    pub fn toggle_pause(&self) {
        let value = self.control.load(Ordering::Acquire);
        if value == 0 || value == 3 {
            let _ = self.control.compare_exchange(
                value,
                if value == 0 { 3 } else { 0 },
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }
}
fn process_of(window: usize) -> u32 {
    let mut process = 0;
    unsafe {
        GetWindowThreadProcessId(window as HWND, &mut process);
    }
    process
}
impl Drop for Job {
    fn drop(&mut self) {
        self.control.store(2, Ordering::Release);
    }
}
fn valid_target(window: usize, region: Rect, target: &Candidate, dpi: u32) -> bool {
    let mut panel: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(window as HWND, &mut panel) } == 0 {
        return false;
    }
    if Rect::from_edges(panel.left, panel.top, panel.right, panel.bottom)
        .ok()
        .is_some_and(|r| r.intersection(region).is_some())
    {
        return false;
    }
    if unsafe {
        IsWindow(target.window as HWND) == 0
            || IsIconic(target.window as HWND) != 0
            || GetDpiForWindow(target.window as HWND) != dpi
    } {
        return false;
    }
    let windows = capture::window_candidates();
    let Some(i) = windows
        .iter()
        .position(|w| w.window == target.window && w.rect == target.rect)
    else {
        return false;
    };
    windows[..i]
        .iter()
        .all(|w| w.rect.intersection(region).is_none())
        && unobstructed(target.window, region)
}

// Enum every visible top-level window, including untitled popups omitted from
// capture candidates. Stop at the target in Z order; the panel is outside it.
fn unobstructed(target: usize, region: Rect) -> bool {
    struct Check {
        target: usize,
        region: Rect,
        found: bool,
        covered: bool,
    }
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
        let check = unsafe { &mut *(data as *mut Check) };
        if hwnd as usize == check.target {
            check.found = true;
            return 0;
        }
        if unsafe { IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 } {
            return 1;
        }
        let mut cloaked = 0u32;
        unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED as u32,
                &mut cloaked as *mut _ as *mut _,
                4,
            );
        }
        if cloaked != 0 {
            return 1;
        }
        let mut r: RECT = unsafe { std::mem::zeroed() };
        if unsafe { GetWindowRect(hwnd, &mut r) } != 0
            && Rect::from_edges(r.left, r.top, r.right, r.bottom)
                .ok()
                .is_some_and(|r| r.intersection(check.region).is_some())
        {
            check.covered = true;
            return 0;
        }
        1
    }
    let mut check = Check {
        target,
        region,
        found: false,
        covered: false,
    };
    unsafe {
        EnumWindows(Some(visit), &mut check as *mut _ as LPARAM);
    }
    check.found && !check.covered
}

fn wheel_down(target: &Candidate, region: Rect, process: u32, control: &AtomicU8) -> bool {
    let hwnd = target.window as HWND;
    let point = POINT {
        x: region.x + region.width as i32 / 2,
        y: region.y + region.height as i32 * 3 / 5,
    };
    // Wheel messages use signed 16-bit screen coordinates, including on monitors
    // left/above the primary display. Refuse unrepresentable coordinates.
    if i16::try_from(point.x).is_err() || i16::try_from(point.y).is_err() {
        return false;
    }
    unsafe {
        let mut current_process = 0;
        GetWindowThreadProcessId(hwnd, &mut current_process);
        if process == 0
            || current_process != process
            || GetAncestor(GetForegroundWindow(), GA_ROOT) != hwnd
        {
            return false;
        }
        if [1, 2, 4, 5, 6, 16, 17, 18, 91, 92]
            .iter()
            .any(|key| GetAsyncKeyState(*key) < 0)
        {
            return false;
        }
        let child = WindowFromPoint(point);
        if child.is_null()
            || GetAncestor(child, GA_ROOT) != hwnd
            || control.load(Ordering::Acquire) != 0
        {
            return false;
        }
        let coordinates = ((point.y as u16 as u32) << 16) | point.x as u16 as u32;
        let mut result = 0;
        SendMessageTimeoutW(
            child,
            WM_MOUSEWHEEL,
            (u16::MAX as usize - 119) << 16,
            coordinates as isize,
            SMTO_ABORTIFHUNG | SMTO_BLOCK | SMTO_ERRORONEXIT,
            250,
            &mut result,
        ) != 0
    }
}
fn deliver(tx: &SyncSender<Event>, control: &AtomicU8, mut event: Event) {
    while control.load(Ordering::Acquire) != 2 {
        match tx.try_send(event) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => return,
            Err(TrySendError::Full(e)) => {
                event = e;
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
fn run(
    window: usize,
    region: Rect,
    target: Target,
    budget: usize,
    automatic: bool,
    control: Arc<AtomicU8>,
    tx: SyncSender<Event>,
) {
    let started = Instant::now();
    let mut settler = Settler::default();
    let mut stitcher: Option<Stitcher> = None;
    let mut notice = None;
    let mut stepper = Stepper::default();
    let mut pause_reason = None;
    let mut was_paused = false;
    let mut stopping_since = None;
    let mut pending_step = false;
    let Target {
        candidate: target,
        dpi,
        process,
    } = target;
    loop {
        match control.load(Ordering::Acquire) {
            2 => return,
            1 => {
                if stopping_since.is_none() {
                    stopping_since = Some(started.elapsed());
                    settler = Settler::default();
                }
                if started.elapsed().saturating_sub(stopping_since.unwrap())
                    >= Duration::from_secs(2)
                {
                    notice = Some(Status::FinalUnconfirmed);
                    break;
                }
            }
            3 => {
                was_paused = true;
                let _ = tx.try_send(Event::Progress(
                    pause_reason.unwrap_or(Status::Paused),
                    stitcher.as_ref().map_or(0, |s| s.height),
                ));
                std::thread::sleep(Duration::from_millis(120));
                continue;
            }
            _ => {}
        }
        if was_paused && stopping_since.is_none() {
            settler = Settler::default();
            stepper.resume();
            pause_reason = None;
            was_paused = false;
        }
        let mut status = Status::Settling;
        if process == 0
            || process_of(target.window) != process
            || !valid_target(window, region, &target, dpi)
            || (automatic
                && stopping_since.is_none()
                && unsafe { GetForegroundWindow() } != target.window as HWND)
        {
            settler = Settler::default();
            status = Status::TargetChanged;
        } else if !automatic || stepper.ready(started.elapsed()) {
            match capture::snapshot(region) {
                Ok(frame) => {
                    if let Some(stable) = settler.offer(frame, started.elapsed()) {
                        // Sampling and overlap matching may outlive a target change.
                        // Never commit pixels from a moved, covered or cancelled source.
                        if control.load(Ordering::Acquire) == 2 {
                            return;
                        }
                        if process_of(target.window) != process
                            || !valid_target(window, region, &target, dpi)
                        {
                            settler = Settler::default();
                            if stopping_since.is_some() {
                                notice = Some(Status::TargetChanged);
                                break;
                            }
                            pause_reason = Some(Status::TargetChanged);
                            let _ =
                                control.compare_exchange(0, 3, Ordering::AcqRel, Ordering::Acquire);
                            continue;
                        }
                        let result = if let Some(s) = &mut stitcher {
                            s.add_with_memory(stable, crate::memory::available_scroll_bytes())
                        } else {
                            Stitcher::new_with_memory(
                                stable,
                                budget,
                                crate::memory::available_scroll_bytes(),
                            )
                            .map(|s| {
                                stitcher = Some(s);
                                Outcome::Added
                            })
                        };
                        match result {
                            Ok(outcome @ (Outcome::Added | Outcome::Unchanged)) => {
                                status = Status::Capturing;
                                if outcome == Outcome::Added {
                                    pending_step = false;
                                }
                                if stopping_since.is_some() {
                                    // An unchanged old frame can settle before a delayed
                                    // renderer handles the already-issued wheel.
                                    if !pending_step {
                                        break;
                                    }
                                } else if control.load(Ordering::Acquire) != 0 {
                                    continue;
                                }
                                if automatic && stopping_since.is_none() {
                                    match stepper.observe(outcome, started.elapsed()) {
                                        Action::Wheel => {
                                            // Stitch validation may take time on large frames;
                                            // recheck the full region before delivering input.
                                            if !valid_target(window, region, &target, dpi)
                                                || !wheel_down(&target, region, process, &control)
                                            {
                                                status = Status::InputBlocked;
                                            } else {
                                                pending_step = true;
                                            }
                                            settler = Settler::default();
                                        }
                                        Action::Finish => {
                                            notice = Some(Status::NoNewContent);
                                            break;
                                        }
                                        Action::Wait | Action::Pause => {}
                                    }
                                }
                            }
                            Ok(Outcome::Ambiguous) => status = Status::Ambiguous,
                            Ok(Outcome::Limit) => {
                                notice = Some(Status::Limit);
                                break;
                            }
                            Err(_) => {
                                notice = Some(Status::CaptureFailed);
                                break;
                            }
                        }
                    } else if settler.stalled(started.elapsed()) {
                        status = Status::Unstable;
                    }
                }
                Err(_) => {
                    settler = Settler::default();
                    status = Status::CaptureFailed;
                }
            }
        }
        if stopping_since.is_some() {
            match status {
                Status::TargetChanged
                | Status::Unstable
                | Status::Ambiguous
                | Status::CaptureFailed
                | Status::InputBlocked => {
                    notice = Some(status);
                    break;
                }
                _ => status = Status::Finishing,
            }
        }
        if automatic
            && matches!(
                status,
                Status::TargetChanged
                    | Status::Unstable
                    | Status::Ambiguous
                    | Status::CaptureFailed
                    | Status::InputBlocked
            )
        {
            pause_reason = Some(status);
            let _ = control.compare_exchange(0, 3, Ordering::AcqRel, Ordering::Acquire);
        }
        let _ = tx.try_send(Event::Progress(
            status,
            stitcher.as_ref().map_or(0, |s| s.height),
        ));
        std::thread::sleep(Duration::from_millis(120));
    }
    let image = stitcher
        .ok_or_else(|| "No stable scrolling frame was captured.".to_string())
        .and_then(|s| s.finish_with_memory(crate::memory::available_scroll_bytes()));
    deliver(&tx, &control, Event::Done(image, notice));
}

pub fn controller_position(
    region: Rect,
    display: Rect,
    width: u32,
    height: u32,
) -> Option<(i32, i32)> {
    let gap = 12;
    for (x, y) in [
        (region.x - width as i32 - gap, region.y),
        (region.right() + gap, region.y),
        (display.x, region.y - height as i32 - gap),
        (display.x, region.bottom() + gap),
    ] {
        if let Ok(r) = Rect::new(x, y, width, height) {
            if r.intersection(display) == Some(r) && r.intersection(region).is_none() {
                return Some((x, y));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controls_never_cover_capture_and_full_display_is_rejected() {
        let display = Rect::new(-1000, 0, 2000, 1000).unwrap();
        let region = Rect::new(0, 100, 240, 400).unwrap();
        let (x, y) = controller_position(region, display, 400, 150).unwrap();
        assert!(
            Rect::new(x, y, 400, 150)
                .unwrap()
                .intersection(region)
                .is_none()
        );
        assert_eq!(controller_position(display, display, 400, 150), None);
    }
}

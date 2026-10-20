use crate::{capture::Candidate, frame::Rect, scroll_detect::Request};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    os::windows::{io::AsRawHandle, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};

const PROVIDER_LIMIT: Duration = Duration::from_secs(4);

struct ProviderJob(windows_sys::Win32::Foundation::HANDLE);

impl ProviderJob {
    fn attach(child: &Child) -> std::io::Result<Self> {
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let job = Self(handle);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle()) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(job)
        }
    }
}

impl Drop for ProviderJob {
    fn drop(&mut self) {
        // Windows also closes this handle when the main app exits unexpectedly.
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

#[derive(Serialize, Deserialize)]
struct Input {
    point: (i32, i32),
    desktop: [i32; 4],
    windows: Vec<(usize, [i32; 4])>,
}

type Output = Option<(usize, [i32; 4])>;

pub struct Result {
    pub pane: Option<(usize, Rect)>,
    pub timed_out: bool,
}

#[derive(Debug)]
enum WaitError {
    Timeout,
    Cancelled,
    Io,
}

fn wait_for_child(
    child: &mut Child,
    limit: Duration,
    cancelled: &AtomicBool,
    mut on_poll: impl FnMut(),
) -> std::result::Result<ExitStatus, WaitError> {
    let started = Instant::now();
    let failure = loop {
        if cancelled.load(Ordering::Acquire) {
            break WaitError::Cancelled;
        }
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {}
            Err(_) => break WaitError::Io,
        }
        if started.elapsed() >= limit {
            break WaitError::Timeout;
        }
        on_poll();
        std::thread::sleep(Duration::from_millis(20));
    };
    let _ = child.kill();
    let _ = child.wait();
    Err(failure)
}

struct JobDirectory(PathBuf);

impl JobDirectory {
    fn new() -> std::io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "snapzy-scroll-{}-{stamp}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for JobDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn encode_rect(rect: Rect) -> [i32; 4] {
    [rect.x, rect.y, rect.width as i32, rect.height as i32]
}

fn decode_rect(rect: [i32; 4]) -> Option<Rect> {
    Rect::new(
        rect[0],
        rect[1],
        rect[2].try_into().ok()?,
        rect[3].try_into().ok()?,
    )
    .ok()
}

pub fn run(request: &Request, cancelled: &AtomicBool, mut on_ready: impl FnMut()) -> Result {
    let mut timed_out = false;
    let pane = (|| {
        let directory = JobDirectory::new().ok()?;
        let input_path = directory.0.join("input.json");
        let ready_path = directory.0.join("ready");
        let output_path = directory.0.join("result.json");
        let input = Input {
            point: request.point,
            desktop: encode_rect(request.desktop),
            windows: request
                .windows
                .iter()
                .map(|w| (w.window, encode_rect(w.rect)))
                .collect(),
        };
        fs::write(&input_path, serde_json::to_vec(&input).ok()?).ok()?;
        let mut command = Command::new(std::env::current_exe().ok()?);
        command
            .arg("--scroll-detect-worker")
            .arg(&input_path)
            .creation_flags(0x08000000)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if crate::settings::is_test_run() {
            command
                .arg("--test-data-dir")
                .arg(crate::settings::data_dir());
        }
        let mut child = command.spawn().ok()?;
        let _provider_job = match ProviderJob::attach(&child) {
            Ok(job) => job,
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        };
        let mut notified = false;
        match wait_for_child(&mut child, PROVIDER_LIMIT, cancelled, || {
            if !notified && ready_path.exists() {
                notified = true;
                on_ready();
            }
        }) {
            Ok(status) if status.success() => {}
            Err(WaitError::Timeout) => {
                timed_out = true;
                return None;
            }
            _ => return None,
        }
        if fs::metadata(&output_path).ok()?.len() > 4096 {
            return None;
        }
        let output: Output = serde_json::from_slice(&fs::read(output_path).ok()?).ok()?;
        let (window, bounds) = output?;
        Some((window, decode_rect(bounds)?))
    })();
    Result { pane, timed_out }
}

pub fn worker_main(input_path: &Path) -> std::result::Result<(), String> {
    unsafe {
        windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(
            windows_sys::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }
    if fs::metadata(input_path).map_err(|e| e.to_string())?.len() > 1024 * 1024 {
        return Err("Invalid detector request.".into());
    }
    let input: Input = serde_json::from_slice(&fs::read(input_path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if input.windows.len() > 4096 {
        return Err("Too many detector targets.".into());
    }
    let desktop = decode_rect(input.desktop).ok_or("Invalid desktop.")?;
    let windows = input
        .windows
        .into_iter()
        .map(|(window, rect)| {
            Some(Candidate {
                window,
                rect: decode_rect(rect)?,
            })
        })
        .collect::<Option<Vec<_>>>()
        .ok_or("Invalid target bounds.")?;
    let request = Request {
        owner: 0,
        notifier: 0,
        notify_message: 0,
        generation: 0,
        point: input.point,
        desktop,
        windows,
        snapshot: None,
    };
    let directory = input_path.parent().ok_or("Invalid detector directory.")?;
    // Fault injection is restricted to isolated test instances.
    if crate::settings::is_test_run() {
        if let Ok(delay) = std::env::var("SNAPZY_TEST_UIA_STALL_MS") {
            if let Ok(delay) = delay.parse::<u64>() {
                std::thread::sleep(Duration::from_millis(delay.min(30_000)));
            }
        }
    }
    let result = crate::scroll_detect::detect_uia(&request, || {
        let _ = fs::write(directory.join("ready"), b"1");
    });
    unsafe { windows::Win32::System::Com::CoUninitialize() };
    let output = result.map(|detected| (detected.pane.window, encode_rect(detected.pane.bounds)));
    fs::write(
        directory.join("result.json"),
        serde_json::to_vec(&output).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::process::CommandExt;

    #[test]
    fn stalled_provider_is_reaped_and_the_next_request_can_finish() {
        let mut stalled = Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        let cancelled = AtomicBool::new(false);
        let started = Instant::now();
        assert!(matches!(
            wait_for_child(&mut stalled, Duration::from_millis(100), &cancelled, || {}),
            Err(WaitError::Timeout)
        ));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(stalled.try_wait().unwrap().is_some());

        let mut next = Command::new("cmd.exe")
            .args(["/c", "exit", "0"])
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        assert!(
            wait_for_child(&mut next, Duration::from_secs(5), &cancelled, || {})
                .unwrap()
                .success()
        );
    }

    #[test]
    fn cancellation_reaps_the_provider_without_waiting_for_its_timeout() {
        let mut stalled = Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        let cancelled = AtomicBool::new(true);
        assert!(matches!(
            wait_for_child(&mut stalled, Duration::from_secs(10), &cancelled, || {}),
            Err(WaitError::Cancelled)
        ));
        assert!(stalled.try_wait().unwrap().is_some());
    }

    #[test]
    fn dropping_provider_job_terminates_its_child() {
        let mut child = Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        let job = ProviderJob::attach(&child).unwrap();
        drop(job);
        assert!(
            wait_for_child(
                &mut child,
                Duration::from_secs(2),
                &AtomicBool::new(false),
                || {}
            )
            .is_ok()
        );
        assert!(child.try_wait().unwrap().is_some());
    }
}

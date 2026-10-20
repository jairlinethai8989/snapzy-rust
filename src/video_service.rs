use crate::{
    video::{Command as VideoCommand, Control, Event, Request},
    video_file,
};
use std::{
    io::{BufReader, BufWriter, Write},
    os::windows::{io::AsRawHandle, process::CommandExt},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::Duration,
};
use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};

enum BrokerCommand {
    Send(VideoCommand),
    Kill,
}

pub struct Service {
    session: u64,
    commands: mpsc::Sender<BrokerCommand>,
    events: mpsc::Receiver<Event>,
}

impl Service {
    pub fn start(request: Request) -> Result<Self, String> {
        let request_bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        if request_bytes.len() + 1 > crate::video::MESSAGE_LIMIT {
            return Err("Video request exceeds its size limit.".into());
        }
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut child = Command::new(executable)
            .arg("--video-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|e| format!("Cannot start the video worker: {e}"))?;
        let job =
            Job::attach(&child).map_err(|e| format!("Cannot contain the video worker: {e}"))?;
        let mut stdin = BufWriter::new(
            child
                .stdin
                .take()
                .ok_or("Video worker has no command pipe.")?,
        );
        stdin
            .write_all(&request_bytes)
            .and_then(|_| stdin.write_all(b"\n"))
            .and_then(|_| stdin.flush())
            .map_err(|e| format!("Cannot send the video request: {e}"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Video worker has no event pipe.")?;
        let (event_tx, events) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let session = request.session;
        std::thread::spawn(move || {
            broker(child, job, stdin, stdout, session, command_rx, event_tx)
        });
        Ok(Self {
            session,
            commands: command_tx,
            events,
        })
    }

    pub fn command(&self, command: VideoCommand) -> bool {
        self.commands.send(BrokerCommand::Send(command)).is_ok()
    }

    pub fn session(&self) -> u64 {
        self.session
    }

    pub fn poll(&self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            events.push(event);
        }
        events
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.commands.send(BrokerCommand::Kill);
    }
}

fn broker(
    mut child: Child,
    _job: Job,
    mut stdin: BufWriter<ChildStdin>,
    stdout: impl std::io::Read + Send + 'static,
    session: u64,
    commands: mpsc::Receiver<BrokerCommand>,
    events: mpsc::Sender<Event>,
) {
    let (worker_tx, worker_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match video_file::read_message::<Event>(&mut reader) {
                Ok(Some(event)) => {
                    if worker_tx.send(Ok(event)).is_err() {
                        break;
                    }
                }
                Ok(None) => {
                    let _ = worker_tx.send(Err("Video worker closed its event pipe.".into()));
                    break;
                }
                Err(error) => {
                    let _ = worker_tx.send(Err(error));
                    break;
                }
            }
        }
    });
    let mut terminal = false;
    while !terminal {
        match commands.recv_timeout(Duration::from_millis(20)) {
            Ok(BrokerCommand::Send(command)) => {
                if let Err(detail) = write_control(&mut stdin, Control { session, command }) {
                    let _ = events.send(Event::Failed {
                        session,
                        detail: format!("Cannot control the video worker: {detail}"),
                        partial: None,
                    });
                    break;
                }
            }
            Ok(BrokerCommand::Kill) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        while let Ok(message) = worker_rx.try_recv() {
            match message {
                Ok(event) if event_session(&event) == session => {
                    terminal = matches!(
                        event,
                        Event::Saved { .. } | Event::Cancelled { .. } | Event::Failed { .. }
                    );
                    let _ = events.send(event);
                }
                Ok(_) => {}
                Err(detail) => {
                    let _ = events.send(Event::Failed {
                        session,
                        detail,
                        partial: None,
                    });
                    terminal = true;
                }
            }
        }
        if !terminal && child.try_wait().ok().flatten().is_some() {
            let _ = events.send(Event::Failed {
                session,
                detail: "Video worker stopped unexpectedly.".into(),
                partial: None,
            });
            terminal = true;
        }
    }
    let _ = child.wait();
}

fn write_control(writer: &mut impl Write, control: Control) -> Result<(), String> {
    let bytes = serde_json::to_vec(&control).map_err(|e| e.to_string())?;
    if bytes.len() + 1 > crate::video::MESSAGE_LIMIT {
        return Err("Video command exceeds its size limit.".into());
    }
    writer
        .write_all(&bytes)
        .and_then(|_| writer.write_all(b"\n"))
        .and_then(|_| writer.flush())
        .map_err(|e| e.to_string())
}

fn event_session(event: &Event) -> u64 {
    match event {
        Event::Ready { session, .. }
        | Event::Started { session }
        | Event::Progress { session, .. }
        | Event::Finalizing { session, .. }
        | Event::Saved { session, .. }
        | Event::Cancelled { session }
        | Event::Failed { session, .. } => *session,
    }
}

struct Job(isize);
impl Job {
    fn attach(child: &Child) -> std::io::Result<Self> {
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let job = Self(handle as isize);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as _,
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
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0 as _);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn event_sessions_cover_every_terminal_and_progress_message() {
        let values = [
            Event::Ready {
                session: 9,
                width: 1,
                height: 1,
            },
            Event::Started { session: 9 },
            Event::Progress {
                session: 9,
                elapsed_ms: 1,
                frames: 1,
            },
            Event::Finalizing {
                session: 9,
                reason: "stop".into(),
            },
            Event::Saved {
                session: 9,
                path: "x".into(),
                elapsed_ms: 1,
                frames: 1,
                reason: "stop".into(),
            },
            Event::Cancelled { session: 9 },
            Event::Failed {
                session: 9,
                detail: "x".into(),
                partial: None,
            },
        ];
        assert!(values.iter().all(|event| event_session(event) == 9));
    }
}

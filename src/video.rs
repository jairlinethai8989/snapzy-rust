use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MESSAGE_LIMIT: usize = 8192;
pub const START_SPACE: u64 = 512 * 1024 * 1024;
pub const STOP_SPACE: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Source {
    Monitor { handle: u64 },
    Window { handle: u64, pid: u32 },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    #[serde(default)]
    pub audio: crate::audio::Mode,
    pub session: u64,
    pub source: Source,
    pub output: PathBuf,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Start,
    Stop,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub session: u64,
    pub command: Command,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    Ready {
        session: u64,
        width: u32,
        height: u32,
    },
    Started {
        session: u64,
    },
    Progress {
        session: u64,
        elapsed_ms: u64,
        frames: u64,
    },
    Finalizing {
        session: u64,
        reason: String,
    },
    Saved {
        session: u64,
        path: PathBuf,
        elapsed_ms: u64,
        frames: u64,
        reason: String,
    },
    Cancelled {
        session: u64,
    },
    Failed {
        session: u64,
        detail: String,
        partial: Option<PathBuf>,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Preparing,
    Countdown,
    Starting,
    Recording,
    Finalizing,
    Complete,
    Cancelled,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Start,
    Timeout,
}
pub struct State {
    pub phase: Phase,
    pub since: u64,
    started: Option<u64>,
    elapsed: u64,
}
impl State {
    pub fn new(now: u64) -> Self {
        Self {
            phase: Phase::Preparing,
            since: now,
            started: None,
            elapsed: 0,
        }
    }
    pub fn ready(&mut self, now: u64) -> bool {
        if self.phase != Phase::Preparing {
            return false;
        }
        self.phase = Phase::Countdown;
        self.since = now;
        true
    }
    pub fn started(&mut self, now: u64) -> bool {
        if self.phase != Phase::Starting {
            return false;
        }
        self.phase = Phase::Recording;
        self.started = Some(now);
        self.since = now;
        true
    }
    pub fn stop(&mut self, now: u64) -> bool {
        if !matches!(
            self.phase,
            Phase::Preparing | Phase::Countdown | Phase::Starting | Phase::Recording
        ) {
            return false;
        }
        self.elapsed = self.elapsed(now);
        self.phase = Phase::Finalizing;
        self.since = now;
        true
    }
    pub fn tick(&mut self, now: u64) -> Action {
        let elapsed = now.saturating_sub(self.since);
        if self.phase == Phase::Countdown && elapsed >= 3000 {
            self.phase = Phase::Starting;
            self.since = now;
            return Action::Start;
        }
        if matches!(self.phase, Phase::Preparing | Phase::Starting) && elapsed >= 20_000
            || self.phase == Phase::Finalizing && elapsed >= 30_000
        {
            self.phase = Phase::Failed;
            return Action::Timeout;
        }
        Action::None
    }
    pub fn elapsed(&self, now: u64) -> u64 {
        if self.phase == Phase::Recording {
            now.saturating_sub(self.started.unwrap_or(now))
        } else {
            self.elapsed
        }
    }
}
pub fn elapsed_text(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
pub fn dimensions(width: u32, height: u32) -> Result<(u32, u32), String> {
    let w = width
        .checked_add(width % 2)
        .ok_or("Video dimensions overflow.")?;
    let h = height
        .checked_add(height % 2)
        .ok_or("Video dimensions overflow.")?;
    let bytes = u64::from(w)
        .checked_mul(u64::from(h))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or("Video dimensions overflow.")?;
    if w == 0 || h == 0 || bytes > 32 * 1024 * 1024 {
        return Err("Video source exceeds the 32 MiB frame limit.".into());
    }
    Ok((w, h))
}
pub fn bitrate(width: u32, height: u32) -> Result<u32, String> {
    dimensions(width, height)?;
    Ok(((width as u64) * (height as u64) * 30 / 8).clamp(8_000_000, 32_000_000) as u32)
}
pub fn media_time(milliseconds: u64) -> Result<i64, String> {
    milliseconds
        .checked_mul(10_000)
        .and_then(|v| v.try_into().ok())
        .ok_or("Recording timestamp overflow.".into())
}
pub fn can_start(space: Option<u64>) -> bool {
    space.is_some_and(|n| n >= START_SPACE)
}
pub fn must_stop(space: Option<u64>) -> bool {
    space.is_none_or(|n| n < STOP_SPACE)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_mode_requests_default_to_silent_and_reject_unknown_sources() {
        let base =
            serde_json::json!({"session":1,"source":{"Monitor":{"handle":1}},"output":"test.mp4"});
        let legacy: Request = serde_json::from_value(base.clone()).unwrap();
        assert_eq!(serde_json::to_value(legacy).unwrap()["audio"], "none");
        let mut selected = base;
        selected["audio"] = serde_json::json!("both");
        let request: Request = serde_json::from_value(selected.clone()).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap()["audio"], "both");
        selected["audio"] = serde_json::json!("unknown");
        assert!(serde_json::from_value::<Request>(selected).is_err());
    }
    #[test]
    fn countdown_starts_once_and_elapsed_begins_at_first_sample() {
        let mut s = State::new(10);
        assert!(s.ready(100));
        assert!(!s.ready(200));
        assert_eq!(s.tick(3099), Action::None);
        assert_eq!(s.tick(3100), Action::Start);
        assert_eq!(s.tick(3101), Action::None);
        assert_eq!(s.elapsed(4000), 0);
        assert!(s.started(4100));
        assert_eq!(s.elapsed(5100), 1000);
        assert!(!s.started(5200));
    }
    #[test]
    fn stop_is_idempotent_freezes_elapsed_and_deadline_uses_request_time() {
        let mut s = State::new(0);
        s.ready(0);
        s.tick(3000);
        s.started(3100);
        assert!(s.stop(5100));
        assert!(!s.stop(6000));
        assert_eq!(s.elapsed(9000), 2000);
        assert_eq!(s.tick(35099), Action::None);
        assert_eq!(s.tick(35100), Action::Timeout);
        assert_eq!(s.tick(35101), Action::None);
    }
    #[test]
    fn preparing_and_starting_time_out_but_recording_has_no_duration_limit() {
        let mut s = State::new(50);
        assert_eq!(s.tick(20049), Action::None);
        assert_eq!(s.tick(20050), Action::Timeout);
        let mut s = State::new(0);
        s.ready(0);
        s.tick(3000);
        assert_eq!(s.tick(23000), Action::Timeout);
        let mut s = State::new(0);
        s.ready(0);
        s.tick(3000);
        s.started(3000);
        assert_eq!(s.tick(360_003_000), Action::None);
        assert_eq!(s.elapsed(360_003_000), 360_000_000);
        assert_eq!(elapsed_text(s.elapsed(360_003_000)), "100:00:00");
    }
    #[test]
    fn geometry_pads_instead_of_cropping_and_rejects_excess_memory() {
        assert_eq!(dimensions(641, 479).unwrap(), (642, 480));
        assert_eq!(dimensions(3840, 2160).unwrap(), (3840, 2160));
        for (w, h) in [
            (0, 1),
            (1, 0),
            (8192, 8192),
            (u32::MAX, 2),
            (u32::MAX - 1, u32::MAX - 1),
        ] {
            assert!(dimensions(w, h).is_err());
        }
        assert_eq!(bitrate(1920, 1080).unwrap(), 8_000_000);
        assert_eq!(bitrate(3840, 2160).unwrap(), 31_104_000);
        assert!(media_time(u64::MAX).is_err());
        assert_eq!(media_time(360_000_000).unwrap(), 3_600_000_000_000);
    }
    #[test]
    fn low_or_unknown_space_never_means_unlimited_storage() {
        assert!(!can_start(None));
        assert!(!can_start(Some(START_SPACE - 1)));
        assert!(can_start(Some(START_SPACE)));
        assert!(must_stop(None));
        assert!(must_stop(Some(STOP_SPACE - 1)));
        assert!(!must_stop(Some(STOP_SPACE)));
    }
}

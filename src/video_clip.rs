use std::path::PathBuf;

#[derive(Debug)]
pub struct Clip {
    pub identity: u64,
    pub source: PathBuf,
    pub reason: String,
    pub copied: bool,
    pub saved: bool,
    pub operation: u64,
    pub saving: bool,
}

impl Clip {
    pub fn new(identity: u64, source: PathBuf, reason: String) -> Self {
        Self {
            identity,
            source,
            reason,
            copied: false,
            saved: false,
            operation: 0,
            saving: false,
        }
    }
    pub fn needs_review(&self) -> bool {
        !self.saving && !self.copied && !self.saved
    }
    pub fn begin_save(&mut self) -> Option<u64> {
        if self.saving {
            return None;
        }
        self.operation = self.operation.checked_add(1)?;
        self.saving = true;
        Some(self.operation)
    }
    pub fn finish_save(&mut self, identity: u64, operation: u64, success: bool) -> bool {
        if !self.saving || self.identity != identity || self.operation != operation {
            return false;
        }
        self.saving = false;
        self.saved |= success;
        true
    }
}

pub fn time_label(seconds: f64) -> String {
    let seconds = if seconds.is_finite() && seconds > 0. {
        seconds as u64
    } else {
        0
    };
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn copy_or_successful_save_allows_close_but_failure_requires_review() {
        let mut clip = Clip::new(1, "pending.mp4".into(), "Stopped".into());
        assert!(clip.needs_review());
        clip.copied = true;
        assert!(!clip.needs_review());
        clip.copied = false;
        let op = clip.begin_save().unwrap();
        assert!(
            !clip.needs_review(),
            "saving must block a second close flow"
        );
        assert!(clip.finish_save(1, op, false));
        assert!(clip.needs_review());
        let op = clip.begin_save().unwrap();
        assert!(clip.finish_save(1, op, true));
        assert!(!clip.needs_review());
        assert_eq!(clip.source, PathBuf::from("pending.mp4"));
    }
    #[test]
    fn save_rejects_duplicate_and_stale_completion_without_losing_current_operation() {
        let mut clip = Clip::new(8, "pending.mp4".into(), "Stopped".into());
        let first = clip.begin_save().unwrap();
        assert!(clip.begin_save().is_none());
        assert!(!clip.finish_save(7, first, true));
        assert!(!clip.finish_save(8, first + 1, true));
        assert!(clip.saving);
        assert!(clip.finish_save(8, first, false));
        let second = clip.begin_save().unwrap();
        assert!(second > first);
        assert!(!clip.finish_save(8, first, true));
        assert!(clip.saving);
        assert!(clip.finish_save(8, second, true));
        assert!(!clip.finish_save(8, second, false));
        assert!(clip.saved);
    }
    #[test]
    fn playback_time_handles_long_clips_and_invalid_native_values() {
        for (seconds, expected) in [
            (0., "00:00"),
            (65.9, "01:05"),
            (3601., "1:00:01"),
            (90061., "25:01:01"),
            (-1., "00:00"),
            (f64::NAN, "00:00"),
            (f64::INFINITY, "00:00"),
        ] {
            assert_eq!(time_label(seconds), expected);
        }
    }
}

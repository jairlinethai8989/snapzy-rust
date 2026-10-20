use crate::scroll::Outcome;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Wait,
    Wheel,
    Finish,
    Pause,
}

#[derive(Default)]
pub struct Stepper {
    sent: Option<Duration>,
    retries: u8,
}
impl Stepper {
    pub fn ready(&self, now: Duration) -> bool {
        self.sent
            .is_none_or(|t| now.saturating_sub(t) >= Duration::from_millis(600))
    }
    pub fn observe(&mut self, outcome: Outcome, now: Duration) -> Action {
        if matches!(outcome, Outcome::Added | Outcome::Unchanged) && !self.ready(now) {
            return Action::Wait;
        }
        match outcome {
            Outcome::Ambiguous | Outcome::Limit => return Action::Pause,
            Outcome::Added => self.retries = 0,
            Outcome::Unchanged => {
                if let Some(sent) = self.sent {
                    if now.saturating_sub(sent) < Duration::from_millis(1500) {
                        return Action::Wait;
                    }
                    if self.retries == 2 {
                        return Action::Finish;
                    }
                    self.retries += 1;
                }
            }
        }
        self.sent = Some(now);
        Action::Wheel
    }
    pub fn resume(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }
    #[test]
    fn one_step_requires_stable_acceptance_and_render_delay() {
        let mut s = Stepper::default();
        assert!(s.ready(ms(0)));
        assert_eq!(s.observe(Outcome::Added, ms(240)), Action::Wheel);
        assert!(!s.ready(ms(839)));
        assert!(s.ready(ms(840)));
        assert_eq!(s.observe(Outcome::Unchanged, ms(840)), Action::Wait);
        assert_eq!(s.observe(Outcome::Added, ms(960)), Action::Wheel);
        assert!(!s.ready(ms(1500)));
    }
    #[test]
    fn premature_stable_frame_cannot_request_another_wheel() {
        let mut s = Stepper::default();
        assert_eq!(s.observe(Outcome::Added, ms(0)), Action::Wheel);
        assert_eq!(s.observe(Outcome::Added, ms(599)), Action::Wait);
        assert_eq!(s.observe(Outcome::Added, ms(600)), Action::Wheel);
    }
    #[test]
    fn quiet_views_retry_boundedly_without_claiming_document_end() {
        let mut s = Stepper::default();
        assert_eq!(s.observe(Outcome::Added, ms(0)), Action::Wheel);
        for n in [1500, 3000] {
            assert_eq!(s.observe(Outcome::Unchanged, ms(n - 1)), Action::Wait);
            assert_eq!(s.observe(Outcome::Unchanged, ms(n)), Action::Wheel);
        }
        assert_eq!(s.observe(Outcome::Unchanged, ms(4500)), Action::Finish);
    }
    #[test]
    fn progress_resets_retries_and_resume_discards_pending_command() {
        let mut s = Stepper::default();
        s.observe(Outcome::Added, ms(0));
        s.observe(Outcome::Unchanged, ms(1500));
        assert_eq!(s.observe(Outcome::Added, ms(2100)), Action::Wheel);
        assert_eq!(s.observe(Outcome::Unchanged, ms(3600)), Action::Wheel);
        s.resume();
        assert!(s.ready(ms(3601)));
        assert_eq!(s.observe(Outcome::Unchanged, ms(3601)), Action::Wheel);
        assert_eq!(s.observe(Outcome::Ambiguous, ms(4300)), Action::Pause);
        assert_eq!(s.observe(Outcome::Limit, ms(4400)), Action::Pause);
    }
}

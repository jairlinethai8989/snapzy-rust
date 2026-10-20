#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub identity: u64,
    pub generation: u64,
    pub revision: u64,
}
pub struct Guard {
    pub token: Token,
    pub failed: bool,
}
impl Guard {
    pub fn new(identity: u64) -> Self {
        Self {
            token: Token {
                identity,
                generation: 0,
                revision: 0,
            },
            failed: false,
        }
    }
    pub fn observe(&mut self, revision: u64) {
        if self.token.revision != revision {
            self.token.revision = revision;
            self.token.generation += 1;
            self.failed = false;
        }
    }
    pub fn accepts(&self, token: Token) -> bool {
        self.token == token
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn undo_aba_and_native_handle_reuse_cannot_accept_stale_results() {
        let mut g = Guard::new(100);
        g.observe(1);
        let first = g.token;
        g.observe(2);
        g.observe(1);
        assert!(!g.accepts(first));
        assert!(g.accepts(g.token));
        let another = Guard::new(101);
        assert!(!another.accepts(g.token));
    }
    #[test]
    fn same_revision_does_not_reschedule_and_edits_clear_failure() {
        let mut g = Guard::new(1);
        g.observe(1);
        let token = g.token;
        g.failed = true;
        g.observe(1);
        assert_eq!(g.token, token);
        assert!(g.failed);
        g.observe(2);
        assert!(!g.failed);
        assert_ne!(g.token, token);
    }
}

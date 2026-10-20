use crate::settings::OpenMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub window: usize,
    pub identity: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct Route {
    pub target: Option<Key>,
    pub origin: Option<Key>,
}
pub struct Host {
    pub key: Key,
    pub tabs: Vec<Key>,
    pub active: Option<Key>,
}
pub fn close_set_matches(expected: &[(Key, u64)], live: &[(Key, u64)]) -> bool {
    expected.len() == live.len()
        && expected.iter().enumerate().all(|(i, member)| {
            !expected[..i].iter().any(|earlier| earlier.0 == member.0)
                && live.iter().filter(|candidate| *candidate == member).count() == 1
        })
}
#[derive(Default)]
pub struct Workspace {
    pub hosts: Vec<Host>,
    next_host: u64,
    last: Option<Key>,
    activation: Vec<Key>,
}
impl Workspace {
    pub fn add_host(&mut self, window: usize) -> Key {
        if let Some(host) = self.hosts.iter().find(|h| h.key.window == window) {
            return host.key;
        }
        self.next_host += 1;
        let key = Key {
            window,
            identity: self.next_host,
        };
        self.hosts.push(Host {
            key,
            tabs: Vec::new(),
            active: None,
        });
        self.activate(key);
        key
    }
    pub fn host(&self, key: Key) -> Option<&Host> {
        self.hosts.iter().find(|h| h.key == key)
    }
    pub fn host_for(&self, doc: Key) -> Option<Key> {
        self.hosts
            .iter()
            .find(|h| h.tabs.contains(&doc))
            .map(|h| h.key)
    }
    pub fn remove_empty_host(&mut self, host: Key) -> bool {
        let Some(index) = self
            .hosts
            .iter()
            .position(|h| h.key == host && h.tabs.is_empty())
        else {
            return false;
        };
        self.hosts.remove(index);
        self.activation.retain(|h| *h != host);
        self.last = self.activation.last().copied();
        true
    }
    pub fn add_doc(&mut self, host: Key, doc: Key) -> bool {
        if self.hosts.iter().map(|h| h.tabs.len()).sum::<usize>() >= 16
            || self
                .hosts
                .iter()
                .flat_map(|h| &h.tabs)
                .any(|d| d.window == doc.window || d.identity == doc.identity)
        {
            return false;
        }
        let Some(h) = self.hosts.iter_mut().find(|h| h.key == host) else {
            return false;
        };
        h.tabs.push(doc);
        h.active = Some(doc);
        self.activate(host);
        true
    }
    pub fn activate(&mut self, host: Key) {
        if self.host(host).is_some() {
            self.activation.retain(|h| *h != host);
            self.activation.push(host);
            self.last = Some(host);
        }
    }
    pub fn select(&mut self, host: Key, doc: Key) -> bool {
        let Some(h) = self
            .hosts
            .iter_mut()
            .find(|h| h.key == host && h.tabs.contains(&doc))
        else {
            return false;
        };
        h.active = Some(doc);
        self.activate(host);
        true
    }
    pub fn cycle(&mut self, host: Key, back: bool) -> Option<Key> {
        let h = self.host(host)?;
        let index = h.tabs.iter().position(|d| Some(*d) == h.active)?;
        let step = if back { h.tabs.len() - 1 } else { 1 };
        let doc = h.tabs[(index + step) % h.tabs.len()];
        self.select(host, doc);
        Some(doc)
    }
    pub fn remove_doc(&mut self, doc: Key) -> bool {
        let Some(index) = self.hosts.iter().position(|h| h.tabs.contains(&doc)) else {
            return false;
        };
        let h = &mut self.hosts[index];
        let tab = h.tabs.iter().position(|d| *d == doc).unwrap();
        h.tabs.remove(tab);
        if h.active == Some(doc) {
            h.active = h.tabs.get(tab).or_else(|| h.tabs.last()).copied();
        }
        if h.tabs.is_empty() {
            let key = h.key;
            self.hosts.remove(index);
            self.activation.retain(|h| *h != key);
            self.last = self.activation.last().copied();
        }
        true
    }
    pub fn route(&self, mode: OpenMode, origin: Option<Key>) -> Route {
        let target = if mode == OpenMode::Tab {
            origin.and_then(|d| self.host_for(d)).or(self.last)
        } else {
            None
        };
        Route { target, origin }
    }
    pub fn resolve(&self, route: Route) -> Result<Option<Key>, &'static str> {
        if route.origin.is_some_and(|d| self.host_for(d).is_none()) {
            return Err("The originating document was closed.");
        }
        if route.target.is_some_and(|h| self.host(h).is_none()) {
            return Err("The destination window was closed.");
        }
        Ok(route.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn doc(n: usize) -> Key {
        Key {
            window: n,
            identity: n as u64 + 100,
        }
    }
    #[test]
    fn open_target_is_captured_not_changed_by_later_activation() {
        let mut w = Workspace::default();
        let a = w.add_host(1);
        let b = w.add_host(2);
        assert!(w.add_doc(a, doc(10)));
        assert!(w.add_doc(b, doc(20)));
        w.activate(a);
        let route = w.route(OpenMode::Tab, None);
        w.activate(b);
        assert_eq!(w.resolve(route), Ok(Some(a)));
        assert_eq!(
            w.resolve(w.route(OpenMode::Tab, Some(doc(20)))),
            Ok(Some(b))
        );
        assert_eq!(
            w.resolve(w.route(OpenMode::Window, Some(doc(20)))),
            Ok(None)
        );
    }
    #[test]
    fn stale_host_and_origin_cannot_receive_a_pending_result() {
        let mut w = Workspace::default();
        let h = w.add_host(1);
        assert!(w.add_doc(h, doc(10)));
        let tab_route = w.route(OpenMode::Tab, None);
        let window_route = w.route(OpenMode::Window, Some(doc(10)));
        assert!(w.remove_doc(doc(10)));
        let reused = w.add_host(1);
        assert!(w.add_doc(reused, doc(11)));
        assert_ne!(h, reused);
        assert!(w.resolve(tab_route).is_err());
        assert!(w.resolve(window_route).is_err());
    }
    #[test]
    fn close_active_selects_right_then_left_and_inactive_close_keeps_selection() {
        let mut w = Workspace::default();
        let h = w.add_host(1);
        for n in [10, 11, 12] {
            assert!(w.add_doc(h, doc(n)));
        }
        assert!(w.select(h, doc(11)));
        assert!(w.remove_doc(doc(11)));
        assert_eq!(w.host(h).unwrap().active, Some(doc(12)));
        assert!(w.remove_doc(doc(10)));
        assert_eq!(w.host(h).unwrap().active, Some(doc(12)));
        assert!(w.remove_doc(doc(12)));
        assert!(w.host(h).is_none());
    }
    #[test]
    fn cycling_wraps_within_host_and_never_selects_another_hosts_document() {
        let mut w = Workspace::default();
        let h = w.add_host(1);
        let other = w.add_host(2);
        for n in [10, 11, 12] {
            assert!(w.add_doc(h, doc(n)));
        }
        assert!(w.add_doc(other, doc(20)));
        assert_eq!(w.cycle(h, false), Some(doc(10)));
        assert_eq!(w.cycle(h, true), Some(doc(12)));
        assert!(!w.select(h, doc(20)));
        assert_eq!(w.host(h).unwrap().active, Some(doc(12)));
    }
    #[test]
    fn document_limit_is_shared_across_hosts_and_duplicate_ids_are_rejected() {
        let mut w = Workspace::default();
        let a = w.add_host(1);
        let b = w.add_host(2);
        for n in 10..26 {
            assert!(w.add_doc(if n < 18 { a } else { b }, doc(n)));
        }
        assert!(!w.add_doc(a, doc(26)));
        assert!(w.remove_doc(doc(10)));
        assert!(!w.add_doc(a, doc(20)));
        assert!(w.add_doc(a, doc(26)));
        assert_eq!(w.host(a).unwrap().tabs.len(), 8);
        assert_eq!(w.host(b).unwrap().tabs.len(), 8);
    }
    #[test]
    fn closing_last_active_host_returns_to_most_recent_live_host() {
        let mut w = Workspace::default();
        let a = w.add_host(1);
        let b = w.add_host(2);
        let c = w.add_host(3);
        for (h, n) in [(a, 10), (b, 20), (c, 30)] {
            assert!(w.add_doc(h, doc(n)));
        }
        w.activate(b);
        w.activate(a);
        w.activate(c);
        w.remove_doc(doc(30));
        assert_eq!(w.resolve(w.route(OpenMode::Tab, None)), Ok(Some(a)));
        w.remove_doc(doc(10));
        assert_eq!(w.resolve(w.route(OpenMode::Tab, None)), Ok(Some(b)));
    }
    #[test]
    fn last_tab_left_neighbor_and_failed_host_creation_do_not_leave_a_destination() {
        let mut w = Workspace::default();
        let a = w.add_host(1);
        assert!(w.add_doc(a, doc(10)));
        assert!(w.add_doc(a, doc(11)));
        assert!(!w.remove_empty_host(a));
        w.remove_doc(doc(11));
        assert_eq!(w.host(a).unwrap().active, Some(doc(10)));
        let empty = w.add_host(2);
        let pending = w.route(OpenMode::Tab, None);
        assert!(w.remove_empty_host(empty));
        assert!(w.resolve(pending).is_err());
        assert_eq!(w.resolve(w.route(OpenMode::Tab, None)), Ok(Some(a)));
    }
    #[test]
    fn close_commit_requires_the_whole_original_set_and_revisions() {
        let expected = [(doc(10), 2), (doc(11), 4)];
        assert!(close_set_matches(&expected, &[(doc(11), 4), (doc(10), 2)]));
        assert!(!close_set_matches(&expected, &[(doc(10), 2)]));
        assert!(!close_set_matches(&expected, &[(doc(10), 2), (doc(11), 5)]));
        assert!(!close_set_matches(
            &expected,
            &[
                (doc(10), 2),
                (
                    Key {
                        window: 11,
                        identity: 999
                    },
                    4
                )
            ]
        ));
        assert!(!close_set_matches(
            &expected,
            &[(doc(10), 2), (doc(11), 4), (doc(12), 0)]
        ));
        assert!(!close_set_matches(&expected, &[(doc(10), 2), (doc(10), 2)]));
    }
}

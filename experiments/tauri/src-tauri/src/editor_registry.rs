use crate::protocol::CaptureDestination;
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct SessionRegistry<T> {
    values: HashMap<String, T>,
    limit: usize,
    next: u64,
}

impl<T> SessionRegistry<T> {
    pub fn new(limit: usize) -> Self {
        Self {
            values: HashMap::new(),
            limit,
            next: 0,
        }
    }

    pub fn insert(&mut self, value: T) -> Result<String, String> {
        if self.values.len() >= self.limit {
            return Err("Close an older capture before opening another image.".into());
        }
        self.next = self.next.wrapping_add(1).max(1);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let id = format!("{:032x}", stamp ^ self.next as u128);
        if self.values.insert(id.clone(), value).is_some() {
            return Err("Could not allocate a unique editor session.".into());
        }
        Ok(id)
    }

    pub fn get(&self, id: &str) -> Option<&T> {
        self.values.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut T> {
        self.values.get_mut(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<T> {
        self.values.remove(id)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn take_all(&mut self) -> Vec<T> {
        self.values.drain().map(|(_, value)| value).collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostAssignment {
    pub label: String,
    pub created: bool,
}

#[derive(Default)]
struct Host {
    sessions: Vec<String>,
    active: Option<String>,
}

#[derive(Default)]
pub struct EditorWorkspace {
    hosts: HashMap<String, Host>,
    activation: Vec<String>,
    next_host: u64,
}

impl EditorWorkspace {
    pub fn assign(
        &mut self,
        session: &str,
        destination: CaptureDestination,
    ) -> Result<HostAssignment, String> {
        if self
            .hosts
            .values()
            .any(|host| host.sessions.iter().any(|item| item == session))
        {
            return Err("The editor session is already assigned.".into());
        }
        let existing = matches!(destination, CaptureDestination::Tab)
            .then(|| self.activation.last().cloned())
            .flatten()
            .filter(|label| self.hosts.contains_key(label));
        let (label, created) = if let Some(label) = existing {
            (label, false)
        } else {
            (self.create_host(), true)
        };
        let host = self
            .hosts
            .get_mut(&label)
            .ok_or_else(|| "The editor window is unavailable.".to_string())?;
        host.sessions.push(session.to_string());
        host.active = Some(session.to_string());
        self.activate_host(&label);
        Ok(HostAssignment { label, created })
    }

    pub fn activate(&mut self, label: &str, session: &str) -> bool {
        let Some(host) = self.hosts.get_mut(label) else {
            return false;
        };
        if !host.sessions.iter().any(|item| item == session) {
            return false;
        }
        host.active = Some(session.to_string());
        self.activate_host(label);
        true
    }

    pub fn add_to_host(&mut self, label: &str, session: &str) -> bool {
        if self
            .hosts
            .values()
            .any(|host| host.sessions.iter().any(|item| item == session))
        {
            return false;
        }
        let Some(host) = self.hosts.get_mut(label) else {
            return false;
        };
        host.sessions.push(session.to_string());
        host.active = Some(session.to_string());
        self.activate_host(label);
        true
    }

    pub fn remove(&mut self, session: &str) -> Option<String> {
        let label = self.hosts.iter().find_map(|(label, host)| {
            host.sessions
                .iter()
                .any(|item| item == session)
                .then(|| label.clone())
        })?;
        let host = self.hosts.get_mut(&label)?;
        let index = host.sessions.iter().position(|item| item == session)?;
        host.sessions.remove(index);
        if host.active.as_deref() == Some(session) {
            host.active = host
                .sessions
                .get(index)
                .or_else(|| host.sessions.last())
                .cloned();
        }
        if host.sessions.is_empty() {
            self.hosts.remove(&label);
            self.activation.retain(|item| item != &label);
        }
        Some(label)
    }

    pub fn sessions(&self, label: &str) -> &[String] {
        self.hosts
            .get(label)
            .map(|host| host.sessions.as_slice())
            .unwrap_or_default()
    }

    pub fn hosts(&self) -> Vec<String> {
        self.hosts.keys().cloned().collect()
    }

    pub fn active(&self, label: &str) -> Option<&str> {
        self.hosts.get(label)?.active.as_deref()
    }

    pub fn contains_host(&self, label: &str) -> bool {
        self.hosts.contains_key(label)
    }

    pub fn host_for(&self, session: &str) -> Option<&str> {
        self.hosts.iter().find_map(|(label, host)| {
            host.sessions
                .iter()
                .any(|item| item == session)
                .then_some(label.as_str())
        })
    }

    pub fn create_host(&mut self) -> String {
        self.next_host = self.next_host.wrapping_add(1).max(1);
        let label = format!("editor-{}", self.next_host);
        self.hosts.insert(label.clone(), Host::default());
        self.activate_host(&label);
        label
    }

    pub fn remove_host(&mut self, label: &str) -> Vec<String> {
        self.activation.retain(|item| item != label);
        self.hosts
            .remove(label)
            .map(|host| host.sessions)
            .unwrap_or_default()
    }

    fn activate_host(&mut self, label: &str) {
        self.activation.retain(|item| item != label);
        self.activation.push(label.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_have_opaque_ids_and_never_cross_state() {
        let mut sessions = SessionRegistry::new(2);
        let first = sessions.insert(String::from("first")).unwrap();
        let second = sessions.insert(String::from("second")).unwrap();

        assert_ne!(first, second);
        assert_eq!(first.len(), 32);
        assert_eq!(sessions.get(&first).map(String::as_str), Some("first"));
        assert_eq!(sessions.get(&second).map(String::as_str), Some("second"));
        assert!(sessions.insert(String::from("third")).is_err());
        assert_eq!(sessions.remove(&first).as_deref(), Some("first"));
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn workspace_routes_tabs_to_the_latest_host_and_windows_to_new_hosts() {
        let mut workspace = EditorWorkspace::default();
        let first = workspace.assign("one", CaptureDestination::Tab).unwrap();
        assert!(first.created);
        assert_eq!(workspace.sessions(&first.label), ["one"]);

        let second = workspace.assign("two", CaptureDestination::Tab).unwrap();
        assert!(!second.created);
        assert_eq!(second.label, first.label);
        assert_eq!(workspace.sessions(&first.label), ["one", "two"]);
        assert_eq!(workspace.active(&first.label), Some("two"));

        let separate = workspace
            .assign("three", CaptureDestination::Window)
            .unwrap();
        assert!(separate.created);
        assert_ne!(separate.label, first.label);
        assert_eq!(workspace.sessions(&separate.label), ["three"]);

        assert!(workspace.activate(&first.label, "one"));
        let fourth = workspace.assign("four", CaptureDestination::Tab).unwrap();
        assert_eq!(fourth.label, first.label);
        assert_eq!(workspace.active(&first.label), Some("four"));
    }

    #[test]
    fn closing_sessions_selects_a_neighbor_and_removes_empty_hosts() {
        let mut workspace = EditorWorkspace::default();
        let host = workspace
            .assign("one", CaptureDestination::Tab)
            .unwrap()
            .label;
        workspace.assign("two", CaptureDestination::Tab).unwrap();
        workspace.assign("three", CaptureDestination::Tab).unwrap();
        assert!(workspace.activate(&host, "two"));

        assert_eq!(workspace.remove("two"), Some(host.clone()));
        assert_eq!(workspace.active(&host), Some("three"));
        assert_eq!(workspace.remove("one"), Some(host.clone()));
        assert_eq!(workspace.remove("three"), Some(host.clone()));
        assert!(!workspace.contains_host(&host));
    }

    #[test]
    fn closing_a_host_returns_all_sessions_and_registry_can_drain_on_exit() {
        let mut workspace = EditorWorkspace::default();
        let host = workspace
            .assign("one", CaptureDestination::Tab)
            .unwrap()
            .label;
        workspace.assign("two", CaptureDestination::Tab).unwrap();
        assert_eq!(workspace.remove_host(&host), ["one", "two"]);
        assert!(!workspace.contains_host(&host));

        let mut sessions = SessionRegistry::new(16);
        sessions.insert(1).unwrap();
        sessions.insert(2).unwrap();
        let mut values = sessions.take_all();
        values.sort_unstable();
        assert_eq!(values, [1, 2]);
        assert_eq!(sessions.len(), 0);
    }

    #[test]
    fn an_existing_editor_can_add_a_file_to_its_own_host() {
        let mut workspace = EditorWorkspace::default();
        let first = workspace.assign("one", CaptureDestination::Window).unwrap();
        let other = workspace
            .assign("other", CaptureDestination::Window)
            .unwrap();
        assert!(workspace.add_to_host(&first.label, "two"));
        assert_eq!(workspace.host_for("two"), Some(first.label.as_str()));
        assert_eq!(workspace.sessions(&first.label), ["one", "two"]);
        assert_eq!(workspace.active(&first.label), Some("two"));
        assert_eq!(workspace.sessions(&other.label), ["other"]);
    }

    #[test]
    fn a_blank_editor_host_receives_the_next_tab_capture() {
        let mut workspace = EditorWorkspace::default();
        let host = workspace.create_host();
        assert!(workspace.sessions(&host).is_empty());
        let capture = workspace.assign("one", CaptureDestination::Tab).unwrap();
        assert_eq!(capture.label, host);
        assert!(!capture.created);
    }
}

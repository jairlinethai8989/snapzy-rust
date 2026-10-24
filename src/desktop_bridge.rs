use serde::{Deserialize, Serialize};
use std::sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
};

pub const COPYDATA_ID: usize = 0x535A_5431;
pub const MAX_REQUEST_BYTES: usize = 2048;
pub const MAX_EVENT_BYTES: usize = 1024;
pub const READY: isize = 1;
pub const BUSY: isize = 2;
static ACTIVE: OnceLock<()> = OnceLock::new();
static OWNER: AtomicUsize = AtomicUsize::new(0);

pub fn set_owner(window: usize) {
    OWNER.store(window, Ordering::Release);
}

pub fn owner() -> usize {
    OWNER.load(Ordering::Acquire)
}

pub fn activate() {
    let _ = ACTIVE.set(());
}

pub fn is_active() -> bool {
    ACTIVE.get().is_some()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Area,
    Window,
    Desktop,
    Scrolling,
    VideoMonitor,
    VideoWindow,
    Open,
    Hotkeys,
    Status,
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureKind {
    Area,
    Window,
    Desktop,
    Scrolling,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureDestination {
    Tab,
    Window,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum EngineEvent {
    CaptureReady {
        version: u8,
        token: String,
        kind: CaptureKind,
        destination: CaptureDestination,
    },
}

impl EngineEvent {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_EVENT_BYTES {
            return Err("Invalid engine event size.".into());
        }
        let event: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let Self::CaptureReady { version, token, .. } = &event;
        if *version != 1
            || token.len() != 32
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("Invalid capture result event.".into());
        }
        Ok(event)
    }

    pub fn encode_line(&self) -> Result<Vec<u8>, String> {
        let mut bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        Self::decode(&bytes)?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub language: String,
    pub delay_seconds: u32,
    pub scroll_automatic: bool,
    pub open_in_tab: bool,
    #[serde(default = "silent_audio")]
    pub video_audio: String,
}

fn silent_audio() -> String {
    "none".into()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub action: Action,
    #[serde(default)]
    pub preferences: Option<Preferences>,
}

impl Request {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty() || bytes.len() > MAX_REQUEST_BYTES {
            return Err("Invalid desktop request size.".into());
        }
        let request: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if let Some(preferences) = &request.preferences
            && (!matches!(preferences.language.as_str(), "en" | "th")
                || ![0, 3, 5, 10].contains(&preferences.delay_seconds)
                || !matches!(
                    preferences.video_audio.as_str(),
                    "none" | "system" | "microphone" | "both"
                ))
        {
            return Err("Invalid desktop preferences.".into());
        }
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn accepts_only_known_commands_and_delay_presets() {
        for action in [
            "area",
            "window",
            "desktop",
            "scrolling",
            "video_monitor",
            "video_window",
            "open",
            "hotkeys",
            "status",
            "shutdown",
        ] {
            assert!(Request::decode(format!(r#"{{"action":"{action}"}}"#).as_bytes()).is_ok());
        }
        assert!(Request::decode(br#"{"action":"execute"}"#).is_err());
        assert!(Request::decode(br#"{"action":"area","preferences":{"language":"en","delay_seconds":600,"scroll_automatic":true,"open_in_tab":true}}"#).is_err());
        assert!(Request::decode(br#"{"action":"open","path":"C:/private"}"#).is_err());
    }

    #[test]
    fn rejects_oversized_invalid_and_unknown_language_requests() {
        assert!(Request::decode(&vec![b' '; MAX_REQUEST_BYTES + 1]).is_err());
        assert!(Request::decode(&[0xff]).is_err());
        assert!(Request::decode(br#"{"action":"area","preferences":{"language":"xx","delay_seconds":0,"scroll_automatic":true,"open_in_tab":true}}"#).is_err());
        let request = Request::decode(br#"{"action":"area","preferences":{"language":"th","delay_seconds":5,"scroll_automatic":false,"open_in_tab":false}}"#).unwrap();
        let preferences = request.preferences.unwrap();
        assert_eq!(preferences.language, "th");
        assert_eq!(preferences.delay_seconds, 5);
    }

    #[test]
    fn capture_events_accept_only_the_versioned_bounded_result_contract() {
        let event = EngineEvent::decode(
            format!(
                r#"{{"version":1,"event":"capture_ready","token":"{TOKEN}","kind":"scrolling","destination":"window"}}"#
            )
            .as_bytes(),
        )
        .unwrap();
        assert_eq!(
            event,
            EngineEvent::CaptureReady {
                version: 1,
                token: TOKEN.into(),
                kind: CaptureKind::Scrolling,
                destination: CaptureDestination::Window,
            }
        );
        let encoded = event.encode_line().unwrap();
        assert!(encoded.ends_with(b"\n"));
        assert_eq!(
            EngineEvent::decode(&encoded[..encoded.len() - 1]).unwrap(),
            event
        );
    }

    #[test]
    fn capture_events_reject_paths_unknown_values_and_oversized_messages() {
        for invalid in [
            format!(r#"{{"version":2,"event":"capture_ready","token":"{TOKEN}","kind":"area","destination":"tab"}}"#),
            r#"{"version":1,"event":"capture_ready","token":"../private","kind":"area","destination":"tab"}"#.into(),
            r#"{"version":1,"event":"capture_ready","token":"C:\\private","kind":"area","destination":"tab"}"#.into(),
            format!(r#"{{"version":1,"event":"capture_ready","token":"{TOKEN}","kind":"video","destination":"tab"}}"#),
            format!(r#"{{"version":1,"event":"capture_ready","token":"{TOKEN}","kind":"area","destination":"other"}}"#),
            format!(r#"{{"version":1,"event":"unknown","token":"{TOKEN}","kind":"area","destination":"tab"}}"#),
            format!(r#"{{"version":1,"event":"capture_ready","token":"{TOKEN}","kind":"area","destination":"tab","path":"C:/private"}}"#),
        ] {
            assert!(EngineEvent::decode(invalid.as_bytes()).is_err(), "accepted {invalid}");
        }
        assert!(EngineEvent::decode(&vec![b'x'; MAX_EVENT_BYTES + 1]).is_err());
    }
}

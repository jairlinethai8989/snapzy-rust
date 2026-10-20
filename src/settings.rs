use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf, sync::OnceLock};

static TEST_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

pub fn use_test_directory(path: PathBuf) -> Result<(), String> {
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    TEST_DIRECTORY
        .set(path)
        .map_err(|_| "Test directory was already set.".into())
}

pub fn is_test_run() -> bool {
    TEST_DIRECTORY.get().is_some()
}

pub fn instance_mutex_name(test_run: bool, process_id: u32) -> String {
    if test_run {
        format!("Local\\SnapZyRustPreview.Test.{process_id}")
    } else {
        "Local\\SnapZyRustPreview.Instance".into()
    }
}

pub const HOTKEY_MODIFIERS: [u32; 4] = [3, 6, 5, 7];
pub const HOTKEY_KEYS: [u32; 5] = [119, 120, 121, 122, 44];
pub const CAPTURE_DELAY_PRESETS: [u32; 4] = [0, 3, 5, 10];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyBinding {
    pub enabled: bool,
    pub modifiers: u32,
    pub key: u32,
}

impl HotkeyBinding {
    pub fn new(enabled: bool, modifiers: u32, key: u32) -> Self {
        Self {
            enabled,
            modifiers,
            key,
        }
    }

    pub fn is_valid(self) -> bool {
        HOTKEY_MODIFIERS.contains(&self.modifiers) && HOTKEY_KEYS.contains(&self.key)
    }
}

impl Default for HotkeyBinding {
    fn default() -> Self {
        Self::new(false, HOTKEY_MODIFIERS[0], HOTKEY_KEYS[0])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    Area,
    Window,
    Desktop,
    Scrolling,
    Video,
    Delay,
}

impl HotkeyAction {
    pub const ALL: [Self; 6] = [
        Self::Area,
        Self::Window,
        Self::Desktop,
        Self::Scrolling,
        Self::Video,
        Self::Delay,
    ];

    pub const fn label_key(self) -> &'static str {
        match self {
            Self::Area => "hotkey_area",
            Self::Window => "hotkey_window",
            Self::Desktop => "hotkey_desktop",
            Self::Scrolling => "hotkey_scroll",
            Self::Video => "hotkey_video",
            Self::Delay => "hotkey_delay",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub video_audio: crate::audio::Mode,
    pub open_mode: OpenMode,
    pub language: String,
    pub scroll_automatic: bool,
    pub hotkey_enabled: bool,
    pub hotkey_modifiers: u32,
    pub hotkey_key: u32,
    pub area_hotkey: HotkeyBinding,
    pub window_hotkey: HotkeyBinding,
    pub desktop_hotkey: HotkeyBinding,
    pub scrolling_hotkey: HotkeyBinding,
    pub video_hotkey: HotkeyBinding,
    pub delay_hotkey: HotkeyBinding,
    pub capture_delay_seconds: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            video_audio: crate::audio::Mode::None,
            open_mode: OpenMode::Tab,
            language: "en".into(),
            scroll_automatic: true,
            hotkey_enabled: false,
            hotkey_modifiers: 3,
            hotkey_key: 119,
            area_hotkey: HotkeyBinding::default(),
            window_hotkey: HotkeyBinding::default(),
            desktop_hotkey: HotkeyBinding::default(),
            scrolling_hotkey: HotkeyBinding::default(),
            video_hotkey: HotkeyBinding::default(),
            delay_hotkey: HotkeyBinding::default(),
            capture_delay_seconds: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenMode {
    #[default]
    #[serde(rename = "tab")]
    Tab,
    #[serde(rename = "window")]
    Window,
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.language.as_str(), "en" | "th")
            || !HOTKEY_MODIFIERS.contains(&self.hotkey_modifiers)
            || !HOTKEY_KEYS.contains(&self.hotkey_key)
            || !CAPTURE_DELAY_PRESETS.contains(&self.capture_delay_seconds)
            || !HotkeyAction::ALL
                .into_iter()
                .all(|action| self.binding(action).is_valid())
        {
            return Err("Invalid language or experimental hotkey settings.".into());
        }
        Ok(())
    }

    pub fn binding(&self, action: HotkeyAction) -> HotkeyBinding {
        match action {
            HotkeyAction::Area => self.area_hotkey,
            HotkeyAction::Window => self.window_hotkey,
            HotkeyAction::Desktop => self.desktop_hotkey,
            HotkeyAction::Scrolling => self.scrolling_hotkey,
            HotkeyAction::Video => self.video_hotkey,
            HotkeyAction::Delay => self.delay_hotkey,
        }
    }

    pub fn set_binding(&mut self, action: HotkeyAction, binding: HotkeyBinding) {
        match action {
            HotkeyAction::Area => self.area_hotkey = binding,
            HotkeyAction::Window => self.window_hotkey = binding,
            HotkeyAction::Desktop => self.desktop_hotkey = binding,
            HotkeyAction::Scrolling => self.scrolling_hotkey = binding,
            HotkeyAction::Video => self.video_hotkey = binding,
            HotkeyAction::Delay => self.delay_hotkey = binding,
        }
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
        let has_new_hotkeys = value.get("area_hotkey").is_some()
            || value.get("window_hotkey").is_some()
            || value.get("desktop_hotkey").is_some()
            || value.get("scrolling_hotkey").is_some()
            || value.get("video_hotkey").is_some()
            || value.get("delay_hotkey").is_some();
        let mut settings: Self = serde_json::from_value(value).map_err(|e| e.to_string())?;
        if !has_new_hotkeys {
            settings.area_hotkey = HotkeyBinding::new(
                settings.hotkey_enabled,
                settings.hotkey_modifiers,
                settings.hotkey_key,
            );
        }
        Ok(settings)
    }

    pub fn load() -> Self {
        let path = data_dir().join("settings.json");
        match fs::read_to_string(&path)
            .ok()
            .and_then(|json| Self::from_json(&json).ok())
        {
            Some(value) if value.validate().is_ok() => value,
            _ => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        self.validate()?;
        let directory = data_dir();
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let temp = directory.join("settings.json.tmp");
        let result = (|| {
            let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
            let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            fs::rename(&temp, directory.join("settings.json")).map_err(|e| e.to_string())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }
}

pub fn data_dir() -> PathBuf {
    if let Some(path) = TEST_DIRECTORY.get() {
        return path.clone();
    }
    PathBuf::from(
        std::env::var_os("LOCALAPPDATA").unwrap_or_else(|| std::env::temp_dir().into_os_string()),
    )
    .join("SnapZyRustPreview")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_mode_legacy_settings_are_silent_and_explicit_choices_round_trip() {
        let old: Settings = serde_json::from_str(r#"{"language":"th"}"#).unwrap();
        assert_eq!(serde_json::to_value(old).unwrap()["video_audio"], "none");
        for mode in ["none", "system", "microphone", "both"] {
            let value: Settings =
                serde_json::from_value(serde_json::json!({"video_audio":mode})).unwrap();
            assert_eq!(serde_json::to_value(value).unwrap()["video_audio"], mode);
        }
        assert!(
            serde_json::from_value::<Settings>(serde_json::json!({"video_audio":"unknown"}))
                .is_err()
        );
    }
    #[test]
    fn old_settings_open_new_documents_in_tabs_and_window_choice_roundtrips() {
        let value: Settings = serde_json::from_str(r#"{"language":"th"}"#).unwrap();
        let stored = serde_json::to_value(&value).unwrap();
        assert_eq!(stored["open_mode"], "tab");
        let window: Settings = serde_json::from_str(r#"{"open_mode":"window"}"#).unwrap();
        assert_eq!(serde_json::to_value(window).unwrap()["open_mode"], "window");
    }
    #[test]
    fn invalid_open_mode_cannot_be_loaded_as_an_accepted_preference() {
        let value = serde_json::from_str::<Settings>(r#"{"open_mode":"elsewhere"}"#);
        assert!(value.is_err() || value.unwrap().validate().is_err());
    }
    #[test]
    fn test_instances_are_isolated_without_changing_the_product_mutex() {
        assert_eq!(
            instance_mutex_name(false, 42),
            "Local\\SnapZyRustPreview.Instance"
        );
        assert_eq!(
            instance_mutex_name(true, 42),
            "Local\\SnapZyRustPreview.Test.42"
        );
    }

    #[test]
    fn old_settings_default_to_automatic_and_manual_roundtrips() {
        let mut value: Settings = serde_json::from_str(r#"{"language":"th"}"#).unwrap();
        assert!(value.scroll_automatic);
        value.scroll_automatic = false;
        let stored = serde_json::to_vec(&value).unwrap();
        let loaded: Settings = serde_json::from_slice(&stored).unwrap();
        assert!(!loaded.scroll_automatic);
        assert_eq!(loaded.language, "th");
    }
    #[test]
    fn defaults_do_not_claim_an_existing_apps_hotkey() {
        let value = Settings::default();
        assert_eq!(value.language, "en");
        assert!(!value.hotkey_enabled);
        assert!(value.validate().is_ok());
        assert!(
            Settings {
                language: "other".into(),
                ..value.clone()
            }
            .validate()
            .is_err()
        );
        assert!(
            Settings {
                hotkey_modifiers: 0,
                ..value.clone()
            }
            .validate()
            .is_err()
        );
        assert!(
            Settings {
                hotkey_key: 0,
                ..value
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn legacy_single_hotkey_migrates_to_area_binding() {
        let value = Settings::from_json(
            r#"{
            "hotkey_enabled": true,
            "hotkey_modifiers": 6,
            "hotkey_key": 121
        }"#,
        )
        .unwrap();
        assert_eq!(value.area_hotkey.enabled, true);
        assert_eq!(value.area_hotkey.modifiers, 6);
        assert_eq!(value.area_hotkey.key, 121);
        assert!(!value.desktop_hotkey.enabled);
    }

    #[test]
    fn hotkey_bindings_and_delay_roundtrip() {
        let mut value = Settings::default();
        value.desktop_hotkey.enabled = true;
        value.desktop_hotkey.modifiers = 5;
        value.desktop_hotkey.key = 44;
        value.capture_delay_seconds = 5;
        let loaded = Settings::from_json(&serde_json::to_string(&value).unwrap()).unwrap();
        assert_eq!(loaded.desktop_hotkey, value.desktop_hotkey);
        assert_eq!(loaded.capture_delay_seconds, 5);
        assert!(loaded.validate().is_ok());
    }

    #[test]
    fn delay_accepts_only_supported_capture_presets() {
        for seconds in [0, 3, 5, 10] {
            assert!(
                Settings {
                    capture_delay_seconds: seconds,
                    ..Settings::default()
                }
                .validate()
                .is_ok()
            );
        }
        assert!(
            Settings {
                capture_delay_seconds: 2,
                ..Settings::default()
            }
            .validate()
            .is_err()
        );
    }
}

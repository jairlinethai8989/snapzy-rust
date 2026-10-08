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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub hotkey_enabled: bool,
    pub hotkey_modifiers: u32,
    pub hotkey_key: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "en".into(),
            hotkey_enabled: false,
            hotkey_modifiers: 3,
            hotkey_key: 119,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.language.as_str(), "en" | "th")
            || ![3, 6, 5, 7].contains(&self.hotkey_modifiers)
            || ![119, 120, 121, 122, 44].contains(&self.hotkey_key)
        {
            return Err("Invalid language or experimental hotkey settings.".into());
        }
        Ok(())
    }

    pub fn load() -> Self {
        let path = data_dir().join("settings.json");
        match fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
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
}

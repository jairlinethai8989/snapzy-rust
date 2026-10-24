use crate::protocol::{CaptureDestination, CaptureKind, EngineEvent};
use std::{fs, path::Path};

const MAX_CAPTURE_BYTES: u64 = 256 * 1024 * 1024;
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

pub struct AdoptedCapture {
    pub bytes: Vec<u8>,
    pub kind: CaptureKind,
    pub destination: CaptureDestination,
}

pub fn adopt(directory: &Path, event: &EngineEvent) -> Result<AdoptedCapture, String> {
    let encoded = serde_json::to_vec(event).map_err(|e| e.to_string())?;
    let validated = EngineEvent::decode(&encoded)?;
    let EngineEvent::CaptureReady {
        token,
        kind,
        destination,
        ..
    } = validated;
    let path = directory.join(format!("{token}.png"));
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    if metadata.len() == 0 || metadata.len() > MAX_CAPTURE_BYTES {
        let _ = fs::remove_file(&path);
        return Err("The captured image has an invalid size.".into());
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string());
    let _ = fs::remove_file(&path);
    let bytes = bytes?;
    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err("The capture engine returned an invalid PNG image.".into());
    }
    Ok(AdoptedCapture {
        bytes,
        kind,
        destination,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    fn directory() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "snapzy-tauri-adopt-{}-{stamp}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn event(token: &str) -> EngineEvent {
        EngineEvent::CaptureReady {
            version: 1,
            token: token.into(),
            kind: CaptureKind::Window,
            destination: CaptureDestination::Tab,
        }
    }

    #[test]
    fn adopts_and_deletes_only_the_named_owned_png() {
        let directory = directory();
        let token = "0123456789abcdef0123456789abcdef";
        let expected = include_bytes!("../../../../tests/fixtures/ocr/english.png");
        fs::write(directory.join(format!("{token}.png")), expected).unwrap();
        fs::write(directory.join("keep.txt"), b"keep").unwrap();

        let adopted = adopt(&directory, &event(token)).unwrap();

        assert_eq!(adopted.bytes, expected);
        assert_eq!(adopted.kind, CaptureKind::Window);
        assert_eq!(adopted.destination, CaptureDestination::Tab);
        assert!(!directory.join(format!("{token}.png")).exists());
        assert!(directory.join("keep.txt").exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn missing_or_invalid_results_fail_and_invalid_owned_files_are_removed() {
        let directory = directory();
        let token = "fedcba9876543210fedcba9876543210";
        assert!(adopt(&directory, &event(token)).is_err());
        fs::write(directory.join(format!("{token}.png")), b"not png").unwrap();
        assert!(adopt(&directory, &event(token)).is_err());
        assert!(!directory.join(format!("{token}.png")).exists());
        fs::remove_dir_all(directory).unwrap();
    }
}

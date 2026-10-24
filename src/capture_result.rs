use crate::{
    desktop_bridge::{CaptureDestination, CaptureKind, EngineEvent},
    frame::Frame,
};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

static RESULT_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
static NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);

pub fn activate(directory: PathBuf) -> Result<(), String> {
    if !directory.is_absolute() {
        return Err("The Tauri capture result directory must be absolute.".into());
    }
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    RESULT_DIRECTORY
        .set(directory)
        .map_err(|_| "The Tauri capture result channel is already active.".to_string())
}

pub fn emit(
    frame: &Frame,
    kind: CaptureKind,
    destination: CaptureDestination,
) -> Result<(), String> {
    let directory = RESULT_DIRECTORY
        .get()
        .ok_or_else(|| "The Tauri capture result channel is unavailable.".to_string())?;
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    write_to(directory, frame, kind, destination, &mut output).map(|_| ())
}

fn write_to(
    directory: &Path,
    frame: &Frame,
    kind: CaptureKind,
    destination: CaptureDestination,
    output: &mut impl Write,
) -> Result<String, String> {
    fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos()
        ^ ((std::process::id() as u128) << 64)
        ^ NEXT_TOKEN.fetch_add(1, Ordering::Relaxed) as u128;
    let token = format!("{seed:032x}");
    let temporary = directory.join(format!(".{token}.tmp"));
    let completed = directory.join(format!("{token}.png"));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        frame.write_png(&mut file)?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        fs::rename(&temporary, &completed).map_err(|e| e.to_string())?;
        let event = EngineEvent::CaptureReady {
            version: 1,
            token: token.clone(),
            kind,
            destination,
        };
        output
            .write_all(&event.encode_line()?)
            .and_then(|()| output.flush())
            .map_err(|e| e.to_string())
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        let _ = fs::remove_file(&completed);
        return Err(error);
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temporary_directory(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("snapzy-{name}-{}-{stamp}", std::process::id()))
    }

    #[test]
    fn atomically_writes_png_and_emits_its_opaque_token() {
        let directory = temporary_directory("capture-transfer");
        let frame = Frame::new(3, 2, vec![127; 3 * 2 * 4]).unwrap();
        let mut output = Vec::new();
        let token = write_to(
            &directory,
            &frame,
            CaptureKind::Area,
            CaptureDestination::Tab,
            &mut output,
        )
        .unwrap();

        assert_eq!(token.len(), 32);
        assert_eq!(
            EngineEvent::decode(&output[..output.len() - 1]).unwrap(),
            EngineEvent::CaptureReady {
                version: 1,
                token: token.clone(),
                kind: CaptureKind::Area,
                destination: CaptureDestination::Tab,
            }
        );
        let entries = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(entries, [format!("{token}.png")]);
        let bytes = fs::read(directory.join(format!("{token}.png"))).unwrap();
        assert!(crate::image_io::decode(&bytes, crate::image_limits::IMAGE_LIMIT).is_ok());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn removes_the_owned_file_when_event_delivery_fails() {
        struct BrokenWriter;
        impl io::Write for BrokenWriter {
            fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("closed"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let directory = temporary_directory("capture-transfer-failure");
        let frame = Frame::new(2, 2, vec![255; 2 * 2 * 4]).unwrap();
        assert!(
            write_to(
                &directory,
                &frame,
                CaptureKind::Desktop,
                CaptureDestination::Window,
                &mut BrokenWriter,
            )
            .is_err()
        );
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
        fs::remove_dir_all(directory).unwrap();
    }
}

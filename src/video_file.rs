use serde::de::DeserializeOwned;
use std::{
    fs::{self, File, OpenOptions},
    io::BufRead,
    os::windows::{ffi::OsStrExt, io::AsRawHandle},
    path::{Path, PathBuf},
};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, GetDiskFreeSpaceExW, GetFileInformationByHandle,
    MOVEFILE_WRITE_THROUGH, MoveFileExW,
};

pub fn read_message<T: DeserializeOwned>(reader: &mut impl BufRead) -> Result<Option<T>, String> {
    let mut bytes = Vec::new();
    loop {
        let available = reader.fill_buf().map_err(|e| e.to_string())?;
        if available.is_empty() {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                Err("Truncated video message.".into())
            };
        }
        let end = available.iter().position(|b| *b == b'\n');
        let take = end.map_or(available.len(), |n| n + 1);
        if bytes.len() + take > crate::video::MESSAGE_LIMIT {
            return Err("Video message exceeds its size limit.".into());
        }
        bytes.extend_from_slice(&available[..take]);
        reader.consume(take);
        if end.is_some() {
            return serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| format!("Invalid video message: {e}"));
        }
    }
}
fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}
fn identity(file: &File) -> Result<(u32, u32, u32), String> {
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok((
        info.dwVolumeSerialNumber,
        info.nFileIndexHigh,
        info.nFileIndexLow,
    ))
}
pub struct Staging {
    pub path: PathBuf,
    output: PathBuf,
    identity: Option<(u32, u32, u32)>,
}
impl Staging {
    pub fn new(output: &Path, session: u64) -> Result<Self, String> {
        if session == 0
            || !output.is_absolute()
            || output.to_str().is_none_or(|s| s.contains('\0'))
            || !output
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("mp4"))
        {
            return Err("Invalid recording destination.".into());
        }
        if output.try_exists().map_err(|e| e.to_string())? {
            return Err("Choose a new filename; existing files are not replaced.".into());
        }
        let mut name = output
            .file_name()
            .ok_or("Invalid recording filename.")?
            .to_os_string();
        name.push(format!(".{}-{session}.partial.mp4", std::process::id()));
        let path = output.with_file_name(name);
        if path.try_exists().map_err(|e| e.to_string())? {
            return Err("A recording staging file already exists.".into());
        }
        Ok(Self {
            path,
            output: output.to_owned(),
            identity: None,
        })
    }
    fn adopt(&mut self) -> Result<(), String> {
        if self.identity.is_some() {
            return Err("Recording staging file was already adopted.".into());
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;
        self.identity = Some(identity(&file)?);
        Ok(())
    }
    pub fn publish(mut self) -> Result<PathBuf, String> {
        if self.identity.is_none() {
            self.adopt()?;
        }
        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;
        if Some(identity(&file)?) != self.identity
            || file.metadata().map_err(|e| e.to_string())?.len() == 0
        {
            return Err("Recording staging file is empty or was replaced.".into());
        }
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if unsafe {
            MoveFileExW(
                wide(&self.path).as_ptr(),
                wide(&self.output).as_ptr(),
                MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(format!(
                "Cannot publish recording; partial retained: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(self.output.clone())
    }
}
impl Drop for Staging {
    fn drop(&mut self) {
        if let Ok(file) = File::open(&self.path) {
            let empty = identity(&file).ok() == self.identity
                && file.metadata().is_ok_and(|m| m.len() == 0);
            drop(file);
            if empty {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}
pub fn free_space(path: &Path) -> Option<u64> {
    let mut available = 0;
    (unsafe {
        GetDiskFreeSpaceExW(
            wide(path).as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } != 0)
        .then_some(available)
}

pub fn export_completed(source: &Path, output: &Path, session: u64) -> Result<PathBuf, String> {
    let input = File::open(source).map_err(|e| e.to_string())?;
    if input.metadata().map_err(|e| e.to_string())?.len() == 0 {
        return Err("The completed recording is empty.".into());
    }
    if source == output {
        return Ok(source.to_owned());
    }
    copy_completed(input, Staging::new(output, session)?)
}

fn copy_completed(mut input: File, stage: Staging) -> Result<PathBuf, String> {
    let source_length = input.metadata().map_err(|e| e.to_string())?.len();
    if source_length == 0 {
        return Err("The completed recording is empty.".into());
    }
    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stage.path)
        .map_err(|e| e.to_string())?;
    let copied = std::io::copy(&mut input, &mut destination).map_err(|e| e.to_string())?;
    if copied != source_length
        || input.metadata().map_err(|e| e.to_string())?.len() != source_length
    {
        return Err("The completed recording changed while saving.".into());
    }
    destination.sync_all().map_err(|e| e.to_string())?;
    drop(destination);
    let published = stage.publish()?;
    drop(input);
    Ok(published)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::{Cursor, Write},
        sync::atomic::{AtomicU64, Ordering},
    };
    fn directory() -> PathBuf {
        static ID: AtomicU64 = AtomicU64::new(0);
        loop {
            let path = std::env::temp_dir().join(format!(
                "snapzy-video-files-{}-{}",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return path,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create video test directory: {error}"),
            }
        }
    }
    #[test]
    fn preview_export_preserves_clipboard_source_and_does_not_replace_user_files() {
        let d = directory();
        let source = d.join("คลิปต้นฉบับ.mp4");
        let output = d.join("คลิปบันทึก.mp4");
        let bytes = vec![0x79; 256 * 1024 + 3];
        fs::write(&source, &bytes).unwrap();
        let id = identity(&File::open(&source).unwrap()).unwrap();
        assert_eq!(export_completed(&source, &output, 91).unwrap(), output);
        assert_eq!(fs::read(&source).unwrap(), bytes);
        assert_eq!(identity(&File::open(&source).unwrap()).unwrap(), id);
        assert_eq!(fs::read(&output).unwrap(), bytes);
        fs::write(&output, b"user-clip").unwrap();
        assert!(export_completed(&source, &output, 92).is_err());
        assert_eq!(fs::read(&output).unwrap(), b"user-clip");
        assert!(export_completed(&source, &d.join("missing").join("clip.mp4"), 93).is_err());
        assert_eq!(fs::read(&source).unwrap(), bytes);
        assert_eq!(export_completed(&source, &source, 94).unwrap(), source);
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn completed_video_export_streams_bytes_to_unicode_destination_and_retains_source() {
        let d = directory();
        let source = d.join("pending.mp4");
        let output = d.join("คลิปทดสอบ.mp4");
        let bytes = vec![0x5a; 256 * 1024 + 3];
        fs::write(&source, &bytes).unwrap();
        let stage = Staging::new(&output, 1).unwrap();
        assert_eq!(
            copy_completed(File::open(&source).unwrap(), stage).unwrap(),
            output
        );
        assert_eq!(fs::read(&output).unwrap(), bytes);
        assert_eq!(fs::read(&source).unwrap(), bytes);
        assert_eq!(fs::read_dir(&d).unwrap().count(), 2);
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn same_volume_video_export_retains_clipboard_file_identity() {
        let d = directory();
        let source = d.join("pending.mp4");
        let output = d.join("saved.mp4");
        fs::write(&source, b"completed-clip").unwrap();
        let original_identity = identity(&File::open(&source).unwrap()).unwrap();
        export_completed(&source, &output, 5).unwrap();
        assert_eq!(
            identity(&File::open(&source).unwrap()).unwrap(),
            original_identity
        );
        assert_ne!(
            identity(&File::open(&output).unwrap()).unwrap(),
            original_identity
        );
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn completed_video_save_failure_keeps_clip_and_existing_destination_unchanged() {
        let d = directory();
        let source = d.join("pending.mp4");
        let output = d.join("existing.mp4");
        fs::write(&source, b"completed-clip").unwrap();
        fs::write(&output, b"existing-user-video").unwrap();
        assert!(export_completed(&source, &output, 2).is_err());
        assert_eq!(fs::read(&source).unwrap(), b"completed-clip");
        assert_eq!(fs::read(&output).unwrap(), b"existing-user-video");
        assert!(export_completed(&source, &d.join("missing").join("clip.mp4"), 3).is_err());
        assert_eq!(fs::read(&source).unwrap(), b"completed-clip");
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn completed_video_save_rejects_missing_or_empty_source_and_accepts_its_current_path() {
        let d = directory();
        let source = d.join("pending.mp4");
        let output = d.join("clip.mp4");
        assert!(export_completed(&source, &output, 4).is_err());
        fs::write(&source, []).unwrap();
        assert!(export_completed(&source, &output, 4).is_err());
        assert!(!output.exists());
        fs::write(&source, b"completed-clip").unwrap();
        assert_eq!(export_completed(&source, &source, 4).unwrap(), source);
        assert_eq!(fs::read(&source).unwrap(), b"completed-clip");
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn messages_are_structured_bounded_and_reject_truncation() {
        let mut reader = Cursor::new(b"{\"session\":3}\n{\"session\":4}\n");
        let a: serde_json::Value = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(a["session"], 3);
        let b: serde_json::Value = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(b["session"], 4);
        assert!(
            read_message::<serde_json::Value>(&mut reader)
                .unwrap()
                .is_none()
        );
        for bytes in [
            vec![b'x'; 8193],
            b"{\"session\":3}".to_vec(),
            b"broken\n".to_vec(),
        ] {
            assert!(read_message::<serde_json::Value>(&mut Cursor::new(bytes)).is_err());
        }
    }
    #[test]
    fn publication_adopts_a_completed_encoder_file_after_the_writer_releases_it() {
        let d = directory();
        let target = d.join("clip.mp4");
        let stage = Staging::new(&target, 1).unwrap();
        fs::write(&stage.path, b"encoded-test-data").unwrap();

        assert_eq!(stage.publish().unwrap(), target);
        assert_eq!(fs::read(target).unwrap(), b"encoded-test-data");
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn publication_never_replaces_an_existing_destination_even_after_staging() {
        let d = directory();
        let target = d.join("clip.mp4");
        let mut stage = Staging::new(&target, 1).unwrap();
        fs::write(&stage.path, b"encoded-test-data").unwrap();
        stage.adopt().unwrap();
        fs::write(&target, b"original-user-file").unwrap();
        assert!(stage.publish().is_err());
        assert_eq!(fs::read(&target).unwrap(), b"original-user-file");
        assert_eq!(fs::read_dir(&d).unwrap().count(), 2);
        assert!(Staging::new(&target, 2).is_err());
        fs::remove_dir_all(d).unwrap();
    }
    #[test]
    fn publication_moves_the_owned_nonempty_file_and_empty_cleanup_is_scoped() {
        let d = directory();
        let target = d.join("clip.mp4");
        let mut stage = Staging::new(&target, 1).unwrap();
        let stage_path = stage.path.clone();
        fs::write(&stage.path, b"encoded-test-data").unwrap();
        stage.adopt().unwrap();
        assert_eq!(stage.publish().unwrap(), target);
        assert!(!stage_path.exists());
        assert_eq!(fs::read(&target).unwrap(), b"encoded-test-data");
        let mut empty = Staging::new(&d.join("empty.mp4"), 2).unwrap();
        let empty_path = empty.path.clone();
        fs::File::create(&empty_path).unwrap();
        empty.adopt().unwrap();
        drop(empty);
        assert!(!empty_path.exists());
        let mut failed = Staging::new(&d.join("failed.mp4"), 3).unwrap();
        let partial = failed.path.clone();
        fs::File::create(&partial).unwrap();
        failed.adopt().unwrap();
        fs::File::options()
            .append(true)
            .open(&partial)
            .unwrap()
            .write_all(b"partial")
            .unwrap();
        drop(failed);
        assert_eq!(fs::read(partial).unwrap(), b"partial");
        assert!(free_space(&d).unwrap() > 0);
        fs::remove_dir_all(d).unwrap();
    }
}

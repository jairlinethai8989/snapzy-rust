use std::{
    fs::{self, OpenOptions},
    io::Write,
    sync::{Mutex, OnceLock},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

static LOG: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();

pub fn record(stage: &str, start: Instant) {
    let file = LOG.get_or_init(|| {
        let directory = crate::settings::data_dir().join("Logs");
        let _ = fs::create_dir_all(&directory);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Mutex::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(directory.join(format!("performance-{stamp}-{}.log", std::process::id())))
                .ok(),
        )
    });
    if let Ok(mut file) = file.lock() {
        if let Some(file) = file.as_mut() {
            let _ = writeln!(
                file,
                "pid={} {} {:.3}ms",
                std::process::id(),
                stage,
                start.elapsed().as_secs_f64() * 1000.0
            );
        }
    }
}

pub fn test_note(message: &str) {
    if !crate::settings::is_test_run() {
        return;
    }
    let path = crate::settings::data_dir()
        .join("Logs")
        .join("capture-diagnostics.log");
    if let Some(directory) = path.parent() {
        let _ = fs::create_dir_all(directory);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

use std::{
    ffi::c_void,
    os::windows::ffi::OsStrExt,
    path::Path,
    ptr,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};
use windows::{
    Win32::{
        Foundation::RECT,
        Media::MediaFoundation::*,
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
            CoUninitialize,
        },
    },
    core::{BSTR, HRESULT, Interface, implement},
};
use windows_sys::Win32::{
    Foundation::FreeLibrary,
    System::LibraryLoader::{GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW},
};

type Startup = unsafe extern "system" fn(u32, u32) -> HRESULT;
type Shutdown = unsafe extern "system" fn() -> HRESULT;
type Attributes = unsafe extern "system" fn(*mut *mut c_void, u32) -> HRESULT;

struct Runtime {
    library: usize,
    shutdown: Shutdown,
    initialized: bool,
}
impl Runtime {
    fn load() -> Result<(Self, IMFAttributes), String> {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let initialized = hr.is_ok();
            if !initialized && hr.0 != 0x80010106u32 as i32 {
                return Err(hr.to_string());
            }
            let library = LoadLibraryExW(
                crate::ui::wide("mfplat.dll").as_ptr(),
                ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            );
            if library.is_null() {
                if initialized {
                    CoUninitialize();
                }
                return Err("Windows media components are unavailable.".into());
            }
            let start = GetProcAddress(library, c"MFStartup".as_ptr().cast());
            let stop = GetProcAddress(library, c"MFShutdown".as_ptr().cast());
            let attributes = GetProcAddress(library, c"MFCreateAttributes".as_ptr().cast());
            let (Some(start), Some(stop), Some(attributes)) = (start, stop, attributes) else {
                FreeLibrary(library);
                if initialized {
                    CoUninitialize();
                }
                return Err("Windows media functions are unavailable.".into());
            };
            let start: Startup = std::mem::transmute(start);
            if let Err(e) = start(MF_VERSION, MFSTARTUP_FULL).ok() {
                FreeLibrary(library);
                if initialized {
                    CoUninitialize();
                }
                return Err(e.to_string());
            }
            let runtime = Self {
                library: library as usize,
                shutdown: std::mem::transmute(stop),
                initialized,
            };
            let attributes: Attributes = std::mem::transmute(attributes);
            let mut raw = ptr::null_mut();
            attributes(&mut raw, 3).ok().map_err(|e| e.to_string())?;
            if raw.is_null() {
                return Err("Windows returned empty playback attributes.".into());
            }
            Ok((runtime, IMFAttributes::from_raw(raw)))
        }
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe {
            let _ = (self.shutdown)();
            FreeLibrary(self.library as _);
            if self.initialized {
                CoUninitialize();
            }
        }
    }
}

#[implement(IMFMediaEngineNotify)]
struct Notify {
    error: Arc<AtomicU32>,
}
impl IMFMediaEngineNotify_Impl for Notify_Impl {
    fn EventNotify(&self, event: u32, param1: usize, param2: u32) -> windows::core::Result<()> {
        // No UI/COM reentry from decoder callbacks; the preview polls bounded state.
        if event == MF_MEDIA_ENGINE_EVENT_ERROR.0 as u32 {
            self.error.store(
                if param2 != 0 { param2 } else { param1 as u32 }.max(1),
                Ordering::Release,
            );
        }
        Ok(())
    }
}

pub struct Player {
    engine: Option<IMFMediaEngine>,
    error: Arc<AtomicU32>,
    _runtime: Runtime,
}
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub ready: bool,
    pub paused: bool,
    pub ended: bool,
    pub audio: bool,
    pub current: f64,
    pub duration: f64,
    pub muted: bool,
    pub volume: f64,
}
impl Player {
    pub fn open(window: usize, path: &Path) -> Result<Self, String> {
        if window == 0
            || !path.is_absolute()
            || !path.metadata().is_ok_and(|m| m.is_file() && m.len() > 0)
        {
            return Err("The recorded clip is missing or empty.".into());
        }
        let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        if wide.contains(&0) {
            return Err("Invalid clip path.".into());
        }
        let (runtime, attributes) = Runtime::load()?;
        let error = Arc::new(AtomicU32::new(0));
        let notify: IMFMediaEngineNotify = Notify {
            error: error.clone(),
        }
        .into();
        let native = (|| -> windows::core::Result<IMFMediaEngine> {
            unsafe {
                attributes.SetUnknown(&MF_MEDIA_ENGINE_CALLBACK, &notify)?;
                attributes.SetUINT64(&MF_MEDIA_ENGINE_PLAYBACK_HWND, window as u64)?;
                let factory: IMFMediaEngineClassFactory =
                    CoCreateInstance(&CLSID_MFMediaEngineClassFactory, None, CLSCTX_INPROC_SERVER)?;
                factory.CreateInstance(0, &attributes)
            }
        })()
        .map_err(|e| format!("Cannot create native video preview: {e}"))?;
        let player = Self {
            engine: Some(native),
            error,
            _runtime: runtime,
        };
        unsafe {
            let engine = player.engine();
            engine.SetAutoPlay(false).map_err(|e| e.to_string())?;
            engine
                .SetPreload(MF_MEDIA_ENGINE_PRELOAD_AUTOMATIC)
                .map_err(|e| e.to_string())?;
            engine
                .SetSource(&BSTR::from_wide(&wide))
                .map_err(|e| format!("Cannot load video preview: {e}"))?;
        }
        Ok(player)
    }
    fn engine(&self) -> &IMFMediaEngine {
        self.engine.as_ref().unwrap()
    }
    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let error = self.error.load(Ordering::Acquire);
        if error != 0 {
            return Err(format!("Video playback failed (0x{error:08X})."));
        }
        unsafe {
            let e = self.engine();
            Ok(Snapshot {
                ready: e.GetReadyState() >= 2,
                paused: e.IsPaused().as_bool(),
                ended: e.IsEnded().as_bool(),
                audio: e.HasAudio().as_bool(),
                current: e.GetCurrentTime(),
                duration: e.GetDuration(),
                muted: e.GetMuted().as_bool(),
                volume: e.GetVolume(),
            })
        }
    }
    pub fn toggle(&self) -> Result<(), String> {
        let s = self.snapshot()?;
        unsafe {
            if s.ended {
                self.seek(0.)?;
            }
            if s.paused || s.ended {
                self.engine().Play()
            } else {
                self.engine().Pause()
            }
        }
        .map_err(|e| e.to_string())
    }
    pub fn pause(&self) -> Result<(), String> {
        unsafe { self.engine().Pause() }.map_err(|e| e.to_string())
    }
    pub fn seek(&self, seconds: f64) -> Result<(), String> {
        let s = self.snapshot()?;
        if !seconds.is_finite() || !s.duration.is_finite() || s.duration <= 0. {
            return Err("Video is not ready to seek.".into());
        }
        unsafe { self.engine().SetCurrentTime(seconds.clamp(0., s.duration)) }
            .map_err(|e| e.to_string())
    }
    pub fn volume(&self, value: f64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("Invalid volume.".into());
        }
        unsafe { self.engine().SetVolume(value.clamp(0., 1.)) }.map_err(|e| e.to_string())
    }
    pub fn mute(&self, muted: bool) -> Result<(), String> {
        unsafe { self.engine().SetMuted(muted) }.map_err(|e| e.to_string())
    }
    pub fn resize(&self, width: i32, height: i32) -> Result<(), String> {
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        unsafe {
            let (mut w, mut h) = (0, 0);
            self.engine()
                .GetVideoAspectRatio(Some(&mut w), Some(&mut h))
                .map_err(|e| e.to_string())?;
            if w == 0 || h == 0 {
                return Ok(());
            }
            let scale = (width as f64 / w as f64).min(height as f64 / h as f64);
            let (w, h) = ((w as f64 * scale) as i32, (h as f64 * scale) as i32);
            let rect = RECT {
                left: (width - w) / 2,
                top: (height - h) / 2,
                right: (width + w) / 2,
                bottom: (height + h) / 2,
            };
            let ex: IMFMediaEngineEx = self.engine().cast().map_err(|e| e.to_string())?;
            ex.UpdateVideoStream(None, Some(&rect), None)
                .map_err(|e| e.to_string())
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        if let Some(engine) = self.engine.take() {
            unsafe {
                let _ = engine.Shutdown();
            }
            drop(engine);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{frame::Frame, video_media::Encoder};
    use std::{
        ptr,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    fn wait(player: &Player, condition: impl Fn(Snapshot) -> bool) -> Snapshot {
        let start = Instant::now();
        loop {
            unsafe {
                let mut msg = std::mem::zeroed();
                while PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            let state = player.snapshot().unwrap();
            if condition(state) {
                return state;
            }
            assert!(
                start.elapsed() < Duration::from_secs(15),
                "playback timeout: {state:?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    #[test]
    fn native_preview_loads_paused_seeks_replays_and_releases_real_mp4() {
        let dir = std::env::temp_dir().join(format!("snapzy-playback-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for audio in [false, true] {
            let path = dir.join(format!("ไทย-{audio}.mp4"));
            let mut encoder = Encoder::new(&path, 64, 48, false, audio).unwrap();
            for i in 0..60 {
                let frame =
                    Frame::new(64, 48, vec![if i < 30 { 70 } else { 190 }; 64 * 48 * 4]).unwrap();
                encoder.write(&frame, i * 333_333, 333_333).unwrap();
                if audio {
                    encoder
                        .write_audio(i as u64 * 1600, &vec![0; 3200])
                        .unwrap();
                }
            }
            encoder.finish().unwrap();
            let window = unsafe {
                CreateWindowExW(
                    0,
                    crate::ui::wide("STATIC").as_ptr(),
                    ptr::null(),
                    WS_POPUP,
                    -10000,
                    -10000,
                    320,
                    240,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null(),
                )
            };
            assert!(!window.is_null());
            let player = Player::open(window as usize, &path).unwrap();
            let state = wait(&player, |s| s.ready);
            assert!(state.paused && state.current < 0.1);
            assert_eq!(state.audio, audio);
            assert!((1.9..=2.2).contains(&state.duration), "{state:?}");
            player.mute(true).unwrap();
            player.volume(0.25).unwrap();
            assert!(player.snapshot().unwrap().muted);
            assert_eq!(player.snapshot().unwrap().volume, 0.25);
            player.resize(300, 180).unwrap();
            player.toggle().unwrap();
            wait(&player, |s| !s.paused && s.current > 0.15);
            player.pause().unwrap();
            assert!(player.snapshot().unwrap().paused);
            player.seek(1.2).unwrap();
            wait(&player, |s| (1.1..1.4).contains(&s.current));
            player.toggle().unwrap();
            wait(&player, |s| s.ended);
            player.toggle().unwrap();
            wait(&player, |s| !s.ended && s.current < 0.8 && !s.paused);
            player.pause().unwrap();
            drop(player);
            unsafe {
                DestroyWindow(window);
            }
            std::fs::rename(&path, path.with_extension("released.mp4")).unwrap();
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}

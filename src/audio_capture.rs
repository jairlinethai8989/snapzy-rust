use crate::audio::{self, Kind, Mixer, Mode};
use windows::Win32::{
    Media::Audio::{
        AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR, AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_LOOPBACK,
        AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, IAudioCaptureClient, IAudioClient,
        IMMDeviceEnumerator, MMDeviceEnumerator, WAVE_FORMAT_PCM, WAVEFORMATEX, eCapture,
        eMultimedia, eRender,
    },
    System::{
        Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize},
        Performance::{QueryPerformanceCounter, QueryPerformanceFrequency},
    },
};

struct Com;
impl Drop for Com {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

struct Endpoint {
    kind: Kind,
    capture: IAudioCaptureClient,
    client: IAudioClient,
    started: bool,
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        if self.started {
            let _ = unsafe { self.client.Stop() };
        }
    }
}

pub struct Capture {
    endpoints: Vec<Endpoint>,
    epoch: Option<u64>,
    _com: Option<Com>,
}

impl Capture {
    pub fn new(mode: Mode) -> Result<Self, String> {
        if !mode.enabled() {
            return Ok(Self {
                endpoints: Vec::new(),
                epoch: None,
                _com: None,
            });
        }
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .map_err(|e| format!("Audio capture: cannot initialize the audio thread: {e}"))?;
        let com = Com;
        let devices: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
                .map_err(|e| format!("Audio capture: cannot discover audio devices: {e}"))?;
        let mut capture = Self {
            endpoints: Vec::new(),
            epoch: None,
            _com: Some(com),
        };
        for kind in [Kind::System, Kind::Microphone] {
            if mode.includes(kind) {
                capture.endpoints.push(Endpoint::new(&devices, kind)?);
            }
        }
        Ok(capture)
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.endpoints.is_empty() {
            return Ok(());
        }
        self.epoch = Some(qpc_time()?);
        for endpoint in &mut self.endpoints {
            unsafe { endpoint.client.Start() }
                .map_err(|e| failure(endpoint.kind, &e.to_string()))?;
            endpoint.started = true;
        }
        Ok(())
    }

    pub fn poll(&mut self, mixer: &mut Mixer) -> Result<(), String> {
        if self.endpoints.is_empty() {
            return Ok(());
        }
        let epoch = self.epoch.ok_or("Audio capture has not started.")?;
        for endpoint in &self.endpoints {
            endpoint.poll(epoch, mixer)?;
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        for endpoint in &mut self.endpoints {
            if endpoint.started {
                let _ = unsafe { endpoint.client.Stop() };
                endpoint.started = false;
            }
        }
    }
}

impl Endpoint {
    fn new(devices: &IMMDeviceEnumerator, kind: Kind) -> Result<Self, String> {
        let prepare = || -> windows::core::Result<Self> {
            unsafe {
                let device = devices.GetDefaultAudioEndpoint(
                    if kind == Kind::System {
                        eRender
                    } else {
                        eCapture
                    },
                    eMultimedia,
                )?;
                let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
                let format = WAVEFORMATEX {
                    wFormatTag: WAVE_FORMAT_PCM as u16,
                    nChannels: audio::CHANNELS as u16,
                    nSamplesPerSec: audio::RATE,
                    nAvgBytesPerSec: audio::RATE * 4,
                    nBlockAlign: 4,
                    wBitsPerSample: 16,
                    cbSize: 0,
                };
                let flags = AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                    | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY
                    | if kind == Kind::System {
                        AUDCLNT_STREAMFLAGS_LOOPBACK
                    } else {
                        0
                    };
                client.Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 2_000_000, 0, &format, None)?;
                if client.GetBufferSize()? as usize > audio::CAPACITY_FRAMES {
                    return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                        0x80070057u32 as i32,
                    )));
                }
                let capture = client.GetService::<IAudioCaptureClient>()?;
                Ok(Self {
                    kind,
                    client,
                    capture,
                    started: false,
                })
            }
        };
        prepare().map_err(|e| failure(kind, &e.to_string()))
    }

    fn poll(&self, epoch: u64, mixer: &mut Mixer) -> Result<(), String> {
        for _ in 0..32 {
            let available = unsafe { self.capture.GetNextPacketSize() }
                .map_err(|e| failure(self.kind, &e.to_string()))?;
            if available == 0 {
                break;
            }
            let mut data = std::ptr::null_mut();
            let mut frames = 0;
            let mut flags = 0;
            let mut position = 0;
            unsafe {
                self.capture.GetBuffer(
                    &mut data,
                    &mut frames,
                    &mut flags,
                    None,
                    Some(&mut position),
                )
            }
            .map_err(|e| failure(self.kind, &e.to_string()))?;
            if frames == 0 {
                break;
            }
            let result = (|| {
                if frames as usize > audio::CAPACITY_FRAMES {
                    return Err("The audio packet exceeds its memory limit.".into());
                }
                let count = frames as usize * audio::CHANNELS;
                let samples = if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
                    vec![0; count]
                } else {
                    if data.is_null() {
                        return Err("Windows returned an empty audio packet.".into());
                    }
                    unsafe { std::slice::from_raw_parts(data, count * 2) }
                        .chunks_exact(2)
                        .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
                        .collect()
                };
                let start =
                    if position == 0 || flags & AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR.0 as u32 != 0 {
                        audio::packet_frame(qpc_time()?, epoch)?.saturating_sub(frames as i64)
                    } else {
                        audio::packet_frame(position, epoch)?
                    };
                mixer.push(self.kind, start, &samples)
            })();
            let release = unsafe { self.capture.ReleaseBuffer(frames) }
                .map_err(|e| failure(self.kind, &e.to_string()));
            result.map_err(|e: String| failure(self.kind, &e))?;
            release?;
        }
        Ok(())
    }
}

fn failure(kind: Kind, detail: &str) -> String {
    format!(
        "Audio capture: {}: {detail}. Check the default audio device and microphone access in Windows settings.",
        if kind == Kind::System {
            "system audio"
        } else {
            "microphone"
        }
    )
}

fn qpc_time() -> Result<u64, String> {
    let mut ticks = 0;
    let mut frequency = 0;
    unsafe {
        QueryPerformanceCounter(&mut ticks).and_then(|_| QueryPerformanceFrequency(&mut frequency))
    }
    .map_err(|e| format!("Audio capture clock is unavailable: {e}"))?;
    if ticks < 0 || frequency <= 0 {
        return Err("Audio capture clock is invalid.".into());
    }
    u64::try_from(ticks as u128 * 10_000_000 / frequency as u128)
        .map_err(|_| "Audio capture clock overflowed.".into())
}

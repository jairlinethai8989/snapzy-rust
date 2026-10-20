use crate::{frame::Frame, video};
use std::{
    ffi::c_void,
    path::{Path, PathBuf},
    ptr,
};
#[cfg(test)]
use windows::Win32::Media::MediaFoundation::IMF2DBuffer;
use windows::{
    Win32::{
        Media::MediaFoundation::{
            IMFAttributes, IMFByteStream, IMFMediaBuffer, IMFMediaSink, IMFMediaType, IMFSample,
            IMFSinkWriter, MF_ACCESSMODE_WRITE, MF_FILE_ACCESSMODE, MF_FILE_FLAGS,
            MF_FILE_OPENMODE, MF_FILEFLAGS_NONE, MF_MT_AVG_BITRATE, MF_MT_DEFAULT_STRIDE,
            MF_MT_FIXED_SIZE_SAMPLES, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE, MF_MT_INTERLACE_MODE,
            MF_MT_MAJOR_TYPE, MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE, MF_OPENMODE_FAIL_IF_EXIST,
            MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, MF_VERSION, MFMediaType_Video, MFSTARTUP_FULL,
            MFVideoFormat_H264, MFVideoFormat_RGB32, MFVideoInterlace_Progressive,
        },
        System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize},
    },
    core::{HRESULT, Interface, PCWSTR},
};
use windows_sys::Win32::{
    Foundation::FreeLibrary,
    System::LibraryLoader::{GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW},
};

type Startup = unsafe extern "system" fn(u32, u32) -> HRESULT;
type Shutdown = unsafe extern "system" fn() -> HRESULT;
type CreateObject = unsafe extern "system" fn(*mut *mut c_void) -> HRESULT;
type CreateAttributes = unsafe extern "system" fn(*mut *mut c_void, u32) -> HRESULT;
type CreateFile = unsafe extern "system" fn(
    MF_FILE_ACCESSMODE,
    MF_FILE_OPENMODE,
    MF_FILE_FLAGS,
    PCWSTR,
    *mut *mut c_void,
) -> HRESULT;
type CreateFmp4 =
    unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT;
type CreateWriter =
    unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT;
type CreateBuffer = unsafe extern "system" fn(u32, *mut *mut c_void) -> HRESULT;
#[cfg(test)]
type CreateReader = unsafe extern "system" fn(PCWSTR, *mut c_void, *mut *mut c_void) -> HRESULT;

struct ComGuard(bool);

impl ComGuard {
    fn mta() -> Result<Self, String> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .map_err(|error| format!("Cannot initialize the video thread: {error}"))?;
        Ok(Self(true))
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}

struct Libraries {
    mfplat: isize,
    mf: isize,
    readwrite: isize,
}

impl Drop for Libraries {
    fn drop(&mut self) {
        unsafe {
            if self.readwrite != 0 {
                FreeLibrary(self.readwrite as _);
            }
            if self.mf != 0 {
                FreeLibrary(self.mf as _);
            }
            if self.mfplat != 0 {
                FreeLibrary(self.mfplat as _);
            }
        }
    }
}

struct Api {
    _com: ComGuard,
    libraries: Libraries,
    shutdown: Shutdown,
    create_media_type: CreateObject,
    create_attributes: CreateAttributes,
    create_file: CreateFile,
    create_fmp4: CreateFmp4,
    create_writer: CreateWriter,
    create_buffer: CreateBuffer,
    create_sample: CreateObject,
    #[cfg(test)]
    create_reader: CreateReader,
    started: bool,
}

impl Api {
    fn load() -> Result<Self, String> {
        let com = ComGuard::mta()?;
        let mfplat = load("mfplat.dll")?;
        let mf = match load("mf.dll") {
            Ok(value) => value,
            Err(error) => {
                unsafe { FreeLibrary(mfplat as _) };
                return Err(error);
            }
        };
        let readwrite = match load("mfreadwrite.dll") {
            Ok(value) => value,
            Err(error) => {
                unsafe {
                    FreeLibrary(mf as _);
                    FreeLibrary(mfplat as _);
                }
                return Err(error);
            }
        };
        let libraries = Libraries {
            mfplat,
            mf,
            readwrite,
        };
        let startup: Startup = symbol(mfplat, "MFStartup")?;
        let shutdown = symbol(mfplat, "MFShutdown")?;
        let create_media_type = symbol(mfplat, "MFCreateMediaType")?;
        let create_attributes = symbol(mfplat, "MFCreateAttributes")?;
        let create_file = symbol(mfplat, "MFCreateFile")?;
        let create_buffer = symbol(mfplat, "MFCreateMemoryBuffer")?;
        let create_sample = symbol(mfplat, "MFCreateSample")?;
        let create_fmp4 = symbol(mf, "MFCreateFMPEG4MediaSink")?;
        let create_writer = symbol(readwrite, "MFCreateSinkWriterFromMediaSink")?;
        #[cfg(test)]
        let create_reader = symbol(readwrite, "MFCreateSourceReaderFromURL")?;
        hresult(
            unsafe { startup(MF_VERSION, MFSTARTUP_FULL) },
            "Cannot start Windows video services",
        )?;
        Ok(Self {
            _com: com,
            libraries,
            shutdown,
            create_media_type,
            create_attributes,
            create_file,
            create_fmp4,
            create_writer,
            create_buffer,
            create_sample,
            #[cfg(test)]
            create_reader,
            started: true,
        })
    }

    unsafe fn object<T: Interface>(
        &self,
        function: CreateObject,
        label: &str,
    ) -> Result<T, String> {
        let mut raw = ptr::null_mut();
        hresult(unsafe { function(&mut raw) }, label)?;
        if raw.is_null() {
            return Err(format!("{label}: Windows returned an empty object."));
        }
        Ok(unsafe { T::from_raw(raw) })
    }

    unsafe fn media_type(&self) -> Result<IMFMediaType, String> {
        unsafe { self.object(self.create_media_type, "Cannot create a video format") }
    }

    unsafe fn attributes(&self, count: u32) -> Result<IMFAttributes, String> {
        let mut raw = ptr::null_mut();
        hresult(
            unsafe { (self.create_attributes)(&mut raw, count) },
            "Cannot create video settings",
        )?;
        Ok(unsafe { IMFAttributes::from_raw(raw) })
    }
}

impl Drop for Api {
    fn drop(&mut self) {
        if self.started {
            let _ = unsafe { (self.shutdown)() };
            self.started = false;
        }
        let _ = &self.libraries;
    }
}

fn load(name: &str) -> Result<isize, String> {
    let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
    let handle =
        unsafe { LoadLibraryExW(wide.as_ptr(), ptr::null_mut(), LOAD_LIBRARY_SEARCH_SYSTEM32) };
    if handle.is_null() {
        Err(format!("Windows video component {name} is unavailable."))
    } else {
        Ok(handle as isize)
    }
}

fn symbol<T: Copy>(library: isize, name: &str) -> Result<T, String> {
    let c_name = std::ffi::CString::new(name).map_err(|error| error.to_string())?;
    let address = unsafe { GetProcAddress(library as _, c_name.as_ptr() as _) }
        .ok_or_else(|| format!("Windows video component is missing {name}."))?;
    debug_assert_eq!(std::mem::size_of_val(&address), std::mem::size_of::<T>());
    Ok(unsafe { std::mem::transmute_copy(&address) })
}

fn hresult(result: HRESULT, label: &str) -> Result<(), String> {
    result.ok().map_err(|error| format!("{label}: {error}"))
}

fn wide(path: &Path) -> Result<Vec<u16>, String> {
    use std::os::windows::ffi::OsStrExt;
    if path.as_os_str().encode_wide().any(|unit| unit == 0) {
        return Err("The video path contains an invalid character.".into());
    }
    Ok(path.as_os_str().encode_wide().chain(Some(0)).collect())
}

unsafe fn set_guid(
    a: &IMFAttributes,
    key: &windows::core::GUID,
    value: &windows::core::GUID,
) -> Result<(), String> {
    unsafe { a.SetGUID(key, value) }.map_err(|error| error.to_string())
}
unsafe fn set_u32(a: &IMFAttributes, key: &windows::core::GUID, value: u32) -> Result<(), String> {
    unsafe { a.SetUINT32(key, value) }.map_err(|error| error.to_string())
}
unsafe fn set_u64(a: &IMFAttributes, key: &windows::core::GUID, value: u64) -> Result<(), String> {
    unsafe { a.SetUINT64(key, value) }.map_err(|error| error.to_string())
}

pub struct Encoder {
    api: Api,
    writer: Option<IMFSinkWriter>,
    sink: Option<IMFMediaSink>,
    stream: Option<IMFByteStream>,
    path: PathBuf,
    source_width: u32,
    source_height: u32,
    width: u32,
    bytes: u32,
    last_end: i64,
    audio_stream: Option<u32>,
    audio_end: i64,
    finished: bool,
}

impl Encoder {
    pub fn new(
        path: &Path,
        width: u32,
        height: u32,
        hardware: bool,
        audio: bool,
    ) -> Result<Self, String> {
        let (padded_width, padded_height) = video::dimensions(width, height)?;
        let padded_bytes = padded_width
            .checked_mul(padded_height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or("Video frame size overflowed.")?;
        let api = Api::load()?;
        let output = wide(path)?;
        unsafe {
            let output_type = api.media_type()?;
            set_guid(&output_type, &MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            set_guid(&output_type, &MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
            set_u32(
                &output_type,
                &MF_MT_AVG_BITRATE,
                video::bitrate(padded_width, padded_height)?,
            )?;
            set_u64(
                &output_type,
                &MF_MT_FRAME_SIZE,
                ratio(padded_width, padded_height),
            )?;
            set_u64(&output_type, &MF_MT_FRAME_RATE, ratio(30, 1))?;
            set_u64(&output_type, &MF_MT_PIXEL_ASPECT_RATIO, ratio(1, 1))?;
            set_u32(
                &output_type,
                &MF_MT_INTERLACE_MODE,
                MFVideoInterlace_Progressive.0 as u32,
            )?;

            let input_type = api.media_type()?;
            set_guid(&input_type, &MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            set_guid(&input_type, &MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
            set_u64(
                &input_type,
                &MF_MT_FRAME_SIZE,
                ratio(padded_width, padded_height),
            )?;
            set_u64(&input_type, &MF_MT_FRAME_RATE, ratio(30, 1))?;
            set_u64(&input_type, &MF_MT_PIXEL_ASPECT_RATIO, ratio(1, 1))?;
            set_u32(
                &input_type,
                &MF_MT_INTERLACE_MODE,
                MFVideoInterlace_Progressive.0 as u32,
            )?;
            set_u32(&input_type, &MF_MT_FIXED_SIZE_SAMPLES, 1)?;
            set_u32(
                &input_type,
                &MF_MT_DEFAULT_STRIDE,
                padded_width
                    .checked_mul(4)
                    .ok_or("Video row is too wide.")?,
            )?;

            let audio_output = if audio {
                Some(audio_type(&api, true)?)
            } else {
                None
            };
            let mut stream_raw = ptr::null_mut();
            hresult(
                (api.create_file)(
                    MF_ACCESSMODE_WRITE,
                    MF_OPENMODE_FAIL_IF_EXIST,
                    MF_FILEFLAGS_NONE,
                    PCWSTR(output.as_ptr()),
                    &mut stream_raw,
                ),
                "Cannot create the video file",
            )?;
            let stream = IMFByteStream::from_raw(stream_raw);
            let mut sink_raw = ptr::null_mut();
            hresult(
                (api.create_fmp4)(
                    stream.as_raw(),
                    output_type.as_raw(),
                    audio_output
                        .as_ref()
                        .map_or(ptr::null_mut(), Interface::as_raw),
                    &mut sink_raw,
                ),
                "Cannot create the MP4 container",
            )?;
            let sink = IMFMediaSink::from_raw(sink_raw);
            let audio_stream = if audio { Some(1) } else { None };
            let settings = api.attributes(1)?;
            set_u32(
                &settings,
                &MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS,
                hardware as u32,
            )?;
            let mut writer_raw = ptr::null_mut();
            hresult(
                (api.create_writer)(sink.as_raw(), settings.as_raw(), &mut writer_raw),
                "Cannot create the H.264 encoder",
            )?;
            let writer = IMFSinkWriter::from_raw(writer_raw);
            writer
                .SetInputMediaType(0, &input_type, None)
                .map_err(|error| format!("Cannot configure the H.264 encoder: {error}"))?;
            if let Some(index) = audio_stream {
                writer
                    .SetInputMediaType(index, &audio_type(&api, false)?, None)
                    .map_err(|error| format!("Cannot configure the AAC encoder: {error}"))?;
            }
            writer
                .BeginWriting()
                .map_err(|error| format!("Cannot start the H.264 encoder: {error}"))?;
            Ok(Self {
                api,
                writer: Some(writer),
                sink: Some(sink),
                stream: Some(stream),
                path: path.to_owned(),
                source_width: width,
                source_height: height,
                width: padded_width,
                bytes: padded_bytes,
                last_end: -1,
                audio_stream,
                audio_end: -1,
                finished: false,
            })
        }
    }

    pub fn write(&mut self, frame: &Frame, time: i64, duration: i64) -> Result<(), String> {
        if self.finished || time < 0 || duration <= 0 || time < self.last_end {
            return Err("Video frame timestamps must be positive and monotonic.".into());
        }
        if (frame.width, frame.height) != (self.source_width, self.source_height) {
            return Err("The recording source changed size.".into());
        }
        if frame.bgra.len() != Frame::byte_len(frame.width, frame.height)? {
            return Err("The video frame is incomplete.".into());
        }
        let writer = self.writer.as_ref().ok_or("The video encoder is closed.")?;
        unsafe {
            let mut buffer_raw = ptr::null_mut();
            hresult(
                (self.api.create_buffer)(self.bytes, &mut buffer_raw),
                "Cannot allocate a video frame",
            )?;
            let buffer = IMFMediaBuffer::from_raw(buffer_raw);
            let mut data = ptr::null_mut();
            let mut maximum = 0;
            buffer
                .Lock(&mut data, Some(&mut maximum), None)
                .map_err(|error| format!("Cannot access a video frame: {error}"))?;
            let copy_result: Result<(), String> = (|| {
                if maximum < self.bytes || data.is_null() {
                    return Err("Windows allocated an incomplete video frame.".into());
                }
                ptr::write_bytes(data, 0, self.bytes as usize);
                let source_stride = self.source_width as usize * 4;
                let target_stride = self.width as usize * 4;
                for y in 0..self.source_height as usize {
                    ptr::copy_nonoverlapping(
                        frame.bgra.as_ptr().add(y * source_stride),
                        data.add(y * target_stride),
                        source_stride,
                    );
                }
                Ok(())
            })();
            let unlock_result = buffer.Unlock().map_err(|error| error.to_string());
            copy_result?;
            unlock_result.map_err(|error| format!("Cannot release a video frame: {error}"))?;
            buffer
                .SetCurrentLength(self.bytes)
                .map_err(|error| format!("Cannot size a video frame: {error}"))?;
            let sample: IMFSample = self
                .api
                .object(self.api.create_sample, "Cannot create a video sample")?;
            sample
                .AddBuffer(&buffer)
                .map_err(|error| error.to_string())?;
            sample
                .SetSampleTime(time)
                .map_err(|error| error.to_string())?;
            sample
                .SetSampleDuration(duration)
                .map_err(|error| error.to_string())?;
            writer
                .WriteSample(0, &sample)
                .map_err(|error| format!("Cannot encode a video frame: {error}"))?;
        }
        self.last_end = time
            .checked_add(duration)
            .ok_or("Video timestamp overflowed.")?;
        Ok(())
    }

    pub fn write_audio(&mut self, first_frame: u64, samples: &[i16]) -> Result<(), String> {
        let stream = self
            .audio_stream
            .ok_or("This recording has no audio track.")?;
        if self.finished
            || samples.is_empty()
            || samples.len() % crate::audio::CHANNELS != 0
            || samples.len() / crate::audio::CHANNELS > crate::audio::CAPACITY_FRAMES
        {
            return Err("Invalid audio sample block.".into());
        }
        let end_frame = first_frame
            .checked_add((samples.len() / crate::audio::CHANNELS) as u64)
            .ok_or("Audio position overflowed.")?;
        let time = crate::audio::frame_time(first_frame)?;
        let end = crate::audio::frame_time(end_frame)?;
        if time < self.audio_end || end <= time {
            return Err("Audio timestamps must be monotonic.".into());
        }
        let writer = self.writer.as_ref().ok_or("The audio encoder is closed.")?;
        unsafe {
            let length = (samples.len() * 2) as u32;
            let mut raw = ptr::null_mut();
            hresult(
                (self.api.create_buffer)(length, &mut raw),
                "Cannot allocate an audio block",
            )?;
            let buffer = IMFMediaBuffer::from_raw(raw);
            let mut data = ptr::null_mut();
            let mut maximum = 0;
            buffer
                .Lock(&mut data, Some(&mut maximum), None)
                .map_err(|e| e.to_string())?;
            let valid = !data.is_null() && maximum >= length;
            if valid {
                ptr::copy_nonoverlapping(samples.as_ptr().cast::<u8>(), data, length as usize);
            }
            buffer.Unlock().map_err(|e| e.to_string())?;
            if !valid {
                return Err("Windows returned an incomplete audio buffer.".into());
            }
            buffer.SetCurrentLength(length).map_err(|e| e.to_string())?;
            let sample: IMFSample = self
                .api
                .object(self.api.create_sample, "Cannot create an audio sample")?;
            sample.AddBuffer(&buffer).map_err(|e| e.to_string())?;
            sample.SetSampleTime(time).map_err(|e| e.to_string())?;
            sample
                .SetSampleDuration(end - time)
                .map_err(|e| e.to_string())?;
            writer
                .WriteSample(stream, &sample)
                .map_err(|e| format!("Cannot encode audio: {e}"))?;
        }
        self.audio_end = end;
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), String> {
        if self.audio_stream.is_some() && self.audio_end < 0 {
            return Err("No audio samples were encoded.".into());
        }
        let result = unsafe {
            self.writer
                .as_ref()
                .ok_or("The video encoder is closed.")?
                .Finalize()
                .map_err(|error| format!("Cannot finalize the MP4 file: {error}"))?;
            self.writer.take();
            if let Some(sink) = self.sink.take() {
                sink.Shutdown()
                    .map_err(|error| format!("Cannot close the MP4 container: {error}"))?;
            }
            self.stream.take();
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.path)
                .and_then(|file| file.sync_all())
                .map_err(|error| format!("Cannot flush the MP4 file to disk: {error}"))?;
            Ok(())
        };
        self.finished = result.is_ok();
        result
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        self.writer.take();
        if let Some(sink) = self.sink.take() {
            let _ = unsafe { sink.Shutdown() };
        }
        self.stream.take();
    }
}

fn ratio(numerator: u32, denominator: u32) -> u64 {
    ((numerator as u64) << 32) | denominator as u64
}

unsafe fn audio_type(api: &Api, encoded: bool) -> Result<IMFMediaType, String> {
    use windows::Win32::Media::MediaFoundation::*;
    unsafe {
        let format = api.media_type()?;
        set_guid(&format, &MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
        set_guid(
            &format,
            &MF_MT_SUBTYPE,
            if encoded {
                &MFAudioFormat_AAC
            } else {
                &MFAudioFormat_PCM
            },
        )?;
        set_u32(
            &format,
            &MF_MT_AUDIO_NUM_CHANNELS,
            crate::audio::CHANNELS as u32,
        )?;
        set_u32(&format, &MF_MT_AUDIO_SAMPLES_PER_SECOND, crate::audio::RATE)?;
        set_u32(&format, &MF_MT_AUDIO_BITS_PER_SAMPLE, 16)?;
        set_u32(
            &format,
            &MF_MT_AUDIO_AVG_BYTES_PER_SECOND,
            if encoded {
                24_000
            } else {
                crate::audio::RATE * 4
            },
        )?;
        if encoded {
            set_u32(&format, &MF_MT_AAC_PAYLOAD_TYPE, 0)?;
            set_u32(&format, &MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION, 0x29)?;
            set_u32(&format, &MF_MT_AUDIO_BLOCK_ALIGNMENT, 1)?;
        } else {
            set_u32(&format, &MF_MT_AUDIO_BLOCK_ALIGNMENT, 4)?;
            set_u32(&format, &MF_MT_ALL_SAMPLES_INDEPENDENT, 1)?;
        }
        Ok(format)
    }
}

#[cfg(test)]
fn decode(path: &Path) -> Result<Vec<(i64, Frame)>, String> {
    use windows::Win32::Media::MediaFoundation::{
        IMFSourceReader, MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING,
        MF_SOURCE_READER_FIRST_VIDEO_STREAM, MF_SOURCE_READERF_ENDOFSTREAM,
    };
    let api = Api::load()?;
    let input = wide(path)?;
    unsafe {
        let mut reader_raw = ptr::null_mut();
        let settings = api.attributes(1)?;
        set_u32(
            &settings,
            &MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING,
            1,
        )?;
        hresult(
            (api.create_reader)(PCWSTR(input.as_ptr()), settings.as_raw(), &mut reader_raw),
            "Cannot reopen the MP4 file",
        )?;
        let reader = IMFSourceReader::from_raw(reader_raw);
        let output_type = api.media_type()?;
        set_guid(&output_type, &MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        set_guid(&output_type, &MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
        let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
        reader
            .SetCurrentMediaType(stream, None, &output_type)
            .map_err(|error| format!("Cannot decode MP4 pixels: {error}"))?;
        let current = reader
            .GetCurrentMediaType(stream)
            .map_err(|error| error.to_string())?;
        let packed = current
            .GetUINT64(&MF_MT_FRAME_SIZE)
            .map_err(|error| error.to_string())?;
        let width = (packed >> 32) as u32;
        let height = packed as u32;
        let mut frames = Vec::new();
        loop {
            let mut flags = 0;
            let mut timestamp = 0;
            let mut sample = None;
            reader
                .ReadSample(
                    stream,
                    0,
                    None,
                    Some(&mut flags),
                    Some(&mut timestamp),
                    Some(&mut sample),
                )
                .map_err(|error| format!("Cannot read the MP4 file: {error}"))?;
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                break;
            }
            let Some(sample) = sample else { continue };
            let buffer = sample
                .ConvertToContiguousBuffer()
                .map_err(|error| error.to_string())?;
            frames.push((timestamp, copy_decoded(&buffer, width, height)?));
        }
        Ok(frames)
    }
}

#[cfg(test)]
unsafe fn copy_decoded(buffer: &IMFMediaBuffer, width: u32, height: u32) -> Result<Frame, String> {
    let row = width as usize * 4;
    let mut bytes = vec![0; Frame::byte_len(width, height)?];
    if let Ok(two_d) = buffer.cast::<IMF2DBuffer>() {
        let mut scanline = ptr::null_mut();
        let mut pitch = 0;
        unsafe { two_d.Lock2D(&mut scanline, &mut pitch) }.map_err(|error| error.to_string())?;
        let result: Result<(), String> = (|| {
            if scanline.is_null() || (pitch.unsigned_abs() as usize) < row {
                return Err("Decoded MP4 row is incomplete.".into());
            }
            for y in 0..height as usize {
                let source = unsafe { scanline.offset(y as isize * pitch as isize) };
                unsafe { ptr::copy_nonoverlapping(source, bytes.as_mut_ptr().add(y * row), row) };
            }
            Ok(())
        })();
        let unlock = unsafe { two_d.Unlock2D() }.map_err(|error| error.to_string());
        result?;
        unlock?;
    } else {
        let mut data = ptr::null_mut();
        let mut current = 0;
        unsafe { buffer.Lock(&mut data, None, Some(&mut current)) }
            .map_err(|error| error.to_string())?;
        let result: Result<(), String> = if data.is_null() || (current as usize) < bytes.len() {
            Err("Decoded MP4 frame is incomplete.".into())
        } else {
            unsafe { ptr::copy_nonoverlapping(data, bytes.as_mut_ptr(), bytes.len()) };
            Ok(())
        };
        let unlock = unsafe { buffer.Unlock() }.map_err(|error| error.to_string());
        result?;
        unlock?;
    }
    Frame::new(width, height, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn decoded_audio(path: &Path) -> Result<(u32, u32, Vec<(i64, Vec<i16>)>), String> {
        use windows::Win32::Media::MediaFoundation::*;
        let api = Api::load()?;
        let filename = wide(path)?;
        unsafe {
            let mut raw = ptr::null_mut();
            hresult(
                (api.create_reader)(PCWSTR(filename.as_ptr()), ptr::null_mut(), &mut raw),
                "Cannot decode audio",
            )?;
            let reader = IMFSourceReader::from_raw(raw);
            let format = api.media_type()?;
            set_guid(&format, &MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
            set_guid(&format, &MF_MT_SUBTYPE, &MFAudioFormat_PCM)?;
            set_u32(&format, &MF_MT_AUDIO_BITS_PER_SAMPLE, 16)?;
            let stream = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;
            reader
                .SetCurrentMediaType(stream, None, &format)
                .map_err(|e| e.to_string())?;
            let current = reader
                .GetCurrentMediaType(stream)
                .map_err(|e| e.to_string())?;
            let rate = current
                .GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND)
                .map_err(|e| e.to_string())?;
            let channels = current
                .GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS)
                .map_err(|e| e.to_string())?;
            let mut blocks = Vec::new();
            loop {
                let mut flags = 0;
                let mut time = 0;
                let mut sample = None;
                reader
                    .ReadSample(
                        stream,
                        0,
                        None,
                        Some(&mut flags),
                        Some(&mut time),
                        Some(&mut sample),
                    )
                    .map_err(|e| e.to_string())?;
                if let Some(sample) = sample {
                    let buffer = sample
                        .ConvertToContiguousBuffer()
                        .map_err(|e| e.to_string())?;
                    let mut data = ptr::null_mut();
                    let mut length = 0;
                    buffer
                        .Lock(&mut data, None, Some(&mut length))
                        .map_err(|e| e.to_string())?;
                    let values = if data.is_null() || length % 2 != 0 {
                        Err("Invalid decoded audio buffer".to_string())
                    } else {
                        Ok(std::slice::from_raw_parts(data, length as usize)
                            .chunks_exact(2)
                            .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
                            .collect::<Vec<_>>())
                    };
                    buffer.Unlock().map_err(|e| e.to_string())?;
                    blocks.push((time, values?));
                    if blocks.len() > 1000 {
                        return Err("Audio decode exceeded the fixture budget".into());
                    }
                }
                if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                    break;
                }
            }
            Ok((rate, channels, blocks))
        }
    }

    #[test]
    fn audio_video_mp4_reopens_with_real_stereo_tone_and_matching_duration() {
        let directory =
            std::env::temp_dir().join(format!("snapzy-audio-codec-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("synthetic-audio.mp4");
        let mut encoder = Encoder::new(&path, 64, 48, false, true).unwrap();
        for i in 0..30u64 {
            let frame =
                Frame::new(64, 48, vec![if i < 15 { 70 } else { 190 }; 64 * 48 * 4]).unwrap();
            encoder.write(&frame, i as i64 * 333_333, 333_333).unwrap();
            let first = i * 1600;
            let tone: Vec<i16> = (first..first + 1600)
                .flat_map(|sample| {
                    let phase = sample as f64 / 48_000.0 * std::f64::consts::TAU;
                    [
                        (phase * 440.0).sin() * 8000.0,
                        (phase * 880.0).sin() * 4000.0,
                    ]
                    .map(|n| n as i16)
                })
                .collect();
            encoder.write_audio(first, &tone).unwrap();
            if i == 2 {
                assert!(encoder.write_audio(0, &[1, 2]).is_err());
                assert!(encoder.write_audio(first + 1600, &[1]).is_err());
            }
        }
        encoder.finish().unwrap();
        assert_eq!(decode(&path).unwrap().len(), 30);
        let (rate, channels, blocks) = decoded_audio(&path).unwrap();
        assert_eq!((rate, channels), (48_000, 2));
        assert!(!blocks.is_empty());
        assert!(blocks.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(blocks[0].0.abs() <= 250_000);
        let samples: Vec<i16> = blocks
            .iter()
            .flat_map(|(_, values)| values.iter().copied())
            .collect();
        assert!((47_000..=51_000).contains(&(samples.len() / 2)));
        let end = blocks.last().unwrap().0
            + (blocks.last().unwrap().1.len() as i64 / 2) * 10_000_000 / 48_000;
        assert!((9_500_000..=10_500_000).contains(&end), "audio end {end}");
        let energy = |channel: usize| {
            samples
                .iter()
                .skip(channel)
                .step_by(2)
                .map(|sample| (*sample as f64).powi(2))
                .sum::<f64>()
                / (samples.len() / 2) as f64
        };
        assert!(energy(0) > 4_000_000.0 && energy(1) > 1_000_000.0);
        assert!(energy(0) > energy(1) * 2.0);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn fragmented_mp4_accepts_delayed_audio_without_stalling_video_writes() {
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                let directory = std::env::temp_dir()
                    .join(format!("snapzy-audio-latency-{}", std::process::id()));
                std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                let path = directory.join("delayed-audio.mp4");
                let mut encoder = Encoder::new(&path, 64, 48, false, true)?;
                let frame = Frame::new(64, 48, vec![100; 64 * 48 * 4])?;
                for i in 0..6 {
                    encoder.write(&frame, i * 333_333, 333_333)?;
                }
                for i in 0..20 {
                    encoder.write_audio(i * 480, &vec![1000; 960])?;
                }
                encoder.finish()?;
                assert_eq!(decode(&path)?.len(), 6);
                let (_, _, blocks) = decoded_audio(&path)?;
                assert!(blocks.iter().map(|(_, pcm)| pcm.len()).sum::<usize>() >= 18_000);
                std::fs::remove_dir_all(directory).map_err(|e| e.to_string())?;
                Ok(())
            })();
            let _ = sender.send(result);
        });
        receiver
            .recv_timeout(std::time::Duration::from_secs(20))
            .expect("Video writes stalled while waiting for the delayed audio track")
            .unwrap();
    }
    #[test]
    fn silent_mp4_reopens_with_changing_pixels_correct_orientation_and_timestamps() {
        let directory =
            std::env::temp_dir().join(format!("snapzy-video-codec-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("synthetic.mp4");
        let mut encoder = Encoder::new(&path, 321, 181, false, false).unwrap();
        assert!(encoder.write_audio(0, &[1, 2]).is_err());
        for i in 0..30 {
            let mut pixels = vec![0u8; 321 * 181 * 4];
            for y in 0..181 {
                for x in 0..321 {
                    let color = if y < 90 {
                        if i < 15 {
                            [0, 0, 255, 255]
                        } else {
                            [0, 255, 0, 255]
                        }
                    } else {
                        [255, 0, 0, 255]
                    };
                    pixels[(y * 321 + x) * 4..(y * 321 + x) * 4 + 4].copy_from_slice(&color);
                }
            }
            let frame = Frame::new(321, 181, pixels).unwrap();
            encoder.write(&frame, i * 333_333, 333_333).unwrap();
            if i == 2 {
                assert!(encoder.write(&frame, 0, 333_333).is_err());
            }
        }
        encoder.finish().unwrap();
        let decoded = decode(&path).unwrap();
        assert!(decoded_audio(&path).is_err());
        assert_eq!(decoded.len(), 30);
        assert_eq!((decoded[0].1.width, decoded[0].1.height), (322, 182));
        assert_eq!(decoded[0].0, 0);
        assert!(decoded.last().unwrap().0 >= 9_000_000);
        let pixel = |frame: &Frame, x: usize, y: usize| -> [u8; 3] {
            frame.bgra[(y * frame.width as usize + x) * 4..(y * frame.width as usize + x) * 4 + 3]
                .try_into()
                .unwrap()
        };
        let first = &decoded[0].1;
        let last = &decoded.last().unwrap().1;
        assert!(
            pixel(first, 20, 20)[2] > 180,
            "top red: {:?}",
            pixel(first, 20, 20)
        );
        assert!(
            pixel(first, 20, 140)[0] > 180,
            "bottom blue: {:?}",
            pixel(first, 20, 140)
        );
        assert!(
            pixel(last, 20, 20)[1] > 180,
            "last top green: {:?}",
            pixel(last, 20, 20)
        );
        let bytes = std::fs::read(&path).unwrap();
        let mut cursor = 0;
        let mut fragments = 0;
        while cursor + 8 <= bytes.len() {
            let size = u32::from_be_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            assert!(size >= 8 && cursor + size <= bytes.len());
            if &bytes[cursor + 4..cursor + 8] == b"moof" {
                fragments += 1;
            }
            cursor += size;
        }
        assert!(fragments > 0, "not fragmented MP4");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
